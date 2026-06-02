# Changelog — iir-to-intel8008

All notable changes to this crate are documented here.
Format follows [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).

## [0.1.0] — 2026-06-02 (A2 — crate skeleton)

### Added — `HLT`-only emission

First release.  Implements item A2 of the
[multi-language architecture backends plan][plan]: a crate skeleton
that lowers any IIR module to a single Intel 8008 `HLT` instruction
(opcode `0x76`).

#### Public surface

```rust
pub struct IIRIntel8008Config { pub module_name: String }
impl IIRIntel8008Config {
    pub fn new(module_name: impl Into<String>) -> Self;
}

pub enum IIRIntel8008Error {
    ValidationFailed(Vec<String>),
    UnsupportedOp     { function: String, op: String },
    UnsupportedType   { function: String, type_hint: String },
    InvalidOperand    { function: String, detail: String },
}

pub fn validate_for_intel8008(module: &IIRModule) -> Vec<String>;
pub fn lower_iir_to_intel8008(
    module: &IIRModule,
    cfg: &IIRIntel8008Config,
) -> Result<Vec<u8>, IIRIntel8008Error>;

pub const HLT: u8 = 0x76;
```

#### Why an Intel 8008 backend?

The 8008 (1972) is the first commercial 8-bit microprocessor and is
**Oct's native target** — Oct programs are written specifically to
round-trip through 8008 silicon (or the in-tree `intel8008-simulator`).
A2 establishes the second architecture backend alongside RV32I (A1) and
lays the groundwork for A4 (Intel 4004), which shares the historical-
microprocessor backend shape.

#### Why `Vec<u8>` output, not textual asm?

* **Round-trips with `intel8008-simulator`** — `Simulator::run` takes
  raw `&[u8]` instruction streams directly.
* **Deterministic test surface** — `assert_eq!(bytes, vec![0x76])` is
  unambiguous; 8008 mnemonics have Intel-spec vs MCS-8 historical
  divergence.
* **Trivial output size** — 8008 instructions are 1, 2, or 3 bytes;
  textual round-trip contributes nothing.

#### What is NOT in v0.1.0

* **No instruction lowering.**  Function bodies in the input
  `IIRModule` are ignored.  v0.2.0 (A2+) lowers MVI / MOV / basic
  arithmetic.
* **No `lang-aot --target=intel8008`.**  Deferred to v0.4.0 (A2+++).
* **No external assembler / linker integration.**

#### Tests added (6 total)

* `validate_returns_empty_for_empty_module`
* `lower_emits_exactly_one_byte`
* `lower_emits_the_canonical_hlt_byte` (exact `0x76`)
* `default_config_has_nonempty_module_name`
* `new_sets_module_name`
* `errors_display_without_panic`

[plan]: ../../../specs/MULTILANG-ARCHITECTURE-BACKENDS.md
