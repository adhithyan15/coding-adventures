---
category: BUILD files & dependency management
---

# Rust scaffold dependency resolution must not topologically sort dev-dependency cycles

The Rust scaffold generator discovers every local `path = "../..."` entry in
each dependency's `Cargo.toml` without distinguishing `[dependencies]` from
`[dev-dependencies]`. Security-composition crates can therefore form a valid
Cargo dev-dependency cycle that the generator reports as a production circular
dependency, leaving no package but creating a generator-local `Cargo.lock`.

Before passing an existing Rust composition stack to `--depends-on`, inspect
its complete path-entry graph. If dev-only edges make that graph cyclic,
generate the empty Rust scaffold, remove the generator-local lockfile, then
author the new crate's direct manifest dependencies and BUILD recipe from the
reviewed Cargo graph. Do not weaken or omit the crate's real production
dependencies merely to make the scaffold resolver succeed.
