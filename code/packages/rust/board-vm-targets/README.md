# board-vm-targets

Host-side Board VM target registry for supported boards.

The registry gives generic front ends and language bindings one stable place to
ask which boards are known, what runtime id they use, what Rust target they
compile for, where their onboard LED lives, and which host transports are
available. Board-specific runtime crates remain responsible for HAL behavior,
pin validation, wireless stack integration, and upload mechanics.

Arduino coverage is deliberately split in two:

- `board-vm-uno-r4` remains the rich, board-specific Renesas RA4M1 target for
  Uno R4 Minima and Uno R4 WiFi.
- `board-vm-arduino` registers the broader Arduino-family backend contract for
  non-Uno-R4 boards such as Uno R3, Nano, Mega 2560, Leonardo/Micro, Due, Zero,
  MKR WiFi 1010, Nano Every, Nano R4, Nano 33 IoT, Nano 33 BLE Rev2, Nano RP2040
  Connect, Nano ESP32, GIGA R1 WiFi, and Portenta H7.

Wireless metadata separates physical support from the generic front-end sugar:

- every current target exposes `transport.serial`
- Arduino Uno R4 WiFi exposes Wi-Fi and Bluetooth LE through its ESP32-S3
  coprocessor
- ESP32 DevKit V1 exposes Wi-Fi, Bluetooth LE, and Bluetooth Classic natively
- Raspberry Pi Pico exposes no wireless transports
- Raspberry Pi Pico W exposes Wi-Fi, Bluetooth LE, and Bluetooth Classic through
  its CYW43439 radio

Wi-Fi OTA is marked as a feasible Board VM update path for the wireless targets;
Bluetooth command channels are tracked separately because OTA over Bluetooth is
possible but not the first practical update path.

Host-side validation:

```sh
cargo test -p board-vm-targets
```
