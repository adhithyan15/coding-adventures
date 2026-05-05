# BVM06 - Board VM Target Matrix

## Overview

Board VM must not accidentally become "the boards that Rust already supports
comfortably today." The IR, protocol, host SDKs, and conformance tests are
intended to outlive any one chip family. This spec makes the intended board and
ISA targets explicit so implementation choices stay portable.

The rule is:

```
If a board family is important and Rust support is missing, Rust support becomes
part of the backend work. The board family is not excluded from Board VM.
```

Everything is assembly eventually. The firmware runtime is Rust because Rust is
the implementation language we want, not because we only support chips with a
polished Rust ecosystem.

## Layer Position

```
BVM00 architecture
        |
        v
BVM06 target matrix
        |
        +--> target descriptors
        +--> Rust codegen/compiler support needs
        +--> HAL adapter needs
        +--> runtime profile limits
        +--> eject and AOT backend declarations
        |
        v
BVM03 Rust runtime + board target crates
```

This spec is a planning and compatibility contract. It does not define a new
wire protocol or bytecode instruction.

## Target Family Model

A Board VM target is described at three levels:

| Level | Example | Meaning |
|---|---|---|
| ISA/core family | `armv7e-m`, `avr`, `mcs51`, `rv32imc`, `xtensa-lx6` | What the Rust runtime eventually compiles to |
| MCU family | RA4M1, RP2040, STM32F4, ATmega328P, AT89S52 | Memory map, peripherals, timers, flash model |
| Board family | Uno R4, Pico, Nucleo, Uno R3, 8051 dev kit | Pin names, onboard LED, transport, flashing path |

Board VM should reuse one runtime core across all of them. The target-specific
crate supplies:

- board descriptor,
- memory limits,
- pin map,
- capability table,
- HAL bindings,
- flashing/eject metadata,
- optional AOT metadata,
- optional panic/recovery policy.

## Runtime Profiles

The same bytecode format can run with different resource profiles.

| Profile | Intended targets | Shape |
|---|---|---|
| `full` | Uno R4, RP2040, RP2350, STM32, ESP32 | larger stack, more handles, background run, optional stored programs |
| `small` | low-end Cortex-M0/M0+, smaller STM32, SAMD21 | smaller stack/handles, GPIO/time/PWM first |
| `tiny` | ATmega, ATtiny, 8051/MCS-51, PIC-class targets | minimal stack, few handles, no heap, maybe foreground only |

Profiles are limits, not separate languages. A host SDK asks the board
descriptor for its actual limits and compiles accordingly.

## Primary Target Matrix

### Arduino / Renesas RA

| Board family | First boards | Core / ISA | Profile | Notes |
|---|---|---|---|---|
| Arduino Uno R4 | Uno R4 Minima, Uno R4 WiFi | Renesas RA4M1, Arm Cortex-M4F, Armv7E-M | `full` | First implementation target. Good modern Arduino baseline. |

Uno R4 is the first target because it is the hardware in hand and has enough
memory for a comfortable runtime.

### Raspberry Pi Pico

| Board family | First boards | Core / ISA | Profile | Notes |
|---|---|---|---|---|
| Pico 1 | Pico, Pico H, Pico W, Pico WH | RP2040, dual Arm Cortex-M0+, Armv6-M | `full` or `small` | Excellent docs, cheap boards, PIO is a later advanced capability. |
| Pico 2 | Pico 2, Pico 2 W | RP2350, dual Arm Cortex-M33 or dual Hazard3 RISC-V | `full` | Important because one board family spans Arm and RISC-V execution modes. |

Pico support should prove that the board target layer is not tied to Arduino
pin numbering.

### ESP32

| Board family | First boards | Core / ISA | Profile | Notes |
|---|---|---|---|---|
| ESP32 classic | ESP32 DevKit | Xtensa LX6 | `full` | Huge ecosystem, Wi-Fi/BLE, non-Arm compiler path. |
| ESP32-S3 | ESP32-S3 DevKit | Xtensa LX7 | `full` | Modern Xtensa target with USB and strong maker adoption. |
| ESP32-C3 | ESP32-C3 DevKit | 32-bit RISC-V, RV32IMC | `full` | Good first ESP32 target if we want upstream-friendly RISC-V codegen. |
| ESP32-C6/H2 | ESP32-C6, ESP32-H2 boards | 32-bit RISC-V | `full` | Later wireless targets for Wi-Fi 6 / Thread / BLE use cases. |

ESP32 support should stress transport and wireless assumptions, but the first
Board VM pass should still use serial/USB transport.

### STM32

| Board family | First boards | Core / ISA | Profile | Notes |
|---|---|---|---|---|
| STM32 Nucleo-32 | NUCLEO-G431KB, NUCLEO-F303K8 | Arm Cortex-M | `small` or `full` | Compact boards; Nano-style headers on some variants. |
| STM32 Nucleo-64 | NUCLEO-F446RE, NUCLEO-G491RE, NUCLEO-L152RE | Arm Cortex-M | `full` | Strong first STM32 target class; Arduino Uno headers plus ST-LINK. |
| STM32 Nucleo-144 | NUCLEO-H723ZG, NUCLEO-U575ZI-Q | Arm Cortex-M | `full` | Larger devices for richer peripherals and more handles. |

STM32 should be treated as a family of target descriptors over common STM32 HAL
patterns, not as one board.

### Classic Arduino AVR

| Board family | First boards | Core / ISA | Profile | Notes |
|---|---|---|---|---|
| Arduino Uno R3 | Uno R3, Uno R3 SMD | ATmega328P, 8-bit AVR | `tiny` | Historic Arduino baseline; memory pressure is the main design challenge. |
| Arduino Nano classic | Nano 3.x | ATmega328 / ATmega328P, 8-bit AVR | `tiny` | Same conceptual target as Uno R3 with different board shape. |
| Arduino Mega | Mega 2560 Rev3 | ATmega2560, 8-bit AVR | `tiny` or `small` | More flash/SRAM and many pins/UARTs; good larger AVR target. |
| Arduino Leonardo/Micro | Leonardo, Micro | ATmega32U4, 8-bit AVR with USB | `tiny` | Useful because USB is on the main MCU. |
| Pro Mini / compatible | Pro Mini class | ATmega328P, 8-bit AVR | `tiny` | No built-in USB transport; external serial adapter expected. |

AVR is not 8051. It is Atmel/Microchip's 8-bit AVR RISC family. It deserves a
dedicated target family because it is central to Arduino history and education.

### ATtiny / tinyAVR

| Board family | First boards | Core / ISA | Profile | Notes |
|---|---|---|---|---|
| ATtiny classic | ATtiny85 dev boards, Digispark-style boards | 8-bit AVR | `tiny` | Very constrained; likely GPIO/time first only. |
| modern tinyAVR | ATtiny1607/3217/3227 class boards | 8-bit AVR | `tiny` | Newer peripherals may make capability mapping more interesting. |

ATtiny support is important as a lower bound. If a feature cannot degrade
gracefully to ATtiny, the descriptor must expose that clearly.

### Atmel / Microchip 8051

| Board family | First boards | Core / ISA | Profile | Notes |
|---|---|---|---|---|
| AT89 classic | AT89C51, AT89S51, AT89S52 dev boards | MCS-51 / 8051-compatible | `tiny` | Historic Atmel 8051-compatible family. |
| AT89LP | AT89LP series boards | enhanced 8051-compatible | `tiny` | Faster single-cycle-style derivatives; still an 8051-family target. |
| Silicon Labs EFM8 | EFM8 starter kits | CIP-51 / 8051-compatible | `tiny` | Modern 8051-family chips with better tools and peripherals. |
| STC / Nuvoton 8051 | common low-cost dev boards | 8051-compatible | `tiny` | Later support once the generic MCS-51 backend exists. |

8051/MCS-51 support must be explicit even if Rust support is missing at first.
The work item is to make Rust reach the target, whether through an LLVM backend,
a custom codegen path, or a restricted runtime subset lowered through assembly.

### Atmel / Microchip SAM

| Board family | First boards | Core / ISA | Profile | Notes |
|---|---|---|---|---|
| Arduino Due | Due | SAM3X8E, Arm Cortex-M3 | `full` | Important older 32-bit Arduino. |
| Arduino Zero / MKR | Zero, MKR class | SAMD21, Arm Cortex-M0+ | `small` | Common Arduino Arm family. |
| SAMD51 boards | Metro M4, Feather M4-style boards | SAMD51, Arm Cortex-M4F | `full` | Popular high-capability maker boards. |

SAM targets bridge classic Arduino UX and modern Arm Cortex-M firmware.

### MBed-Style Boards

| Board family | First boards | Core / ISA | Profile | Notes |
|---|---|---|---|---|
| NXP LPC1768 | mbed LPC1768 | Arm Cortex-M3 | `full` | Original mbed-style board class. |
| NXP FRDM | FRDM-K64F, FRDM-KL25Z | Arm Cortex-M | `small` or `full` | Common education/dev boards. |
| STM32 Nucleo via Mbed | Nucleo boards with Mbed support | Arm Cortex-M | `small` or `full` | Should share STM32 target crates where possible. |

Mbed OS/platform itself is heading to end-of-life, so it must not become a
foundation dependency. Mbed is a compatibility layer and board lineage, not the
core Board VM architecture.

### Other Future Families

| Family | Examples | Why it matters |
|---|---|---|
| PIC | PIC16, PIC18, PIC24/dsPIC | Microchip ecosystem beyond AVR/8051; severe `tiny` constraints for 8-bit parts. |
| MSP430 | LaunchPad boards | Different low-power 16-bit architecture. |
| RISC-V MCUs | CH32V, GD32VF, ESP32-C3/C6 | Increasingly common open ISA target class. |
| Nordic nRF52/nRF53 | micro:bit v2, BLE dev kits | BLE-first Arm Cortex-M boards. |
| Teensy | Teensy 3.x/4.x | High-performance maker boards; good stress test for richer capabilities. |

These are not first-wave targets, but the architecture should not rule them out.

## Rust Backend Policy

Board VM runtime code is Rust. For each target family:

1. Prefer an existing stable Rust target if it exists.
2. If the target exists but embedded HAL support is incomplete, build the HAL
   adapter crate.
3. If the Rust target exists only out-of-tree or nightly, document that status
   and keep the target descriptor separate from the runtime core.
4. If no Rust target exists, create a backend work item. Options include:
   - upstream or local LLVM backend work,
   - Cranelift/codegen backend work,
   - restricted Rust subset lowered through a custom assembly emitter,
   - hand-written assembly runtime shim for the smallest `tiny` profile.

Missing Rust support changes the implementation plan. It does not change the
Board VM target matrix.

## Eject Backend Policy

Every target should declare which eject modes it supports:

| Eject mode | Meaning | Required target support |
|---|---|---|
| `bytecode-store` | Store a BVM module in board nonvolatile storage | safe flash/EEPROM/filesystem write path |
| `embedded-bytecode` | Build firmware containing the VM and a static bytecode module | board firmware build and flashing path |
| `aot-native` | Lower bytecode into target-native firmware and omit the interpreter | native lowering backend plus semantic conformance tests |

AOT is an optimization over the portable bytecode contract. The bytecode module
remains the session artifact, conformance artifact, and cross-language format.
Target AOT backends may be implemented through direct assembly emission, target
IR generation, Rust or LLVM artifacts, or another compiler path that can be
validated against VM execution.

`tiny` targets are a major reason to keep AOT explicit. ATmega, ATtiny, 8051,
PIC-class, and similar boards may benefit from eliminating the interpreter in
final firmware even when the interactive VM remains the development path on
larger boards or on a reduced target profile.

If `aot-native` is unsupported for a board, the host must still be able to use
the VM-backed eject modes when the board has enough storage and runtime
capacity. If none of the eject modes are available, the target can still be a
descriptor and fake-backend target, but it is not complete for standalone use.

## Capability Rollout by Profile

| Capability | `full` | `small` | `tiny` |
|---|---|---|---|
| `gpio.digital` | required | required | required |
| `time.sleep` | required | required | required |
| `serial.log` | required for dev builds | optional | optional |
| `program.ram_exec` | required | required | optional |
| `program.store` | optional | optional | optional |
| `pwm.output` | early | early | later |
| `adc.input` | early | early | later |
| `i2c.master` | early | later | unlikely first |
| `spi.master` | early | later | unlikely first |
| events/interrupts | later | later | profile-specific |

The host must always use the actual board descriptor, not a hardcoded profile
assumption.

## Conformance Requirements

Every target crate must provide:

- descriptor tests for MCU/core/ISA/memory/pin metadata,
- capability-table tests,
- blink bytecode execution against a fake backend,
- invalid-pin tests,
- stack/handle limit tests for its profile,
- at least one transport declaration,
- an eject strategy declaration: stored bytecode, embedded firmware,
  AOT-native, or none.

Targets that cannot yet compile Rust firmware may still provide descriptor and
fake-backend tests. That lets the support matrix advance before compiler work is
complete.

## Future Extensions

- Machine-readable target descriptor schema.
- Generated target tables for host SDKs.
- Automatic conformance test runner across all target crates.
- Runtime-size reports per target profile.
- Rust backend status dashboard for unsupported ISAs.
- AOT backend status dashboard for targets that can eliminate the VM at eject.
- Per-target flashing/eject tool integration.
