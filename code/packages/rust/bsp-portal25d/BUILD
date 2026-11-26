cargo test --locked -p bsp-portal25d -- --nocapture
cargo test --locked -p bsp-portal25d --release -- --nocapture
cargo run --locked --manifest-path ../ca-capability-analyzer/Cargo.toml -- --dir . --check-module
cargo run --locked --manifest-path ../ca-capability-analyzer/Cargo.toml -- --dir . --verify-cert effect_certificate.json
