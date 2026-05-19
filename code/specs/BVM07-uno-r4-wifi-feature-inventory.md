# BVM07 - Arduino UNO R4 WiFi Feature Inventory

## Overview

The Arduino UNO R4 WiFi is the first concrete Board VM hardware target. This
inventory records the features visible from the installed Arduino Renesas UNO
core and maps them into Board VM capability work. The intent is to keep host
SDKs thin: Ruby, Python, Lua, and CLI clients should discover capabilities from
the board descriptor and lower calls into BVM bytecode or protocol messages,
not bypass the VM with ad hoc sketches.

Source snapshot:

- Arduino core: `arduino:renesas_uno` version `1.5.3`.
- Board id: `arduino:renesas_uno:unor4wifi`.
- Variant: `UNOWIFIR4`.
- MCU/core: Renesas RA4M1, Arm Cortex-M4F.
- Firmware upload: `bossac`/SAM-BA with 1200 bps bootloader touch.

## Capability Ledger

| Board feature | Board surface | VM status | Proposed VM capability |
|---|---|---|---|
| Digital GPIO | D0-D19, with A0-A5 also usable as digital pins | implemented | `gpio.open`, `gpio.write`, `gpio.read`, `gpio.close` |
| Built-in LED | D13 | implemented through GPIO | GPIO metadata should identify `onboard_led_pin = 13` |
| Millisecond time | runtime clock/sleep | implemented | `time.sleep_ms`, `time.now_ms` |
| RAM program execution | protocol upload/run path | implemented | `program.ram_exec` |
| LED matrix | 12x8 matrix, 96 pixels, UNO R4 WiFi only | implemented | `led_matrix.frame` |
| PWM output | D3, D5, D6, D9, D10, D11 in the Arduino variant init path | implemented as direct write | `pwm.write` |
| Analog input | A0-A5 | implemented as direct read | `adc.read` |
| DAC output | A0, one 12-bit DAC channel | implemented as direct write | `dac.write_u12` |
| I2C master | `Wire` on A4/A5, `Wire1` on Qwiic D27/D26 | bus metadata, handle open, single-byte write/read, byte-buffer write/read, and write-read transfer implemented | `i2c.open`, `i2c.write_u8`, `i2c.read_u8`, `i2c.write`, `i2c.read`, `i2c.transfer` |
| SPI master | COPI D11, CIPO D12, SCK D13, conventional CS D10 | bus metadata, handle open, bounded write-then-read transfer, and transfer-backed read/write SDK commands implemented | `spi.open`, `spi.transfer`; SDK `spi.write`/`spi.read` lower to transfer |
| Hardware UART | SerialUSB plus UART pin pairs D22/D23, D1/D0, D24/D25 | descriptor metadata, handle open, and single-byte write/read implemented | `uart.open`, `uart.write`, `uart.read`, transport descriptors |
| CAN | CAN0 TX D10, RX D13 | descriptor metadata, handle open, and single-byte write/read implemented | `can.open`, `can.write`, `can.read` |
| RTC | one RTC instance in the core | descriptor metadata and direct bytecode operations implemented | `rtc.now`, `rtc.set`, alarms/events later |
| EEPROM/flash emulation | 8 KiB flash-backed EEPROM area | descriptor metadata, bounded bytecode read/write, and region-size query implemented | `storage.write`, `storage.read`, `storage.size`; stored program / KV layers later |
| Watchdog | Renesas WDT library | descriptor metadata and direct bytecode operations implemented | `watchdog.configure`, `watchdog.kick` |
| OPAMP | UNO R4 OPAMP library | pending | analog board-specific capability after ADC/DAC |
| WiFi | WiFiS3 library through onboard network module | descriptor metadata, network capability strings, first TCP/UDP socket bytecode tranches, bounded TCP/UDP socket-control queries, bounded UDP payload I/O, bounded WiFi association control, bounded DNS resolution, resolver server policy, first DNS query/response message payloads, first bounded DNS-over-UDP exchange, retry/backoff DNS-over-UDP exchange policy, and resolver fallback policy implemented | `transport.wifi`, `network.ipv4`, `network.tcp`, `network.udp`, `network.dns`, `network.tcp.open`, `network.tcp.write`, `network.tcp.read`, `network.tcp.close`, `network.tcp.connected`, `network.tcp.available`, `network.udp.open`, `network.udp.write`, `network.udp.read`, `network.udp.write_bytes`, `network.udp.read_bytes`, `network.udp.available`, `network.udp.close`, `network.wifi.associate`, `network.wifi.disconnect`, `network.wifi.status`, `network.dns.resolve`, `network.dns.set_server`, `network.dns.query`, `network.dns.response_ipv4`, `network.dns.exchange_udp`, `network.dns.exchange_udp_retry`, `network.dns.exchange_udp_fallback` |
| USB/HID | TinyUSB device support | pending | transport/device descriptors first; HID later |
| OTA/SDU | OTAUpdate and SDU libraries | pending | firmware-management capability, separate from bytecode VM |

## Internal and Reserved Pins

The variant exposes more pin indices than the user-facing D0-D19/A0-A5 header:

- D20 measures AVCC and should be board metadata, not a general GPIO.
- D21 controls the USB switch and should be reserved by the firmware.
- D22/D23 are a UART pair.
- D24/D25 are the WiFi-module UART pair.
- D26/D27 are the Qwiic I2C pins.
- D28-D38 are used by the UNO R4 WiFi LED matrix driver and must be reserved
  while `led_matrix.frame` is active.

The target descriptor should eventually distinguish header pins, internal
service pins, transport pins, and board-owned display pins so the runtime can
reject conflicting handles.

## Rollout Order

1. Keep `gpio.*`, `time.*`, `program.ram_exec`, transports, and
   `led_matrix.frame` as the proven vertical slice.
2. Descriptor metadata for header pins, onboard LED, LED matrix dimensions,
   PWM-capable pins, and analog-capable pins is present; keep deepening it with
   hidden pins, reserved pins, and bus pin groups.
3. `pwm.write`, `adc.read`, and `dac.write_u12` are the first direct
   analog-adjacent VM operations. Add handle-oriented variants only when a
   later streaming or event tranche needs persistent resource ownership.
4. Implement I2C and SPI as bus handles with explicit transfer boundaries;
   `i2c.open` establishes the first I2C handle tranche, while `i2c.write_u8`
   and `i2c.read_u8` prove the Rust-owned transfer path through Arduino
   `Wire`/`Wire1`. `i2c.write`, `i2c.read`, and `i2c.transfer` cover the
   bounded byte-buffer transfer path. `spi.open` establishes the SPI handle
   tranche over Rust-owned UNO R4 bus metadata; `spi.transfer` layers a bounded
   chip-select-scoped write-then-read transaction on that handle, with host SDK
   `spi.write` and `spi.read` wrappers covering write-only and read-only SPI
   cases without adding duplicate VM opcodes.
5. Add UART, CAN, RTC, watchdog, EEPROM/store, and WiFi/network
   capabilities in separate tranches with conformance tests. UART now has
   descriptor metadata for Minima `Serial1` and WiFi `Serial1`/`Serial2`/
   `Serial3`, while `uart.open`, `uart.write`, and `uart.read` establish the
   first UART handle and byte I/O tranche. CAN now has descriptor metadata for
   UNO R4 `CAN0` on D10/TX and D13/RX, and `can.open`, `can.write`, and
   `can.read` establish the first CAN handle and byte I/O tranche. RTC now has
   descriptor metadata for the single UNO R4 `RA4M1 RTC` instance, and `rtc.now`
   plus `rtc.set` establish the first direct RTC bytecode tranche. Watchdog now
   has descriptor metadata for the UNO R4 `RA4M1 WDT` instance, and
   `watchdog.configure` plus `watchdog.kick` establish the first direct
   watchdog bytecode tranche.
   EEPROM/store now has descriptor metadata for the 8 KiB flash-backed
   EEPROM/data-flash area, and `storage.write`, `storage.read`, plus
   `storage.size` establish bounded byte-buffer I/O and region capacity queries
   for that region. WiFi/network now has
   Rust-owned descriptor metadata for the onboard WiFiS3 interface plus
   `network.ipv4`, `network.tcp`, `network.udp`, and `network.dns` capability
   strings surfaced through the language target APIs. `network.tcp.open`,
   `network.tcp.write`, `network.tcp.read`, `network.tcp.close`, `network.udp.open`,
   `network.udp.write`, `network.udp.read`, `network.udp.available`, and
   `network.udp.close` now establish the first socket bytecode tranches with
   runtime handle-table ownership, host builders, device capability descriptors,
   and UNO R4 WiFi backend hooks, and `network.tcp.connected` now establishes
   the first bounded socket-control query over an existing persistent TCP handle.
   `network.tcp.available` adds TCP read-readiness over that same persistent
   handle shape, `network.udp.available` mirrors the readiness control path for
   UDP, and `network.udp.write_bytes`/`network.udp.read_bytes` add bounded UDP
   payload I/O so later DNS UDP exchange code can stay thin over Rust-owned
   transport bytecode. `network.wifi.associate`,
   `network.wifi.disconnect`, and `network.wifi.status` now establish bounded
   association control over the same byte-buffer operand path used by storage
   and bus transfers, while `network.dns.resolve` establishes the first
   hostname-to-IPv4 bytecode tranche over that bounded byte-buffer operand path.
   `network.dns.set_server` adds the first DNS client-policy bytecode tranche,
   letting host-built bytecode select a resolver IPv4 address while keeping
   language frontends thin over Rust-owned bytecode assembly and target
   metadata. `network.dns.query` adds the first DNS message-payload tranche by
   constructing a bounded recursive A-record query in the Rust runtime.
   `network.dns.response_ipv4` follows with bounded response-payload parsing for
   the first IPv4 A-answer. `network.dns.exchange_udp` now composes the bounded
   DNS query builder with UDP port 53 byte-buffer I/O and returns raw response
   bytes for the parser. `network.dns.exchange_udp_retry` adds the first
   retry/backoff policy layer over that same bounded transport path.
   `network.dns.exchange_udp_fallback` layers primary/fallback resolver policy
   on the same Rust-owned bytecode path, keeping larger response handling for
   later follow-ups.
   `program.store` now has a protocol capability descriptor, `STORE_PROGRAM`
   device dispatch, runtime HAL hook, and an initial UNO R4 storage-backed
   slot-0 layout that writes a compact header plus module chunks through the
   same bounded storage substrate. Higher-level KV storage, larger DNS response
   handling, and deeper socket controls remain later capability tranches.

Every tranche should include the same layers: spec entry, IR/protocol capability id,
runtime HAL method, UNO R4 target descriptor, firmware backend, host builder,
language SDK wrapper, and fake-backend tests.
