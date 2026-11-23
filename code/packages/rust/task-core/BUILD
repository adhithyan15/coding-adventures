# Test both configurations: the zero-dependency default model, and the serde
# feature that host-facing crates enable for JSON (de)serialisation.
cargo test --locked -p task-core -- --nocapture
cargo test --locked -p task-core --all-features -- --nocapture
