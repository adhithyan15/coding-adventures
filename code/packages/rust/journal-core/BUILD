# Test both configurations: the zero-dependency default model, and the serde
# feature that the host facade enables for JSON (de)serialisation.
cargo test --locked -p journal-core -- --nocapture
cargo test --locked -p journal-core --all-features -- --nocapture
