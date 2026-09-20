# Run Rust workspace commands from the workspace manifest directory

The repository's Rust packages are not governed by a Cargo workspace manifest
at `code/`. Running `cargo test -p ...` there fails before package resolution.
Before using workspace-style package selection, locate the controlling
`Cargo.toml` with `rg --files -g Cargo.toml`, then run from that manifest's
directory or pass its explicit `--manifest-path`.
