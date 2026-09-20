# CLR11: canonical strict-scalar minus-one encoding

## Baseline and scope

CLR09 merged in #15711 and taught `clr-simulator` to execute the standard
compact `ldc.i4.m1` opcode (`0x15`). CLR08 had introduced a local workaround in
`lower_typed_scalars_to_cil`: an i32 constant of -1 was forced to the five-byte
full `ldc.i4` form because compact execution was unavailable at that time.

Remove only that obsolete workaround. Route every accepted i32 constant,
including -1, through `CILBytecodeBuilder::emit_ldc_i4`; the shared builder must
select the canonical one-byte opcode. Preserve i64 `ldc.i8`, CLR01's immediate
range refusal, boolean constants, all strict type validation and default source
routing. This slice does not add branches, host input or new scalar types.

Microsoft documents `ldc.i4.m1` as hexadecimal `15`, pushing int32 -1:
https://learn.microsoft.com/en-us/dotnet/api/system.reflection.emit.opcodes.ldc_i4_m1

## Validation

Replace the old full-form byte assertion with an exact compact-body assertion.
Execute the resulting strict artifact through `clr-simulator` and require
`Value::Int(-1)`. Retain positive tests around i64 -1 so the compact i32 path
cannot accidentally narrow wide constants. Run the complete
`iir-to-cil-bytecode` and `clr-simulator` suites, the affected `lang-aot`
strict-scalar artifact suite, and strict Clippy for the changed backend.

Update backend documentation and changelog to remove the obsolete simulator
qualification. No matrix declaration changes are involved.

Numbering note: this contract was committed under CLR10 before implementation.
It was renamed CLR11 after #15728 landed the independent CLR10 long-branch slice.
