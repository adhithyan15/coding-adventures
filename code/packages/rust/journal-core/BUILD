# Test both configurations: the zero-dependency default model, and the serde
# feature that the host facade enables for JSON (de)serialisation.
cargo test -p journal-core -- --nocapture
cargo test -p journal-core --all-features -- --nocapture
