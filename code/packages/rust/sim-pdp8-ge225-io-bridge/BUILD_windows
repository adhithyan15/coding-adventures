cargo test -p sim-pdp8-ge225-io-bridge -- --nocapture
cargo test -p sim-pdp8-ge225-io-bridge --release -- --nocapture
cargo run --manifest-path ../ca-capability-analyzer/Cargo.toml -- --dir . --check-module
cargo run --manifest-path ../ca-capability-analyzer/Cargo.toml -- --dir . --verify-cert effect_certificate.json
