# board-vm-uno-r4-firmware

Uno R4 firmware smoke test for the Board VM runtime.

This crate embeds the BVM1 blink module and provides firmware binaries for Uno
R4 WiFi bring-up. The early probes check the Rust target, linker script, GPIO
mapping, runtime, and in-memory stream path. `uno-r4-wifi-uart-server` is the
first interactive firmware: it serves Board VM wire frames over the Uno R4 WiFi
SCI9 UART route exposed by `board-vm-uno-r4-uart`.

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

The crate injects the Cortex-M `link.x` linker script for each firmware binary
and an Uno R4 sketch-slot `memory.x` from `build.rs`, so the firmware keeps its
vector table and text sections even when built from the workspace root. The
flash origin is `0x4000`, matching the Arduino Renesas core's bootloader-managed
sketch region.

To produce the bootloader-ready binary:

```sh
"$(dirname "$(rustup which rustc)")/../lib/rustlib/aarch64-apple-darwin/bin/llvm-objcopy" \
  -O binary \
  target/thumbv7em-none-eabihf/release/uno-r4-vm-blink-smoke \
  target/thumbv7em-none-eabihf/release/uno-r4-vm-blink-smoke.bin
```

On an Uno R4 WiFi, the built-in D13 LED is RA4M1 `P102`; the Uno R4 Minima maps
D13 differently, to `P111`, and should use a separate Minima smoke backend. For
the WiFi board, upload through Arduino CLI with the board's serial port and the
Arduino-patched BOSSA uploader:

The smoke backend configures `P102` directly through the RA4M1 PFS and PORT
registers. Arduino's Uno R4 WiFi bootloader leaves `LED_BUILTIN` in a PWM
peripheral state, so the firmware must clear peripheral mode before GPIO writes
can drive the visible LED.

For hardware bring-up, `uno-r4-wifi-raw-blink-probe` uses the same LED register
driver without the Board VM runtime. Use it to separate reset/vector/GPIO issues
from bytecode runtime issues:

```sh
RUSTC="$(rustup which rustc)" rustup run stable cargo build --target thumbv7em-none-eabihf --bin uno-r4-wifi-raw-blink-probe --release
```

`uno-r4-wifi-stream-handshake-probe` validates the board-side protocol path
without requiring USB CDC yet. It creates an in-memory COBS-delimited `HELLO`
request, serves it through `DeviceStreamEndpoint` and the Uno R4 `BoardVmDevice`,
decodes the emitted `HELLO_ACK`, and then blinks the yellow `L` LED. A repeating
three-fast-blink pattern means the stream/protocol/device path passed; a slow
single blink means the probe failed before or during response validation.

```sh
RUSTC="$(rustup which rustc)" rustup run stable cargo build --target thumbv7em-none-eabihf --bin uno-r4-wifi-stream-handshake-probe --release
```

`uno-r4-wifi-stream-session-probe` extends the same idea to a complete scripted
session. It feeds `HELLO`, `CAPS_QUERY`, `PROGRAM_BEGIN`, `PROGRAM_CHUNK`,
`PROGRAM_END`, and `RUN` wire frames into the device stream endpoint, verifies
each response, and runs the uploaded blink module through the Uno R4 runtime. A
repeating four-fast-blink pattern means the full scripted upload/run path passed.

```sh
RUSTC="$(rustup which rustc)" rustup run stable cargo build --target thumbv7em-none-eabihf --bin uno-r4-wifi-stream-session-probe --release
```

`uno-r4-wifi-uart-server` keeps a `DeviceStreamEndpoint` attached to
`UartByteStream<UnoR4WifiSerialUart>` and serves host requests indefinitely. It
uses the Uno R4 WiFi Arduino `Serial1` route on D22/D23 over SCI9. The built-in
USB-C serial device is Arduino `SerialUSB`, so it will not answer this UART
server until a separate USB CDC transport backend exists. Use a level-compatible
USB-to-UART adapter on D22/D23 for this firmware.

Build the UART server firmware:

```sh
RUSTC="$(rustup which rustc)" rustup run stable cargo build --target thumbv7em-none-eabihf --bin uno-r4-wifi-uart-server --release
```

After flashing the generated `.bin`, run the host smoke test against the adapter
serial port:

```sh
cargo run -p board-vm-cli --bin board-vm -- smoke \
  --port /dev/cu.usbmodem... \
  --baud 115200 \
  --timeout-ms 1000
```

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
