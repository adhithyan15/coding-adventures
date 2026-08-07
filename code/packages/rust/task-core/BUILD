# Test both configurations: the zero-dependency default model, and the serde
# feature that host-facing crates enable for JSON (de)serialisation.
cargo test -p task-core -- --nocapture
cargo test -p task-core --all-features -- --nocapture
