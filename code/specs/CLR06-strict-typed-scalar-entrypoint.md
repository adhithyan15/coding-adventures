# CLR06: strict typed scalar CIL artifact entrypoint

## Scope and integration boundary

Add public lower_typed_scalars_to_cil(module, config) returning the existing
CILProgramArtifact/ IIRClrError types. Keep lower_iir_to_cil, codegen dispatch,
IIRClrConfig and lang-aot source routing unchanged in this slice. An explicit
entrypoint prevents accidental ABI selection from isolated hints inside mixed
closure or heap programs. This fixes the direct typed-scalar artifact path;
it does not claim the default source pipeline overflow is repaired.

Audit finding: lang-aot compile_source_to_cil_artifact explicitly calls
concretize_scalar_any_for_cil(module, true), narrowing i64 before lowering.
Default validation refuses any/polymorphic except special closure paths. A
future source-routing change must remove that narrowing only after checking
all consumer contracts. No added config field or global widening is permitted.

## Accepted contract

Whole module must consist of straight-line scalar functions, each ending in
exactly one final ret. Support const, mov, add/sub/mul/div, and/or/xor, neg,
and direct call. No branches, labels, comparisons, shifts, mod, heap operations,
closures, host calls, strings, floats, native ints, casts or void in this slice.
Their refusal is explicit; do not fall back to legacy lowering. Branches and
comparison booleans are a later extension rather than inferred from hints.

Function parameters, returns and every instruction hint must be i32 or i64.
Every destination is a uniquely defined local, distinct from parameters. Uses
must refer to parameters or earlier definitions. Names and function names must
be unique/nonempty. Every operand count and destination shape must be exact.
Ret has no destination, one Var operand matching the function return hint.
All functions and direct call signatures are checked, including uncalled ones.
Entry point, when present, must name a function; preserve module method order.

Const takes exactly one Int and produces its hinted type; retain CLR01's i32
immediate-range refusal even for i64 constants during this prerequisite slice.
Emit ldc.i8 for i64 constants so arithmetic can produce full-width results.
Mov/unary/binary operations require matching source and destination widths.
Direct call src0 is Var naming a defined function; remaining operands are Var
arguments matching callee parameter widths/count, with result destination and
hint matching its return type. No literals are accepted in call arguments.
Use MethodDef tokens from module order. No implicit conversion is performed.

Populate local_types, parameter_types and return_type with actual int32/int64
metadata. Emit loads/stores respecting parameter vs local slots. Preserve i32
wrapping arithmetic; i64 wraps at 64 bits, with simulator division refusing zero
and MIN/-1. All errors return Result::Err before an artifact is returned; avoid
panics from unsupported shapes. Reuse builder and error/artifact types, but
keep strict validation independent of legacy validator exceptions.

## Validation

Execute actual artifacts through clr-simulator from lang-aot integration tests.
2147483647+1 must become Int64(2147483648) with int64 local/signature metadata;
i32 version remains Int(-2147483648). Cover i64 intermediate multiplication,
negative signed division, bitwise high bits, moves and direct-call argument/
return transport. Inspect exact emitted ldc.i8 vs ldc.i4 bytes and metadata.
Negative tests cover each excluded family, widths, duplicate names/destinations,
undefined/forward uses, malformed arities, call signatures, returns, entrypoint
and out-of-range immediates. Preserve legacy refusal and existing consumer tests.
Run backend/simulator/builder/BrainfuckCLR/NibCLR suites, Clippy and docs checks.
Spec precedes implementation; no matrix declaration changes.
Encoding limits: reject more than 256 parameters or 65536 local slots per
function and more than 0xFFFFFF methods before narrowing indices/tokens.
Use checked conversions for indices. Preserve method order and entry_label;
callers must resolve entry_label rather than assume entry_method() selects it
(the existing artifact convenience method returns the first method).
