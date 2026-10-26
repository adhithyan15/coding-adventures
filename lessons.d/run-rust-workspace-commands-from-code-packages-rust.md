# Run Rust workspace commands from code packages rust

The repository root has no `Cargo.toml`. Package-scoped commands such as
`cargo fmt -p`, `cargo test -p`, and `cargo clippy -p` therefore fail there
before resolving the package. Run them from `code/packages/rust`, or pass that
directory's manifest explicitly. Package `BUILD` files may still be launched
from the repository root when their command assumes the build tool's package
working-directory behavior.
