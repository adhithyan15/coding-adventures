# CLR07: strict scalar comparisons and boolean transport

## Baseline and scope

CLR06's explicit lower_typed_scalars_to_cil refuses a bool-returning cmp_eq
module as UnsupportedType. A separately hand-encoded pair of ldc.i8 1 values,
ceq and ret produces simulator Int(1), not Int64. The external probe and log
are lang-vm-clr07-baseline-probe.rs and lang-vm-clr07-baseline.log.
interpreter-ir/src/opcodes.rs classifies cmp_eq/ne/lt/le/gt/ge as bool producers.

Extend only the explicit strict entrypoint; default/source routing, CLR01
literal/input gates, branches, heap/closure/boxing and long slots remain out
of scope. Keep the existing 256 parameter/local bounds and method token limit.

## Semantic types and representation

Track logical i32, i64 and bool separately even though bool uses the existing
artifact int32 metadata and simulator Int(0)/Int(1) representation. Never use
shared metadata spelling as permission to mix bool and integer operands.
This is a normalized integer boolean ABI, not a new System.Boolean boxing ABI.
Bool constants accept only Operand::Bool; integer constants accept only Int
and preserve the existing i32 immediate range gate, including i64 constants.

Bool values may be moved, passed to exact bool parameters and returned from
exact bool functions. Calls and returns compare logical types, so a bool cannot
be passed to an i32 parameter or returned as i32 (or vice versa). Bool arithmetic,
bitwise operators, negation and bool comparisons are refused in this slice.
A bool entry parameter follows the same caller-supplied value contract as other
entry parameters; no new host invocation or dynamic value validation is added.

## Comparisons

Accept cmp_eq, cmp_ne, cmp_lt, cmp_le, cmp_gt, cmp_ge with exactly two prior
Var operands and one unique destination. Both inputs must share logical i32
or i64 type; the instruction hint and destination are bool. Mixed widths,
bool operands, literal operands, bad arity and non-bool result hints refuse.
Existing whole-module validation, including uncalled functions, still applies.

Emit ceq for equality, clt/cgt for signed ordering. For ne/le/ge, invert the
appropriate comparison with ldc.i4.0; ceq. The intermediate comparison result
is always int32, including comparisons of i64 values; do not insert conv.i8.
Store the final normalized bool using existing local transport.

Microsoft's [ceq documentation](https://learn.microsoft.com/en-us/dotnet/api/system.reflection.emit.opcodes.ceq)
specifies int32 zero/one results. Its [CIL instruction specification](https://download.microsoft.com/download/7/3/3/733ad403-90b2-4064-a81e-01035a7fe13c/ms%20partition%20iii.pdf)
describes signed clt/cgt and comparison operand compatibility. Reuse existing
builder comparison APIs and simulator operations; no new opcode is needed.

## Validation before publication

Execute actual strict artifacts from lang-aot tests for all six comparisons,
both widths, equal/less/greater cases and signed negative values. Generate i64
values above i32 through arithmetic with in-range literals. Assert result
Int(0/1), int32 boolean metadata and int64 operand metadata, and exact comparison
byte sequences including i32 inversion. Test bool constants, move, direct-call
parameters/returns, forward method references and nonfirst entry labels.

Negative tests pin logical bool/i32 separation in moves/calls/returns, both mixed
integer-width orders, every forbidden bool arithmetic family, bad comparison
hints/arity/shapes, integer-vs-bool constant mismatch and undefined operands.
Preserve all CLR06 refusal tests except bool cases intentionally newly accepted.
Run backend/simulator/builder/Brainfuck CLR/Nib CLR suites and actual artifact
tests, Clippy, documentation build/shard checks and lessons validation. Commit
this detailed contract before production changes; security review before push.

Execution-discovered prerequisite: i32 -1 uses builder's compact ldc.i4.m1
(0x15), which the current simulator refuses. The strict entrypoint must emit
full ldc.i4 with signed -1 payload for this value until compact opcode support
has independent execution coverage. Preserve semantics and the existing literal
gate; add an actual i32 -1 regression. No simulator opcode change in this slice.
