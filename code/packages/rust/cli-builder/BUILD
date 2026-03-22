cargo test -p cli-builder -- --nocapture
if [ "$(uname)" = "Linux" ]; then cargo tarpaulin -p cli-builder --out Stdout --skip-clean; fi
