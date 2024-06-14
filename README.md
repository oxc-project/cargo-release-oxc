# Release management for the oxc project

## `cargo release-oxc`

```
Usage: cargo-release-oxc COMMAND ...

Available options:
    -h, --help  Prints help information

Available commands:
    update      Generate CHANGELOG.md and bump versions for all published crates
    publish
```

## Output

Saves two files to `./target`:

* version: `./target/OXC_VERSION`
* changelog: `./target/OXC_CHANGELOG`
