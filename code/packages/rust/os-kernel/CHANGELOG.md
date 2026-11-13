# Changelog

## [Unreleased]

### Changed — `BUILD`/`BUILD_windows` build and test instead of skipping

Both recipes were an `echo SKIP` ("cannot be built, tested, or clippy-checked on
a std host").  That is true of the *binary* on the host triple, but not of the
package: rust-std for `x86_64-unknown-uefi` comes from rustup on every OS, and
the `Kernel` state machine in `lib.rs` is plain `no_std` Rust whose unit tests
run on the host.  The recipe (identical on Linux, macOS and Windows) is now:

- `rustup target add x86_64-unknown-uefi`
- `cargo test --lib` (the three `Kernel` state tests, on the host)
- `cargo build --release --target x86_64-unknown-uefi` (the real UEFI image,
  `os-kernel.efi`, 12 KiB)
- `cargo clippy -D warnings` for the UEFI target and for the host lib/tests.

The QEMU boot test (`tests/qemu-boot-test.sh`) is still manual: it needs QEMU and
OVMF firmware, neither of which CI installs.
