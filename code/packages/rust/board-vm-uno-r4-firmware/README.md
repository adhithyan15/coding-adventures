# board-vm-uno-r4-firmware

Uno R4 firmware smoke test for the Board VM runtime.

This crate embeds the BVM1 blink module and provides a tiny firmware binary that
executes the bytecode against the Uno R4 D13 LED HAL. It is not the interactive
serial protocol firmware yet; it is the first real-board check that the Rust
target, linker script, Uno R4 GPIO mapping, and Board VM runtime can produce a
flashable image.

Host-side validation:

```sh
cargo test -p board-vm-uno-r4-firmware
```

Target compile from this crate directory:

```sh
rustup target add thumbv7em-none-eabihf
cargo build --target thumbv7em-none-eabihf --bin uno-r4-vm-blink-smoke --release
```

If `cargo` comes from Homebrew or another non-rustup toolchain but the target was
installed with rustup, force cargo to use rustup's `rustc`:

```sh
RUSTC="$(rustup which rustc)" rustup run stable cargo build --target thumbv7em-none-eabihf --bin uno-r4-vm-blink-smoke --release
```

The resulting ELF is under:

```text
../../target/thumbv7em-none-eabihf/release/uno-r4-vm-blink-smoke
```

Uploading that artifact is intentionally left to the next bundle, where we can
choose the least surprising Arduino CLI/probe-rs path for the board on hand.
