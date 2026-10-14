# CV04a — float-array call descriptor consistency

Independent review of da45489be6f found that supported array<f32> allocations
emit NEWARRAY T_FLOAT ([F) while type_to_jvm_descriptor falls back to [I. This
can produce an invalid valid-looking call, and the newly added reference guard
can also accept a float-array value as an integer-array argument. Correct the
descriptor helper to [F before accepting the bounded direct-call boundary.

Commit tests first. Build a caller with a local float-array allocation, confirm
the exact callee descriptor ([F)V and allocation opcode, and reject passing
the float-array value to an integer-array formal. Also construct an IIR program
that allocates one float, writes 1.0, calls a float-array procedure that reads
and returns it, compares 1.0 and returns42. Execute its serialized class on the
installed real JVM with -Xverify:all and a checked launcher; unavailable JVMs
must be reported, never counted as execution. Use only existing own crates and
existing test dependencies. Preserve old primitive/reference/closure descriptors.

Actual da validation81763 is terminal EXIT1: unfiltered LANG463PASS9FAIL, all
remaining failures are original NativeAOT arity refusals (eight dedicated rows
plus matrix duplicate). The previous JVM stack VerifyError is gone. Revised
original8 real JVM rows, completeJVM198 and strict3 suites passed on da, but
independent review withheld acceptance for this descriptor inconsistency.
Those source-specific receipts remain immutable, not final-source acceptance.

After this correction, rerun complete JVM tests, original8 real JVM array rows,
new float-array real JVM control, strict affected lint, independent exact-head
security review, documentation and durable backlog/issue evidence. Native ABI
stack-argument implementation and general-hop resource failures remain open.
Full Closure parity and complete transformation lineage are still open. Miri
is DEFERRED/PENDING, never passed; frozen checkout/run/inputs/target/receipts
remain untouched and unpolled. No publication or merge until required gates.

Revised source c5b00f83 actually passes the unchanged computation control on
the real JVM: float-array store/call/load/return/comparison computes42 and the
process exits0 under full verification. A descriptor-only [F to [I mutation
of that emitted class is independently rejected with VerifyError in read([I)F.
Initial serializer API and missing launcher UTF8 failures are preserved as
harness failures. The positive control exposed an additional typed-comparison
failure, repaired under CV04a-jvm-float-comparison.md. Full JVM200/3, original8
JVM rows, IEEE84 and strict3 pass; scoped independent review PASS. Native ABI
and whole/publication/final-source Miri gates remain open, Miri DEFERRED/PENDING.
