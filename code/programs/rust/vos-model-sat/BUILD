# VOS-040a (VOS31 sections 5 and 6): the encoder and its three cross-checks, no solver yet.
# The cross-checks interpret the Warrant evaluator over millions of cases, so every cargo line
# is --release (the same lines run on Windows: BUILD_windows is identical).
cargo test --release -- --nocapture
cargo run -q --manifest-path ../../../packages/rust/Cargo.toml -p vos-shape-check -- .
# The loader refuses a `..` in the crate path, so run from inside the contract crate, the way
# the contract crate's own BUILD runs `check-contract .`.
cd ../../../packages/rust/vos-sched-contract && cargo run -q --release --manifest-path ../../../programs/rust/vos-model-sat/Cargo.toml -- cross-check .
