use std::{fs, time::Duration};

use anyhow::{Context, Result};
use cargo_metadata::{Metadata, MetadataCommand, Package};
use crates_io_api::SyncClient;

use crate::{
    cargo_command::CargoCommand,
    config::{ReleaseConfig, ReleaseSet},
    Options,
};

pub struct Publish {
    release_set: ReleaseSet,
    metadata: Metadata,
    cargo: CargoCommand,
    client: SyncClient,
    dry_run: bool,
}

impl Publish {
    pub fn new(options: Options) -> Result<Self> {
        let cwd = options.path;

        super::check_git_clean(&cwd)?;

        let release_set = ReleaseConfig::new(&cwd)?.get_release(&options.release)?;

        let metadata = MetadataCommand::new().current_dir(&cwd).no_deps().exec()?;
        let cargo = CargoCommand::new(metadata.workspace_root.clone().into_std_path_buf());
        let client =
            SyncClient::new("Boshen@users.noreply.github.com", Duration::from_millis(1000))
                .context("failed to get client")?;
        Ok(Self { release_set, metadata, cargo, client, dry_run: options.dry_run })
    }

    pub fn run(self) -> Result<()> {
        let packages = self.get_packages();

        let Some(root_package) = &packages.iter().find(|package| package.name == "oxc") else {
            anyhow::bail!("root package 'oxc' not found.");
        };

        let root_version = root_package.version.to_string();

        let packages = release_order::release_order(&packages)?;
        let packages = packages.into_iter().map(|package| &package.name).collect::<Vec<_>>();

        eprintln!("Publishing packages: {packages:?}");
        for package in &packages {
            if self.dry_run {
                // check each crate individually to prevent feature unification.
                self.cargo.check(package)?;
            }
            if self.skip_published(package, &root_version) {
                continue;
            }
            self.cargo.publish(package, self.dry_run)?;
        }
        eprintln!("Published packages: {packages:?}");

        let version = format!("{}_v{root_version}", self.release_set.name);
        fs::write("./target/OXC_VERSION", version)?;

        Ok(())
    }

    fn skip_published(&self, package: &str, root_version: &str) -> bool {
        let Ok(krate) = self.client.get_crate(package) else {
            eprintln!("Cannot get {package}");
            return false;
        };
        let versions = krate.versions.into_iter().map(|version| version.num).collect::<Vec<_>>();
        let is_already_published = versions.iter().any(|v| v == root_version);
        if is_already_published {
            eprintln!("Already published {package} {root_version}");
        }
        is_already_published
    }

    fn get_packages(&self) -> Vec<&Package> {
        // `publish.is_none()` means `publish = true`.
        self.metadata.workspace_packages().into_iter().filter(|p| p.publish.is_none()).collect()
    }
}

mod release_order {
    use anyhow::Result;
    use cargo_metadata::Package;

    /// Return packages in an order they can be released.
    /// In the result, the packages are placed after all their dependencies.
    /// Return an error if a circular dependency is detected.
    pub fn release_order<'a>(packages: &'a [&Package]) -> Result<Vec<&'a Package>> {
        let mut order = vec![];
        let mut passed = vec![];
        for p in packages {
            release_order_inner(packages, p, &mut order, &mut passed)?;
        }
        Ok(order)
    }

    /// The `passed` argument is used to track packages that you already visited to
    /// detect circular dependencies.
    fn release_order_inner<'a>(
        packages: &[&'a Package],
        pkg: &'a Package,
        order: &mut Vec<&'a Package>,
        passed: &mut Vec<&'a Package>,
    ) -> Result<()> {
        if is_package_in(pkg, order) {
            return Ok(());
        }
        passed.push(pkg);

        for d in &pkg.dependencies {
            // Check if the dependency is part of the packages we are releasing.
            if let Some(dep) = packages.iter().find(|p| {
                d.name == p.name
              // Exclude the current package.
              && p.name != pkg.name
            }) {
                anyhow::ensure!(
                    !is_package_in(dep, passed),
                    "Circular dependency detected: {} -> {}",
                    dep.name,
                    pkg.name,
                );
                release_order_inner(packages, dep, order, passed)?;
            }
        }

        order.push(pkg);
        passed.clear();
        Ok(())
    }

    /// Return true if the package is part of a packages array.
    /// This function exists because `package.contains(pkg)` is expensive,
    /// because it compares the whole package struct.
    fn is_package_in(pkg: &Package, packages: &[&Package]) -> bool {
        packages.iter().any(|p| p.name == pkg.name)
    }
}
