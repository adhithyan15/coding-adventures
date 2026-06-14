# board-vm-esp32

Abstract ESP32 target descriptor and HAL adapter for Board VM.

This crate does not configure ESP-IDF, flash firmware, or emulate Xtensa. It
keeps the Board VM runtime portable by describing common ESP32 DevKit-class
board metadata and by adapting a board-specific backend to the generic
`board-vm-runtime` HAL traits.

The first target is `ESP32_DEVKIT_V1`, covering the common ESP32-WROOM-32
development board shape: 3.3V GPIO, onboard LED on GPIO2 on many boards,
Xtensa LX6, and the GPIO input-only pins 34-39 modeled explicitly so host
language sugar can reject invalid writes before they hit a board backend.

Host-side validation:

```sh
cargo test -p board-vm-esp32
```
