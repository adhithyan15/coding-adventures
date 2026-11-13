---
category: Rust
---

# Tests that locate cargo --emit output must not hard-code target/release/deps; Cargo 1.100 moved it to release/build/<crate>/<hash>/out

The optimized-LLVM-IR gates (hkdf, chacha20-poly1305, aes, aes-modes,
tls13-traffic-key-material) ran `cargo rustc --release --lib -- --emit=llvm-ir`
into a temp target dir and read `release/deps/<crate>-<hash>.ll`. The macOS CI
runner moved to Rust/Cargo 1.100, whose new build-dir layout writes that file to
`release/build/<crate>/<hash>/out/` and creates no `release/deps`, so the tests
died with "release dependency directory must exist". The log also printed IR
fragments from the `#[should_panic]` parser tests, which looked like IR-shape
drift but were expected panic output; the real failure was the missing
directory. The IR shape assertions needed no change.

Fix: search the release tree for the `.ll` file (skipping `incremental`)
instead of hard-coding either layout.

Do differently: when a security gate fails after a toolchain bump, first work
out which test actually failed (read the `test ... FAILED` lines, not the
panic text of should_panic tests), and reproduce with the new toolchain
(`rustup toolchain install <ver> --profile minimal -c clippy`) before touching
any assertion. Never hard-code Cargo's intermediate-file layout in tests.
