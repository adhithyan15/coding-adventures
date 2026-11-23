cargo test --locked -p avm-bytecode-lifter -- --nocapture
cargo test --locked -p avm-bytecode-lifter --release -- --nocapture
cargo run --locked --manifest-path ../ca-capability-analyzer/Cargo.toml -- --dir . --check-module
cargo run --locked --manifest-path ../ca-capability-analyzer/Cargo.toml -- --dir . --verify-cert effect_certificate.json
