# Changelog
All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.0.28](https://github.com/oxc-project/cargo-release-oxc/compare/v0.0.27...v0.0.28) - 2025-03-02

### Other

- *(deps)* update marcoieni/release-plz-action digest to 7049379 ([#71](https://github.com/oxc-project/cargo-release-oxc/pull/71))
- *(deps)* lock file maintenance ([#69](https://github.com/oxc-project/cargo-release-oxc/pull/69))
- *(deps)* update github-actions ([#68](https://github.com/oxc-project/cargo-release-oxc/pull/68))

## [0.0.27](https://github.com/oxc-project/cargo-release-oxc/compare/v0.0.26...v0.0.27) - 2025-02-22

### Other

- Rust Edition 2024
- *(deps)* update dependency rust to v1.85.0 (#66)
- *(deps)* pin dependencies (#65)
- pinGitHubActionDigestsToSemver
- use macos-13

## [0.0.26](https://github.com/oxc-project/cargo-release-oxc/compare/v0.0.25...v0.0.26) - 2025-02-09

### Other

- *(deps)* update rust crate toml to 0.8.20 (#63)
- *(deps)* update rust crates (#62)
- *(deps)* update dependency rust to v1.84.1 (#61)
- *(deps)* update rust crates (#60)
- *(deps)* update rust crate serde_json to 1.0.137 (#59)
- *(deps)* update rust crate serde_json to 1.0.136 (#58)
- *(deps)* update rust crate serde_json to 1.0.135 (#57)
- *(deps)* update dependency rust to v1.84.0 (#56)
- *(deps)* update rust crates
- *(deps)* update rust crates
- *(deps)* update rust crate serde to 1.0.216
- *(deps)* update rust crates
- *(deps)* update dependency rust to v1.83.0 (#55)
- *(deps)* update rust crates
- *(deps)* update rust crates
- *(deps)* update rust crate anyhow to 1.0.93

## [0.0.25](https://github.com/oxc-project/cargo-release-oxc/compare/v0.0.24...v0.0.25) - 2024-11-04

### Other

- add newline to end of package.json
- *(deps)* update rust crates
- *(deps)* update rust crates
- *(deps)* update rust crates
- *(deps)* update dependency rust to v1.82.0 ([#52](https://github.com/oxc-project/cargo-release-oxc/pull/52))

## [0.0.24](https://github.com/oxc-project/cargo-release-oxc/compare/v0.0.23...v0.0.24) - 2024-10-09

### Other

- *(deps)* update rust crates ([#51](https://github.com/oxc-project/cargo-release-oxc/pull/51))
- *(deps)* update rust crates ([#50](https://github.com/oxc-project/cargo-release-oxc/pull/50))
- *(renovate)* bump
- *(deps)* update dependency rust to v1.81.0 ([#48](https://github.com/oxc-project/cargo-release-oxc/pull/48))

## [0.0.23](https://github.com/oxc-project/cargo-release-oxc/compare/v0.0.22...v0.0.23) - 2024-08-29

### Added
- only matching scopes can participate in braking change detection.

### Other
- bump rust
- bump deps
- *(deps)* update rust crates
- *(deps)* update rust crates
- trigger release-binaries manually

## [0.0.22](https://github.com/oxc-project/cargo-release-oxc/compare/v0.0.21...v0.0.22) - 2024-08-07

### Other
- check before publish

## [0.0.21](https://github.com/oxc-project/cargo-release-oxc/compare/v0.0.20...v0.0.21) - 2024-07-27

### Other
- macos-12

## [0.0.20](https://github.com/oxc-project/cargo-release-oxc/compare/v0.0.19...v0.0.20) - 2024-07-25

### Other
- *(deps)* update dependency rust to v1.80.0 ([#42](https://github.com/oxc-project/cargo-release-oxc/pull/42))
- *(deps)* update rust crates
- *(deps)* update rust crate toml_edit to v0.22.15
- *(deps)* update rust crates
- *(deps)* update rust crates ([#40](https://github.com/oxc-project/cargo-release-oxc/pull/40))

## [0.0.19](https://github.com/oxc-project/cargo-release-oxc/compare/v0.0.18...v0.0.19) - 2024-06-27

### Fixed
- Need to publish if it's a new package

### Other
- update help and README

## [0.0.18](https://github.com/oxc-project/cargo-release-oxc/compare/v0.0.17...v0.0.18) - 2024-06-19

### Other
- skip not found packages

## [0.0.17](https://github.com/oxc-project/cargo-release-oxc/compare/v0.0.16...v0.0.17) - 2024-06-14

### Other
- remove cargo check from publish
- add dry-run
- print to file instead of to terminal

## [0.0.16](https://github.com/oxc-project/cargo-release-oxc/compare/v0.0.15...v0.0.16) - 2024-06-14

### Other
- *(deps)* update dependency rust to v1.79.0 ([#34](https://github.com/oxc-project/cargo-release-oxc/pull/34))
- *(deps)* update rust crates

## [0.0.15](https://github.com/oxc-project/cargo-release-oxc/compare/v0.0.14...v0.0.15) - 2024-06-07

### Added
- add `changelog` command

### Other
- clean up some code
- unify options

## [0.0.14](https://github.com/oxc-project/cargo-release-oxc/compare/v0.0.13...v0.0.14) - 2024-06-06

### Added
- remove change log header from the printed out version

## [0.0.13](https://github.com/oxc-project/cargo-release-oxc/compare/v0.0.12...v0.0.13) - 2024-06-05

### Added
- support versioning non-workspace cargo toml

### Other
- update
- update git cliff
- add rust cache

## [0.0.12](https://github.com/oxc-project/cargo-release-oxc/compare/v0.0.11...v0.0.12) - 2024-06-04

### Fixed
- fallback to current_dir

## [0.0.11](https://github.com/oxc-project/cargo-release-oxc/compare/v0.0.10...v0.0.11) - 2024-06-04

### Added
- print tag version in publish

## [0.0.10](https://github.com/oxc-project/cargo-release-oxc/compare/v0.0.9...v0.0.10) - 2024-06-04

### Added
- check git status before running anything
- add `--release name`
- update package.json versions
- add configuration
- calculate next version from changelog
- `update` print version
- remove cargo check in update command
- customize tag prefix

### Other
- add release manual trigger
- remove `semver`
- unwrap parent
- refactor out versioning crates
- alias r in justfile

## [0.0.9](https://github.com/oxc-project/cargo-release-oxc/compare/v0.0.8...v0.0.9) - 2024-06-02

### Added
- remove git operations

### Fixed
- skip package publish if the package is already published

## [0.0.8](https://github.com/oxc-project/cargo-release-oxc/compare/v0.0.7...v0.0.8) - 2024-05-20

### Fixed
- use rustls

### Other
- allow branch `renovate/**`
- *(deps)* lock file maintenance rust crates ([#17](https://github.com/oxc-project/cargo-release-oxc/pull/17))
- release ([#9](https://github.com/oxc-project/cargo-release-oxc/pull/9))

## [0.0.7](https://github.com/oxc-project/cargo-release-oxc/compare/v0.0.6...v0.0.7) - 2024-05-07

### Added
- check version and return if version is already published before publish

### Other
- format toml
- update renovate.json
- *(deps)* lock file maintenance rust crates ([#14](https://github.com/oxc-project/cargo-release-oxc/pull/14))
- update renovate
- *(deps)* update rust crate anyhow to v1.0.83 ([#13](https://github.com/oxc-project/cargo-release-oxc/pull/13))
- *(deps)* update rust crate git_cmd to v0.6.5 ([#12](https://github.com/oxc-project/cargo-release-oxc/pull/12))
- *(deps)* update dependency rust to v1.78.0 ([#11](https://github.com/oxc-project/cargo-release-oxc/pull/11))
- *(deps)* update rust crates ([#10](https://github.com/oxc-project/cargo-release-oxc/pull/10))
- *(renovate)* add rust-toolchain

## [0.0.6](https://github.com/oxc-project/cargo-release-oxc/compare/v0.0.5...v0.0.6) - 2024-04-08

### Added
- run `cargo check` when Cargo.toml is update

## [0.0.5](https://github.com/oxc-project/cargo-release-oxc/compare/v0.0.4...v0.0.5) - 2024-04-03

### Added
- switch to new branch and commit changes

### Fixed
- reset lower version (minor, patch) numbers to 0 when bump versions

### Other
- fix clippy warnings
- add [workspace.lints.clippy]

## [0.0.4](https://github.com/oxc-project/cargo-release-oxc/compare/v0.0.3...v0.0.4) - 2024-04-03

### Fixed
- check before publish and also fix publishing order

## [0.0.3](https://github.com/oxc-project/cargo-release-oxc/compare/v0.0.2...v0.0.3) - 2024-04-03

### Added
- use `--bump` for version update
- add publish command
- add `regenerate_changelogs` command

### Other
- improve `regenerate_changelogs`

## [0.0.2](https://github.com/oxc-project/cargo-release-oxc/compare/v0.0.1...v0.0.2) - 2024-04-01

### Fixed
- fix repository link in Cargo.toml

## [0.0.1](https://github.com/oxc-project/release-oxc/compare/v0.0.0...v0.0.1) - 2024-03-31

### Other
- add release-binaries
