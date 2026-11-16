---
category: Testing & coverage
---

# A Warrant operation from bytes to a scalar has no applicable negative control, so gen_evidence cannot bind it (fixed by WX-FAULT1, #19508)

The WX-P1a adoption of `hash-functions` (eight functions of the shape
`&[u8] -> u32/u64`) was written, type-checked and its 67 examples confirmed
against the crate before the generator was run, and then `gen_evidence` refused
to bind it: every `op` line needs at least one `faults=` entry, and the
composite adapter (any operation with a `Bytes` parameter beside a number, or a
`Bytes` result) accepts only faults that fit its result: `flip-result-bit` and
`drop-last-result-byte` for a `Bytes` result, `missing-error` and `ok-as-err`
for a `Result`, the `*-consumed` and field faults for a tuple or record, and the
limit faults for an `output` bound. The arithmetic faults (`narrow-width`,
`saturate-*`) belong to the scalar adapter, which needs scalar parameters. A
bytes-to-scalar operation (a hash, a checksum, a length) therefore has no
fault, and a document with an unbindable operation cannot be generated at all.

Fix (made in WX-FAULT1, PR #19508; it was not made in the adoption because the generator is a shared crate): let
`flip-result-bit` apply to a fixed-width integer result, in the fault check of
`composite_adapter` and in `fault_flip_result_bit` of `evidence_prelude.rs.txt`.

Do differently: before writing a long `.warrant` for a crate, bind one operation
of each distinct signature shape (parameter types and result type) and run
`gen_evidence` on it. Generator acceptance of the shape is the cheapest check
and the one that failed, long after the models were written. Related: the
negative-control lesson `a-warrant-negative-control-for-an-operation-is-answered-only-by-claims`.

**Resolution (WX-P1a, branch `claude/warrant-hash-functions`).**

WX-FAULT1 let `flip-result-bit` apply to a fixed-width integer result and the
`hash-functions` adoption was wired on top of it: all eight operations bind, the
generated harness (277 tests) compiles the new `V::Fixed` arm unchanged and passes,
and each of the nine sabotage lines is caught. The lesson's advice held: binding
one operation of each signature shape first would have surfaced the blocker before
the 489-line document was written.

A second, different mismatch surfaced only at the last step, and the same advice
applies to it: before writing the document, run the package's capability analyzer
and the report generator on the crate. `hash-functions` declared
`"capabilities": []` but its `analysis.rs` calls `getrandom`, so the analyzer
fails; declaring `rand:read:*` instead breaks the VER08b jail's `rand-syscall`
negative control (RCG009) and leaves the report's CWE-330 row open. The pure
functions and the randomness need to live apart (DT17a section 7).
