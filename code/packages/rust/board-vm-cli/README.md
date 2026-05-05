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

`smoke` opens the selected serial device, sends `HELLO`, queries capabilities,
uploads the standard onboard LED blink module, and starts it with a bounded
instruction budget. It is transport-hosting glue only; the board firmware still
owns the protocol dispatcher and HAL behavior.
