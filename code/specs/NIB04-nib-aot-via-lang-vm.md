# NIB04 — Nib on the LANG VM AOT chain

**Status:** Draft — 2026-05-20
**Depends on:** NIB00, LANG43, LANG44, LANG75
**Related:** the lang-aot driver (PR #3673)

## Motivation

Nib already compiles end-to-end to native binaries on the LANG VM AOT
chain for the trivial case — `fn main() -> u8 { return 42; }` builds
through `lang-aot` and exits with code 42 on Windows / Linux / macOS
hosts.  But the surface that *actually* compiles today is a tiny
fraction of the Nib language as specified in NIB00:

| Nib feature | NIB00 spec | AOT today | Gap |
|---|---|---|---|
| `fn main() -> u8 { return <literal>; }` | ✓ | ✓ | none |
| `fn main() -> u8 { return <arith expr>; }` | ✓ | ✓ | none |
| `if`/`else` | ✓ | ✓ | none |
| `let x: u8 = <expr>;` | ✓ | ✓ | none |
| User-defined functions (non-`main`) | ✓ | **partial** — frontend doesn't yet emit cross-function `call` (lang-aot wires PR #3673 added the runtime side; nib-iir-compiler needs updating) |
| Function calls | ✓ | **broken** |
| Loops (`while`) | ✓ | **broken** — `nib-iir-compiler` lowers via tail-call optimization; without cross-function call, loops don't terminate properly |
| Strings | ✓ | **not yet** — backends have no string support (LANG75 `print_string` + LANG76 byte buffers needed first) |
| Arrays | ✓ | **not yet** |
| `print(x)` builtin | ✓ | **not yet** — needs `call_builtin "print_i64"` from LANG75 |
| ASCII / Unicode | ✓ | **not yet** |

NIB04 specs the work to close every gap in this table.

## Non-goals

- **Closures**.  Twig has them (LANG34); Nib doesn't per NIB00.  No
  change.
- **Generics / polymorphism**.  Nib has it via `T` parameters but the
  type-checker monomorphises before IIR emission, so no backend
  changes needed.
- **Refinement types**.  Already wired through `iir-refinement-pass`
  (LANG42); orthogonal.

## Required work, in dependency order

### 1. Wire `print_i64` builtin to Nib

Nib's `print(x)` should lower to `call_builtin "print_i64", x` once
LANG75 lands.  Currently `nib-iir-compiler` doesn't emit `print`
calls at all (the `print` keyword may not even be recognised in the
Nib grammar — confirm).

If `print` *is* in the grammar: trivial — extend the lowering pass to
emit `call_builtin "print_i64"`.

If `print` is *not* in the grammar: add it to `nib-lexer` and
`nib-parser`, then to the type-checker, then to the IIR compiler.
~4-file change, no algorithmic content.

### 2. Cross-function calls

The current `nib-iir-compiler` produces functions whose bodies are
self-contained.  For something like:

```nib
fn double(x: u8) -> u8 { return x + x; }
fn main() -> u8 { return double(21); }
```

…the IIR should contain two `IIRFunction`s and a `call double` from
`main`.  Verify the frontend already does this; if not, add it
(should be a small lowering-pass change).

The backend already supports this (PR #3331 added cross-function
patching for x86_64; the aarch64 path always supported it).  So this
is purely a frontend update.

### 3. Loops

NIB00 specifies a `while` loop.  Lowering is standard:

```nib
while cond { body }
```
→
```text
label loop_top
  <evaluate cond into c>
  jmp_if_false c, loop_end
  <body>
  jmp loop_top
label loop_end
```

This is `jmp_if_false` + `jmp` + `label` — all already supported by
both backends.  Frontend change only.

### 4. Strings

Strings need:

- LANG76 byte memory (heap-allocated buffer)
- A string literal table emitted as `_twig_strings` (data section
  alongside `_twig_globals`)
- `call_builtin "print_string", ptr, len`

Frontend changes:
- Lex / parse string literals (likely already done).
- Lower string literal `"hi"` to: emit data at the literal's offset
  in the strings table; in code, `lea str_ptr, [rip + _twig_strings +
  offset]`; pass `str_ptr` and `len` to `print_string`.

Backend changes:
- The packager (`elf_object`, `pe_object`, `macho_object`) needs to
  emit a `.rodata` (or `__cstring`) section for the strings table
  and the corresponding `R_X86_64_PC32` reloc kind.  This is a
  modest extension to LANG39's `_twig_globals` model.

Defer to a separate spec (call it **LANG77** if it lands) once we
have a real Nib program that needs strings.

### 5. Arrays

Stack-allocated arrays of `u8` / `i64` are straightforward with
LANG76's `alloc_bytes`.  Indexing is `load_byte` / `store_byte` (for
`u8[]`) or future word-sized variants.

Type-system integration (`arr: [u8; 4]`) is a frontend concern; the
backend just sees byte ops.

Defer to a follow-up once strings (#4) land — they share infrastructure.

## Tests

### Lib-level

Extend `nib-iir-compiler/src/lib.rs` tests with assertions that the
emitted IIRModule contains the expected ops for each new feature
(e.g. for `print(42)`, a `call_builtin` with `srcs[0] = Var(_) / 42`,
op = `"print_i64"`).

### End-to-end smoke

In `lang-aot/tests/`:

```rust
#[test]
fn nib_user_defined_function_returns_42() {
    let src = "fn double(x: u8) -> u8 { return x + x; }\n\
               fn main() -> u8 { return double(21); }";
    // ... compile + run ...
    assert_eq!(exit_code, 42);
}

#[test]
fn nib_while_loop_counts_to_42() {
    let src = "fn main() -> u8 { \
                 let mut n: u8 = 0; \
                 while n < 42 { n = n + 1; } \
                 return n; \
               }";
    assert_eq!(exit_code, 42);
}

#[test]
fn nib_print_outputs_42() {
    let src = "fn main() -> u8 { print(42); return 0; }";
    assert_eq!(stdout, "42\n");
}
```

Strings + arrays defer to follow-up specs.

## Sequencing

| Step | Cost | Unblocks |
|---|---|---|
| 1. Wire `print` to `call_builtin "print_i64"` | small | end-to-end Nib I/O |
| 2. Cross-function calls in nib-iir-compiler | small | real Nib programs |
| 3. While loops | small | real Nib programs |
| 4. Strings (LANG77 follow-up) | medium | NIB00 "print" with string args |
| 5. Arrays (deferred) | medium | NIB00 arrays |

Steps 1–3 are the V1 cut.  After they land, the full set of integer-
typed, control-flow-using Nib programs compiles natively.

## Risk register

| Risk | Mitigation |
|---|---|
| The Nib grammar doesn't include `print` yet — wiring step 1 requires touching the lexer + parser | Small change; track as the first sub-task of step 1.  Test that existing Nib parsing isn't affected. |
| nib-type-checker may reject `print(x)` because it has no return type signature | Add `print` as a void-returning builtin in the type checker; pre-defined in scope. |
| Strings require the packager to grow a `.rodata` section, which is non-trivial for both ELF and PE | Defer to LANG77.  V1 NIB04 stops at step 3. |
