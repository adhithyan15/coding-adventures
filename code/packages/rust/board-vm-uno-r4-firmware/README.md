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

Target compile from the Rust workspace root:

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
target/thumbv7em-none-eabihf/release/uno-r4-vm-blink-smoke
```

The crate injects the Cortex-M `link.x` linker script and an Uno R4 sketch-slot
`memory.x` from `build.rs`, so the firmware keeps its vector table and text
sections even when built from the workspace root. The flash origin is `0x4000`,
matching the Arduino Renesas core's bootloader-managed sketch region.

To produce the bootloader-ready binary:

```sh
"$(dirname "$(rustup which rustc)")/../lib/rustlib/aarch64-apple-darwin/bin/llvm-objcopy" \
  -O binary \
  target/thumbv7em-none-eabihf/release/uno-r4-vm-blink-smoke \
  target/thumbv7em-none-eabihf/release/uno-r4-vm-blink-smoke.bin
```

On an Uno R4 WiFi, upload through Arduino CLI with the board's serial port and
the Arduino-patched BOSSA uploader:

```sh
arduino-cli upload \
  -p /dev/cu.usbmodem... \
  -b arduino:renesas_uno:unor4wifi \
  -i target/thumbv7em-none-eabihf/release/uno-r4-vm-blink-smoke.bin
```

On Apple Silicon systems without Rosetta, Arduino's packaged macOS `bossac`
may be x86-only. Build Arduino's patched BOSSA uploader locally and override the
tool path when uploading:

```sh
git clone --depth 1 https://github.com/arduino/BOSSA.git /tmp/arduino-bossa
make -C /tmp/arduino-bossa bossac \
  CXX=clang++ \
  VERSION=1.9.1-arduino2-3-g37600d1 \
  CXXFLAGS="-arch arm64 -mmacosx-version-min=11.0 -Wno-error=unqualified-std-cast-call -Wno-error=vla-cxx-extension" \
  LDFLAGS="-arch arm64 -mmacosx-version-min=11.0"

arduino-cli upload \
  -p /dev/cu.usbmodem... \
  -b arduino:renesas_uno:unor4wifi \
  -i target/thumbv7em-none-eabihf/release/uno-r4-vm-blink-smoke.bin \
  --upload-property runtime.tools.bossac-1.9.1-arduino5.path=/tmp/arduino-bossa/bin
```
