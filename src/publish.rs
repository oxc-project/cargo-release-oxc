use std::{
    fs,
    path::{Path, PathBuf},
    time::Duration,
};

use anyhow::{Context, Result};
use cargo_metadata::{DependencyKind, Metadata, MetadataCommand, Package};
use crates_io_api::SyncClient;

use crate::{
    cargo_command::CargoCommand,
    config::{ReleaseConfig, ReleaseSet},
};

pub struct Publish {
    release_set: ReleaseSet,
    metadata: Metadata,
    cargo: CargoCommand,
    client: SyncClient,
    dry_run: bool,
}

impl Publish {
    pub fn new(cwd: &PathBuf, release_name: &str, dry_run: bool) -> Result<Self> {
        let release_set = ReleaseConfig::new(cwd)?.get_release(release_name)?;

        let metadata = MetadataCommand::new().current_dir(cwd).no_deps().exec()?;
        let cargo = CargoCommand::new(metadata.workspace_root.clone().into_std_path_buf());
        let client = SyncClient::new("Boshen@users.noreply.github.com", Duration::from_secs(1))
            .context("failed to get client")?;
        Ok(Self { release_set, metadata, cargo, client, dry_run })
    }

    pub fn run(self) -> Result<()> {
        let packages = self.get_packages();

        let Some(root_crate) = &self.release_set.root_crate else {
            anyhow::bail!("root_crate must be specified in the [[releases]] config");
        };

        let Some(root_package) =
            &packages.iter().find(|package| package.name.as_str() == root_crate)
        else {
            anyhow::bail!("root package '{root_crate}' not found.");
        };

        let root_version = root_package.version.to_string();

        validate_packages(&packages)?;

        let packages = release_order::release_order(&packages)?;
        let packages: Vec<&str> =
            packages.into_iter().map(|package| package.name.as_str()).collect();

        let total = packages.len();
        eprintln!("Publishing {total} package(s):");
        for package in &packages {
            eprintln!("  - {package}");
        }

        let mut published: Vec<&str> = vec![];
        let mut skipped: Vec<&str> = vec![];
        for (idx, package) in packages.iter().enumerate() {
            eprintln!("\n[{}/{total}] {package}", idx + 1);
            let summary = || publish_failure_summary(package, idx, &packages, &published, &skipped);
            if self.dry_run {
                // check each crate individually to prevent feature unification.
                self.cargo.check(package).with_context(summary)?;
            }
            if self.skip_published(package, &root_version)? {
                skipped.push(package);
                continue;
            }
            self.cargo.publish(package, self.dry_run).with_context(summary)?;
            published.push(package);
            eprintln!("  ✓ published");
        }

        eprintln!(
            "\nDone. {} published, {} skipped (already on registry), {total} total.",
            published.len(),
            skipped.len(),
        );

        let release_name = &self.release_set.name;
        let version = format!("{release_name}_v{root_version}");
        let var = format!("{}_VERSION", release_name.to_uppercase());
        let file = Path::new("./target").join(var);
        fs::write(file, version)?;
        Ok(())
    }

    fn skip_published(&self, package: &str, root_version: &str) -> Result<bool> {
        match self.client.get_crate(package) {
            Ok(krate) => {
                let is_already_published =
                    krate.versions.iter().any(|version| version.num == root_version);
                if is_already_published {
                    eprintln!("  · already on crates.io @ {root_version}, skipping");
                }
                Ok(is_already_published)
            }
            Err(crates_io_api::Error::NotFound(_)) => {
                // Brand-new crate, never published; not an error.
                eprintln!("  · not yet on crates.io (new crate)");
                Ok(false)
            }
            Err(err) => Err(err).with_context(|| {
                format!(
                    "failed to query crates.io for `{package}` — \
                     cannot determine whether to publish or skip"
                )
            }),
        }
    }

    fn get_packages(&self) -> Vec<&Package> {
        // `publish.is_none()` means `publish = true`.
        self.metadata.workspace_packages().into_iter().filter(|p| p.publish.is_none()).collect()
    }
}

/// Catch metadata issues that would fail at upload time before we burn
/// minutes of publish-loop work to discover them one crate at a time.
fn validate_packages(packages: &[&Package]) -> Result<()> {
    let mut errors: Vec<String> = vec![];
    for pkg in packages {
        if pkg.description.as_deref().is_none_or(str::is_empty) {
            errors.push(format!(
                "`{}`: missing `description` field — crates.io requires it",
                pkg.name,
            ));
        }
        if pkg.license.is_none() && pkg.license_file.is_none() {
            errors.push(format!(
                "`{}`: missing `license` or `license-file` field — crates.io requires one",
                pkg.name,
            ));
        }
        for dep in &pkg.dependencies {
            if dep.kind == DependencyKind::Development {
                // Dev-deps are stripped at publish time; missing version is fine.
                continue;
            }
            // `source` is `None` for path-only deps with no registry. A `req` of
            // `*` means no version requirement was given, so cargo would refuse
            // to verify the manifest at upload time.
            if dep.source.is_none() && dep.req.to_string() == "*" {
                errors.push(format!(
                    "`{}`: dependency `{}` is path-only without a version requirement \
                     — cargo refuses to publish (use a `version` or route through \
                     `[workspace.dependencies]`)",
                    pkg.name, dep.name,
                ));
            }
        }
    }
    if errors.is_empty() {
        return Ok(());
    }
    anyhow::bail!(
        "Pre-flight validation found {} issue(s):\n  - {}",
        errors.len(),
        errors.join("\n  - "),
    );
}

fn publish_failure_summary(
    failed: &str,
    failed_idx: usize,
    all: &[&str],
    published: &[&str],
    skipped: &[&str],
) -> String {
    use std::fmt::Write;
    let remaining = &all[failed_idx + 1..];
    let mut out =
        format!("publish failed at `{failed}` ({}/{} in order)", failed_idx + 1, all.len());
    if !published.is_empty() {
        write!(out, "\n  published before failure: {published:?}").unwrap();
    }
    if !skipped.is_empty() {
        write!(out, "\n  skipped (already on registry): {skipped:?}").unwrap();
    }
    if !remaining.is_empty() {
        write!(out, "\n  not attempted: {remaining:?}").unwrap();
    }
    out
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
                d.name == p.name.as_str()
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
