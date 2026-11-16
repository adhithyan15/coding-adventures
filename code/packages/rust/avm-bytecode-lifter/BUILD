cargo test -p avm-bytecode-lifter -- --nocapture
cargo test -p avm-bytecode-lifter --release -- --nocapture
cargo run --manifest-path ../ca-capability-analyzer/Cargo.toml -- --dir . --check-module
cargo run --manifest-path ../ca-capability-analyzer/Cargo.toml -- --dir . --verify-cert effect_certificate.json
