# Changelog
All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

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
