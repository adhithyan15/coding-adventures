cargo test -p chief-of-staff-daemon-config -- --nocapture
if [ "$(uname)" = "Linux" ]; then cargo tarpaulin -p chief-of-staff-daemon-config --out Stdout --skip-clean; fi
