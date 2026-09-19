# Encoded CLR wide-lowering audit (2026-09-19)

Audit following CLR02, not permission to remove CLR01 validation.

A real encoded artifact for i64 IIR constants 2147483647 and 1 followed by add
and ret returned Value::Int(-2147483648), rather than 2147483648. All three
artifact locals were int32. Probe ran against merged CLR02 plus backlog commit
04d510b73c. Evidence is saved outside the checkout in lang-vm-clr03-baseline.log
and lang-vm-clr03-audit-probe.rs under the worktrees directory.

The lower.rs artifact construction unconditionally declares local, parameter,
and return types int32. Const and direct-call immediates use ldc.i4. Int32 boxing
and synthetic closure dispatch arrays need separate treatment from i64 language
values. Structural indices and dispatch identifiers must remain i32. Comparisons
produce i32 booleans even with Int64 operands; conversion support is therefore a
prerequisite to a consistent lowering contract. The simulator currently implements
only a subset of builder arithmetic opcodes, so builder availability is not
execution evidence.

Next: pin explicit conv.i4/conv.i8 semantics from authoritative CIL documentation,
audit each emitted arithmetic opcode and select a bounded prerequisite. Commit
its detailed contract before implementation. Do not globally widen Int(i32),
silently promote mixed widths, or remove wide-immediate/input refusal during
this audit. Full locals/signatures/boxing/closure lowering remains separate.
