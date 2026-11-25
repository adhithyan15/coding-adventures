cargo test --locked -p state-machine -- --nocapture
if [ "$(uname)" = "Linux" ]; then cargo tarpaulin -p state-machine --out Stdout --skip-clean; fi
