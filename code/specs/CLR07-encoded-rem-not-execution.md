# CLR07: encoded remainder and bitwise-NOT execution

## Baseline and scope

CLR06's audit confirmed that the default IIR-to-CIL lowerer emits `rem` (`0x5d`)
for IIR `mod`, but `clr-simulator` has no execution case for that opcode. A
follow-up dispatch audit finds the same gap for the lowerer's raw `not` opcode
(`0x66`). The artifact can therefore be created successfully and then fail as
an unknown opcode when executed. This slice closes only that simulator/lowerer
contract gap; it does not expand CLR06's opt-in strict-scalar opcode set or the
encoded host-input ABI.

Implement signed `rem` and bitwise `not` for both existing stack integer widths.
`rem` requires two initialized operands of the same width and returns that
width. Its quotient truncates toward zero, the result sign follows the dividend,
zero divisors refuse, and `MIN % -1` refuses as arithmetic overflow for both
i32 and i64. `not` requires one initialized integer and returns the same width
with every bit complemented. References, uninitialized slots, stack underflow
and mixed remainder widths refuse before stack or program-counter mutation.

Structural indices and booleans remain int32. No implicit conversion is added.
`rem.un` (`0x5e`) is not implemented because the lowerer emits signed `rem` and
encoded unsigned-64 semantics remain out of scope.

## Validation

Use independently encoded instruction bytes to prove positive and negative
signed remainder, full-width i64 values, bit preservation for `not`, zero and
overflow refusal, and malformed-state immutability. Add source-to-artifact
execution proofs in `lang-aot`, the crate that owns both the lowerer and
simulator dependencies: a Nib `%` program must execute its emitted `rem`, and a
Nib `~0u8` program must execute `not` plus the existing narrow-width mask.

Run the complete `clr-simulator`, `iir-to-cil-bytecode`, and affected
`lang-aot` CLR suites plus Clippy. Keep CLR input refusal and the strict scalar
entrypoint unchanged.

Authoritative CIL references:

- `rem` is opcode `0x5d`, uses truncation-toward-zero division, keeps the
  dividend's sign, and rejects integral division by zero:
  https://learn.microsoft.com/en-us/dotnet/api/system.reflection.emit.opcodes.rem
- `not` is opcode `0x66` and returns the bitwise complement with the same stack
  type as its operand:
  https://learn.microsoft.com/en-us/dotnet/api/system.reflection.emit.opcodes.not
