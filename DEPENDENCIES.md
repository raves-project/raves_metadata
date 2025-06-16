# Dependencies

`raves_metadata` takes dependency usage seriously. I don't want to overuse dependencies, as doing so can lead to problems with portability, increase compile times, and cause other problems. This file explains each dependency in the project. But, first, a quick list of dependencies that will never appear:

- `syn`: causes compile times to be slow; repo owned by a single person; usability issues.
- `serde`: uses `syn`.
- format implementations, like `libavif`/friends: not required, as we can implement formats ourselves!
- other metadata parsers (like `Exiv2` and `XMP-Toolkit-SDK`): unportable.

Note that, unless you're building this workspace without Cargo, these aren't system dependencies and will be installed for you. (if you do want to build this crate without Cargo, and you're having trouble, please reach out!)

## `raves_metadata`

Don't add dependencies to this crate unless required for manual parsing. System/dynamic dependencies aren't usable due to a lack of portability, and static dependencies bloat compile times.

### Runtime Dependencies

#### `log`

This crate provides runtime logging to a "logger implementation." I'd usually prefer `tracing`, but it's slower, and we can't even use its instrumentation due to the `syn` requirement. `log` has no dependencies.

#### `xmltree`

Parses XML, because that's a little outta scope at the moment.

`xmltree` has one dependency:

- `xml-rs`: performs the actual parsing (lol).

#### `winnow`

Provides parser combinators to make parsing easy. No dependencies!

### Development Dependencies

Adding more of these is fine for improved testing.

#### `env_logger`

A lightweight logging implementation for the `log` crate.

It only depends on:

- `env_filter`: provides environment parsing.
  - We don't actually need this, but you can't turn it off, so it's fine...

## `raves_metadata_types`

I'm more lenient for dependencies in here. Please make an issue before doing so, nonetheless! :D

### Build Dependencies

#### `yaml-rust2`

To parse the IPTC tech reference YAML file, we use `yaml-rust2`. This might be removed in the future, as there's technically no blockers to just use the `cargo-expand` output directly (or, more likely, to just `include!()` the current output and put the dependency + build script behind a feature flag).

`yaml-rust2` has a few dependencies itself:

- `arraydeque` makes the YAML buffer [faster to scan](https://github.com/Ethiraric/yaml-rust2/blob/399f481990f11120b144ccd550657580284a3a30/documents/2024-03-15-FirstRelease.md?plain=1#L50).
- `hashlink` provides `hashlink::LinkedHashMap`, which maintains the order of its entries. It's used in `yaml-rust2` to describe the YAML document in order.
  - `hashbrown` is another implementation of `std::collections::HashMap`, though it works `#[no_std]`.
    - `foldhash` is `hashbrown`'s default hasher.

