cargo test -p swf-conformance-corpus -- --nocapture
cargo test -p swf-conformance-corpus --release -- --nocapture
cargo run --manifest-path ../ca-capability-analyzer/Cargo.toml -- --dir . --check-module
cargo run --manifest-path ../ca-capability-analyzer/Cargo.toml -- --dir . --verify-cert effect_certificate.json
