# BF03 core text/data conformance profile

Profile `core-text-v1`, schema version 1. This is a bounded data and text
component of [BF03](BF03-aot-native-compiler.md), specified before its tests.
It does not execute instructions, resolve labels, allocate machine memory,
invoke syscalls, lower code or implement source maps. Native lane adoption
and six missing-language ports remain separately pending. Package presence,
native extension support and reference-source agreement do not qualify them.

## Typed domain

Programs are exactly `{version, entry, data, instructions}`. Version is integer
1. Data is an ordered array of exactly `{label, size, init}` records. Size is
0..65535 and init is 0..255. Declaration labels are unique. Instructions are
ordered, exactly `{op, operands, id}` records. Operands have these closed tags:

| Kind | Exact other member | Domain |
|---|---|---|
| register | index | integer 0..65535 |
| immediate | value | integer -2147483648..2147483647 |
| label | name | ASCII identifier below |
| text | value | COMMENT payload below, never an execution callback |

All integer fields reject booleans/floats/exponents and overflow before
narrowing. Label names (including entry/data names) match
`[A-Za-z_][A-Za-z0-9_]{0,63}` and exclude `v[0-9]+`, which would be parsed as
a register operand. Keywords remain legal labels: the grammar position and
colon distinguish them. No locale or Unicode normalization applies.
COMMENT payloads are 1..256 printable ASCII characters, without leading or
trailing spaces, `;`, `:`, or a leading `#`. CR, LF and tabs are excluded.
These deliberate restrictions avoid lossy unescaped text; they do not shrink
the general native public type domain. Adapters must explicitly project native
comment payloads to the neutral `text` tag.

Only LABEL and COMMENT have ID -1; they are exempt from ordinary ID uniqueness.
All other instructions, including NOP, have unique IDs 0..2147483647.
Preserve exact IDs, operand/data/instruction order and all values. IDs may be
sparse, nonmonotone in the final stream, or start above zero. Never normalize
illegal LABEL/COMMENT IDs into -1. LABEL is ControlFlow; the exception is not
an exemption for every Meta opcode. Duplicate instruction LABEL names are
rejected; no requirement checks that entry/jump/data references are defined.

New independent ID generators start at 0 and allocate increasing ordinary IDs.
The conformance operation `id_sequence(start, count)` models a bounded suffix:
start 0..2147483647, count 0..256, values `start..start+count-1`, next counter
`start+count`. It fails atomically if an allocated ID exceeds 2147483647.
A next counter of 2147483648 after the last legal allocation is an exhausted
observation, not an allocatable ID. This fixture operation does not assert
that every existing native generator supports a supplied start/reset API.

## Exact named operation table

R = register, I = immediate, L = label, T = text; R/I permits either tag.
Categories are neutral annotations, not native enum ordinals or methods.

| Name | Category | Operand tags |
|---|---|---|
| LOAD_IMM | constants | R I |
| LOAD_ADDR | constants | R L |
| LOAD_BYTE | memory | R R R/I |
| STORE_BYTE | memory | R R R/I |
| LOAD_WORD | memory | R R R/I |
| STORE_WORD | memory | R R R/I |
| ADD | arithmetic | R R R |
| ADD_IMM | arithmetic | R R I |
| SUB | arithmetic | R R R |
| AND | arithmetic | R R R |
| AND_IMM | arithmetic | R R I |
| CMP_EQ | comparison | R R R |
| CMP_NE | comparison | R R R |
| CMP_LT | comparison | R R R |
| CMP_GT | comparison | R R R |
| LABEL | control_flow | L |
| JUMP | control_flow | L |
| BRANCH_Z | control_flow | R L |
| BRANCH_NZ | control_flow | R L |
| CALL | control_flow | L |
| RET | control_flow | (empty) |
| SYSCALL | system | I |
| HALT | system | (empty) |
| NOP | meta | (empty) |
| COMMENT | meta | T |

These 25 names are case-sensitive and closed. MUL, DIV, OR, OR_IMM, XOR,
XOR_IMM, NOT and other extensions are outside this profile. Rust's broader
unversioned lookup remains separate; no enum renumbering or version-2
completion follows. Memory offsets accept a register or an immediate as the
BF03 example requires. No arithmetic or machine overflow semantics are added.

## Canonical text

UTF-8, ASCII within this profile, LF only and one final LF. Parsing in this
profile accepts **only canonical text**; permissive/debug parsers are separate.
Begin `.version 1\n`. Each data record contributes a blank line then
`.data LABEL SIZE INIT\n`. Then a blank line and `.entry ENTRY\n`.
Each LABEL contributes a blank line and `NAME:\n`; each COMMENT contributes
`  ; TEXT\n`. Each ordinary instruction is two spaces, the operation name,
operands, two spaces and `; #ID\n`. With operands, right-pad the operation name
to width 11 and join operand spellings with comma-space. Without operands,
do not pad the opcode. Registers spell `vN`; immediates/IDs/sizes/initializers
use canonical decimal (zero `0`, no `+`, leading zeros, or negative zero).
No other blank lines, annotations, directives, tabs, trailing spaces or aliases
are canonical. Missing/duplicate/reordered/unsupported version/entry and data
after entry are rejected. Exactly these bytes are compared, not stripped text.

The empty-instruction program is `.version 1\n\n.entry _start\n`.
A sparse-ID golden, authored independently of native printers, is:

```text
.version 1

.entry _start

_start:
  LOAD_IMM   v0, -1  ; #42
  ; note
  NOP  ; #7
  HALT  ; #99
```

Both directions preserve this admitted domain exactly:
`parse(print(P)) == P` and `print(parse(T)) == T`. Rust currently retains
padding for zero-operand instructions while TypeScript trims it; this profile
chooses the latter bytes explicitly. Existing native parsers admit additional
forms/version defaults and need separately owned strict adoption; the neutral
validator cannot claim those native rejections or production coverage.

## Corpus, bounds and qualification

`fixtures/compiler-ir-v1/cases.json` is exactly
`{schema_version, profile, opcodes, programs, invalid_programs, invalid_texts,
id_sequences}`. Opcode rows are exactly `{name, category, operands}` in table
order, tags spelled `register`, `immediate`, `label`, `text` or
`register|immediate`. Program cases are `{name, program, text}`; rejection
cases are `{name, program}` or `{name, text}`; ID cases are
`{name, start, count, values, next}`. Case names are unique ASCII identifiers.
Every invalid case must fail with a controlled validation error, never execute
IR. The canonical corpus fixes a complete roster; tests independently pin the
sparse-ID golden and literal opcode semantics against coordinated corruption.
The schema checks structure; the independent standard-library oracle checks
all semantic relations and exact print/parse bytes. No native package is an
oracle and no expected answers are generated from production output.

Bounds: JSON file 1048576 bytes including whitespace, checked by reading at
most limit+1; depth 16 counting root as one before JSON decoding. Maximum 128
total cases, 256 instructions and 32 declarations per program, 4096 aggregate
instructions, 512 aggregate declarations and 12288 aggregate operands across
positive and rejection programs. Source and printed text each have a 65536-byte
ceiling, checked before splitting. Maximum three operands/instruction. Strings
contain Unicode scalars only; semantic domains then impose ASCII limits.
The direct decoded-object validator also rejects cyclic/over-budget trees
(100000 visited values/keys, 1048576 cumulative UTF-8 string bytes).
Reject duplicate decoded JSON keys, malformed UTF-8, BOM, lone surrogates,
nonfinite/float/huge integer literals, trailing JSON, unknown/missing fields
and wrong types. Integer literals are limited to ten decimal digits plus an
optional minus before conversion. These are fixture/oracle work bounds, not
compiler/runtime/sandbox resource or termination guarantees.

Foundation delivery requires specification, schema/goldens, adversarial tests,
measured validator statement/branch coverage, CLI/CI integration, lint/build,
collision/metadata/diff and independent security review. It qualifies no native
lane. Later adopters call actual APIs, test ownership/strict admission, retain
extensions separately, run real package/downstream BUILD fronts and measure
production coverage. Source-map composition remains separately owned.
