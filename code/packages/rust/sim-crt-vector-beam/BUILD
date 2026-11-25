cargo test --locked -p sim-crt-vector-beam -- --nocapture
cargo test --locked -p sim-crt-vector-beam --release -- --nocapture
cargo run --locked --manifest-path ../ca-capability-analyzer/Cargo.toml -- --dir . --check-module
cargo run --locked --manifest-path ../ca-capability-analyzer/Cargo.toml -- --dir . --verify-cert effect_certificate.json
