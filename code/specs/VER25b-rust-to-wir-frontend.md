# VER25b — Rust-to-WIR Lowering Front End (VER-P023)

**Parent:** [VER25](VER25-wir-core-interpreter.md), [VER25a](VER25a-rust-subset-grammar.md), [VER26 §4–§5](VER26-ct-compare-pilot.md).

---

## 1. Purpose & Scope

This specification establishes the lowering front end that translates the Confined Rust AST ([VER25a](VER25a-rust-subset-grammar.md)) into the Warrant Intermediate Representation ([VER25](VER25-wir-core-interpreter.md)). It delivers **VER-P023**.

The lowering pipeline transforms algorithmic and cryptographic Rust code (specifically `code/packages/rust/ct-compare`) into strongly typed, basic-block control flow graphs in WIR, ready for:
1. Operational execution and testing via the WIR reference interpreter.
2. Differential validation against native Rust implementations.
3. Observable execution trace comparison and constant-time verification ([VER26 §3](VER26-ct-compare-pilot.md)).

---

## 2. Lowering Rules

### 2.1 Types and Values
Rust types in the confined subset map deterministically to `WirType`:

| Rust Type | WIR Type | Representation |
|---|---|---|
| `bool` | `WirType::Bool` | Boolean truth value |
| `u8`, `i8` | `WirType::U8`, `WirType::I8` | 8-bit scalar integer |
| `u16`, `i16` | `WirType::U16`, `WirType::I16` | 16-bit scalar integer |
| `u32`, `i32` | `WirType::U32`, `WirType::I32` | 32-bit scalar integer |
| `u64`, `i64` | `WirType::U64`, `WirType::I64` | 64-bit scalar integer |
| `usize` | `WirType::U64` | Pointer-width unsigned integer (64-bit target) |
| `&[u8]` | `WirType::Pointer(prov)` | Pointer with provenance bound to base allocation |
| `[u8; N]` | `WirType::Array(U8, N)` | Fixed-size contiguous buffer |
| `Vec<u8>` | `WirType::Pointer(prov)` | Dynamic heap allocation with explicit provenance |

### 2.2 Arithmetic & Overflow Semantics
Rust cryptographic code explicitly specifies wrapping semantics (e.g. `.wrapping_sub`, `.wrapping_neg`) or bitwise manipulations.
- All lowered integer arithmetic defaults to `OverflowMode::Wrap` unless marked checked/trapping.
- Bitwise operators (`^`, `|`, `&`, `>>`, `<<`) map directly to `WirBinaryOp::BitXor`, `BitOr`, `BitAnd`, `Shl`, `Shr`.

### 2.3 Control Flow & Basic Blocks
- Structured control flow (`if`/`else`, `for` range loops, `return`) is linearized into standard Basic Blocks with single terminators (`WirTerminator::Branch`, `CondBranch`, `Return`).
- Range loops `for i in start..end { body }` lower into:
  1. Header block: checks loop termination `i < end` via `CondBranch`.
  2. Body block: executes loop statements.
  3. Latch block: increments `i += 1` with `OverflowMode::Wrap` and jumps back to Header.
  4. Exit block: continues subsequent execution.

### 2.4 Volatile Barriers & Inlining
- Cryptographic crates like `ct-compare` isolate compiler barriers in micro-functions (e.g. `volatile_barrier::accumulate(&mut acc, &left[i], &right[i])`).
- The lowering engine inlines known barrier primitives into explicit memory load/store operations:
  ```text
  val_left  = Load(left_ptr + i)
  val_right = Load(right_ptr + i)
  diff      = val_left ^ val_right
  acc_curr  = Load(acc_ptr)
  acc_next  = acc_curr | diff
  Store(acc_ptr, acc_next)
  ```
- Every `Load` and `Store` records the target address in `ObservableTrace`, enabling constant-time memory address verification.
- `core::hint::black_box(x)` lowers to an identity assignment with trace preservation.

---

## 3. Differential Validation Contract

The lowered WIR programs are validated differentially against native compiled Rust code:
1. **`ct_eq_u64`:** Sweeps identical pairs, random pairs, single-bit perturbations (`base ^ (1 << bit)` for all 64 bits), asserting that WIR interpretation matches native Rust return values.
2. **`ct_eq_equal_length` / `ct_eq`:** Evaluates byte buffers of varying lengths (1 to 256 bytes), identical buffers, buffers differing at first byte, middle byte, and last byte, asserting exact agreement with native Rust.
3. **`ct_select_bytes`:** Evaluates conditional selection with `choice = true` and `choice = false`, verifying output buffers match native results.
4. **Observable Trace Constant-Time Agreement:** For secret-labelled inputs of equal length, the execution trace (sequence of branch decisions and memory load/store addresses) must be identical regardless of whether inputs differ.

---

## 4. Verification Standards

- **Zero External Dependencies:** Built solely with standard Rust and internal workspace crates (`warrant-ir`).
- **Strict Coverage Thresholds:** ≥ 90% Line Coverage AND ≥ 90% Branch Coverage across all new lowering modules.
- **Fail-Closed Lowering:** Unrecognized AST constructs return structured `LoweringError` without panicking.

---

## 5. Note (RS01, 2026-11-12): lowering over CST views

When the confined subset moves onto the RS01 grammars ([VER25a §5](VER25a-rust-subset-grammar.md#5-amendment-rs01-2026-11-12)),
this lowering will **walk the CST through borrowed views**, as LANG84 C1.6
defines them, instead of converting it into `rust_subset::ast` first. RS01
decision Q5 applies LANG84 C1 and C2 (no typed lifts, no hand-written
parsers) to the WIR front end, although it is not an IIR front end.

- A view is a thin accessor over one grammar rule's children (for example an
  `IfView<'a>` with `cond()` and `then_block()`); it allocates no owned tree
  and does not desugar.
- Analyses the lowering needs (types of locals, free variables) run over the
  CST and produce side tables keyed by CST node identity.
- The lowering keeps its fail-closed rule: an unrecognized CST shape is a
  structured `LoweringError`, and anything the `rs-confined` gate admits but
  the lowering cannot model is refused the same way.
- For gf256 (RS01 §7.2, slice B4), a `static` initialized by a `const fn`
  call is lowered by lowering that `const fn` and running it, bounded, to
  initialize the WIR global. `OnceLock` and other lazy statics are not
  modelled and are refused.
- §3's differential contract is unchanged, and the RS01 differential adds
  one more check: the WIR lowered from the CST must be byte-identical to the
  WIR this front end lowers from the hand-written AST today.
