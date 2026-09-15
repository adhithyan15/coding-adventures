## 0.215.0 - 2026-07-14 (AOT00 T7 seed — generative differential harness)

Adds `t7_differential_random_u8_expressions_agree` to `tests/lang_matrix.rs` — the
first **generative** conformance test (AOT00 track T7). A deterministic zero-dep
`xorshift64` PRNG builds random total `u8` expression trees over `+ & | ^`; each is
run on every available engine and their results (normalised to the low byte, the
`u8` exit observable) MUST agree, with the in-process VM as the reference oracle.
Fast in-process engines (WASM/JIT) run on every program; the toolchain engines
(native/LLVM/CLR) on a deterministic sample. Where the hand-written matrix cells
each pin one (program, backend) result, this *generates* programs and auto-detects
cross-backend disagreements — the safety net that would have caught the E6d union
tagged-vs-boxed bug without a bespoke cell.

On its first run it already surfaced a conformance finding (**T7-find-1**): a
`uN`-returning function's value is not narrowed to its declared width before `ret`
(CLR's printed `int32` shows `300` for a `u8` `200+100`; the exit-code backends
only mask to 8 bits via OS exit truncation). Filed separately; the harness compares
the low byte until it lands. Design: `code/specs/lang-full-t7-differential-harness.md`.

