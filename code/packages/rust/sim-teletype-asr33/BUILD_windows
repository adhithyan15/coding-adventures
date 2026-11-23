cargo test --locked -p sim-teletype-asr33 -- --nocapture
cargo test --locked -p sim-teletype-asr33 --release -- --nocapture
cargo run --locked --manifest-path ../ca-capability-analyzer/Cargo.toml -- --dir . --check-module
cargo run --locked --manifest-path ../ca-capability-analyzer/Cargo.toml -- --dir . --verify-cert effect_certificate.json
