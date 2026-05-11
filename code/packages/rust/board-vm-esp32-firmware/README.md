# board-vm-esp32-firmware

ESP32 Board VM firmware server scaffolding.

This crate intentionally stops short of selecting ESP-IDF, `esp-hal`, USB, or
UART concrete drivers. It wires the generic `board-vm-device` stream endpoint to
the ESP32 board adapter so the eventual firmware entrypoint only has to provide
an `Esp32Backend` and a byte stream.

Host-side validation:

```sh
cargo test -p board-vm-esp32-firmware
```
