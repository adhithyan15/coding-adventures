# board-vm-pico

Abstract Raspberry Pi Pico target descriptor and HAL adapter for Board VM.

This crate does not configure `embassy-rp`, flash firmware, or emulate RP2040.
It describes Pico/Pico W target metadata and adapts a board-specific backend to
the generic `board-vm-runtime` HAL traits.

The standard Pico exposes the onboard LED as RP2040 GPIO25. Pico W routes the
onboard LED through the CYW43 wireless chip, so the descriptor records that as a
wireless-chip LED rather than pretending it is a normal GPIO pin. Header GPIO
continues to work through the same Board VM GPIO HAL.

Host-side validation:

```sh
cargo test -p board-vm-pico
```
