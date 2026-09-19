# CLR03: explicit integer width conversions

Selected after the encoded width audit. This bounded prerequisite adds simulator
and builder conv.i4 (0x69) and conv.i8 (0x6a), without changing IIR lowering.

conv.i8 sign-extends Value::Int(i32) to Value::Int64(i64), and preserves Int64.
conv.i4 preserves Int and truncates Int64 to its low 32 bits interpreted as i32.
These are non-overflow-checking integer conversions. Preserve one stack value,
advance pc by one, and include the conversion mnemonic in the trace. A null or
heap reference, empty stack, or uninitialized stack slot must refuse with a clear
conversion diagnostic before changing stack or pc. Floating-point and native
integer representations are outside the simulator's supported value model.

Expose CILOpcode variants and emit_conv_i4/emit_conv_i8 builder methods. Validate
fixed literal opcode bytes independently from hand-encoded simulator programs.
Cover i32 MIN/MAX/-1/0/1 widening; i64 MIN/MAX, both i32 boundaries and neighbors,
2^32 and -2^32 narrowing; same-width identity; comparison boolean promotion
before i64 arithmetic; conversion round trips and malformed operand diagnostics.
Prove mixed-width arithmetic still refuses without an explicit conversion.

Authoritative definitions:
https://learn.microsoft.com/en-us/dotnet/api/system.reflection.emit.opcodes.conv_i4
https://learn.microsoft.com/en-us/dotnet/api/system.reflection.emit.opcodes.conv_i8
The documented integer narrowing behavior discards high bits. Signed widening
preserves the signed integer value. Overflow-checked conversions are separate
opcodes and are not introduced here.

The IIR 2147483647+1 width bug is not fixed by exposing these instructions alone.
Keep CLR01's wide-immediate/input refusals and existing typed lowering unchanged.
Future work must cover the rest of emitted arithmetic and locals/signatures,
array element and boxing/closure representations before enabling wide IIR.
