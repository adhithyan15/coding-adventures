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
- Portenta H7 Lite, Portenta H7 Lite Connected, Portenta C33, Nicla Vision,
  Nicla Sense ME, Nicla Voice, and the Opta Lite/RS485/WiFi variants for
  Pro, Nicla, and industrial PLC-style Arduino coverage

Each target currently proves the shared GPIO/time backend path. Board-specific
firmware/upload crates can extend the same descriptor with richer peripherals
without falling back to Uno R4 assumptions. The descriptors also carry the
Arduino CLI platform/FQBN for each board so host SDKs can ask Rust for upload
metadata instead of maintaining language-local board tables. The upload
descriptor also records whether the board appears through a USB serial bridge,
native USB bootloader, or external serial adapter, which keeps Pro Mini-style
boards from being treated like onboard-USB Arduinos.

Opta terminals are modeled as board-local input and relay-output pins instead
of pretending the PLC has Uno-style headers. Nicla sensor/audio/vision hardware
is similarly registered as its own target family so follow-up capability work
can add the real sensor, camera, microphone, and wireless paths without changing
language frontends.

Host-side validation:

```sh
cargo test -p board-vm-arduino
```
