# Release management for the oxc project

## `cargo release-oxc`

```
Usage: cargo-release-oxc COMMAND ...

Available options:
    -h, --help             Prints help information

Available commands:
    update                 Generate CHANGELOG.md and bump versions for all published packages.
    changelog              Generate changelog summary.
    regenerate-changelogs  Regenerate CHANGELOG.md for all published packages.
    publish                Publish all `versioned_files` specified in `oxc_release.toml`.

Available options:
    --release=NAME         Select the release specified in `oxc_release.toml`.
    --dry-run              Run `cargo publish` with `--dry-run`
```

## Specify `oxc_release.toml`

```toml
[[releases]]
name = "crates"
versioned_files = [
  "Cargo.toml",
  "npm/oxc-parser/package.json",
  "npm/oxc-transform/package.json",
  "wasm/parser/package.json",
]

[[releases]]
name = "oxlint"
versioned_files = [
  "apps/oxlint/Cargo.toml",
  "crates/oxc_linter/Cargo.toml",
  "editors/vscode/package.json",
  "npm/oxlint/package.json",
]
```

## Output

Saves two files to `./target`:

* version: `./target/OXC_VERSION`
* changelog: `./target/OXC_CHANGELOG`
