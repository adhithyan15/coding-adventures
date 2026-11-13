# The kernel crate is no_std and links as a UEFI application, so the real build
# is a cross-build to the UEFI target (rust-std for it comes from rustup). The
# `Kernel` state machine in lib.rs is plain Rust and its unit tests run on the
# host. os-kernel is excluded from the workspace, so this runs in its own
# directory with its own target/.
rustup target add x86_64-unknown-uefi
cargo test --lib
cargo build --release --target x86_64-unknown-uefi
cargo clippy --target x86_64-unknown-uefi --all-targets -- -D warnings
cargo clippy --lib --tests -- -D warnings
