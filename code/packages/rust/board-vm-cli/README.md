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
cargo run -p board-vm-cli --bin board-vm -- smoke \
  --endpoint serial:///dev/cu.usbmodem... \
  --board uno-r4-wifi \
  --baud 115200 \
  --timeout-ms 1000
```

```sh
cargo run -p board-vm-cli --bin board-vm -- smoke \
  --endpoint tcp://board-vm.local:4170 \
  --timeout-ms 1000
```

```sh
cargo run -p board-vm-cli --bin board-vm -- smoke \
  --endpoint 'ble://AA:BB:CC:DD:EE:FF?service=6e400001-b5a3-f393-e0a9-e50e24dcca9e&write=6e400002-b5a3-f393-e0a9-e50e24dcca9e&notify=6e400003-b5a3-f393-e0a9-e50e24dcca9e' \
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

`smoke` opens the selected serial device or endpoint, sends `HELLO`, queries
capabilities, uploads the standard onboard LED blink module, starts it with
a bounded instruction budget, verifies the board still answers `PING`, and
then sends `STOP` to prove host control is recovered. It is transport-hosting
glue only; the board firmware still owns the protocol dispatcher and HAL
behavior. The serial path
asks `board-vm-language-core` for the runtime serial open plan, then applies
CLI `--baud` and `--timeout-ms` overrides before touching the OS port. That
keeps DTR/open-settle, stale-byte clearing, endpoint metadata, and wire
protocol ownership in the Rust target layer while this crate performs the
platform-specific open/read/write call. The endpoint path accepts Rust-owned
serial endpoint metadata (`serial://...`), TCP endpoint metadata (`tcp://...` or
bare `host:port`), and Board VM Bluetooth endpoints (`ble://`, `btspp://`, or
`rfcomm://`) through the Rust-owned transport adapters. Endpoint transport
classification comes from the shared `board-vm-language-core`
`parse_host_endpoint_with_error` summary, so the CLI does not maintain its own
endpoint scheme or parse-error tables. Endpoint connection labels also come
from `host_endpoint_connection_label`, which keeps the serial-only baud label
policy in the shared Rust layer. Endpoint open paths consume
`host_endpoint_session_summary`, so the parsed endpoint metadata and display
label arrive together before this crate touches platform transport APIs. The
smoke report starts with a stable `connection transport=...` field so hardware
logs can distinguish serial, TCP socket, BLE GATT, and RFCOMM runs without
parsing endpoint strings. The default run budget is intentionally small because
the current firmware executes blink bytecode synchronously while it prepares
the run report.

`repl` opens the same serial-plan-backed transport, sends `HELLO`, and then
accepts a small interactive command set: `caps`, `upload-blink`, `upload-gpio-read <pin>
[mode]`, `upload-time-now`, `run [budget]`, `blink [budget]`, `gpio-read <pin>
[mode] [budget]`, `time-now [budget]`, `ping`, `stop`, `hello`, `help`, and `quit`.
This is the first language-agnostic host shell: it drives the binary protocol
through the shared client library, while future frontend packages can put
richer syntax on top of the same transport calls. `gpio-read` and `time-now`
print Rust-decoded run-report return values from the board. The endpoint path
uses the same serial, TCP socket, and Bluetooth wire transports as `smoke`,
with loopback coverage for interactive upload/run flows over serial endpoint
metadata, TCP, BLE GATT, and RFCOMM.

`eject blink` writes the current blink MVP as embeddable Rust constants with a
program id, slot, boot policy, BVM module metadata, required capabilities,
bytecode CRC, and BVM module bytes. The command report mirrors that metadata so
language frontends can stay thin over the Rust-owned artifact contract instead
of parsing generated Rust constants. The output is intentionally board-agnostic
so Uno R4, ESP32, Pico, and future firmware backends can consume the same
ejected artifact format while owning their own HAL and startup behavior.
