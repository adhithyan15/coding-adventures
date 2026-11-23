# These package-wide test runs include the explicit `warrant_validate_state`
# model/API evaluator and `validate_state_sat` bounded miter integration targets.
cargo test --locked -p game-clock -- --nocapture
cargo test --locked -p game-clock --release -- --nocapture
cargo run --locked --manifest-path ../ca-capability-analyzer/Cargo.toml -- --dir . --check-module
cargo run --locked --manifest-path ../ca-capability-analyzer/Cargo.toml -- --dir . --verify-cert effect_certificate.json
