# STP01 — Standalone Starlark opcode-provider compatibility

Status: specified for serial implementation. Profile: `standalone-provider-v1`.

## 1. Component and version boundary

This profile freezes the existing, inert opcode and operator data exposed by
the standalone `starlark-compiler` table providers. It is a component contract,
not acceptance of every API under that heterogeneous package identity. In
particular, Python currently has a metadata-only stub and Ruby exposes a
source/evaluation facade; neither becomes a qualified provider through this
specification. Their existing implementation and applicability owners remain.

[PY03](PY03-starlark-vm.md) describes a future shared Python pipeline with
restricted-mode instruction rules. Those rules do not silently remove bytes
from this retained compatibility profile. `LoadModule`, `ImportFrom`, and
`Print` below are data labels. Possessing or decoding them performs no load,
printing, evaluation, capability grant, sandbox enforcement, or termination
check. A future restricted-mode profile needs its own explicit contract.

AST emission, operands, stack effects, VM execution, host module resolution,
build-tool activation, and source-language syntax remain in their respective
specifications. This profile introduces no production API or authority.

## 2. Normative opcode table

The symbols are case-sensitive ASCII names. Within each row, consecutive
symbols correspond in order to every integer in the inclusive byte range.
The category is a neutral annotation of an assigned opcode, not a promise
that every lane exposes a method with that name.

| Bytes (hexadecimal) | Category | Symbols in ascending byte order |
|---|---|---|
| 01–06 | stack | LoadConst Pop Dup LoadNone LoadTrue LoadFalse |
| 10–15 | variable | StoreName LoadName StoreLocal LoadLocal StoreClosure LoadClosure |
| 20–2D | arithmetic | Add Sub Mul Div FloorDiv Mod Power Negate BitAnd BitOr BitXor BitNot LShift RShift |
| 30–38 | comparison | CmpEq CmpNe CmpLt CmpGt CmpLe CmpGe CmpIn CmpNotIn Not |
| 40–44 | control_flow | Jump JumpIfFalse JumpIfTrue JumpIfFalseOrPop JumpIfTrueOrPop |
| 50–53 | function | MakeFunction CallFunction CallFunctionKw Return |
| 60–64 | collection | BuildList BuildDict BuildTuple ListAppend DictSet |
| 70–74 | subscript_attribute | LoadSubscript StoreSubscript LoadAttr StoreAttr LoadSlice |
| 80–82 | iteration | GetIter ForIter UnpackSequence |
| 90–91 | module | LoadModule ImportFrom |
| A0 | io | Print |
| FF | vm_control | Halt |

Exactly 61 bytes are assigned. The remaining 195 are unassigned, including
`00`. Decode every integer 0 through 255: an assigned byte yields its exact
symbol and category; an unassigned byte yields absence for both. A high nibble
alone never assigns a hole. Encoding an assigned symbol yields its listed byte,
and decode/encode are inverses on these 61 values. Behavior of invalid enum
casts or encoding unknown symbols is outside the typed production API domain;
transport readers reject invalid inputs before any narrowing conversion.

Enumerations use ascending byte order in the neutral representation. Native
enum/atom spellings may be projected explicitly to the table's symbols; do not
change values, synthesize instructions, or infer symbol names by casing rules.

## 3. Exact operator maps

These four separately keyed map domains have the following entries, written
`operator -> symbol`. The order shown is the neutral corpus order; native map
iteration order is immaterial. No trimming, case folding, Unicode substitution,
or other normalization is permitted.

```text
binary (12)
+ -> Add       - -> Sub       * -> Mul       / -> Div
// -> FloorDiv % -> Mod       ** -> Power    & -> BitAnd
| -> BitOr     ^ -> BitXor    << -> LShift   >> -> RShift

compare (8)
== -> CmpEq    != -> CmpNe    < -> CmpLt     > -> CmpGt
<= -> CmpLe    >= -> CmpGe    in -> CmpIn    not in -> CmpNotIn

augmented_assign (12)
+= -> Add     -= -> Sub      *= -> Mul      /= -> Div
//= -> FloorDiv %= -> Mod    **= -> Power   &= -> BitAnd
|= -> BitOr   ^= -> BitXor   <<= -> LShift  >>= -> RShift

unary (2)
- -> Negate   ~ -> BitNot
```

A lookup of a spelling outside its chosen group's exact domain yields absence.
The existence of `Not` does not add `not` or `+` to the unary map. Mutable maps
or enumerations returned to callers must not let one caller corrupt a later
result. Immutable values satisfy the same independence requirement without
allocating mutable copies. Native adoption tests must exercise this property;
JSON table validation cannot prove native ownership behavior.

## 4. Closed corpus transport

`fixtures/starlark-opcode-provider-v1/cases.json` is one UTF-8 JSON object with
exactly these members, with no extension fields:

- `schema_version`: the JSON integer 1, excluding booleans and floats.
- `profile`: exactly `standalone-provider-v1`.
- `opcodes`: the 61 ascending records, each exactly `{symbol, byte, category}`.
- `decode`: 256 ascending records, each exactly `{byte, symbol, category}`;
  the last two values are both null precisely for the 195 unassigned bytes.
- `operators`: exactly the four group names above; each value is the ordered
  list of its exact `{operator, symbol}` records.
- `unknown_lookups`: eight ordered `{group, operator, symbol}` records. Their
  symbol is null. The pairs are binary `@`, binary `++`, binary `＋` (U+FF0B),
  compare `IN`, compare `not  in` (two spaces), augmented_assign `:=`, unary `+`,
  and unary `not`.

JSON bytes are limited to 131,072 including whitespace. Container nesting is
limited to eight, counting the root object as one. Total table rows are 359
(61 + 256 + 34 + 8), always below the ceiling of 512. Symbols/category labels
are at most 64 characters and operator spellings at most 32. Every string must
contain only Unicode scalar values; lone surrogate escapes are invalid.

Reject duplicate object keys, nonfinite JSON constants, trailing content,
unknown members, wrong types, booleans/floats in integer positions, integers
outside 0..255 in byte positions, omissions, extra/duplicate/reordered records,
and any departure from the normative tables. Invalid input is a controlled
validation error, never silently coerced or wrapped to a byte. These are corpus
transport/work bounds; they do not invent runtime resource guarantees for a
language compiler or typed opcode API.

`schema.json` documents the closed structural schema. The standard-library
validator additionally checks exact normative values and completeness against a
separately authored literal oracle. It must not import a provider, execute
Starlark, or generate its expected answers from a fixture or native output.
A coordinated mutation of fixture symbols, bytes, decode and operator records
must still fail. JSON Schema alone does not establish those semantic equalities.

## 5. Delivery and qualification

The neutral foundation delivers the tables, structural schema, strict validator,
corruption/CLI tests, documentation and existing metadata-gate integration.
Commit this spec before tests and source. Measure real validator branch coverage
and test malformed transport, schema drift and coordinated table corruption.

No lane becomes a committed shared-corpus adopter in this foundation. Existing
native reference suites and external actual-API witnesses can substantiate
compatibility checks but do not replace each separately owned lane's durable
corpus reader, BUILD execution, production coverage and ownership tests.
Rust, TypeScript, Go and Elixir reference tables, Python's missing implementation,
and Ruby's unresolved component applicability remain distinct records. Keep
OCaml emerging until its promotion gates pass; this profile does not change the
15-language established denominator or grant any exception.


## 6. Go native adoption

The Go provider's committed test reader consumes the fixed shared cases.json
through standard-library JSON token parsing. It enforces section 4 transport, shape and domain checks before any
provider call or byte conversion, including decoded duplicate keys, scalar-only
Unicode, integer syntax, explicit null versus omission, and bounded input/depth. Semantic table values are checked against actual
provider APIs and independently by the neutral oracle.
All 359 rows must be compared with actual AllOps, OpFromByte, category/name/byte,
four map constructors and direct lookup helpers. Corruption cases must fail;
coordinated fixture changes must not regenerate expected provider behavior.

Retain two returned slices/maps while changing one, then verify both the other
and a fresh result. Cover all four map families, deletion/replacement/insertion,
and slice overwrite/reorder/append; do not assume append allocates new storage.

Only the exact shared cases.json path seeds go/starlark-compiler in native CI
selection, across supported platforms and all/Go filters. Other language filters
and nearby spec/schema/documentation paths do not acquire this reader. Union
normal package changes and reverse dependencies; missing required consumers
fail before writing a plan. Nil/empty changes and force behavior stay intact.
This adoption runs no Starlark source, imports modules or grants host authority.
