# Validate original fixture bytes and its independent Python reference before
# the Rust adapter consumes the same canonical case set.
python3 ../../../specs/fixtures/game-input-frame-v1/validate.py
python3 -m unittest discover -s ../../../specs/fixtures/game-input-frame-v1 -p 'test_*.py'
cargo test --locked -p game-input-frame -- --nocapture
cargo test --locked -p game-input-frame --release -- --nocapture
if [ "$(uname -s)" = "Linux" ] && cargo tarpaulin --version >/dev/null 2>&1; then cargo tarpaulin --manifest-path Cargo.toml --tests --engine llvm --skip-clean --fail-under 95 --include-files 'game-input-frame/src/*' --out Stdout; fi
cargo run --locked --manifest-path ../ca-capability-analyzer/Cargo.toml -- --dir . --check-module
cargo run --locked --manifest-path ../ca-capability-analyzer/Cargo.toml -- --dir . --verify-cert effect_certificate.json
