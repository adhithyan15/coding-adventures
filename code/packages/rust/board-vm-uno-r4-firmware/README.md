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

`uno-r4-wifi-serialusb-server` is the matching entrypoint for the built-in
USB-C `SerialUSB` route. It attaches `DeviceStreamEndpoint` to
`UsbCdcByteStream<UnoR4WifiSerialUsb>` and uses the same Uno R4 WiFi board
runtime as the UART server. The reusable server helper is host-tested with a
fake CDC transport so the Board VM wire-frame path is validated before the
Arduino/TinyUSB link layer is wired in.

The Rust entrypoint now starts Arduino Renesas USB through the `__USBStart()`
C++ ABI symbol exposed by the Arduino core. The `board-vm-uno-r4-usb-cdc`
backend supplies the serial-install descriptor hook and the Uno R4 WiFi USB-C
mux, USB post-initialization hook, and mutable RAM vector-table setup from
Rust, so the final link set does not need Arduino's full `SerialUSB.cpp` object
or WiFi `variant.cpp` just to expose CDC. The RAM vector table matters because
Arduino's `IRQManager.cpp` installs USBFS interrupt handlers dynamically during
`__USBStart()`.

`src/arduino_usb_link.rs` records the Arduino Renesas/TinyUSB link contract for
the built-in USB path: Arduino core version `1.5.3`, FQBN
`arduino:renesas_uno:unor4wifi`, required TinyUSB C objects, `USB.cpp`,
`IRQManager.cpp`, the Uno R4 WiFi `libfsp.a`, include directories, compile
defines, and the Rust-provided C++ ABI hook symbols. The firmware build script
uses the same manifest when `BOARD_VM_UNO_R4_LINK_ARDUINO_USB=1`, keeping CI
independent from a local Arduino install while giving hardware builds an
explicit link path.

Build the firmware library and host-tested server path now:

```sh
cargo test -p board-vm-uno-r4-firmware -- --nocapture
RUSTC="$(rustup which rustc)" rustup run stable cargo build --target thumbv7em-none-eabihf -p board-vm-uno-r4-firmware --lib
```

Build the SerialUSB server with the Arduino/TinyUSB link path enabled:

```sh
BOARD_VM_UNO_R4_LINK_ARDUINO_USB=1 \
BOARD_VM_UNO_R4_ARDUINO_CORE="$HOME/Library/Arduino15/packages/arduino/hardware/renesas_uno/1.5.3" \
RUSTC="$(rustup which rustc)" \
rustup run stable cargo build \
  --target thumbv7em-none-eabihf \
  -p board-vm-uno-r4-firmware \
  --bin uno-r4-wifi-serialusb-server \
  --release
```

The build script compiles the manifest's Arduino/TinyUSB C/C++ sources into a
small archive under Cargo's `OUT_DIR`, links it with the Uno R4 WiFi `libfsp.a`
for `uno-r4-wifi-serialusb-server`, and leaves the other firmware binaries on
the pure-Rust link path. Override `BOARD_VM_UNO_R4_ARM_GCC`,
`BOARD_VM_UNO_R4_ARM_GXX`, or `BOARD_VM_UNO_R4_ARM_AR` if the Arduino-packaged
toolchain cannot run on the host. When using a native ARM compiler that does
not carry Newlib/libstdc++ headers, point `BOARD_VM_UNO_R4_ARM_COMPAT_ROOT` at
Arduino's packaged `arm-none-eabi-gcc/7-2017q4` root so the build script can
reuse those headers.

For the built-in USB route, the host artifact helper wraps the repeatable
hardware path: build the linked SerialUSB firmware, convert the ELF to the
bootloader `.bin`, optionally upload it with Arduino CLI, and optionally run the
Board VM smoke session. When `--upload` and `--smoke` are used together, the
helper follows Arduino CLI's reported `New upload port` so the smoke session
targets the runtime CDC port after the board re-enumerates. Use `--print-only`
first to inspect the static command plan:

```sh
cargo run -p board-vm-uno-r4-firmware --bin uno-r4-wifi-serialusb-artifact -- \
  --print-only \
  --core "$HOME/Library/Arduino15/packages/arduino/hardware/renesas_uno/1.5.3" \
  --arm-toolchain-bin /opt/homebrew/bin \
  --bossac-path /tmp/arduino-bossa/bin \
  --port /dev/cu.usbmodem... \
  --upload \
  --smoke
```

When the Arduino Renesas core and a compatible ARM GCC toolchain are available,
drop `--print-only` to produce
`target/thumbv7em-none-eabihf/release/uno-r4-wifi-serialusb-server.bin`, flash
it, follow the post-upload runtime port, and run the host smoke path:

```sh
cargo run -p board-vm-uno-r4-firmware --bin uno-r4-wifi-serialusb-artifact -- \
  --core "$HOME/Library/Arduino15/packages/arduino/hardware/renesas_uno/1.5.3" \
  --arm-toolchain-bin /opt/homebrew/bin \
  --bossac-path /tmp/arduino-bossa/bin \
  --port /dev/cu.usbmodem... \
  --upload \
  --smoke
```

The helper discovers Rust's bundled `llvm-objcopy`, the stable rustup `rustc`,
and `arm-none-eabi-*` tools on `PATH` when available. Override `--rustc`,
`--arm-gcc`, `--arm-gxx`, `--arm-ar`, `--arm-compat-root`, `--objcopy`,
`--arduino-cli`, `--bossac-path`, `--target-dir`, `--baud`, or `--timeout-ms`
for local tooling differences.

After flashing any Board VM server image manually, run the host smoke test
against the adapter serial port:

```sh
cargo run -p board-vm-cli --bin board-vm -- smoke \
  --port /dev/cu.usbmodem... \
  --baud 115200 \
  --timeout-ms 1000
```

For the original `uno-r4-vm-blink-smoke` image, upload the bootloader `.bin`
directly:

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
