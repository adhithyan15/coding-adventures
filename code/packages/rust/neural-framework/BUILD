cargo fmt -p neural-framework -- --check
cargo clippy -p neural-framework --all-targets --no-deps -- -D warnings
cargo test -p neural-framework
python -m unittest discover -s ../../../scripts/tests -p test_neural_framework_rust.py
python ../../../scripts/validate_neural_framework_rust.py
if [ "$(uname -s)" = "Linux" ]; then cargo tarpaulin -p neural-framework --lib --tests --engine llvm --skip-clean --include-files 'neural-framework/src/*' --fail-under 95; fi
