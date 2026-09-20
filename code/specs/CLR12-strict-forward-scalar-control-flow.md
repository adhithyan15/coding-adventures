# CLR12 strict forward scalar control flow

Status: implementation contract, committed before implementation (2026-09-20).
Scope: the opt-in lower_typed_scalars_to_cil API only.

## Boundary and rationale

Extend CLR06/CLR08 scalar lowering with forward-only branches and multiple
returns. CLR10 supplies checked short-to-long branch promotion execution;
CLR11 supplies canonical i32 minus-one constants. Source compilation routing,
input support, host ABI, strings, heap operations and general loops remain out
of scope. No default source overflow correction is claimed.

The first slice deliberately accepts only targets later in the instruction
vector. This is a conservative acyclic subset: even an acyclic graph containing
a backward textual edge is refused. No loop termination analysis or general CFG
support is claimed. Existing straight-line scalar programs remain accepted.

## Instruction shapes

All control instructions have no destination and exactly the following sources:

| Operation | Sources | Hint |
| --- | --- | --- |
| label | one nonempty Var label name | void |
| jmp | one nonempty Var target name | void |
| jmp_if_true | Var condition, Var target name | void |
| jmp_if_false | Var condition, Var target name | void |
| ret | one Var value | exact function return scalar type |

Labels are unique within a function and resolved within that function, in a
separate namespace from value names. Every target must exist and its instruction
index must be strictly greater than the branch index. Labels do not allocate
locals. Conditional operands must be logical Bool, including Bool parameters or
comparison results; i32/i64 truthiness is refused by this API. Control hints are
validated before scalar hint parsing. Unknown operations and malformed shapes
return errors; there is no legacy fallback.

## Graph, reachability and definite assignment

Validate a nonempty instruction vector before emission. Ordinary scalar
instructions and labels have a fallthrough edge to the next instruction. jmp
has only its target edge. Each conditional has both its target and fallthrough
edges, regardless of a constant condition. ret has no successors. Falling off
the vector is invalid, including a trailing label or conditional. Require every
instruction to be reachable from index zero; unreachable instructions are
refused instead of silently discarded. Multiple reachable returns are accepted,
and every terminal path must return an exactly typed scalar.

Keep globally unique single-assignment destinations, disjoint from parameter
names. Preserve the existing requirement that value definitions occur textually
before their uses. In addition, compute definite assignment in instruction order:
entry receives the parameter set; each other instruction receives the
intersection of the outgoing sets of all its predecessors. A value-producing
instruction adds its destination only after checking all its sources. Every
value read, including branch conditions, call arguments and returns, must be in
the incoming set. Label targets and direct callee names are not value reads.
Reject a value assigned on only one arm and read after a join even when its
textual definition precedes that read. Parameters and dominating definitions
survive joins. No phi, duplicate destination per arm, or implicit zero local
initialization may substitute for this proof.

Forward-only edges make this a finite single pass after shape/label/edge
collection. Reject unreachable nodes and malformed graph edges before emitting
an artifact. A failed function or module returns no partial artifact.

## Emission and preserved invariants

Use CILBytecodeBuilder mark and emit_branch with Always/True/False and automatic
short/long selection. Load a validated Bool slot before conditional branches.
Each scalar operation stores its result immediately; each control edge therefore
has an empty evaluation stack. ret loads one validated scalar and returns it.
Use the assembler's structured errors, never raw-byte opcode scanning.

Preserve exact i32/i64/Bool validation, scalar signature checks, direct MethodDef
resolution, canonical constants, CLR01 i32-range integer literal restriction,
256 parameter/local bounds and structural i32 indices. Retain existing arithmetic
and comparison restrictions. No changes to simulator branch semantics, default
source lowering, input gates, token dispatch or public configuration are needed.

## Required acceptance evidence

Backend refusal tests must cover malformed shapes/hints/destinations, duplicate
or missing labels, cross-function targets, self/backward targets, non-Bool
conditions, skipped definitions used at joins, skipped condition/call arguments,
unreachable code, missing returns/falloff and return type mismatches. Include
both conditional kinds and unconditional branches; retain existing scalar tests.

Encoded artifact tests must execute both outcomes of both conditional kinds,
multiple typed returns, a join using dominating values, nested forward branches,
Bool parameters/comparison conditions and i64 values beyond i32 produced through
arithmetic within the literal gate. A forward skip exceeding 127 bytes must
exercise builder promotion and execute correctly; inspect actual method bytes
and resolve the entry by entry_label. Include refusal of a textually earlier but
not definitely assigned value. Preserve tests for scalar boundaries and gates.

Run complete backend tests, relevant lang-aot typed scalar/control flow/long
branch integration tests, builder and simulator suites, backend Clippy, docs
build/check:doc-shards and lessons validation. Update package README/changelog
and backlog with the precise forward-only scope before publication. Security
review must pass at the final full HEAD before every push.
