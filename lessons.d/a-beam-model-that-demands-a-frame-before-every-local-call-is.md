---
category: Compiler / VM / language pipeline
---

# A BEAM model that demands a frame before every local call is stricter than this backend: user functions call helpers frameless

**What went wrong.** LANG83 PR7b extended `iir-to-beam`'s opcode-and-BIF
test model so it could follow local `call`s, because the shared digit parse
became its own function. The new model asserted "local call without an
allocated frame". It then failed on the user function `q_digits(x)`, which
calls `'__iir_f64_digits'/1` with nothing live across the call. This backend
allocates a frame only when `n_yregs > 0`. That is by design: PR6a's writer
call and every IIR `call` are emitted the same way and run on CI's OTP 27.
Since OTP 24 the return address lives on the stack and is not saved by
`allocate`. The assertion encoded erlc's convention, not the emulator's
requirement.

**Fix.** Drop the assertion and keep the model's real checks: clobbered X
registers after a call, uninitialised Y slots, and `deallocate` matching
`allocate`. The new helpers still allocate a zero-slot frame like erlc
(`allocate 0, 1; call; ...; deallocate 0; return`).

**Do differently.** Before a test model enforces a rule, check what the
backend already emits and what CI already proves on the real emulator. A
rule that the passing production code breaks is a modelling mistake until a
real `erl` run says otherwise.
