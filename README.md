# Release management for the oxc project

## `cargo release-oxc`

```
Usage: cargo-release-oxc COMMAND ...

Available options:
    -h, --help             Prints help information

Available commands:
    update                 Bump versions for all published packages. Pass --changelog to also write
                           CHANGELOG.md.
    changelog              Generate changelog summary.
    regenerate-changelogs  Regenerate CHANGELOG.md for all published packages.
    publish                Publish all `versioned_files` specified in `oxc_release.toml`.
```

Every command takes the same options:

```
Usage: cargo-release-oxc update [--release=NAME]... [--dry-run] [--changelog] [--version=VERSION] [PATH]

Available positional items:
    PATH                   Optional path to directory, defaults to current working directory.

Available options:
        --release=NAME     Select the release specified in `oxc_release.toml`.
        --dry-run          Run `cargo publish` with `--dry-run`
        --changelog        For `update`: also write CHANGELOG.md files alongside the version bump.
        --version=VERSION  For `update`: skip conventional-commit version detection and bump to this
                           version.
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

### Changelogs

`update --changelog` writes a `CHANGELOG.md` next to every versioned package,
with the commits that touched the package directory.

```toml
[[releases]]
name = "crates"
versioned_files = ["Cargo.toml", "napi/parser/package.json"]

# Also write the changes of the whole release to one combined changelog.
# The path is exposed to the `cliff.toml` body template as `extra.changelog`,
# so package changelogs can link to it.
changelog = "crates/CHANGELOG.md"

# Include commits from other paths in a package's changelog,
# for packages that are thin wrappers around other packages.
[releases.changelog_includes]
"napi/parser" = ["crates/oxc_parser", "crates/oxc_ast"]
```

In the combined changelog, everything above the `<!-- cargo-release-oxc: releases -->` marker
is regenerated on every release.

A package gets an entry for every version, even when no commit touched it;
the body template decides what an entry without commits looks like.

## Output

Saves two files to `./target`:

* version: `./target/${name}_VERSION`
* changelog: `./target/${name}_CHANGELOG`

# [Sponsored By](https://oxc.rs/sponsor)

<p align="center">
  <a href="https://oxc.rs/sponsor">
    <img src="https://raw.githubusercontent.com/oxc-project/sponsors/main/sponsors.svg" alt="Our sponsors" />
  </a>
</p>
