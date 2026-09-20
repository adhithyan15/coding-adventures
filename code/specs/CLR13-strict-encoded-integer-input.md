# CLR13: strict encoded integer input

Status: implementation contract, committed before production changes.

## Scope and host ABI

Add encoded simulator support for the zero-argument `input_i64` and
`input_more` builtins through two exact MemberRef tokens reserved after the
existing output host calls. Extend only `lower_typed_scalars_to_cil` to accept
`call_builtin` with one Var name source, an i64 destination, and no arguments.
`input_i64` pushes an `Int64`; `input_more` pushes normalized `Int64(0|1)`.
Unknown MemberRef tokens and malformed builtin shapes continue to fail closed.

The simulator owns a byte input buffer and cursor supplied explicitly by the
caller. `input_more` reports whether any byte remains without consuming it.
`input_i64` consumes one line (LF, CRLF, or the final unterminated line), trims
surrounding Unicode-agnostic ASCII whitespace, and parses an exact signed i64.
EOF, an empty line, malformed text, trailing non-whitespace, and overflow all
produce zero after consuming that line. Repeated EOF reads return zero.

Loading bytecode does not invent, clear, or rewind host input. Supplying new
input replaces the buffer and resets its cursor. Host calls do not create call
frames or consume operand-stack arguments; they advance past the five-byte call
instruction and leave existing locals, heap, arguments, and frames intact.

## Validation

Use independently written literal CIL call bytes to prove signed extrema,
malformed/overflow/EOF behavior, LF/CRLF/final-line consumption, repeated
non-consuming peeks, sequential reads, and exact-token refusal without state or
input mutation. Prove strict lowering emits the reserved tokens, preserves i64
metadata, and executes input/peek programs through the simulator. Run the full
simulator and encoded-backend suites, the focused lang-aot artifact tests,
strict Clippy, documentation checks, and repository diff checks.

## Exclusions

Default `lower_iir_to_cil` source routing remains on its legacy int32 register
model and must keep refusing encoded input. String representation and
`input_str`, real CoreCLR metadata/host-class packaging, byte-oriented
Brainfuck input, and general callback registries remain separate work.
