# CLR05: encoded bitwise AND and OR

Baseline hand-encoded ldc.i4.1; ldc.i4.2; and/or; ret fails with unknown
opcode 0x5f/0x60 at PC=2. External lang-vm-clr05-baseline-probe.rs and
lang-vm-clr05-baseline.log retain evidence. Builder emit_and/emit_or exist.

Implement simulator and (0x5f) and or (0x60) for matching Int(i32) pairs or
matching Int64(i64) pairs. Compute per-bit conjunction/disjunction, retaining
width and all sign/high bits. These operations are not boolean truth coercion.
Mixed widths require explicit conversions. Native integers are outside scope.

Validate two initialized operands and matching integer widths before stack/pc
mutation. Missing slots, uninitialized slots, references and mixed widths must
refuse with mnemonic-specific diagnostics. Preserve older stack values. Success
consumes two operands, pushes one same-width result, advances pc one byte and
returns a trace with the mnemonic. Keep existing arithmetic helper behavior
unchanged; it currently pops before validation and cannot supply this guarantee.

Tests use independent literal bytecode with fixed expected values: zero/all-one
identities, alternating/disjoint masks, i32/i64 MIN/MAX and bits above bit31.
Cover both operand orders; assert malformed operand diagnostics and unchanged
stack/pc. Existing builder APIs get fixed byte and branch-offset tests. Run
simulator/builder/IIR/BrainfuckCLR/NibCLR suites, Clippy and docs checks.

Authoritative opcode and per-bit semantics:
https://learn.microsoft.com/en-us/dotnet/api/system.reflection.emit.opcodes.and?view=net-10.0
https://learn.microsoft.com/en-us/dotnet/api/system.reflection.emit.opcodes.or?view=net-10.0
The OR page introductory complement wording is a typo; its opcode table and
stack transition explicitly define OR. Matching widths follow the simulator's
existing Int/Int64 contract; this slice adds no implicit widening.

Keep CLR01 wide-immediate/input gates and all IIR lowering metadata unchanged.
The typed IIR addition overflow remains open; this only supplies two previously
unimplemented builder-emitted arithmetic operations.