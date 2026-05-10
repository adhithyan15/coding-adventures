# board-vm-cli

Command-line smoke tools for Board VM hardware sessions.

The first command surface is intentionally small:

```sh
cargo run -p board-vm-cli --bin board-vm -- list-ports
```

```sh
cargo run -p board-vm-cli --bin board-vm -- smoke \
  --port /dev/cu.usbmodem... \
  --baud 115200 \
  --timeout-ms 1000
```

```sh
cargo run -p board-vm-cli --bin board-vm -- repl \
  --port /dev/cu.usbmodem... \
  --baud 115200 \
  --timeout-ms 1000
```

```sh
cargo run -p board-vm-cli --bin board-vm -- eject blink \
  --out /tmp/board_vm_blink.rs \
  --boot-policy run-if-no-host
```

`smoke` opens the selected serial device, sends `HELLO`, queries capabilities,
uploads the standard onboard LED blink module, and starts it with a bounded
instruction budget. It is transport-hosting glue only; the board firmware still
owns the protocol dispatcher and HAL behavior. The smoke path asserts DTR on
open, waits briefly, and clears the serial buffers before the first request so
USB CDC boards start each run from a clean host session. Its default run budget
is intentionally small because the current firmware executes blink bytecode
synchronously while it prepares the run report.

`repl` opens the same serial transport, sends `HELLO`, and then accepts a small
interactive command set: `caps`, `upload-blink`, `upload-gpio-read <pin>
[mode]`, `upload-time-now`, `run [budget]`, `blink [budget]`, `gpio-read <pin>
[mode] [budget]`, `time-now [budget]`, `stop`, `hello`, `help`, and `quit`.
This is the first language-agnostic host shell: it drives the binary protocol
through the shared client library, while future frontend packages can put
richer syntax on top of the same transport calls. `gpio-read` and `time-now`
print Rust-decoded run-report return values from the board.

`eject blink` writes the current blink MVP as embeddable Rust constants with a
program id, slot, boot policy, bytecode CRC, and BVM module bytes. That output
is intentionally board-agnostic so Uno R4, ESP32, Pico, and future firmware
backends can consume the same ejected artifact format while owning their own HAL
and startup behavior.
