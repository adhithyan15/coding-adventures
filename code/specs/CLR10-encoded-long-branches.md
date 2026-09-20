# CLR10 encoded long branch execution

Status: implementation contract, committed before production changes.

## Scope and semantics

Implement simulator br (0x38), brfalse (0x39), brtrue (0x3a). Each has a
four-byte little-endian signed displacement relative to the next instruction
(pc + 5). br preserves the entire stack; conditional branches consume exactly
one initialized Value, use existing Int/Int64/Ref truthiness, and either select
the target or fall through. Preserve value widths. Zero displacement and
negative offsets are valid when the destination stays in the current method.

Sources: [Microsoft br](https://learn.microsoft.com/en-us/dotnet/api/system.reflection.emit.opcodes.br),
[brfalse](https://learn.microsoft.com/en-us/dotnet/api/system.reflection.emit.opcodes.brfalse),
[brtrue](https://learn.microsoft.com/en-us/dotnet/api/system.reflection.emit.opcodes.brtrue).
The simulator supports the three existing Value variants, not additional CIL
pointer types. Full verification of instruction boundaries, stack agreement,
exception regions and prefix targets is outside this bounded execution slice.

## Validation and state contract

Before changing pc or stack, check the complete displacement bytes, compute
next pc and signed destination without wrapping, and require destination in
[0, bytecode.len()). Reject a target equal to method length or outside it,
including for a conditional branch that would not be taken. For conditional
forms reject empty stack and uninitialized None slot before consuming anything.
Use existing panic diagnostics conventions, including mnemonic and PC, with
specific truncated operand, invalid target or stack operand messages. Preserve
pc, stack, locals and call state on refusal. Unconditional br needs no operand.
Do not modify existing short branch behavior in this slice.

## Baseline and validation

External lang-vm-clr09-long-baseline-probe.rs/log confirms all three opcodes
panic Unknown CLR opcode at PC0 before mutation. The earlier control-flow
audit confirms both short conditional outcomes and strict void-hint refusal.

Independent literal-byte tests must cover all opcodes, forward and backward
signed offsets, zero offset, both conditional outcomes, Int/Int64/Ref truthiness,
stack preservation/consumption and truncated/invalid-target/underflow/None
refusal state. Exercise large positive and negative displacements without
host integer overflow. Add builder-to-simulator integration coverage in
lang-aot for automatic short-to-long promotion for all three branch kinds,
with independently checked opcodes/displacements and actual return values.
Run simulator/builder/backend/Brainfuck CLR/Nib CLR suites, focused lang-aot
artifact tests, simulator Clippy, documentation build/shard checks and lessons.

## Exclusions

No strict scalar branch lowering, CFG validation, default source routing,
CLR01 literal/input gate changes, compact ldc.i4.m1 repair, or other branch
families. The next strict control-flow contract still needs definite assignment
at joins, logical Bool conditions, labels, return and reachability validation.

Numbering note: originally specified as CLR09 before implementation; renamed
to CLR10 after discovering active PR #15711 owns compact constant CLR09.
That prerequisite merged in #15711; long-branch publication can now proceed.
