# Changelog — iir-to-intel8008

All notable changes to this crate are documented here.
Format follows [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).

## [0.3.9] — 2026-06-02 (A2++.5.5 EIGHTH AND FINAL SLICE — real `RET` + `CAL` + module-level call backpatching)

### Added — real function calls and returns

The final A2++.5.5 slice.  Wires the 8008's 7-deep internal
return-address stack into IIR's calling convention:

| IIR op | 8008 lowering |
|--------|---------------|
| `call dest, "fn"` | `CAL` (`0x7E`) + 14-bit `fn` address + (optional `MOV dest_reg, A` to capture return value) |
| `ret <v>` in **non-entry** function | (optional `MOV A, v_reg`) + `RET` (`0x07`) |
| `ret_void` in **non-entry** function | `RET` (`0x07`) |
| `ret <v>` in **entry-point** function | (optional `MOV A, v_reg`) + `HLT` (unchanged from v0.2.0) |
| `ret_void` in **entry-point** function | `HLT` (unchanged) |

#### Why HLT stays for entry-point functions

The 8008's `RET` pops the top of the internal 7-deep return-address
stack and jumps there.  Calling `RET` from the entry-point function
(which was never `CAL`-ed) would underflow the empty stack and pop
a garbage address — undefined behaviour on most simulators, a hard
halt or wild jump on real silicon.

`HLT` is the correct "terminate execution" primitive for the program
entry point.  Non-entry functions correctly use `RET` to return to
their caller's saved address.

#### Module-level call backpatching

The 8008 has no PC-relative call — every `CAL` carries a full 14-bit
absolute target.  Cross-function calls require module-level resolution
because the callee's address depends on its position in the global
byte stream, which isn't known until all preceding functions have been
emitted.

New module-level state in `lower_iir_to_intel8008`:

* `function_addrs: HashMap<String, usize>` — records each function's
  start byte offset as we walk `module.functions` in source order.
* `pending_calls: Vec<(usize, String, String)>` — `(slot, callee_name,
  caller_name)`.  Each `call` emission records a triple; the final
  backpatching pass walks them all and writes the 14-bit address of
  each callee into its placeholder slot.

This mirrors `iir-to-riscv`'s A1++.5.5 module-level `jal` resolution
— cross-function references are the natural separation between
local-jump backpatching (per-function via `pending_jmps`, v0.3.4)
and call-site resolution (module-level, v0.3.9).

#### Calling convention

`call dest, "fn"` lowers to:

```text
CAL <fn_addr>                       ; 3 bytes (0x7E + low + high)
[optional]  MOV dest_reg, A         ; capture return value if dest != A
```

The 8008's return-value convention puts the result in `A`.  If the
IIR `call` has no `dest`, the return value is discarded (void call).

Argument passing is NOT yet supported — calls are zero-arg.  This
mirrors `iir-to-riscv`'s A1++.5.5 / A1++.5.5.5 split where arg
passing arrived in a follow-up slice.  For the 8008, arg passing
would need a per-call register-allocation contract (which registers
the callee preserves) and will likely fold into the lang-aot wiring
in A2+++.

#### New constants

* `pub const RET: u8 = 0x07;` — unconditional return (bit pattern
  `00 000 111`).  Confusion alert: `0x03` is `RFC` (return-if-flag-
  carry-clear), a conditional return.  `0x07` is the unconditional.
* `pub const CAL: u8 = 0x7E;` — unconditional call (bit pattern
  `01 111 110`).  **CRITICAL**: `0x46` is `CFZ` (call-if-flag-zero-
  clear, conditional).  Same family of confusion as `JMP ↔ JFC`
  flagged in v0.3.4 — pinning `0x7E` so the simulator's disassembler
  reports "CAL" and the silicon calls unconditionally.

#### New error variant

* `IIRIntel8008Error::UndefinedFunction { caller, callee }` — `call`
  referenced a name not present in `module.functions`.  Cross-module
  calls (which would need external symbol resolution) defer to
  `lang-aot` in A2+++.

#### Tests added (61 total, was 53)

* `ret_constant_pinned_to_0x07` / `cal_constant_pinned_to_0x7e`.
* `non_entry_function_ret_emits_real_ret_not_hlt` — pins the
  two-function module shape where `main` emits HLT but `helper`
  emits RET.
* `entry_function_ret_still_emits_hlt` — regression for the existing
  3-byte single-function `const v; ret v` shape; v0.3.9 must NOT
  break it.
* `call_emits_cal_with_backpatched_target_address` — pins the full
  byte stream `7E 04 00 76 3E 07 07` for `main → helper` two-function
  module.
* `call_to_undefined_function_is_rejected` — verifies the
  `UndefinedFunction` error path.
* `call_with_no_dest_discards_return_value` — void-call shape.
* `errors_for_undefined_function_display_without_panic`.

A new test helper `multi_fn_module(entry, functions)` was added to
build modules with multiple functions and an explicit entry point.

### What is NOT in v0.3.9 (deferred to v0.4.0 / A2+++)

* **Argument passing** — calls in v0.3.9 are zero-arg.  Arg passing
  needs a per-call register-allocation contract (which registers the
  callee preserves) and folds naturally into the AOT wiring.
* **Cross-module calls** — `call` to a function defined in a
  different module needs external symbol resolution.  Same.
* **`lang-aot --target=intel8008`** — the front-end CLI integration
  for choosing the 8008 backend.  This wraps `lower_iir_to_intel8008`
  with module-loading, linker-style relocation, and emits a `.bin`
  artifact suitable for the `intel8008-simulator` or an external
  emulator.  A2+++.

### A2++.5.5 complete

After eight slices, the 8008 backend now supports:

* All arithmetic + bitwise ops (`add`/`sub`/`adc`/`sbb`/`and`/`or`/`xor`)
* Comparison family (`cmp`/`cmp_ne`/`cmp_lt`/`cmp_gt`/`cmp_gte`/`cmp_lte`)
* Control flow (`label`/`jmp`/`jmp_if_true`/`jmp_if_false`)
* Function calls and returns (`call`/`ret`/`ret_void`)

Roughly 60 IIR opcodes mapped to ~25 distinct 8008 instruction
families, all pinned to byte-exact test sequences and cross-checked
against the in-tree `intel8008-simulator`'s decoder for round-trip
fidelity.  The crate is now feature-complete for AOT wiring.

## [0.3.8] — 2026-06-02 (A2++.5.5 seventh slice — `cmp_gte`/`cmp_lte` + remaining 5 cond-jump opcode constants)

### Added — closed-end ordering comparisons

Extends v0.3.7's six-way `emit_cmp_capture` dispatch with the two
final boolean comparisons:

| IIR op | Skip-jump opcode | Operand swap? | Skip-condition |
|--------|------------------|---------------|----------------|
| `cmp_gte dest, a, b` | **`JTC` (`0x44`)** | no | carry SET (a < b) |
| `cmp_lte dest, a, b` | `JTC` | **yes** | carry SET after swap (b < a) |

#### Why the OR-composition approach in the prompt was unnecessary

The prompt suggested composing `cmp_lt + cmp` and OR-ing the results.
That's correct semantically but requires:
- Two CMP + capture sequences (16 bytes vs 7)
- Two booleans staged into different registers
- An `ORA` to combine
- Final result staging

The simpler observation: `a >= b ⇔ NOT (a < b) ⇔ carry CLEAR after CMP b`.
So we can use the SAME 7-byte single-skip capture pattern, just with
the inverse polarity jump: `JTC` (skip when carry SET) instead of
`JFC` (skip when carry CLEAR).

`cmp_lte a, b ⇔ cmp_gte b, a` plugs in via the same operand-swap
trick v0.3.7 used for `cmp_gt`.  Two new IIR ops, ZERO new helper
code — the dispatch table just grows two entries.

#### Now all six boolean comparisons share one code path

```rust
let (skip_op, swap) = match instr.op.as_str() {
    "cmp"     => (JFZ, false),
    "cmp_ne"  => (JTZ, false),
    "cmp_lt"  => (JFC, false),
    "cmp_gt"  => (JFC, true),
    "cmp_gte" => (JTC, false),  // NEW
    "cmp_lte" => (JTC, true),   // NEW
    _ => unreachable!("outer arm restricts to these 6"),
};
let (left, right) = if swap { (b_reg, a_reg) } else { (a_reg, b_reg) };
emit_cmp_capture(&mut bytes, left, right, dest_reg, skip_op, &f.name)?;
```

The (skip-jump, swap) tuple cleanly tabulates all six comparisons.

#### New opcode constants (5 of them — `JTC` actually wired)

* `pub const JTC: u8 = 0x44;` — jump if carry SET.  **Used by
  `cmp_gte` and `cmp_lte`** lowerings.  Bit pattern `01 000 100`:
  carry flag, T-bit set (complement of `JFC`).  Confusion alert:
  `0x44` is NOT the unconditional `JMP` (which is `0x7C`) and NOT
  any of the call opcodes (`CFC` is at `0x42`).
* `pub const JFS: u8 = 0x50;` — jump if sign clear.  Pinned for
  future signed-integer ordering lowerings.
* `pub const JTS: u8 = 0x54;` — jump if sign set.
* `pub const JFP: u8 = 0x58;` — jump if parity clear (odd).
* `pub const JTP: u8 = 0x5C;` — jump if parity set (even).

The four `JFS`/`JTS`/`JFP`/`JTP` constants aren't yet consumed by
any lowering — they exist so the public surface matches the
encoding cheat-sheet in `code/specs/iir-to-intel8008.md` and so
signed/parity-based extensions in future slices can reach for them
without rediscovering the bit patterns.

#### Tests added (53 total, was 46)

* `jtc_constant_pinned_to_0x44` — guards JTC against the
  `JMP ↔ JTC` confusion.
* `jfs_constant_pinned_to_0x50` / `jts_constant_pinned_to_0x54` /
  `jfp_constant_pinned_to_0x58` / `jtp_constant_pinned_to_0x5c` —
  one-liner regression guards for the four unwired-but-public
  constants.
* `cmp_gte_pins_full_capture_byte_stream` —
  `B8 0E 00 44 0C 00 0E 01` (the JTC-driven capture).
* `cmp_lte_swaps_operands_then_uses_jtc` — pins the
  `MOV A, B; CMP A` (`78 BF`) prefix that distinguishes lte from
  gte by exactly two bytes (same shape as v0.3.7's
  `cmp_gt_swaps_operands_then_uses_jfc`).

### What is NOT in v0.3.8 (deferred to v0.3.9 / A2+++)

* Real `RET` (`0x07`) via `CAL` (**`0x7E`**, NOT `0x46` which is
  `CFZ`) + per-function internal return-stack discipline.  Lands
  in v0.3.9.
* Wiring `JFS`/`JTS`/`JFP`/`JTP` into actual lowerings (waiting on
  signed-integer or parity-based IIR ops to materialise upstream).
* `lang-aot --target=intel8008` wiring + module-level CALL
  backpatching — A2+++.

## [0.3.7] — 2026-06-02 (A2++.5.5 sixth slice — `cmp_ne`/`cmp_lt`/`cmp_gt` via shared capture helper)

### Added — inequality + ordering comparisons

Extends v0.3.6 with three more boolean comparison ops:

| IIR op | Skip-jump opcode | Operand swap? | Semantics |
|--------|------------------|---------------|-----------|
| `cmp_ne dest, a, b` | `JTZ` (`0x4C`) | no | `dest = (a != b) ? 1 : 0` |
| `cmp_lt dest, a, b` | **`JFC` (`0x40`)** | no | `dest = (a <  b) ? 1 : 0` |
| `cmp_gt dest, a, b` | `JFC` | **yes** | `dest = (a >  b) ? 1 : 0` |

All four boolean comparisons (`cmp` + the three new ones) now share
a single capture-emission helper `emit_cmp_capture`.  The four-line
match arm picks the skip-jump opcode and swap flag, then delegates:

```rust
let (skip_op, swap) = match instr.op.as_str() {
    "cmp"    => (JFZ, false),
    "cmp_ne" => (JTZ, false),
    "cmp_lt" => (JFC, false),
    "cmp_gt" => (JFC, true),
    _ => unreachable!("outer arm restricts to these 4"),
};
let (left, right) = if swap { (b_reg, a_reg) } else { (a_reg, b_reg) };
emit_cmp_capture(&mut bytes, left, right, dest_reg, skip_op, &f.name)?;
```

#### Why `cmp_gt = cmp_lt + operand swap`

`cmp_gt a, b` (is `a > b`?) is logically equivalent to `cmp_lt b, a`
(is `b < a`?).  Both compute "carry-set after CMP" — for cmp_lt that
means A=a, CMP b, carry-set ⇔ `a < b`.  For cmp_gt, swapping puts
A=b, CMP a, carry-set ⇔ `b < a` ⇔ `a > b`.  Same skeleton, just
different staging direction.

This avoids needing a new "JFC AND NOT JTZ" combined jump (which the
8008 doesn't have anyway) and keeps the lowering uniform.

#### New constant

* `pub const JFC: u8 = 0x40;` — jump if flag-carry CLEAR (i.e. when
  `A >= r` after `CMP r`).  Bit pattern `01 000 000`: jump family
  `01 ccc T 00` with `ccc = 000` (carry flag) and `T = 0` (test-clear).

#### Refactor — shared `emit_cmp_capture` helper

The v0.3.6 inline CMP + capture sequence was extracted to a helper:

```rust
fn emit_cmp_capture(
    bytes: &mut Vec<u8>,
    left_reg: u8,
    right_reg: u8,
    dest_reg: u8,
    skip_op: u8,
    fn_name: &str,
) -> Result<(), IIRIntel8008Error>;
```

Single source of truth for the 7-byte (or 8-byte with staging MOV)
boolean-comparison capture sequence.  Future cmp_lte / cmp_gte / etc.
hook in by passing their own skip-jump opcode + swap polarity.

#### Tests added (46 total, was 42)

* `jfc_constant_pinned_to_0x40` — guards the JFC constant.
* `cmp_ne_pins_full_capture_byte_stream` — `B8 0E 00 4C 0C 00 0E 01`.
* `cmp_lt_pins_full_capture_byte_stream` — `B8 0E 00 40 0C 00 0E 01`.
* `cmp_gt_swaps_operands_then_uses_jfc` — pins the swap-induced
  `MOV A, B; CMP A` (`78 BF`) prefix.

### What is NOT in v0.3.7 (deferred to v0.3.8 / v0.3.9 / A2+++)

* `cmp_lte` / `cmp_gte` — need either a two-branch capture (`Z=1 OR
  C=1`) or a `cmp_lt` + boolean negation; both more complex than
  the single-skip-jump pattern this helper assumes.  Land alongside
  the remaining 5 conditional-flag jump opcodes.
* The remaining 5 conditional-flag jump opcodes: `JTC` (`0x44`),
  `JFS` (`0x50`), `JTS` (`0x54`), `JFP` (`0x58`), `JTP` (`0x5C`).
* Real `RET` (`0x07`) via `CAL` (`0x7E`, NOT `0x46` which is CFZ) +
  per-function internal return-stack discipline — v0.3.9.
* `lang-aot --target=intel8008` wiring + module-level CALL
  backpatching — A2+++.

## [0.3.6] — 2026-06-02 (A2++.5.5 fifth slice — `cmp` equality with flag-to-bool capture)

### Added — boolean equality comparison

Wires IIR's `cmp dest, a, b` (which produces a boolean `dest = (a == b) ? 1 : 0`)
to the 8008's `CMP` instruction + an inline flag-to-bool capture
sequence.  CMP (family `10 111 sss` = `0xB8 | sss`) computes `A - r`,
sets `Z = 1 iff A == r`, and DISCARDS the difference — so without
a capture sequence the comparison result would be invisible to
the rest of the program.

#### Lowering shape

```text
[optional]  MOV A, a_reg               ; stage left source if not in A
            CMP b_reg     (0xB8|sss)   ; sets Z
            MVI dest, 0                ; default false (2 bytes)
            JFZ <fallthrough>          ; if Z=0 (a != b), skip overwrite
            MVI dest, 1                ; Z=1 path (a == b) → true
            <-- fallthrough -->
```

Total: 8 bytes when `a` is already in A, 9 bytes with the staging MOV.

#### Why an inline forward-JFZ instead of the two-pass backpatcher?

The JFZ's target is always a fixed +4-byte forward offset from the
JFZ opcode itself.  We can compute it at emit time (`target =
bytes.len() + 4`) and write the address bytes directly — no need
to push a `(slot, label)` tuple into `pending_jmps` and resolve
later.  Benefits:

1. **No synthetic label pollution.**  The user-visible `labels`
   map stays clean — no `__cmp_skip_0` / `__cmp_skip_1` names
   leaking through.
2. **No dependency on the two-pass machinery.**  The capture
   sequence is fully self-contained and could move into a helper
   function later without disturbing the backpatcher's invariants.
3. **Smaller error surface.**  No risk of a synthetic label
   accidentally colliding with a user label.

The `AddressOutOfRange` check still runs (the inline computed
target could in principle exceed 14 bits in a hypothetical very
large function — not reachable today with the 7-register cap, but
the guard is cheap and consistent with `jmp`'s).

#### New opcode constant (internal)

* `const ALU_CMP: u8 = 0b111;` — the `ooo` selector for `CMP r`
  (`encode_alu(ALU_CMP, sss) = 0xB8 | sss`).  Internal; no public
  `CMP` constant exposed because CMP's byte varies with `sss` —
  callers should compute via `encode_alu` if they need to.

#### Tests added (42 total, was 38)

* `cmp_equal_pins_full_capture_byte_stream` — pinned 14-byte
  sequence including `B8 0E 00 48 0C 00 0E 01` (the CMP + capture).
* `cmp_with_lhs_not_in_a_emits_staging_mov` — exercises the
  optional `MOV A, B` staging path; pins `CMP C = 0xB9`.
* `cmp_with_same_register_emits_cmp_a_then_capture` — `cmp r v v`
  case; pins `CMP A = 0xBF`.
* `cmp_followed_by_jmp_if_true_composes_correctly` — cross-slice
  composition test: cmp + v0.3.5's `jmp_if_true` produce the
  expected interleaved byte stream.

### What is NOT in v0.3.6 (deferred to v0.3.7 / v0.3.8 / A2+++)

* Less-than / greater-than / less-than-or-equal / etc — need the
  sign + carry flags from CMP, and pair with the other 6
  conditional-jump opcodes for the capture machinery.  Wired in
  v0.3.7.
* The remaining 6 conditional-jump opcodes: `JFC` (`0x40`),
  `JTC` (`0x44`), `JFS` (`0x50`), `JTS` (`0x54`), `JFP` (`0x58`),
  `JTP` (`0x5C`).  Land alongside lt/gt/etc.
* Real `RET` (`0x07`) via `CAL` (`0x7E`, NOT `0x46` which is CFZ)
  + per-function internal return-stack discipline — v0.3.8.
* `lang-aot --target=intel8008` wiring + module-level CALL
  backpatching — A2+++.

## [0.3.5] — 2026-06-02 (A2++.5.5 fourth slice — boolean conditional jumps `jmp_if_true`/`jmp_if_false`)

### Added — boolean-conditional control flow

Wires the next control-flow primitive: IIR's boolean branches lower
to a TEST-A + zero-flag-jump pair on the 8008.  The 8008 has no
"branch on register" — every conditional jump reads ONE of the four
CPU flags from the last arithmetic/logical op.  Boolean cond
variables hold 0 or non-zero, so we provoke the zero flag via
`ANA A` (the 8008's "TEST A" idiom — same role as `test eax, eax`
on x86).

| IIR op | 8008 lowering shape |
|--------|---------------------|
| `jmp_if_true  cond, L` | (optional `MOV A, cond_reg`) + `ANA A` (`0xA7`) + `JFZ L` (`0x48`) + low/high addr |
| `jmp_if_false cond, L` | (optional `MOV A, cond_reg`) + `ANA A` (`0xA7`) + `JTZ L` (`0x4C`) + low/high addr |

#### Why JFZ for "true" / JTZ for "false"?

The 8008's zero flag is SET when the last op produced 0.  So:
- `cond_var == 0` (false) → `ANA A` produces 0 → Z=1
- `cond_var != 0` (true)  → `ANA A` produces non-zero → Z=0

The mnemonic names mirror this:
- `JFZ` = Jump if Flag Zero is Clear (Z=0) → branch when cond was true
- `JTZ` = Jump if Flag Zero is seT  (Z=1) → branch when cond was false

#### Why we even need `ANA A` (TEST A)

`MOV A, cond_reg` does NOT set flags on the 8008 — MOV is
flag-non-affecting.  Without the `ANA A` flag-setter between the
load and the conditional jump, JFZ/JTZ would consume STALE flags
from whatever ALU op last ran.

The `ANA A` is unconditional — emitted even when `cond_reg` is
already A (in which case the MOV is elided but `ANA A` still runs).

#### New constants

* `pub const JFZ: u8 = 0x48;` — jump if zero flag clear
* `pub const JTZ: u8 = 0x4C;` — jump if zero flag set

Both pinned in their own tests.  Bit patterns spelled out so the
v0.3.6 slice (which adds the other 6 flag opcodes) doesn't get the
nibbling wrong.

#### Tests added (38 total, was 32)

* `jfz_constant_pinned_to_0x48`
* `jtz_constant_pinned_to_0x4c`
* `jmp_if_true_emits_ana_a_then_jfz_with_backpatched_target` — full
  pinned byte stream including `A7 48 08 00`.
* `jmp_if_false_emits_ana_a_then_jtz_with_backpatched_target` —
  `A7 4C 08 00`.
* `jmp_if_true_with_cond_not_in_a_emits_staging_mov` — exercises
  the optional `MOV A, B` (`0x78`) staging path.
* `jmp_if_true_to_undefined_label_is_rejected` — confirms the
  existing `UndefinedLabel` error path reaches these new ops via
  the same two-pass backpatcher.

### What is NOT in v0.3.5 (deferred to v0.3.6 / v0.3.7 / A2+++)

* `cmp` — the 8008's CMP (`ooo = 0b111`) sets flags and discards
  the difference; lowering needs a flag-to-bool capture sequence
  using a conditional jump over an `MVI dest, 0/1` pair.  Paired
  with the other flag-jump opcodes in v0.3.6 so the same
  capture machinery is reused.
* The other 6 conditional-jump opcodes: `JFC` (`0x40`), `JTC`
  (`0x44`), `JFS` (`0x50`), `JTS` (`0x54`), `JFP` (`0x58`),
  `JTP` (`0x5C`).  Useful once `cmp` and the carry-flag-producing
  ALU sequences need them.
* Real `RET` (`0x07`) via `CAL` (`0x7E` — NOT `0x46` which is
  CFZ) + the internal return stack.  Lands in v0.3.7.
* `lang-aot --target=intel8008` wiring + module-level CALL
  backpatching — A2+++.

## [0.3.4] — 2026-06-02 (A2++.5.5 third slice — `label` + unconditional `jmp` + two-pass backpatching)

### Added — control flow primitive (unconditional jump)

Wires the first control-flow lowering on the 8008.  Adds two IIR ops:

| IIR op | Intel 8008 lowering |
|--------|---------------------|
| `label "<name>"` | zero bytes; records `(name → current_byte_offset)` in a per-function label table |
| `jmp "<name>"` | `0x7C` + low address byte + high address byte (3 bytes total) |

#### Why the encoding matters

`JMP unconditional = 0x7C` (bit pattern `01 111 100`), NOT `0x44`.

`0x44` is `JFC` (jump if flag-carry clear) — a **conditional** jump.
The 8008's group-01 instruction family disambiguates by `ddd` (bits
5-3): `ddd = 111` is the unconditional variant; `ddd ≤ 011` selects
one of the four flag-tested conditional jumps.  Emitting `0x44`
instead of `0x7C` would compile to a jump that silently takes or
skips based on whichever carry-flag state the silicon happened to
have at that moment — a debugging nightmare.

This nightmare was caught during implementation by cross-checking
against the in-tree `intel8008-simulator`, which is the
crate's authoritative round-trip target.  The new
`jmp_constant_pinned_to_0x7c` test guards the constant against any
future copy-paste regression.

#### Two-pass backpatching

The 8008 has no PC-relative addressing — every `jmp` carries a full
14-bit absolute target.  Pass 1 emits each `jmp` as `0x7C 0x00 0x00`,
recording the placeholder's byte offset and the target label name.
Pass 2 walks the table, resolves each label to its byte offset, and
backpatches the two address bytes:

```
bytes[slot]     = (target & 0xFF) as u8;       // low byte
bytes[slot + 1] = ((target >> 8) & 0x3F) as u8; // high byte (6 bits)
```

The top 2 bits of the high byte are written as zero — the 8008's
address bus is 14 bits wide, so the silicon ignores them.  Emitting
clean zeros means downstream disassemblers reproduce the same
`JMP addr` regardless of how they sign-extend.

Labels are scoped per-function.  Cross-function jumps (which would
also need module-level resolution like `iir-to-riscv`'s A1++.5.5)
aren't supported in v0.3.4.

#### New constants + error variants

* `pub const JMP: u8 = 0x7C;` (exposed for round-trip tests downstream)
* `IIRIntel8008Error::UndefinedLabel { function, label }`
* `IIRIntel8008Error::AddressOutOfRange { function, address }` —
  forward-compatibility guard for the 14-bit (16384-byte) address
  space; not yet triggerable in v0.3.4 because the 7-register
  allocator caps each function at ~25 bytes.

#### Tests added (32 total, was 27)

* `jmp_constant_pinned_to_0x7c` — guards `JMP` against the easy
  `0x44 ↔ 0x7C` confusion.
* `jmp_to_forward_label_backpatches_target_address` — full pinned
  byte stream including `7C 07 00` for a label at offset 7.
* `jmp_to_backward_label_backpatches_target_address` — `7C 00 00`
  for a loop back to offset 0.
* `jmp_to_undefined_label_is_rejected` — pins the `UndefinedLabel`
  error variant and message contents.
* `label_emits_no_bytes` — differential test: a function with vs.
  without a leading `label` must emit identical byte streams.

`errors_display_without_panic` extended to cover the two new
variants.

### What is NOT in v0.3.4 (deferred to v0.3.5 / v0.3.6 / A2+++)

* Conditional jumps (`jmp_if_true` / `jmp_if_false`) — 8 opcodes
  (JFC/JFZ/JFS/JFP and their T-bit complements JTC/JTZ/JTS/JTP),
  same 3-byte shape but driven by the four 8008 condition flags.
* `cmp` — needs the conditional-jump infrastructure above to
  capture the 8008's flag-only CMP result into a register dest.
* Real `RET` (`0x07`) via `CALL` (`0x7E`, not `0x46`!) + the
  per-function internal return stack.
* `lang-aot --target=intel8008` wiring + module-level CALL
  backpatching — A2+++.

## [0.3.3] — 2026-06-02 (A2++.5.5 second slice — carry/borrow ALU `adc`/`sbb`)

### Added — carry-chained accumulator-target ALU

Extends v0.3.2 with two more accumulator-anchored ALU ops in family
`10 ooo sss` that read the carry/borrow flag set by a *prior*
flag-producing op:

| IIR op | Intel 8008 mnemonic | `ooo` | First byte |
|--------|---------------------|-------|------------|
| `adc dest, a, b` | `ACA b_reg` | `001` | `0x88 \| sss` |
| `sbb dest, a, b` | `SCA b_reg` | `011` | `0x98 \| sss` |

The lowering shape is identical to add/sub/and/or/xor — only the
`ooo` selector changes.  No new encoder code; `encode_alu(ooo, sss)`
from v0.3.1 carries the wider dispatch.

#### Carry-flag contract (front-end responsibility)

The 8008's `ACA`/`SCA` consume the carry flag bit set by a prior
flag-affecting ALU op (`ADD`, `SUB`, `ADC`, `SBB`, `ANA`, `ORA`,
`XRA`, `CMP`).  This backend emits instructions in source order with
no reordering — so if the IIR front-end emits

```
add r_lo lo_a lo_b   ; sets carry on overflow
adc r_hi hi_a hi_b   ; consumes that carry
```

the carry survives between the two.  However, the staging MOVs
inserted by the allocator (`MOV A, hi_a`) sit between them.  Per
Intel's 8008 docs MOV does NOT affect flags, so the carry survives
the MOV too.  Front-ends MUST NOT insert flag-clobbering ops between
the producer and the ADC/SBB consumer.

#### Why no `cmp` in this slice?

`cmp` in IIR is shaped `cmp dest, a, b` and produces a boolean dest.
The 8008's `CMP` (`ooo = 0b111`) computes `A - r`, sets flags, and
**discards** the result — there's no register dest in 8008-speak.
Lowering `cmp` therefore requires an additional sequence that
captures the resulting condition into a register, which is typically
done as part of the same lowering that handles conditional branches.
That work lands together in v0.3.4.

#### New opcode constants

* `const ALU_ADC: u8 = 0b001;`
* `const ALU_SBB: u8 = 0b011;`

#### Tests added (27 total, was 24)

* `adc_two_consts_emits_aca_b_after_mov` — pinned full sequence with
  `0x88` (ACA B).
* `sbb_two_consts_emits_sca_b_after_mov` — `0x98` (SCA B).
* `adc_when_lhs_is_already_in_a_skips_the_staging_mov` — `0x8F`
  (ACA A) for the self-ADC idiom, generalising the self-add/self-AND
  tests.

### What is NOT in v0.3.3 (deferred to v0.3.4 / v0.3.5 / A2+++)

* `cmp` — paired with the branch ops in v0.3.4.
* Real `RET` (`0x07`) via `CALL` (`0x46` + 14-bit address) + the
  internal return stack — v0.3.5.
* Conditional + unconditional jumps with 14-bit address backpatching
  — v0.3.4 (alongside `cmp`).
* `lang-aot --target=intel8008` wiring — A2+++.

## [0.3.2] — 2026-06-02 (A2++.5.5 first slice — bitwise ALU `and`/`or`/`xor`)

### Added — bitwise accumulator-target ALU

Extends v0.3.1 with three more accumulator-anchored ALU ops in family
`10 ooo sss`.  Identical lowering shape to add/sub — only the `ooo`
field changes:

| IIR op | Intel 8008 mnemonic | `ooo` | First byte |
|--------|---------------------|-------|------------|
| `and dest, a, b` | `ANA b_reg` | `100` | `0xA0 \| sss` |
| `xor dest, a, b` | `XRA b_reg` | `101` | `0xA8 \| sss` |
| `or  dest, a, b` | `ORA b_reg` | `110` | `0xB0 \| sss` |

The full sequence remains:

```text
if a_reg != A:    MOV A, a_reg
                  ANA/ORA/XRA b_reg     ; result lands in A
if dest_reg != A: MOV dest_reg, A
```

#### Code-gen shape (worked example)

`r = v & w` with v→A, w→B, r→C lowers to:

```
MVI A, v_imm
MVI B, w_imm
ANA B            ; 0xA0
MOV C, A         ; 0x4F
```

i.e. one byte of bitwise op plus the staging move (same as `add`/`sub`).

#### Self-op idiom

`and r v v` where `v` is already in `A` lowers to `ANA A` (`0xA7`,
family `10 100 111`) — same as the self-add shape: no leading
`MOV A, A`.

#### New opcode constants

* `const ALU_AND: u8 = 0b100;`
* `const ALU_XOR: u8 = 0b101;`
* `const ALU_OR:  u8 = 0b110;`

The `encode_alu(ooo, sss)` helper from v0.3.1 carries them all — no
new encoder code, just a wider dispatch in the lowering match arm.

#### Tests added (24 total, was 20)

* `and_two_consts_emits_ana_b_after_mov` — pinned full sequence with
  `0xA0` (ANA B).
* `or_two_consts_emits_ora_b_after_mov` — `0xB0` (ORA B).
* `xor_two_consts_emits_xra_b_after_mov` — `0xA8` (XRA B).
* `and_when_lhs_is_already_in_a_skips_the_staging_mov` — `0xA7`
  (ANA A) for the self-AND idiom, generalising the self-add test.

The `unsupported_op_is_rejected_with_function_name` test still probes
`safepoint`, which remains outside the whitelist.

### What is NOT in v0.3.2 (deferred to v0.3.3 / A2+++)

* `cmp`, `adc`, `sbb` — same family, different `ooo` codes
  (`cmp = 0b111`, `adc = 0b001`, `sbb = 0b011`).  `cmp` needs flag
  observation wiring; `adc`/`sbb` need the carry flag plumbed from a
  prior arithmetic op.  All three land together once the carry-flag
  story is settled.
* Real `RET` (`0x07`) via `CALL` + the internal return stack.
* Conditional + unconditional jumps with 14-bit address backpatching.
* `lang-aot --target=intel8008` wiring — A2+++.

## [0.3.1] — 2026-06-02 (A2++.5 first slice — `add`/`sub` on the accumulator)

### Added — accumulator-target ALU

Extends v0.3.0 with two ALU ops in family `10 ooo sss`.  The 8008's
ALU is *always* accumulator-anchored: left source AND destination are
`A`; only the right source comes from `sss`.

| IIR op | Intel 8008 lowering |
|--------|---------------------|
| `add dest, a, b` | (optional `MOV A, a_reg`) + `ADD b_reg` (`0x80 \| sss`) + (optional `MOV dest_reg, A`) |
| `sub dest, a, b` | (optional `MOV A, a_reg`) + `SUB b_reg` (`0x90 \| sss`) + (optional `MOV dest_reg, A`) |

#### Code-gen shape

```text
if a_reg != A:   MOV A, a_reg
                 ADD/SUB b_reg          ; result lands in A
if dest_reg != A: MOV dest_reg, A
```

The first const allocated to `A` and the next-allocated `dest_reg` ≠
`A` mean the typical sequence for `r = v + w` (where `v` was the first
const) is:

```
ADD b_reg
MOV C, A
```

i.e. two bytes of arithmetic plus the staging move.

#### Self-add idiom

`add r v v` where `v` is already in `A` lowers to `ADD A` (`0x87`,
family `10 000 111`) — the 8008 happily uses `A` as the right source.
No leading `MOV A, A` is emitted.

#### New encoder helper + opcode constants

* `fn encode_alu(ooo: u8, sss: u8) -> u8` — `0x80 | (ooo << 3) | sss`.
* `const ALU_ADD: u8 = 0b000;`
* `const ALU_SUB: u8 = 0b010;`

#### Tests added (20 total, was 17)

* `add_two_consts_returns_their_sum_via_accumulator` — pinned 8-byte
  sequence ending in `0x80 0x4F 0x79 0x76`.
* `sub_two_consts_emits_sub_b_after_mov` — same shape with `0x90`.
* `add_when_lhs_is_already_in_a_skips_the_staging_mov` — pinned
  `ADD A = 0x87` for the self-add case.

The pre-existing `unsupported_op_is_rejected_with_function_name` test
flipped from probing `add` (now supported) to `safepoint` (still
outside the whitelist).

### What is NOT in v0.3.1 (deferred to A2++.5.5 / A2+++)

* `cmp`, `and`, `or`, `xor`, `adc`, `sbb` — same family, different
  `ooo` codes.
* Real `RET` (`0x07`) via `CALL` + the internal return stack.
* Conditional + unconditional jumps with 14-bit address backpatching.
* `lang-aot --target=intel8008` wiring — A2+++.

## [0.3.0] — 2026-06-02 (A2++ — multi-register `const` + `mov` + ret-value staging)

### Added — linear register allocator over A/B/C/D/E/H/L

Extends v0.2.0's accumulator-only `const` to a real allocator that
hands out the 8008's seven general-purpose registers in order:
`A, B, C, D, E, H, L`.  `A` comes first so the trivial `const v; ret v`
case stays at the 3-byte shape (`MVI A, n; HLT`) without an extra
`MOV A, X` round-trip.

| IIR op | Intel 8008 lowering |
|--------|---------------------|
| `const dest, Int(n)` | `MVI dest_reg, n` (`(rrr << 3) \| 0x06` + immediate byte) |
| `mov dest, src` | `MOV dest_reg, src_reg` (family `01 ddd sss`) |
| `ret <var>` | stage `var` into `A` via `MOV A, var_reg` if needed, then `HLT` |
| `ret_void` | `HLT` |

Register encoding: `A=7`, `B=0`, `C=1`, `D=2`, `E=3`, `H=4`, `L=5`,
`M=6` (memory pseudo-register, not allocated).  Pool: `[A, B, C, D,
E, H, L]`.

### New encoder helpers

* `encode_mvi(rrr: u8) -> u8` — `(rrr << 3) \| 0x06` for the
  immediate-load family.
* `encode_mov(ddd: u8, sss: u8) -> u8` — `0x40 \| (ddd << 3) \| sss`
  for the MOV family.

Both `debug_assert!` their inputs fit in 3 bits.

### New error variants

* `IIRIntel8008Error::UndefinedVariable` — `mov` or `ret` referenced
  a name that was never bound.
* `IIRIntel8008Error::OutOfRegisters` — 8th local exhausted the pool.
  Stack spilling lands in A2++.5 or later.

### Tests added (17 total, was 12)

* `two_consts_use_a_then_b_then_mov_a_b_before_hlt` — pinned exact
  6-byte sequence `MVI A,1; MVI B,2; MOV A,B; HLT` (`0x3E 0x01 0x06
  0x02 0x78 0x76`).
* `ret_of_first_const_omits_the_redundant_mov` — regression for the
  v0.2.0 3-byte trivial-case shape (A-first allocator preserves it).
* `mov_lowers_to_canonical_mov_ddd_sss` — `MOV B, A = 0x47`,
  `MOV A, B = 0x78`.
* `allocator_exhaustion_yields_out_of_registers` — 8 consts → fails
  on `v7` with `OutOfRegisters`.
* `undefined_variable_in_mov_is_rejected`.

### What is NOT in v0.3.0 (deferred to A2++.5)

* ALU on the accumulator (`ADD`/`SUB`/`CMP`/etc., family `10 ooo
  sss`).
* Real `RET` (`0x3F`) via `CALL` (`0x44` + 14-bit address) and the
  internal return stack.

## [0.2.0] — 2026-06-02 (A2+ — `const` → MVI A; `ret`/`ret_void` → HLT)

### Added — first real instruction lowering

Extends v0.1.0's HLT-only skeleton with three IIR ops:

| IIR op | Intel 8008 lowering |
|--------|---------------------|
| `const dest, Int(n)` | `MVI A, n` (`0x3E` + immediate byte) |
| `ret <var>` | `HLT` (real RET lands in A2++) |
| `ret_void` | `HLT` |

The accumulator-only first slice: every `const` goes into `A`.  Multi-
register allocation (B/C/D/E/H/L) lands in A2++ alongside `MOV r1, r2`
(family `11 ddd sss`) and ALU on the accumulator.

#### Why `ret` → `HLT` for now

Intel 8008's real `RET` (`0x3F`) requires the CPU to have a non-empty
internal return stack — which means proper `CALL` semantics first.
A2++ adds the `CALL/RET` stack discipline; until then, `HLT` gives the
simulator a clean stopping point for trivial test programs.

#### New constant in the public surface

* `pub const MVI_A: u8 = 0x3E;` — Intel 8008 `MVI A, imm8` first byte
  (bit pattern `00 111 110`, immediate-load family `00 rrr 110` with
  `rrr = 111 = A`).

#### Immediate byte range

`const` accepts integers in `[-128, 255]`:
* `[0, 255]` cast straight to `u8`.
* `[-128, -1]` reinterpreted as two's-complement (`-1 → 0xFF`).
* Anything outside → `InvalidOperand` with a precise message naming
  the 8-bit limit.  The 8008 has no wide-immediate idiom comparable
  to RV32I's `lui` — A2++ will split wider values into multiple
  `MVI` sequences across multiple registers.

#### Tests added (12 total, was 6)

* `const_42_then_ret_lowers_to_mvi_a_42_then_hlt` — pinned exact
  3-byte sequence `0x3E 0x2A 0x76`.
* `mvi_a_constant_pinned_to_0x3e`
* `const_negative_uses_twos_complement_byte` (`-1 → 0xFF`)
* `const_out_of_byte_range_is_rejected` (`1000` overflows)
* `ret_void_alone_emits_just_hlt`
* `unsupported_op_is_rejected_with_function_name` (`add` rejected
  with function name preserved in the error)

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
