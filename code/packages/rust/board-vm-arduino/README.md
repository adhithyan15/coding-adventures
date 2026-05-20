# board-vm-arduino

Shared Board VM target descriptors and abstract HAL backend for Arduino boards
beyond the Uno R4-specific crate.

This crate keeps generic Arduino-family support from being folded into
`board-vm-uno-r4`. It exposes one backend shape for classic AVR, SAM/SAMD,
megaAVR, RP2040, ESP32, STM32/Mbed-style Arduino boards, and future Arduino
families. Concrete firmware crates can provide hardware-specific backends while
host SDKs and target discovery stay on the shared Board VM contract.

The first matrix covers representative Arduino families:

- Uno R3, Nano classic, Pro Mini, Mega 2560, Leonardo, and Micro for classic AVR
- Due, Zero, MKR WiFi 1010, Nano Every, Nano R4, Nano 33 IoT, and Nano 33 BLE
  Rev2 for SAM/SAMD/megaAVR/Renesas/Nordic coverage
- Nano RP2040 Connect, Nano ESP32, GIGA R1 WiFi, and Portenta H7 for modern
  maker/pro Arduino boards

Each target currently proves the shared GPIO/time backend path. Board-specific
firmware/upload crates can extend the same descriptor with richer peripherals
without falling back to Uno R4 assumptions.

Host-side validation:

```sh
cargo test -p board-vm-arduino
```
