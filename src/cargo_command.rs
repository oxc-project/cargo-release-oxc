use std::{
    env,
    io::{BufRead, BufReader},
    path::PathBuf,
    process::{Command, ExitStatus, Stdio},
};

use anyhow::{Context, Result};

const CARGO_REGISTRY_TOKEN: &str = "CARGO_REGISTRY_TOKEN";

pub struct CmdOutput {
    pub status: ExitStatus,
    // pub stdout: String,
    pub stderr: String,
}

pub struct CargoCommand {
    current_dir: PathBuf,
}

impl CargoCommand {
    pub const fn new(current_dir: PathBuf) -> Self {
        Self { current_dir }
    }

    pub fn publish(&self, package_name: &str, dry_run: bool) -> Result<()> {
        let mut args = vec!["--color", "always", "publish", "-p", package_name];
        if dry_run {
            args.push("--dry-run");
        }
        let output = self.run(&args)?;
        if !output.status.success()
            || !output.stderr.contains("Uploading")
            || output.stderr.contains("error:")
        {
            anyhow::bail!("failed to publish {}: {}", package_name, output.stderr);
        }
        Ok(())
    }

    pub fn run(&self, args: &[&str]) -> Result<CmdOutput> {
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
            .current_dir(&self.current_dir)
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

        // let output_stdout = String::from_utf8(output.stdout)?;
        let output_stderr = stderr_lines.join("\n");

        Ok(CmdOutput { status: output.status, stderr: output_stderr })
    }
}
