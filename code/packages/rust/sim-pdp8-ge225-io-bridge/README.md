# sim-pdp8-ge225-io-bridge

Era 0 functional and gate-level peripheral I/O controller connecting DEC PDP-8 (`pdp8-simulator` + `pdp8-gatelevel`) and GE-225 (`ge225-simulator` + `ge225-gatelevel`) to the Teletype Model 33 ASR (`sim-teletype-asr33`), with end-to-end machine-code and telemetry bindings for `hamurabi` and `lunar-lander`.

Part of the **Coding Adventures** polyglot historical game engine collection (`GAME03b`).

---

## Historical Lineage

- **1965 — DEC PDP-8 KL8-E / W076 Teletype Control:** The PDP-8 peripheral architecture mapped I/O devices via 12-bit Input/Output Transfer (`IOT`) instructions (`6xxx`). Device code `0o03` controlled the keyboard/paper-tape reader, and device code `0o04` controlled the teleprinter/paper-tape punch. Instructions generated sub-pulses IOP1 (skip test), IOP2 (clear flag / clear AC), and IOP4 (read buffer / transmit).
- **1961 — GE-225 Console Typewriter & Paper-Tape Subsystem:** The General Electric GE-225 mainframe communicated via an IBM Selectric-based console typewriter and paper-tape reader/punch using a 6-bit BCD character set (`0o00..=0o76`). Controller commands (`TON`, `TYP`, `HPT`, `RON`, `RPT`, `PON`, `WPT`) interfaced CPU memory with teletype peripherals.

---

## Architectural Guarantees

In accordance with the repository's verification architecture (`VER00`, `VER04m`, `VER07`, `VER08a`, `VER08b`, `VER15`, `VER27`):

- **`CAP001` (Zero Ambient Authority):** Declares `"capabilities": []` in `required_capabilities.json`. The package contains zero filesystem (`fs`), network (`net`), process (`proc`), environment variable (`env`), system time (`time`), standard I/O (`stdin`/`stdout`), system random (`rand`), or multithreading (`thread`) access.
- **`CAP002` (Zero Unsafe / Safe Rust):** Compiled with `#![forbid(unsafe_code)]`. Prohibits raw pointers, manual memory allocation, and FFI reachability.
- **Dual-Model Equivalence:** Contains both pure zero-alloc functional models (`FunctionalIotDecoder`, `Pdp8TeletypeBridge`, `Ge225TeletypeBridge`) and digital gate-level logic models (`GateLevelIotDecoder`, `Pdp8TeletypeGateLevelBridge`) built from `logic_gates` primitives (`and_gate`, `or_gate`, `not_gate`, `mux2`, 6-bit equality comparators, pulse gating, 12-bit AC bus multiplexers).
- **Exhaustive Differential Equivalence:** Proved across all 64 device addresses, all 8 pulse combinations, and exhaustive data words, guaranteeing bit-exact matches for skip signals, AC outputs, transmit strobes, clear flags, and interrupt request lines.
- **Zero Allocation Budget (`VER07`):** All hot-path simulation functions (`FunctionalIotDecoder::decode`, `GateLevelIotDecoder::decode`, `Pdp8TeletypeBridge::step`, `ascii_to_ge225_bcd`, `ge225_bcd_to_ascii`, `Ge225TeletypeBridge::typewriter_output_char`) execute with **zero dynamic memory allocations** (`max_alloc_bytes: Some(0)`).
- **End-to-End Game Telemetry:** Executes authentic PDP-8 polling machine-code loops transmitting `hamurabi` and `lunar-lander` telemetry onto both `Asr33Functional` and `Asr33GateLevel` platen paper rolls, asserting identical state at `HLT`, identical CPU cycles, and identical ASCII rendering.
- **26-CWE Weakness Closure (`VER15`):** Complete closure or formal exclusion of all 26 canonical CWE classes.

---

## Domain Types & API

### PDP-8 Teletype Interface (KL8-E / W076)
- `FunctionalIotDecoder` & `GateLevelIotDecoder`:
  - `0o6031` (`KSF`): Skip if Keyboard Flag is set.
  - `0o6032` (`KCC`): Clear Keyboard Flag and Clear AC.
  - `0o6034` (`KRS`): Read Keyboard Buffer (OR into AC).
  - `0o6036` (`KRB`): Clear AC, Read Keyboard Buffer, Clear Flag (`KCC` + `KRS`).
  - `0o6041` (`TSF`): Skip if Teleprinter Flag is set.
  - `0o6042` (`TCF`): Clear Teleprinter Flag.
  - `0o6044` (`TPC`): Load AC into Teleprinter Transmitter Buffer.
  - `0o6046` (`TLS`): Clear Flag, Load AC into Transmitter (`TCF` + `TPC`).
  - Interrupt line: `kb_flag || tp_flag`.
- `Pdp8TeletypeBridge` & `Pdp8TeletypeGateLevelBridge`: Full bridge coordinating CPU steps, IOT events, interrupt requests, and serial ASR-33 clock cycles.

### GE-225 Console Typewriter & BCD Interface
- `ascii_to_ge225_bcd(ascii: u8) -> u8`: Converts 7-bit ASCII to GE-225 6-bit BCD code (`0o00..=0o76`).
- `ge225_bcd_to_ascii(bcd: u8) -> u8`: Converts GE-225 6-bit BCD code to 7-bit uppercase ASCII.
- `Ge225TeletypeBridge`: Bridges GE-225 typewriter output and paper-tape reader streams into ASR-33 functional and gate-level teleprinters.

### Telemetry & Machine-Code Helpers
- `build_pdp8_teletype_print_program`: Emits PDP-8 machine-code vectors utilizing auto-index register `0o0010` and teleprinter polling loops (`TSF` / `JMP .-1` / `TLS`).
