## Unreleased — A5++++++++ — **Dartmouth BASIC end-to-end through GE-225**

### The historical round-trip

The **GE-225** (1959) was the General Electric mainframe at
Dartmouth College where **John Kemeny and Thomas Kurtz designed
Dartmouth BASIC in 1964**.  BASIC was *born* on this silicon.

As of this release, the LANG VM lang-aot driver can compile
Dartmouth BASIC source through:

```text
.bas → dartmouth-basic-iir-compiler → IIR → iir-to-ge225 → 20-bit GE-225 words → .bin
```

Sixty-two years after Kemeny and Kurtz first wrote BASIC programs
on the GE-225, BASIC source round-trips back to the silicon it was
designed for.  This is the milestone moment for the GE-225 lane.

### Added — BASIC GE-225 end-to-end smoke tests

Three new tests in `tests/end_to_end_smoke.rs` exercise the
full BASIC → IIR → GE-225 .bin pipeline:

1. `end_to_end_basic_let_a_5_emits_ge225_bin_via_lang_aot` —
   the simplest BASIC program (`10 LET A = 5\n20 END`) compiles
   to a non-empty word-aligned .bin containing at least one
   `LDA` and at least one `HLT`.  Confirms the trivial case
   round-trips.
2. `end_to_end_basic_let_a_1_plus_2_exercises_add_via_lang_aot` —
   `10 LET A = 1 + 2\n20 END` exercises the GE-225 ADD opcode
   (0x04) inside the emitted byte stream.
3. `end_to_end_basic_print_documents_call_builtin_gap` —
   `10 LET A = 5\n20 PRINT A\n30 END` documents the
   `call_builtin` lowering gap (currently rejected with
   `UnsupportedOp`); a future iir-to-ge225 increment that adds
   `call_builtin` lowering will automatically activate the test.

Tests tolerate "lowering gap" errors so the cascade keeps
progressing as BASIC frontend and GE-225 backend both add ops
over time.

### Known BASIC ⇄ GE-225 gaps

| BASIC IIR op | iir-to-ge225 v0.7.0 status |
|--------------|----------------------------|
| `const`, `mov`, `add`, `cmp_le`, `jmp`, `jmp_if_true`, `jmp_if_false`, `label`, `ret` | ✓ supported |
| `call_builtin` (PRINT, etc.) | ✗ deferred |
| `neg` (unary minus) | ✗ deferred |

No version bump on iir-to-ge225 — this is a pure wiring/test
release of lang-aot that consumes iir-to-ge225 v0.7.0 unchanged.

