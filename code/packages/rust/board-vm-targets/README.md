# board-vm-targets

Host-side Board VM target registry for supported boards.

The registry gives generic front ends and language bindings one stable place to
ask which boards are known, what runtime id they use, what Rust target they
compile for, where their onboard LED lives, and which host transports are
available. Board-specific runtime crates remain responsible for HAL behavior,
pin validation, and wireless stack integration. Upload metadata stays in this
registry so language frontends can ask Rust whether a target uses Arduino CLI,
ESP ROM serial flashing, UF2 mass-storage copy, or another board-specific
adapter.

Arduino coverage is deliberately split in two:

- `board-vm-uno-r4` remains the rich, board-specific Renesas RA4M1 target for
  Uno R4 Minima and Uno R4 WiFi.
- `board-vm-arduino` registers the broader Arduino-family backend contract for
  non-Uno-R4 boards such as Uno R3, Nano, Mega 2560, Leonardo/Micro, Due, Zero,
  MKR WiFi 1010, Nano Every, Nano R4, Nano 33 IoT, Nano 33 BLE Rev2, Nano RP2040
  Connect, Nano ESP32, GIGA R1 WiFi, Portenta H7, Portenta H7 Lite, Portenta H7
  Lite Connected, Portenta C33, Nicla Vision, Nicla Sense ME, Nicla Voice, and
  Opta Lite/RS485/WiFi.

Opta targets expose industrial terminal inputs and relay outputs through the
same target registry shape, but they do not pretend to share Uno header pins.
Nicla and Portenta targets likewise carry their own MCU/runtime descriptors so
the next board-specific firmware and upload adapters can attach without adding
special cases to Ruby, Python, Lua, or other language frontends.

Upload metadata is intentionally profile-shaped rather than a frontend command
builder:

- Arduino-family targets use an Arduino CLI profile so the board package owns
  bootloader reset, programmer selection, firmware artifact layout, and the
  selected platform/FQBN string for each concrete board descriptor.
- ESP32 DevKit targets use an ESP ROM serial profile so image layout and boot
  pin reset remain Rust-owned.
- Raspberry Pi Pico targets use a Pico UF2 mass-storage profile so BOOTSEL mount
  discovery and UF2 copy behavior are exposed without frontend special cases.

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
