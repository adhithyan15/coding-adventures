# CLR09: compact `ldc.i4.m1` execution

## Baseline and scope

CLR08 found that `clr-simulator` executes the compact integer constants zero
through eight (`0x16` through `0x1e`) but refuses the adjacent standard
`ldc.i4.m1` opcode (`0x15`). The shared CIL builder therefore emits a valid
one-byte instruction for `-1` that the simulator cannot execute. CLR08 worked
around the mismatch by forcing the five-byte full `ldc.i4` form in its strict
typed-scalar entrypoint.

This slice closes that builder/simulator contract gap only. Add the named
`OP_LDC_I4_M1` constant, execute raw `0x15` as `Value::Int(-1)`, and teach the
simulator's public `encode_ldc_i4(-1)` helper to select the canonical one-byte
form. Preserve the existing encodings for zero through eight, signed-byte
values outside that range, and full-width i32 values. Do not change CLR08's
strict-emitter workaround in this slice; removing it is a separate consumer
change after compact execution lands.

Like the other no-operand compact constants, `ldc.i4.m1` advances the program
counter by one, pushes exactly one initialized int32 value, and has no heap,
local, argument, or call-frame side effects. Int64 remains distinct and is not
produced by this opcode.

Microsoft documents hexadecimal format `15`, stack result `int32 -1`, and that
the special short encodings push four-byte integers:
https://learn.microsoft.com/en-us/dotnet/api/system.reflection.emit.opcodes.ldc_i4_m1

## Validation

Add independent raw-byte execution coverage that proves `0x15; ret` returns
`Value::Int(-1)`, advances by one byte before `ret`, and traces the canonical
mnemonic. Extend encoding-helper coverage to prove `-1` selects `[0x15]` while
`-2`, `-128`, zero, eight, 127 and out-of-short-range values retain their
existing forms. Execute the helper-produced bytes as an integration check.

Run the complete `clr-simulator` suite and Clippy with warnings denied. Update
the simulator README and changelog. No matrix declarations, source routing,
host input, strict scalar types, or other opcodes change.
