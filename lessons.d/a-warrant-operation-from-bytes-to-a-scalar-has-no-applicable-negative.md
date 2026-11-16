---
category: Testing & coverage
---

# A Warrant operation from bytes to a scalar has no applicable negative control, so gen_evidence cannot bind it

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

Fix (not made here, because the generator is a shared crate): let
`flip-result-bit` apply to a fixed-width integer result, in the fault check of
`composite_adapter` and in `fault_flip_result_bit` of `evidence_prelude.rs.txt`.

Do differently: before writing a long `.warrant` for a crate, bind one operation
of each distinct signature shape (parameter types and result type) and run
`gen_evidence` on it. Generator acceptance of the shape is the cheapest check
and the one that failed, long after the models were written. Related: the
negative-control lesson `a-warrant-negative-control-for-an-operation-is-answered-only-by-claims`.
