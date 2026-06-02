# iir-to-intel8008 — IIR → Intel 8008 machine code backend

**Status:** v0.1.0 — skeleton (A2)
**Plan:** [`MULTILANG-ARCHITECTURE-BACKENDS.md`](MULTILANG-ARCHITECTURE-BACKENDS.md) §A2
**Related:** [`iir-to-riscv`][rv], [`intel8008-simulator`][sim]

[rv]: ../packages/rust/iir-to-riscv/
[sim]: ../packages/rust/intel8008-simulator/

## Why a new crate?

The Intel 8008 (1972) is the first commercial 8-bit microprocessor. In
this codebase it's **Oct's native target** — Oct programs are written
to round-trip through 8008 silicon (or the in-tree
[`intel8008-simulator`][sim]).

Adding the 8008 as a backend gives us:

1. **Historical fidelity for Oct.** Oct can finally run on the actual
   ISA it was designed for.
2. **A second architecture backend** alongside RV32I (A1).  The two sit
   at opposite ends of the design space: RV32I is a clean modern
   load-store RISC; the 8008 is an irregular accumulator-based CISC
   with 8-bit registers and 14-bit addressing.  Each exposes a
   different set of constraints in the backend interface.
3. **A foundation for A4 (Intel 4004)**, which shares much of the
   historical-microprocessor backend shape.

## Why `Vec<u8>` output, not textual asm?

* **Round-trips with `intel8008-simulator`** — its `Simulator::run`
  method consumes raw `&[u8]` instruction streams directly.
* **Deterministic test surface** — `assert_eq!(bytes, vec![0x76])` is
  unambiguous; 8008 mnemonics have Intel-spec vs MCS-8 historical
  divergence.
* **Trivial output size** — 8008 instructions are 1, 2, or 3 bytes;
  emitting them as bytes skips a textual-assembly round-trip that
  contributes nothing.

## Pipeline

```text
IIRModule
  → validate_for_intel8008()      pre-flight, returns Vec<String>
  → lower_iir_to_intel8008()      returns Vec<u8> of 8008 opcodes
  → (optional)
      • intel8008-simulator: Simulator::run for in-process testing
      • write to .bin + external emulator
      • burn to a 1702 EPROM (Oct's intended deployment path)
```

## Scope by version

| Version | Scope | Status |
|---------|-------|--------|
| v0.1.0 (A2) | crate skeleton: any module → single `HLT` (`0x76`) | **merged** |
| v0.2.0 (A2+) | `const` → `MVI A, n` + `ret`/`ret_void` → `HLT` (accumulator-only first slice) | **merged** |
| v0.3.0 (A2++) | Linear register allocator over A/B/C/D/E/H/L + multi-register `const` + `mov` + ret-value staging | **merged** |
| **v0.3.1 (A2++.5 first slice — this PR)** | `add`/`sub` ALU on the accumulator (family `10 ooo sss`) | this PR |
| v0.3.2 (A2++.5.5) | rest of ALU (`cmp`, `and`, `or`, `xor`, `adc`, `sbb`) + real `RET` (`0x07`) via `CALL` + internal return stack | future |
| v0.4.0 (A2+++) | Conditional + unconditional jumps with 14-bit address backpatching + `lang-aot --target=intel8008` | future |

## Public surface (v0.1.0)

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

## The `HLT` encoding (v0.1.0 acceptance criterion)

The 8008's instruction encoding is irregular by modern standards: the
top 2 bits group instructions into four families, and the lower 6 bits
select within the family.  `HLT` lives in the register-register `MOV`
family but encodes "MOV M,M" — semantically a self-move on the
memory-pointer pseudo-register — which the silicon implements as a
halt.

Bit pattern: `01 110 110` = **`0x76`**.  The simulator's
`Simulator::halted()` accessor flips true after this byte executes.

v0.1.0's `lower_emits_the_canonical_hlt_byte` test pins this exactly
so any future revision of the simulator's opcode table that breaks the
encoding will surface immediately.

## Non-goals (v0.1.0)

* No instruction lowering — deferred to A2+.
* No `lang-aot --target=intel8008` wiring — deferred to A2+++.
* No external assembler / linker integration.  Output is raw bytes;
  downstream loaders are the caller's responsibility.

## Tests (v0.1.0)

* `validate_returns_empty_for_empty_module` — stub validator behaves.
* `lower_emits_exactly_one_byte` — output shape.
* `lower_emits_the_canonical_hlt_byte` — exact `0x76`.
* `default_config_has_nonempty_module_name` — config invariant.
* `new_sets_module_name` — builder contract.
* `errors_display_without_panic` — error formatting smoke.
