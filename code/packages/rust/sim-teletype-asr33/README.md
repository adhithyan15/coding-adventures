# sim-teletype-asr33

Cycle-accurate Teletype Model 33 ASR (Automatic Send-Receive) simulator featuring both a pure zero-heap-allocation functional model (`Asr33Functional` / `Asr33FrameState`) and a gate-level digital logic model (`Asr33GateLevel`) constructed from primitive logic gates, with 72-column platen paper-roll rendering via `paint-instructions` and `paint-vm-ascii` (`P2D02`).

Part of the **Coding Adventures** polyglot historical game engine collection (`GAME03a`).

---

## Historical Lineage

- **1963 — Teletype Model 33 ASR:** Introduced by the Teletype Corporation (Western Electric / AT&T), the Model 33 was the defining human-machine interface for early minicomputers (DEC PDP-8, PDP-11, HP 2100, GE-225). Operating at 110 baud (10 characters per second) using an 11-unit asynchronous serial frame code (1 Start bit, 7 ASCII data bits, 1 even parity bit, 2 Stop bits).
- **The 72-Column Platen & Upper-Case Drum:** The Model 33 used a cylindrical typewheel containing a 64-character subset of ASCII (`0x20..=0x5F`). Lowercase characters `'a'..='z'` were physically folded to uppercase `'A'..='Z'` upon printing. The carriage advanced across 72 columns before auto-wrapping.

---

## Architectural Guarantees

In accordance with the repository's verification architecture (`VER00`, `VER04m`, `VER07`, `VER08a`, `VER08b`, `VER15`, `VER27`):

- **`CAP001` (Zero Ambient Authority):** Declares `"capabilities": []` in `required_capabilities.json`. The package contains zero filesystem (`fs`), network (`net`), process (`proc`), environment variable (`env`), system time (`time`), standard I/O (`stdin`/`stdout`), system random (`rand`), or multithreading (`thread`) access.
- **`CAP002` (Zero Unsafe / Safe Rust):** Compiled with `#![forbid(unsafe_code)]`. Prohibits raw pointers, manual memory allocation, and FFI reachability.
- **Dual-Model Equivalence:** Contains both a pure zero-alloc functional model (`Asr33Functional` / `Asr33FrameState`) and a digital gate-level logic model (`Asr33GateLevel`) built on `logic_gates::sequential` (`shift_register`, `d_flip_flop`, `FlipFlopState`) and `logic_gates::gates` (`xor_gate`).
- **Exhaustive Differential Equivalence:** Proved across all 256 byte values and all 11 serial bit clocks, matching every serial wire bit, parity bit, teleprinter flag (`tp_flag`), and keyboard flag (`kb_flag`).
- **Zero Allocation Budget (`VER07`):** All hot-path simulation functions (`step_serial_bit`, `tx_load`, `rx_feed_bit`, `kbd_push_byte`, `kbd_read_clear`, `tp_clear_flag`) execute with **zero dynamic memory allocations** (`max_alloc_bytes: Some(0)`).
- **Declarative 2D Telemetry (`P2D02`):** Generates intermediate representation `PaintScene` rendered through `paint-vm-ascii` displaying the 72-column paper roll and carriage print-head cursor (`█`).
- **26-CWE Weakness Closure (`VER15`):** Complete closure or formal exclusion of all 26 canonical CWE classes.

---

## Domain Types & API

### 1. `Asr33FrameState`

Copy struct maintaining cycle-accurate register and serial wire states:

```rust
pub struct Asr33FrameState {
    pub tx_shift_reg: u16,
    pub tx_bits_remaining: u8,
    pub tx_line: u8,
    pub rx_shift_reg: u16,
    pub rx_bits_received: u8,
    pub rx_line: u8,
    pub rx_active: bool,
    pub kb_buffer: u8,
    pub kb_flag: bool,
    pub tp_buffer: u8,
    pub tp_flag: bool,
    pub platen_col: u8,
    pub platen_row: u32,
    pub bel_count: u32,
    pub last_printed_char: Option<u8>,
}
```

### 2. `Asr33Functional`

Zero-alloc functional teletype simulator with 64-line circular paper roll buffer:

```rust
pub struct Asr33Functional {
    pub state: Asr33FrameState,
    pub paper: [[u8; PLATEN_WIDTH]; MAX_PAPER_LINES],
    pub line_lengths: [u8; MAX_PAPER_LINES],
}
```

Key operations:
- `tx_load(byte)`: loads byte, clears `tp_flag`, constructs 11-bit serial frame.
- `step_serial_bit() -> u8`: clocks 1 serial bit time, returns TX wire level.
- `rx_feed_bit(bit) -> Option<u8>`: feeds serial bit into receiver.
- `kbd_push_byte(byte)`: pushes key to KRB with parity, sets `kb_flag`.
- `kbd_read_clear() -> u8`: reads KRB, clears `kb_flag`.
- `print_char(ch)`: processes character on platen (wrap at col 72, CR, LF, BEL, uppercase fold).
- `to_paint_scene() -> PaintScene`: translates paper roll to 2D scene.
- `render_paper_roll_ascii() -> String`: renders ASCII paper roll + print carriage cursor.

### 3. `Asr33GateLevel`

Gate-level digital logic simulator constructed with `logic-gates`:

```rust
pub struct Asr33GateLevel {
    pub tx_shift_ff: [FlipFlopState; 11],
    pub krb_ff: [FlipFlopState; 8],
    pub tto_ff: [FlipFlopState; 8],
    pub kb_flag_ff: FlipFlopState,
    pub tp_flag_ff: FlipFlopState,
    pub tx_line: u8,
    pub tx_bits_remaining: u8,
    pub rx_shift_ff: [FlipFlopState; 8],
    pub rx_line: u8,
    pub rx_bits_received: u8,
    pub rx_active: bool,
    pub tx_saved_byte: u8,
}
```

---

## Building and Testing

```bash
# Run unit and integration tests
cargo test -p sim-teletype-asr33 -- --nocapture

# Run release-mode tests
cargo test -p sim-teletype-asr33 --release -- --nocapture

# Verify capability confinement and zero ambient authority
cargo run --manifest-path ../ca-capability-analyzer/Cargo.toml -- --dir . --check-module
cargo run --manifest-path ../ca-capability-analyzer/Cargo.toml -- --dir . --verify-cert effect_certificate.json
```
