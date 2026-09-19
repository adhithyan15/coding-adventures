# CLR02: explicit encoded int64 values

Selected 2026-09-19 after CLR01 (#15568). This prerequisite extends the literal
builder and simulator, not IIR lowering or host input. Keep CLR01 refusal intact.

## Contract

Preserve Value::Int(i32) and introduce Value::Int64(i64). Do not globally widen
Int: existing ldc.i4 arithmetic must keep its 32-bit wrap semantics. Add opcode
0x21, encode_ldc_i8 and builder.emit_ldc_i8 with eight little-endian payload bytes.
Even small int64 literals use this encoding so their stack type remains int64.
Decode only a complete payload; truncated instructions must fail explicitly
before changing the evaluation stack or program counter.

Locals, arguments, returns, dup, branches and the existing loose object-array
transport preserve the full tagged value. Array sizes and indices remain i32
and must refuse Int64 rather than narrowing it. Display and truthiness support
Int64. The simulator is not a verifier or a full CLR boxing implementation.

Extend currently supported arithmetic (add, sub, mul, xor, neg, div) to matched
Int64 operands. Add/sub/mul/neg wrap at the operand width; division truncates
toward zero and refuses zero divisors and MIN/-1 overflow. Correct the existing
i32 division overflow case too, with an explicit regression. Mixed i32/i64
arithmetic/comparisons refuse until explicit conversion support is implemented.
Same-width signed comparisons yield Value::Int(0 or 1); preserve the existing
reference comparison behavior but never project an Int64 through i32.

Use existing panic-based simulator error conventions with clear diagnostics;
do not introduce unchecked narrowing, unbounded allocation or host calls.
Unknown MemberRef tokens must still refuse before MethodDef dispatch.

## Evidence and scope boundaries

Microsoft documents ldc.i8 as pushing an int64 using opcode 0x21:
https://learn.microsoft.com/en-us/dotnet/api/system.reflection.emit.opcodes.ldc_i8
Microsoft documents signed div truncation toward zero and arithmetic exception
for an unrepresentable result, including MIN/-1:
https://learn.microsoft.com/en-us/dotnet/api/system.reflection.emit.opcodes.div

The current simulator wraps MIN/-1 via wrapping_div. The current builder has
only encode_ldc_i4/emit_ldc_i4. Value consumers exist in lang-aot's cil_emit,
cil_cons, cil_lambda, cil_predicates, cil_symbols, conformance,
macsyma_conformance and metacircular tests, plus brainfuck-clr-compiler and
nib-clr-compiler. Update exhaustive matches explicitly; do not conceal Int64
with wildcard success or narrow it in test helpers.

## Validation

Use fixed expected byte sequences and values, including i64 MIN/MAX, both i32
neighbors, negative values and a small int64 constant. Execute hand-built encoded
programs independently of builder tests. Cover local/argument/return round trips,
object-array transport, both truth branches, signed comparisons, width wrapping,
division sign/zero/overflow, mixed widths, malformed ldc.i8 and i64 array indices.
Compile all public consumers and run simulator/builder/backend suites plus the
existing encoded CLR consumers. Run appropriate Clippy and document validation.
Security subagent review precedes push of a ready PR.

Later slices must supply missing arithmetic/conversion opcodes and typed IIR
locals/signatures/boxing before wide IIR literals or input_i64 are enabled.
This slice changes no matrix capability declarations and claims no full CLR ABI.
