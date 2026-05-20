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
  Board-specific port hints distinguish USB-serial bridge boards, native-USB
  bootloaders, and external serial adapter boards before a frontend calls the
  shared Arduino CLI adapter.
- ESP32 DevKit targets use an ESP ROM serial profile so image layout and boot
  pin reset remain Rust-owned.
- Raspberry Pi Pico targets use a Pico UF2 mass-storage profile so BOOTSEL mount
  discovery and UF2 copy behavior are exposed without frontend special cases.

`board-vm-language-core` exposes typed Arduino CLI upload options from these
profiles, including platform/FQBN, port-selection step, native USB versus
USB-serial bridge hints, and board-package reset delegation. Language frontends
should call that helper instead of reconstructing Arduino upload tables.
It also exposes Arduino CLI port discovery metadata so native-USB boards can
declare their 1200-baud bootloader touch and runtime rediscovery behavior while
USB-serial bridge and external-adapter boards keep their simpler port paths.

Wireless metadata separates physical support from the generic front-end sugar:

- every current target exposes `transport.serial`
- Arduino Uno R4 WiFi exposes Wi-Fi and Bluetooth LE through its ESP32-S3
  coprocessor
- shared Arduino-family descriptors record physical radios for MKR WiFi 1010,
  Nano 33 IoT, Nano 33 BLE Rev2, Nano RP2040 Connect, Nano ESP32, GIGA R1
  WiFi, Portenta H7 Lite Connected, Portenta C33, Nicla Vision, Nicla Sense ME,
  Nicla Voice, and Opta WiFi without claiming command or OTA transport support
  until board-specific adapters land
- ESP32 DevKit V1 exposes Wi-Fi, Bluetooth LE, and Bluetooth Classic natively
- Raspberry Pi Pico exposes no wireless transports
- Raspberry Pi Pico W exposes Wi-Fi, Bluetooth LE, and Bluetooth Classic through
  its CYW43439 radio

Wi-Fi OTA is marked as a feasible Board VM update path only for targets with a
runtime adapter that already owns that transport. Bluetooth command channels are
tracked separately because OTA over Bluetooth is possible but not the first
practical update path.

Network interface metadata follows the same split. Uno R4 WiFi exposes the
first active Board VM WiFi network adapter, while MKR WiFi 1010, Nano 33 IoT,
Nano RP2040 Connect, Nano ESP32, GIGA R1 WiFi, Portenta H7 Lite Connected,
Portenta C33, Nicla Vision, and Opta WiFi now report their physical WiFi
interfaces with `max_sockets = 0` and no `network.*` capabilities until
board-specific runtime adapters own socket, DNS, and association commands.

Classic shared Arduino targets now expose passive peripheral bus descriptors:
Uno R3, Nano classic, and Pro Mini report their standard Wire/SPI/Serial pins,
while Mega 2560 reports its SDA/SCL header, SPI header, and four hardware UARTs.
These descriptors deliberately leave `i2c.*`, `spi.*`, and `uart.*`
capabilities disabled for the shared Arduino runtime until board-specific
firmware adapters own those bytecode paths.

USB AVR Arduino targets follow the same passive-descriptor shape. Leonardo and
Micro report ATmega32U4 Wire pins on D2/D3, ICSP SPI pins, and the D0/D1
`Serial1` hardware UART, while native USB Serial remains upload/transport
metadata instead of being modeled as a GPIO UART.

The Nano Every megaAVR tranche keeps the Nano form factor on the same metadata
path by reporting A4/A5 Wire, D11/D12/D13 SPI with D10 as the default chip
select, and D0/D1 `Serial1`. Its USB serial bridge remains upload/transport
metadata rather than a second GPIO UART.

Nano R4 follows with the same external header descriptors for its RA4M1 Nano
form factor: A4/A5 `Wire`, D11/D12/D13 SPI with D10 chip select, and D0/D1
`Serial1`. The native USB path remains upload/transport metadata, and the
separate Qwiic `Wire1` connector stays out of this header-pin tranche.

Nano R4 also exposes passive CAN metadata for its D4/D5 header pins. The
descriptor records the RA4M1 CAN0 controller and external transceiver
requirement, while `can.*` bytecode capabilities remain disabled for the shared
Arduino runtime.

Nano R4 RTC metadata records the RA4M1 real-time clock as a passive descriptor.
The shared Arduino runtime still leaves `rtc.*` bytecode adapters disabled until
a board-specific firmware path owns clock access.

Nano R4 Qwiic metadata records the board-local `Wire1` connector separately
from A4/A5 `Wire` header pins. The descriptor names the Qwiic/STEMMA QT
connector and RA4M1 IIC0 controller while the shared Arduino runtime keeps
`i2c.*` bytecode adapters disabled.

Nano R4 native USB metadata records the board-local CDC serial/upload endpoint
separately from D0/D1 `Serial1`. The descriptor keeps the native USB path in
target/upload metadata, not in GPIO UART tables, and leaves deeper
board-specific USB firmware behavior to later adapters.

Host-side validation:

```sh
cargo test -p board-vm-targets
```
