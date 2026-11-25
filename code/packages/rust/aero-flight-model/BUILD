cargo test --locked -p aero-flight-model -- --nocapture
cargo test --locked -p aero-flight-model --release -- --nocapture
cargo run --locked --manifest-path ../ca-capability-analyzer/Cargo.toml -- --dir . --check-module
cargo run --locked --manifest-path ../ca-capability-analyzer/Cargo.toml -- --dir . --verify-cert effect_certificate.json
