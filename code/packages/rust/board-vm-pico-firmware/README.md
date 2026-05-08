# board-vm-pico-firmware

Raspberry Pi Pico Board VM firmware server scaffolding.

This crate intentionally stops short of selecting `embassy-rp`, `rp2040-hal`,
USB, or UART concrete drivers. It wires the generic `board-vm-device` stream
endpoint to the Pico board adapter so the eventual firmware entrypoint only has
to provide a `PicoBackend` and a byte stream.

Host-side validation:

```sh
cargo test -p board-vm-pico-firmware
```
