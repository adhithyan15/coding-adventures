# BF07 — Brainfuck on the LANG VM AOT chain

**Status:** Draft — 2026-05-20
**Depends on:** BF04, LANG75, LANG76
**Unblocks:** end-to-end `lang-aot foo.bf` → native binary

## Motivation

`brainfuck-iir-compiler` (per BF04) already emits an `IIRModule` from
BF source.  `lang-aot` (PR #3673) already routes `.bf` files through
the shared LANG VM chain.  But the chain currently breaks at the
backend: BF programs need byte-level memory ops (LANG76) and
`putchar` / `getchar` (LANG75), neither of which the V1 backends
lower.

Once LANG75 + LANG76 land, BF compilation through the LANG VM is
essentially free — the BF frontend already emits the right IR.  BF07
documents exactly what's needed end-to-end and provides the smoke
test that proves it works.

## Non-goals

- **Optimised tape layout** (e.g. wrap-around 8-bit cells with native
  `add r/m8, imm8`).  V1 uses 64-bit cells in a byte-addressed buffer;
  optimisation comes later if BF programs are too slow.
- **Buffered stdout**.  Each `.` calls `putchar` which calls libc
  `fputc`; for `+++++.` style hot loops this is slow.  Hot-loop
  buffering is a follow-up.
- **Bracket optimisation** (`[-]` → "set cell to zero", `[->+<]` →
  "move").  Pure peephole; doesn't affect correctness.
- **Compatibility with the LANG40 in-process JIT path.**  This spec
  is purely about AOT.

## V1 tape model

- The tape is exactly **30 000 bytes**, allocated once at the start
  of `main` via `alloc_bytes 30000 -> tape_ptr`.
- The data pointer (`ptr` in BF semantics) is held in a virtual
  register named `ptr` (slot type `i64`), initialised to `0`
  (offset from the start of the tape, not an absolute pointer).
- Cell values are 8-bit unsigned wrapping (`0..=255`).

This matches the C reference implementation closely enough that
existing BF programs (`hello.bf`, the prime sieve, `mandelbrot.bf`,
etc.) work without modification.

## IIR → backend mapping

`brainfuck-iir-compiler` already emits the IIR; BF07 specifies how the
backends must lower it.  Every BF program reduces to a single function
named `main` with no parameters returning `i64` (the exit code,
always `0`).

| BF source | IIR sequence (current `brainfuck-iir-compiler` output) | Backend lowering after LANG75+76 |
|---|---|---|
| (preamble) | `alloc_bytes 30000 -> tape_ptr`; `const 0 -> ptr` | LANG76 alloc; const_i64 0 |
| `>` | `const 1 -> c`; `add ptr, c -> ptr` | `add_i64` |
| `<` | `const 1 -> c`; `sub ptr, c -> ptr` | `sub_i64` |
| `+` | `load_byte tape_ptr, ptr -> v`; `const 1 -> c`; `add v, c -> v`; `store_byte tape_ptr, ptr, v` | LANG76 load_byte / store_byte; `add_i64` |
| `-` | (analogous with `sub`) | LANG76 load_byte / store_byte; `sub_i64` |
| `.` | `load_byte tape_ptr, ptr -> v`; `call_builtin "putchar", v` | LANG75 call_builtin |
| `,` | `call_builtin "getchar" -> v`; `store_byte tape_ptr, ptr, v` | LANG75 call_builtin |
| `[` | `label loop_<n>_top`; `load_byte tape_ptr, ptr -> v`; `cmp_eq v, 0 -> k`; `jmp_if_true k, loop_<n>_end` | label, cmp_eq_i64, jmp_if_true |
| `]` | `jmp loop_<n>_top`; `label loop_<n>_end` | jmp, label |
| EOF | `const 0 -> result`; `ret result` | ret_i64 |

The mapping is mechanical because BF has no parameters, no calls (other
than putchar/getchar), no scopes, and no closures.  This is the
simplest possible end-to-end smoke test of LANG75+LANG76.

## Frontend changes

None required.  `brainfuck-iir-compiler::compile_source` already
produces the IIR shape above.  We may want to align the IIR-emission
to use the canonical names `alloc_bytes` and `call_builtin "putchar"`
rather than older ad-hoc spellings — confirm and adjust as a tiny
follow-up commit alongside the BF07 implementation work.

## Tests

### Backend integration

A `brainfuck-iir-compiler` integration test compiles a small BF source
and asserts the produced IIRModule has the expected mnemonics — guards
against frontend drift.

### End-to-end (the proof)

`lang-aot/tests/end_to_end_bf_smoke.rs` (host-gated):

```rust
#[test]
fn end_to_end_bf_hello() {
    // BF "Hello\n"
    let src = include_str!("hello.bf");
    let exe = compile_via_lang_aot(src);
    let out = Command::new(&exe).output().unwrap();
    assert_eq!(out.stdout, b"Hello\n");
}
```

A `hello.bf` source like `++++++++[>+++++++++<-]>.<++++[>++++<-]>+.+++++++..+++.>++++[>+++<-]>.+.--------.<++.<.` (which prints `Hello\n` in plain BF idioms).

Plus a `rot13.bf` round-trip using `getchar` to read stdin.

## Risk register

| Risk | Mitigation |
|---|---|
| 30 000-byte tape is too small for some classical BF programs (`mandelbrot.bf` uses 10 000 cells; should fit) | V1 size is documented in the spec; future spec can expose it as a CLI flag. |
| Wraparound semantics for `255 + 1` differ from C's `unsigned char` if backends use `i64` arithmetic without masking | `store_byte` writes only the low 8 bits, so writeback masks naturally.  Loads zero-extend, then increment, then store — wrap is automatic. |
| Stdout buffering means `.` outputs nothing until program exit | `__twig_putchar` calls `fputc` without `fflush`; programs that produce output and then long-loop will appear hung.  Document this; consider `fflush(stdout)` after every `putchar` if it matters. |
| Self-modifying BF programs (none in canonical corpus) | Not in V1 scope; out-of-scope by language design. |
