# The bridge is test and proof infrastructure. Its test invokes Node on the exact
# checked-in TypeScript fixture and source-mutant copies of those same bytes.
node --version
cargo test -p typescript-warrant-ir -- --nocapture
cargo test -p typescript-warrant-ir --release -- --nocapture
if [ "$(uname -s)" = "Linux" ] && cargo tarpaulin --version >/dev/null 2>&1; then cargo tarpaulin --package typescript-warrant-ir --tests --engine llvm --skip-clean --fail-under 95 --include-files 'typescript-warrant-ir/src/*' --out Stdout; fi
cargo run --manifest-path ../ca-capability-analyzer/Cargo.toml -- --dir . --check-module
