# OS Kernel (Rust)

This package is the first real OS kernel crate in the repository.

It is intentionally tiny:

- one shared `Kernel` abstraction
- no custom bootloader
- no UART dependency
- one UEFI entry wrapper
- cross-compiles to both `x86_64` and `aarch64`

The current milestone proves one thing:

**the same kernel code can boot on two QEMU targets through standard Rust
cross-compilation targets**

## Targets

- `x86_64-unknown-uefi`
- `aarch64-unknown-uefi`

## Build

```bash
~/.cargo/bin/rustup target add x86_64-unknown-uefi aarch64-unknown-uefi
~/.cargo/bin/cargo build -p os-kernel --target x86_64-unknown-uefi
~/.cargo/bin/cargo build -p os-kernel --target aarch64-unknown-uefi
```

## Build and test in CI

The `BUILD` recipe (the same on Linux, macOS and Windows) adds the
`x86_64-unknown-uefi` target, runs the `Kernel` unit tests on the host
(`cargo test --lib`), cross-builds the UEFI image
(`cargo build --release --target x86_64-unknown-uefi`) and runs clippy with
`-D warnings` for both the UEFI target and the host. The QEMU boot test below
is not part of CI.

## Boot tests

```bash
./code/packages/rust/os-kernel/tests/qemu-boot-test.sh x86_64
./code/packages/rust/os-kernel/tests/qemu-boot-test.sh aarch64
```
