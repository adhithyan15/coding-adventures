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

The shared target registry also records physical wireless radios for the
Arduino-family boards that have Wi-Fi or BLE hardware, including MKR/Nano
NINA-based boards, Nano ESP32, GIGA, connected Portenta/Nicla variants, and Opta
WiFi. Those descriptors do not add command or OTA capabilities by themselves;
board-specific firmware and upload adapters still own the actual transport
behavior.

Wi-Fi-capable shared Arduino descriptors also expose passive network-interface
metadata through `board-vm-targets`. The metadata tells language frontends which
board has a physical Wi-Fi stack while leaving `network.*` capabilities disabled
until a concrete board adapter can own sockets, DNS, and association behavior.

The target registry also carries passive classic-Arduino peripheral metadata for
standard Wire/SPI/Serial pins on Uno R3, Nano classic, and Pro Mini, plus the
Mega 2560 SDA/SCL, SPI, and multi-UART surface. These are descriptors only: the
shared Arduino runtime still exposes the GPIO/time MVP until concrete board
adapters own I2C, SPI, and UART bytecode operations.

Leonardo and Micro extend that passive bus metadata to ATmega32U4 boards by
reporting D2/D3 Wire, ICSP SPI, and D0/D1 `Serial1`; native USB remains upload
and transport metadata, not a GPIO UART.

Nano Every extends the passive Nano metadata surface for megaAVR by reporting
A4/A5 Wire, D11/D12/D13 SPI with D10 as the default chip-select pin, and D0/D1
`Serial1`; its USB serial bridge remains upload/transport metadata.

Nano R4 mirrors that header-pin metadata for the Renesas RA4M1 Nano form
factor: A4/A5 `Wire`, D11/D12/D13 SPI with D10 chip select, and D0/D1
`Serial1`. It also records D4/D5 CAN metadata with the external transceiver
requirement plus passive RA4M1 RTC metadata. Native USB, Qwiic `Wire1`, and RTC
bytecode behavior remain owned by later board-specific adapter tranches.

Opta terminals are modeled as board-local input and relay-output pins instead
of pretending the PLC has Uno-style headers. Nicla sensor/audio/vision hardware
is similarly registered as its own target family so follow-up capability work
can add the real sensor, camera, microphone, and wireless paths without changing
language frontends.

Host-side validation:

```sh
cargo test -p board-vm-arduino
```
