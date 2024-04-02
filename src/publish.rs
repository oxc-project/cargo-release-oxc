use std::{
    env,
    io::{BufRead, BufReader},
    path::{Path, PathBuf},
    process::{Command, ExitStatus, Stdio},
};

use anyhow::{Context, Result};
use bpaf::Bpaf;
use cargo_metadata::{Metadata, MetadataCommand, Package};

const CARGO_REGISTRY_TOKEN: &str = "CARGO_REGISTRY_TOKEN";

#[derive(Debug, Clone, Bpaf)]
pub struct PublishOptions {
    #[bpaf(positional("PATH"), fallback(PathBuf::from(".")))]
    path: PathBuf,

    /// Upload to crates.io.
    #[bpaf(switch)]
    upload: bool,
}

#[allow(unused)]
pub struct Publish {
    options: PublishOptions,
    metadata: Metadata,
}

impl Publish {
    pub fn new(options: PublishOptions) -> Result<Self> {
        let metadata = MetadataCommand::new().current_dir(&options.path).no_deps().exec()?;
        Ok(Self { options, metadata })
    }

    pub fn run(self) -> Result<()> {
        let packages = self.get_packages();
        let packages = release_order::release_order(&packages)?;
        let packages = packages.into_iter().map(|package| &package.name).collect::<Vec<_>>();

        println!("Publishing packages: {:?}", packages);

        // check with --dry-run first to make sure all crates compile
        for package in &packages {
            self.run_cargo_publish(package, /* dry_run */ true)?;
        }

        // then publish
        if self.options.upload {
            for package in &packages {
                self.run_cargo_publish(package, /* dry_run */ false)?;
            }
        } else {
            println!("Publish is turned off by default, please use `--upload` to crates.io.");
        }

        println!("Published packages: {:?}", packages);
        Ok(())
    }

    fn get_packages(&self) -> Vec<&Package> {
        // `publish.is_none()` means `publish = true`.
        self.metadata.workspace_packages().into_iter().filter(|p| p.publish.is_none()).collect()
    }

    fn run_cargo_publish(&self, name: &str, dry_run: bool) -> Result<()> {
        let mut args = vec!["--color", "always", "publish", "-p", name];
        if dry_run {
            args.push("--dry-run")
        }
        let output = Self::run_cargo(self.metadata.workspace_root.as_std_path(), &args)?;
        if !output.status.success()
            || !output.stderr.contains("Uploading")
            || output.stderr.contains("error:")
        {
            anyhow::bail!("failed to publish {}: {}", name, output.stderr);
        }
        Ok(())
    }

    fn run_cargo(root: &Path, args: &[&str]) -> Result<CmdOutput> {
        fn cargo_cmd() -> Command {
            let cargo = env::var("CARGO").unwrap_or_else(|_| "cargo".to_owned());
            Command::new(cargo)
        }

        let mut stderr_lines = vec![];

        let mut command = cargo_cmd();
        if let Ok(token) = env::var(CARGO_REGISTRY_TOKEN) {
            command.env(CARGO_REGISTRY_TOKEN, token);
        }
        let mut child = command
            .current_dir(root)
            .args(args)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .context("cannot run cargo")?;

        {
            let stderr = child.stderr.as_mut().expect("cannot get child stderr");

            for line in BufReader::new(stderr).lines() {
                let line = line?;

                eprintln!("{line}");
                stderr_lines.push(line);
            }
        }

        let output = child.wait_with_output()?;

        let output_stdout = String::from_utf8(output.stdout)?;
        let output_stderr = stderr_lines.join("\n");

        Ok(CmdOutput { status: output.status, stdout: output_stdout, stderr: output_stderr })
    }
}

pub struct CmdOutput {
    pub status: ExitStatus,
    pub stdout: String,
    pub stderr: String,
}

mod release_order {
    use anyhow::Result;
    use cargo_metadata::{Dependency, DependencyKind, Package};

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
              && should_dep_be_released_before(d, pkg)
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

    /// Check if the dependency is enabled in features.
    fn is_dep_in_features(pkg: &Package, dep: &str) -> bool {
        pkg.features
            // Discard features name.
            .values()
            // Any feature contains the dependency in the format `dep/feature`.
            .any(|enabled_features| {
                enabled_features
                    .iter()
                    .filter_map(|feature| feature.split_once('/').map(|split| split.0))
                    .any(|enabled_dependency| enabled_dependency == dep)
            })
    }

    /// Check if the dependency should be released before the current package.
    fn should_dep_be_released_before(dep: &Dependency, pkg: &Package) -> bool {
        // Ignore development dependencies. They don't need to be published before the current package...
        matches!(dep.kind, DependencyKind::Normal | DependencyKind::Build)
      // ...unless they are in features. In fact, `cargo-publish` compiles crates that are in features
      // and dev-dependencies, even if they are not present in normal dependencies.
      || is_dep_in_features(pkg, &dep.name)
    }
}
