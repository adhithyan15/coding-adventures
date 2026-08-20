cargo test -p chief-of-staff-hardware-key-approval -- --nocapture
if [ "$(uname)" = "Linux" ]; then cargo tarpaulin -p chief-of-staff-hardware-key-approval --out Stdout --skip-clean --timeout 180; fi
