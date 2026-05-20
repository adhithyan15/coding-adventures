# OCT02 — Oct Rust frontend for the LANG VM AOT chain

**Status:** Draft — 2026-05-20
**Depends on:** OCT00, OCT01, LANG43, LANG44, LANG75
**Related:** LANG76 (for byte arrays / strings, V2)

## Motivation

Oct exists today as a Python-only frontend: `oct-lexer`, `oct-parser`,
`oct-type-checker`, `oct-ir-compiler`.  The IR it emits
(`IrProgram` with `IrOp::LOAD_IMM` etc.) is designed for the
**Intel 8008 simulator backend**, not the LANG VM AOT chain.

`lang-aot` (PR #3673) currently returns a clean
`UnsupportedLanguage` error for `.oct` files, with guidance pointing
at this spec.

OCT02 specs porting Oct to Rust *and* targeting `IIRModule`, so the
language gets the same LANG VM AOT treatment as Twig, Nib, and (soon)
BASIC.  This is the largest of the four follow-up specs — Oct has
the most non-trivial language surface.

## Non-goals (V1)

- **Intel-8008-specific intrinsics** (`in`, `out`, `adc`, `sbb`,
  `rlc`, `rrc`, `ral`, `rar`, `carry`, `parity`).  These are
  meaningful on the 8008 simulator but have no LANG VM equivalent.
  V1 emits a compile error for any program that uses them; future
  spec can define mappings (e.g. `in` → `call_builtin "in_byte"`
  with the simulator's port model).
- **Static variables** with explicit addresses.  Oct lets you place
  variables at specific memory addresses; LANG VM has no equivalent
  abstraction.  V1 treats all variables as normal IIR slots.
- **The 4-locals-per-function limit** baked into the 8008 backend.
  LANG VM has no such limit; V1 just ignores it.
- **Direct memory I/O** beyond what LANG75 + LANG76 expose.

## Scope of the port

Oct's Python implementation has four packages:

| Python package | Lines | LOC equivalent in Rust |
|---|---|---|
| `oct-lexer` | ~300 | ~400 (less concise) |
| `oct-parser` | ~500 | ~700 |
| `oct-type-checker` | ~400 | ~550 |
| `oct-ir-compiler` | ~600 | ~800 (rewritten to emit IIRModule, not IrProgram) |

Total: ~2 500 LOC of new Rust code across four crates.  Largest item
on this spec series by far.

## Strategy decision

Two paths:

**(A) Full Rust port.**  Mirrors the Nib pattern — `oct-lexer`,
`oct-parser`, `oct-type-checker`, `oct-iir-compiler` as four sibling
Rust crates.  Slow but durable; gives Oct first-class status in the
LANG VM ecosystem and makes it usable without a Python install.

**(B) Python bridge.**  Add an `oct-iir-compiler` (Python) that
emits IIRModule as JSON; a thin Rust shim in `lang-aot` shells out
to `python3 -m oct_iir_compiler` and deserialises the JSON into
`interpreter_ir::IIRModule`.  Fast to ship; couples `lang-aot` to a
Python install at AOT time.

**Recommendation: (A).**  Repo precedent is polyglot reimplementation
(every language has Rust + Python + … parallel implementations).  (B)
would be the only language in `lang-aot` that needs a non-Rust
dependency at AOT time; it would also block the
`twig-aot --emit-object` cross-OS workflow (the bridge would need to
work on Windows hosts too).

### V1 cut

To deliver (A) incrementally:

1. **Phase 1**: `oct-lexer` + `oct-parser` in Rust.  Existing Python
   acceptance tests must pass against a `cargo run --bin oct-parse`
   binary that prints the AST as JSON.
2. **Phase 2**: `oct-type-checker` in Rust.
3. **Phase 3**: `oct-iir-compiler` in Rust — emits `IIRModule`, lowers
   the V1 subset of Oct (no 8008 intrinsics, no fixed-address
   variables, no port I/O).
4. **Phase 4**: lang-aot wiring + end-to-end smoke test.

Each phase is its own PR.  Phase 4 is the proof.

## V1 supported Oct

What lowers to IIR in the first cut:

- Integer literals, arithmetic (`+`, `-`, `*`, `/`, `%`).
- Bitwise (`&`, `|`, `^`, `!`).
- Comparisons (`=`, `!=`, `<`, `<=`, `>`, `>=`).
- `if`, `while`, `loop`, `break`.
- User-defined `fn`s with parameters and return values.
- Recursion.
- Local variables.
- `call_builtin "print_i64"` (Oct's `print n` keyword).

What doesn't (each errors out with a clean message):

- 8008 intrinsics → `OctError::Unsupported8008Intrinsic(name)`.
- Fixed-address variables → `OctError::UnsupportedFixedAddress`.
- Port I/O → `OctError::UnsupportedPortIO`.
- Strings → `OctError::StringsNotYetSupported` (until LANG77).

## Crate layout

```
code/packages/rust/oct-lexer/
code/packages/rust/oct-parser/
code/packages/rust/oct-type-checker/
code/packages/rust/oct-iir-compiler/
```

Each gets:
- BUILD, Cargo.toml, CHANGELOG.md, README.md
- `pub fn` matching the Python crate's surface where 1:1 makes sense
- 80 %+ test coverage including the same fixtures the Python crates
  use, ported verbatim

## Public API of the iir-compiler

```rust
use interpreter_ir::module::IIRModule;

#[derive(Debug)]
pub enum OctCompileError {
    Lex(LexError),
    Parse(ParseError),
    Type(TypeError),
    Unsupported8008Intrinsic(String),
    UnsupportedFixedAddress,
    UnsupportedPortIO,
    StringsNotYetSupported,
}

pub fn compile_source(source: &str, module_name: &str)
    -> Result<IIRModule, OctCompileError>;
```

…matching `lang-aot::compile_source_to_iir`'s expectation.

## lang-aot wiring (Phase 4)

After Phase 3 lands:

```rust
// in lang-aot/src/lib.rs
Language::Oct => {
    oct_iir_compiler::compile_source(source, module_name)
        .map_err(|e| LangAotError::FrontendError {
            language,
            message: format!("{e:?}"),
        })
}
```

Plus end-to-end smoke tests for V1 Oct programs.

## Risk register

| Risk | Mitigation |
|---|---|
| ~2 500 LOC port is a multi-week effort; risks stalling for months | Split into four discrete phases (one PR each); each is shippable independently because the Python implementation continues to work. |
| Python and Rust impls drift on edge cases; tests diverge | Phase 1 and 2 PRs port the Python fixtures verbatim and run both impls against them in CI (cross-language conformance test). |
| 8008-intrinsic users will see breaking errors when they switch to lang-aot | Error message tells them to use the existing Intel-8008 simulator backend, not LANG VM, for those programs.  Documented in README. |
| Oct's `static` variables with fixed addresses are semantically untranslatable to LANG VM's slot model | V1 errors out cleanly.  Future spec can introduce a `static` qualifier in IIR if the use case justifies it. |
| Strings appear in many Oct examples (the language has them as first-class), so V1 won't run "real" Oct programs | True.  V1 is a beachhead — it gets the toolchain in place.  V2 follows LANG77 strings and immediately unlocks the bulk of the Oct corpus. |
