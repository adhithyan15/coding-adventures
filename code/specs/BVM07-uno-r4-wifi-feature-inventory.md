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
| DAC output | A0, one 12-bit DAC channel | pending | `dac.write_u12` or `analog.write` |
| I2C master | `Wire` on A4/A5, `Wire1` on Qwiic D27/D26 | pending | `i2c.open`, `i2c.write`, `i2c.read`, `i2c.transfer` |
| SPI master | MOSI D11, MISO D12, SCK D13, CS D10 | pending | `spi.open`, `spi.transfer`, `spi.close` |
| Hardware UART | SerialUSB plus UART pin pairs D22/D23, D1/D0, D24/D25 | partially transport-only | `uart.open`, `uart.write`, `uart.read`, transport descriptors |
| CAN | CAN0 TX D10, RX D13 | pending | `can.open`, `can.write`, `can.read` |
| RTC | one RTC instance in the core | pending | `rtc.now`, `rtc.set`, alarms/events later |
| EEPROM/flash emulation | 8 KiB flash-backed EEPROM area | pending | `program.store` and possibly `kv.store` |
| Watchdog | Renesas WDT library | pending | `watchdog.configure`, `watchdog.kick` |
| OPAMP | UNO R4 OPAMP library | pending | analog board-specific capability after ADC/DAC |
| WiFi | WiFiS3 library through onboard network module | pending | network/transport capabilities, not direct Ruby bypass |
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
3. `pwm.write` and `adc.read` are the first direct analog-adjacent VM
   operations. Add handle-oriented PWM/ADC variants only when a later streaming
   or event tranche needs persistent resource ownership.
4. Add DAC output on A0 as the next scalar analog capability.
5. Implement I2C and SPI as bus handles with explicit transfer boundaries.
6. Add UART, CAN, RTC, watchdog, EEPROM/store, and WiFi/network
   capabilities in separate tranches with conformance tests.

Every tranche should include the same layers: spec entry, IR capability id,
runtime HAL method, UNO R4 target descriptor, firmware backend, host builder,
language SDK wrapper, and fake-backend tests.
