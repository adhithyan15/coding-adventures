cargo test --locked -p swf-conformance-corpus -- --nocapture
cargo test --locked -p swf-conformance-corpus --release -- --nocapture
cargo run --locked --manifest-path ../ca-capability-analyzer/Cargo.toml -- --dir . --check-module
cargo run --locked --manifest-path ../ca-capability-analyzer/Cargo.toml -- --dir . --verify-cert effect_certificate.json
