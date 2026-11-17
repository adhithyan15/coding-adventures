cargo test -p swf-container -- --nocapture
cargo test -p swf-container --release -- --nocapture
cargo run --manifest-path ../ca-capability-analyzer/Cargo.toml -- --dir . --check-module
cargo run --manifest-path ../ca-capability-analyzer/Cargo.toml -- --dir . --verify-cert effect_certificate.json
