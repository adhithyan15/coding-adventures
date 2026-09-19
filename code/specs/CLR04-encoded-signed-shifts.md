# CLR04: encoded signed integer shifts

Baseline: hand-encoded ldc.i4.1; ldc.i4.1; shl/shr; ret both panic
Unknown CLR opcode at PC=2 (0x62/0x63). Probe and log are saved outside
checkout as lang-vm-clr04-baseline-probe.rs and lang-vm-clr04-baseline.log.
Builder already exposes these opcodes; this slice supplies simulator execution.

Implement shl (0x62) and arithmetic shr (0x63) for Int(i32) and Int64(i64).
The upper stack operand must be Int(i32) shift count, even for Int64 values.
This is an opcode-specific operand contract, not implicit mixed-width arithmetic.
Preserve the value width. Left shift discards high bits and fills low bits with
zero; right shift replicates the sign bit. Counts zero through width minus one
are supported. Refuse negative counts and counts >= width explicitly: CIL
leaves oversized results unspecified, and this simulator chooses refusal rather
than host-dependent masking. Do not claim that CoreCLR must reject these counts.
No native-int or unsigned-right-shift support is introduced.

Before modifying stack or pc, validate two initialized operands, integer value,
i32 count and count range. Refuse references, missing/uninitialized slots and
i64 counts with mnemonic-specific diagnostics. On success consume two slots,
push one result, advance pc by one and emit the trace. Other stack slots survive.

Independent hand-encoded tests must cover both widths/opcodes: zero, one,
width-1 counts, positive/negative values, MIN/MAX, left-shift truncation, signed
right shift (-3 >> 1 = -2), and values outside i32. Check invalid counts -1,
width, width+1 and i32MAX, i64 counts, malformed stacks and preservation of
stack/pc on refusal. Exercise existing builder APIs with fixed expected bytes
and a branch crossing both single-byte instructions. Run simulator, builder,
IIR and existing CLR consumer suites and Clippy; update package docs/changelog.

Authoritative definitions:
https://learn.microsoft.com/en-us/dotnet/api/system.reflection.emit.opcodes.shl?view=net-10.0
https://learn.microsoft.com/en-us/dotnet/api/system.reflection.emit.opcodes.shr?view=net-10.0
These specify i32/native-int counts, i32/i64/native-int values, and unspecified
oversized counts. The shr page has a shr.un typo in one paragraph; its table
and sign-replication description define the signed operation selected here.

Keep CLR01 wide-IIR literal and input gates. No IIR metadata/lowering, boxing,
arrays or host ABI changes. The recorded IIR addition overflow remains open.