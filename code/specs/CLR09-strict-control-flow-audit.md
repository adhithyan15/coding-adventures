# CLR09 strict control-flow audit

Status: audit checkpoint, not an implementation contract (2026-09-20).

CLR08 merged as 13e17dec1c05c48570c0c7b3e12dc6b9a599ae8e. No
production changes are authorized by this audit document alone.

## Executed baseline

The external lang-vm-clr09-baseline-probe.rs was run as a temporary lang-aot
integration test, which supplies both lowering and simulator dependencies.
The source and lang-vm-clr09-baseline.log are saved beside the worktree;
the temporary test was removed.

A strict module with const i32, jmp/label with void hints, and ret refuses
UnsupportedType void before opcode dispatch. Independent hand-encoded
brtrue.s (0x2d) and brfalse.s (0x2c) execute both zero/one conditions and
select the expected literal return. Independent long br (0x38) panics with
Unknown CLR opcode: 0x38 at PC=0. This expected baseline failure is not a
passing regression suite. The initial probe also needed correction because
step returns CLRTrace rather than Result; this is recorded in lessons.d.

## Contract decisions still required

Actual IIR branches are jmp, jmp_if_true and jmp_if_false; labels use a Var
name, conditional operands are [Var condition, Var target]. Current strict
lowering admits scalar hints only, single assignments, textual prior uses,
and one final return. A branch extension must define control hints/shapes,
unique labels and valid targets, logical Bool conditions, returns/reachability,
and path-sensitive definite assignment. A skipped assignment cannot become
valid merely because it appears earlier in the instruction vector.

The builder can promote short branches to long encodings. Before enabling
control flow, either implement and independently validate long br/brtrue/
brfalse execution under a preceding detailed spec, or explicitly reject
promoted encodings without returning an unusable artifact. Do not scan raw
bytes for opcode values: immediate payloads can contain those same bytes.
Choose bounded acyclic flow or general CFG deliberately; preserve scalar
widths, 256 slot limits, structural i32 indices and CLR01 literal/input gates.
Default source routing and source i64 narrowing remain separate work.
