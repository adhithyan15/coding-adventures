# VER04r — Warrant Corpus v2 (`warrant-corpus-v2`)

**Parent:** [VER04i](VER04i-polyglot-warrant-evidence.md) (the v1 corpus, lanes, bindings and manifests; this document amends it), [VER04h](VER04h-warrant-derived-evidence.md) (domains, budgets, negative controls), [VER04j](VER04j-warrant-v1-extensions.md) §4.7 and §14.4 (composite domains, `Bytes<N>`, `err_any=`), [VER04k](VER04k-warrant-v1-wave4.md) §4 (WX-G `inout` and `ensures`), [VER04](VER04-warrant-language.md) §3 and §4 (value and arithmetic semantics), [VER29](VER29-layer-contract-linking.md) (layer contracts and strength propagation), [VER30](VER30-warrant-expansion-roadmap.md) §4.4 Track D.

**Status:** specification only (slice S1 of the migration in section 13). Nothing in this document is implemented. The v1 corpus, runners and oracle keep working unchanged until slice S13. Owner decisions of 2026-11-16 are encoded in sections 1, 3, 5, 9, 10 and 13 and are marked **[D1]** (in-lane enumeration plus model IR as a new closed profile), **[D2]** (slim in PR CI, full nightly), **[D3]** (full independent oracle), **[D4]** (one wrapping rule), **[D5]** (Go finishes on v1; v1 deleted in S13) **[D6]** (honest labels preserved) and **[D7]** (every lane runner passes the conformance kit of section 11 before it may write a manifest) where they bind. Owner decisions of 2026-11-17, taken at the start of slice S3, are marked **[D8]** (blind claims), **[D9]** (`ignore-high-bit` stays `ir.fixed`), **[D10]** (when the fixture may be committed) and **[D11]** (eight format details) and are recorded in section 16.3; the normative sections below already read as amended by them.

**Placement.** This is a new spec, not a new section of VER04i. VER04i is already 1,280 lines and carries the history of two shipped lanes; a second profile with its own schema, stream, manifest and migration plan would make it unreadable, and keeping v2 in its own file lets slice S13 delete the v1 text from VER04i without touching this one. VER04i gets a pointer paragraph and one erratum (section 16).

**Two version numbers, never to be confused.** A *document* version is the Warrant language version on the first line of a `.warrant` file: a **v0 document** (VER04) or a **v1 document** (VER04j, VER04k). A *profile* version is the corpus format: the **v1 profile** `warrant-corpus-v1` (VER04i) or the **v2 profile** defined here. They are independent: gf256 is a v0 document compiled to a v2 corpus. This document always says "document" for the first and "profile" or "corpus" for the second.

## 0. Why this exists

VER04i chose the right architecture: enumerate domains inside the lane, evaluate a small language-neutral model IR there, and commit SHA-256 stream digests so that a lane which skips, reorders or mis-evaluates a single case fails. It was built for one adopter, `ct-compare`, and it shows. Measured on the Rust manifests:

| Adopter | Exhaustive cases | Largest suite | Claims |
|---|---:|---:|---:|
| ct-compare | 572,109 | 142,129 | 9 |
| arithmetic | 2,655,437 | 284,258 | 63 |
| logic-gates | 3,770,123 | 1,048,576 | 88 |
| lzw | 17,564,513 | 16,777,216 | 21 |
| wasm-leb128 | 46,565,930 | 16,777,216 | 42 |
| gf256 | 51,472,354 | 16,777,216 | 38 |

Four adopters raise the exhaustive budget to 2^24 in their Rust bindings. Both shipped lane runners cap a suite at 1,000,000 tuples and a run at 8,000,000. Only two adopters fit. Beyond size the v0 generator is closed: one module, five `Bytes<N>` widths, one claim shape (a single subject call compared with a pure reference), no `match`, no tuples, no `inout`, faults chosen by operation name from a Rust `match`, and an oracle pinned to the same shapes. Three different integer semantics are in force today (section 3.1).

**Decision [D1].** Keep the enumerate-in-lane architecture and its model IR, and define a **new closed profile, `warrant-corpus-v2`**. Materialised expected values are rejected: gf256 alone has 51,472,354 exhaustive cases, and a relational claim such as `add(add(a,b),c) == add(a,add(b,c))` has no implementation-independent expected value to materialise.

A v1 extension is impossible by construction: every v1 reader closes its key sets (VER04i section 8), so any new field needs a new profile string. v2 therefore lives beside v1, not in it.

What v2 adds, in one paragraph: structural types and a value codec for every construct the Rust generator supports; predicate claims and multiple subject calls per case; a model IR that grows in declared feature levels; chunk commitments so a lane can run a corpus-chosen subset (`slim`) or everything (`full`) and say which; fault controls as data instead of per-lane code; `inout`/`ensures` operation descriptors; a lane binding contract that covers languages with no `u64` or no byte array; a full independent `.warrant` reader as the oracle; and one wrapping arithmetic rule for generator, oracle and lanes.

What v2 does not change: the three layers of VER04i section 1, the L0 runner protocol of VER04i section 6 (except where this document says so), the failure-id discipline, the rule that evidence is never written by hand, and the honest-limits section (VER04i section 11), which applies to v2 in full.

## 1. Profile identity and compatibility

### 1.1 Directory, files and names

```text
code/specs/fixtures/<module>-warrant-v2/
  corpus.json        authority: types, ops, models, suites, controls, faults, digests (key set: section 1.4)
  chunks/<file>.txt  per-suite chunk commitments; <file> is derived from the claim (section 5.5)
  corpus.profile     neutral line file: budget, chunk size, slim parameters, faults (section 8.4)
  cases.tfv          examples only (TFV v1 value model, section 2.5) [D11g]
  consumers.json  schema.json  consumers.schema.json  README.md  CHANGELOG.md
```

`corpus.json` carries `"profile": "warrant-corpus-v2"` and `"schema_version": 2`. Lane manifests carry `"schema": "warrant-lane-evidence-v2"`. A v1 reader MUST reject a v2 document (unknown profile) and a v2 reader MUST reject a v1 document; there is no mixed mode. The v1 directory `<module>-warrant-v1/` stays byte-frozen until its last consumer has moved (slice S13).

All v2 files are UTF-8 with LF line endings, written by the compiler, and listed in `.gitattributes` as `* -text` so a Windows checkout cannot change a byte. Corpus and chunk files are digest inputs; one CRLF is a different corpus.

### 1.2 The acceptance gate: ct-compare reproduces v1

The generalised compiler is accepted only if **ct-compare compiled under v2 reproduces every one of the nine v1 `inputs_sha256` and nine `expected_sha256` values** of `ct-compare-warrant-v1`. To make that possible a suite keeps v1 bytes whenever v1 can express it:

* A suite whose every type (binders, result, nested) is in the **v1 type set** `{Bool, U8, U64, Nat, Bytes, Bytes<N>, List<v1>, Option<v1>, Result<v1,E>}` uses the `warrant-stream-v1` header and v1 encodings (VER04i section 2.6) for its *suite-level* streams.
* Any other suite uses `warrant-stream-v2` (section 2.4).
* **Chunk digests are an addition, not a replacement.** For every suite the compiler emits the v1 whole-suite digests `inputs_sha256`/`expected_sha256` *and* the chunk list of section 5.5. The v1 whole-suite digest is computed over the unchunked stream. A full-depth lane run reproduces it; a slim run cannot and does not claim it (section 9.2). This is how the gate stays checkable while sharding exists.

### 1.3 Lane manifest path

VER29 section 5.2 puts non-Rust manifests at `evidence/<lane>/test-manifest.json`; VER04i section 8 puts them at `evidence/test-manifest.json` and the shipped Python and TypeScript lanes use the latter. **v2 keeps the VER04i path**, `code/packages/<lane>/<pkg>/evidence/test-manifest.json`, because the lane is already the second path component of the package and a second lane segment would be redundant. VER29 section 5.2 named the other path and is amended by this slice to agree (F3, section 16).

### 1.4 The `corpus.json` key set

`corpus.json` is a **closed** key set: a reader rejects an unknown key, and a writer emits every key below (an empty object or list where there is nothing to say), so that two generators cannot differ by omission. The exceptions are the **conditional keys**, each marked "only" below: when the condition does not hold the key is **omitted**, never written as `null` or an empty string. **Provisional [D11]:** the owner ruled the key set provisional until the oracle's `verify-v2` (S4 part 2) reads it; it grows only by erratum to this section (for example the `applies_to_forms` key of the control row, section 8.3, and S12's `monitored_calls`). It is rendered with the VER04i section 9.1 canonical writer: keys sorted by code point, two-space indent, LF, one final LF, non-ASCII escaped. Sorting loses declaration order, and one rule needs it (a model may call only models declared *before* it, section 4.2 rule 1), so the order is carried as data in `model_order` [D11c]. Counts are JSON integers and every value is a string (section 2.3).

Top level: `profile` (`"warrant-corpus-v2"`), `schema_version` (2), `module`, `document` (`"v0"` or `"v1"`), `spec` (its path), `spec_sha256`, `profile_sha256` (the digest of `corpus.profile`), `generator` (`{"name":..,"version":..}`), `budget`, `chunk_size`, `max_ir_cost`, `levels` (the union of every `requires`), `types`, `ops`, `models`, `model_order`, `fault_models`, `suites`, `examples`, `faults`, `controls`, `accepted_blind_faults`, `assumes`.

* `models` is `{name: {"params": [[name, type]..], "body": IR}}` and `model_order` the array of the same names in declaration order; a consumer validates every `model_call` against that order and rejects a name missing from either.
* `ops` is the shape of section 6.
* `examples` is a list of `{"id", "op", "args", "expected", "result_type", "width"}` with the values untagged and decoded by declared type (section 2.3).
* `faults` rows are those of section 8.1 and `controls` rows those of section 8.3 (`applies_to_forms` is present only on a WX-G control).

A **suite row** has: `id`, `kind` (`"exhaustive"`), `claim`, `ops`, `requires`, `binders` (`[[name, {"type": T, "domain": <descriptor tree>}]..]`, section 5.1), `demoted`, `domain_label` (section 9.3), `admit` (IR), `body` (IR), `oracle` (`{"kind":"reference","subject":IR,"expected":IR}` or `{"kind":"predicate"}`), `result_type`, `stream` (`"warrant-stream-v1"` or `"warrant-stream-v2"`, section 1.2), `product`, `cases`, `discarded`, `expected_calls`, `wraps`, `ir_cost`, `chunks` (the count `m`), `chunks_sha256`, `inputs_sha256`, `expected_sha256` (`reference` suites only), `v1_inputs_sha256` and `v1_expected_sha256` (suites with the v1 header only; the second also only for `reference` suites), `slim_chunks`, `slim_inputs_sha256` and `slim_expected_sha256` (`reference` suites only). The two slim digests are named exactly as the independent oracle emits them [D11b].

## 2. Types and values

### 2.1 Canonical type names

One grammar is used everywhere a type is named: `ops`, binders, stream contexts, manifests and bindings.

```text
type   = "Bool"
       | "U8" | "U16" | "U32" | "U64"
       | "I8" | "I16" | "I32" | "I64"
       | "Nat"
       | "Bytes" | "Bytes<" N ">"        -- N a decimal literal, 0..=4096
       | "(" type { "," type } ")"       -- tuple, 2..=8 components
       | "List<" type ">" | "Option<" type ">"
       | "Result<" type "," ErrName ">"
       | Name ;                          -- a record, variant or error type from `types`
```

`Int` is the mathematical integer of VER04 section 3. It exists **inside the model IR only**: as the type of an intermediate (`Nat - Nat`) and as the type of a `lit` node, `["lit", {"type": "Int", "value": "-3"}]`, which is how a bare decimal literal is written when its context is not `Nat` (a bare literal in a `Nat` context is typed `Nat`) [D11d]. It MUST NOT appear in an op signature, a binder, a stream context, an `examples` row or any other value: a generator meeting it there fails with `W0701`. The one place `Int` may be *written* in a corpus is therefore the `type` field of a `lit` node. (Amended 2026-11-17: the section read "MUST NOT appear in ... a value", which the independent oracle's lowering of a bare literal contradicted.) `Bytes<N>` is accepted for every `N` up to `MAX_FIXED_WIDTH = 4096` (VER04h section 3), not only the five widths the v0 generator knows. Named widths (`Bytes<M>`) stay a static op parameter as in VER04i section 2.5 (`width_params`).

### 2.2 `types` is structural

```json
"types": {
  "SelectError": {"kind": "error",   "cases": ["CT_SELECT_LENGTH_MISMATCH"]},
  "Point":       {"kind": "record",  "fields": [["x","U8"],["y","U8"]]},
  "Decoded":     {"kind": "variant", "ctors": [["Short",[]],["Done",["Nat","Bytes"]]]}
}
```

Rules, all enforced at generation and again by every consumer:

1. Names are unique across kinds; a record or variant may not contain itself, directly or through any chain (no recursion; the depth of a type expression is at most 16).
2. An `error` type has one to 64 cases; case names are unique within it. Cases are compared **by name**, never by position, so reordering declarations in the `.warrant` file cannot change a vector.
3. A record has one to 32 fields; a variant has two to 64 constructors (a one-constructor sum is a record); a constructor has zero to 8 fields.
4. Field and constructor names match `[A-Za-z_][A-Za-z0-9_]*`.

### 2.3 JSON values are decoded by declared type

The corpus never tags a value with its own type. The reader holds the declared type and decodes to it, range-checking; a value that does not fit is a corpus error (`value`), never a truncation.

| Type | JSON form | Decode rule |
|---|---|---|
| `Bool` | `true` / `false` | exactly those literals |
| every integer type, `Nat` | **decimal string** `"18446744073709551615"` | digits only, with one leading `-` allowed for a signed type and only for a non-zero value (`"-0"` is rejected); no `+`, no leading zero except `"0"`; at most 20 digits; then range-checked against the type. A `Nat` *value* is in `0..=2^64-1` (the stream gives it 8 bytes); larger naturals exist only as IR intermediates (section 3.2); an `expected` value of a `Nat` or unsigned result type outside its range, or of a fixed-width type outside its range, or a `Bytes<N>` of another length, cannot be encoded and is `W0701` at generation |
| `Bytes`, `Bytes<N>` | lowercase hex string | even length; for `Bytes<N>` exactly `2N` digits; for `Bytes` at most 4,096 bytes (`MAX_FIXED_WIDTH`) |
| tuple, record | array in component or declared field order | length equals the arity; field names live in `types`, never in the value |
| `List<T>` | array | at most 4,096 elements |
| `Option<T>` | `{"none":true}` or `{"some":v}` | exactly one key |
| `Result<T,E>` | `{"ok":v}` or `{"err":"CASE"}` | exactly one key; `CASE` is in `E` |
| variant | `{"ctor":"Name","fields":[...]}` | exactly those two keys; `Name` declared; field count and types match |

Every integer is text. That removes the JavaScript safe-integer hazard at the source: the only JSON numbers in a v2 document are counts and sizes, each far below 2^53, and the strict reader of VER04i section 9.1 rejects any other number. **Counts are JSON integers; values are strings.**

### 2.4 The canonical stream, version 2

`warrant-stream-v2` extends the VER04i section 2.6 table; every row of that table keeps its bytes.

**Tag allocation (changed 2026-11-16).** This draft first took `60` (tuple, record) and `62` (variant), which collide with VER04l (`60` `Text`, `61` `Char`, merged 2026-11-13) and VER04p (`62` `F32`, `63` `F64`, merged earlier on 2026-11-16). The earliest-merged allocation wins, so VER04l and VER04p keep theirs and this document uses **`64`** (tuple, record) and **`65`** (variant). `11` (signed integers) was free and stays. The single registry of every allocated tag is VER04i section 2.6.1.

| Value | Bytes |
|---|---|
| `Bool` | `00` or `01` |
| `U8`, `U16`, `U32`, `U64`, `Nat` | `10`, then 8 bytes little-endian, unsigned |
| `I8`, `I16`, `I32`, `I64` | `11`, then 8 bytes **two's complement** little-endian, sign-extended to 64 bits |
| `Bytes`, `Bytes<N>` | `20`, u32-LE length, the bytes |
| `List<T>` | `30`, u32-LE count, elements |
| `Option<T>` | `40` (none) or `41` plus the inner value |
| `Ok(v)` / `Err(E.V)` | `50` plus inner / `51`, u32-LE length, UTF-8 case name |
| tuple, record | `64`, u32-LE component count, components in order |
| variant | `65`, u32-LE length, UTF-8 constructor name, u32-LE field count, fields in order |

The two streams, the context prefix, the type names in the prefix and the absence of per-case delimiters are exactly VER04i section 2.6, with `warrant-stream-v2\0` as the header when any type in the suite context is outside the v1 set (section 1.2). A top-level binder tuple remains a plain concatenation of its binder values, as in v1; a tuple *value* (a binder of tuple type, a tuple result) carries the `64` tag. Why a count and a tag where v1 used neither: v1 tuples never occurred nested, so position alone sufficed. Nested composites need self-delimiting values or two different values can serialise to the same bytes.

A worked example. The record `Point(x: U8, y: U8)` with value `Point(1, 2)` encodes as
`64 02000000 10 0100000000000000 10 0200000000000000`. The signed value `-1` as `I8` encodes as `11 ffffffffffffffff`: the type name in the context says it is eight bits, the stream does not.

### 2.5 TFV

`cases.tfv` stays TFV v1. The VER04j section 4.9 requirements are adopted with one resolution of the conflict it recorded: **`Result` stays the `M` form of VER04i section 2.7** (`ok` or `err`). Variants are `L2 S<ctor> L<k> f1..fk`; tuples and records are `L<n> v1..vn`; signed integers are `I<decimal>` with a leading `-` allowed; nesting counts against TFV's 64-container limit. Examples are small and stay materialised in `cases.tfv`; everything large is enumerated. `cases.tfv` holds **examples only**: the phrase "sabotage probes" of the first draft named something no section defined, and v1 has none (its sabotage is the controls of section 8.3, which are data, not cases). A later slice may define probes; until it does, v2 has none [D11g]. For ct-compare the file is byte-identical to the v1 `cases.tfv`.

## 3. Arithmetic: the single rule [D4]

### 3.1 What is wrong today

Three semantics for the same operator exist at the time of writing (measured by reading the sources, not by a test):

| Where | `U64 + U64` overflow | `U8 + U8` overflow |
|---|---|---|
| VER04 section 3 / VER04i section 2.7 (the text) | wraps at 2^64 | wraps at 2^8 |
| TypeScript and Python runners (`ir.ts`, `ir.py`) | **refuse** (a checked error) | refuse |
| the Python oracle (`warrant_corpus_oracle.py`) | unbounded Python integer, no wrap | unbounded |
| the Rust v0 harness (VER04j section 14.4, WD-013) | n/a | evaluated as `Nat`: no wrap |

`ct-compare` has no model arithmetic, so no case has ever exercised the difference. `gf256` is the first adopter that does.

### 3.2 The rule

> **Every `+`, `-`, `*` and unary `-` in the model IR on a fixed-width type wraps modulo `2^w`, in two's complement for signed types. This holds in the generator, in the oracle and in every lane, in every profile level.** (VER04 section 3: "U8, U64 arithmetic wraps".)

| Operand type | `+ - *` and unary `-` | Notes |
|---|---|---|
| `U8 U16 U32 U64` | result mod `2^w`, in `0..2^w-1` | `0u8 - 1u8 = 255u8` |
| `I8 I16 I32 I64` | wrap into `-2^(w-1)..2^(w-1)-1` | `-(-128i8) = -128i8`, `127i8 + 1i8 = -128i8` |
| `Nat` | exact; `Nat - Nat` has type `Int` (VER04 section 4) | an `Int` that must become a `Nat` goes through a typed conversion built-in (`try_nat`, level `ir.fixed`); there is no implicit narrowing |
| `Int` (model-internal) | exact | |

Operands of a binary arithmetic node have the same type after VER04's Nat-to-Int widening; a mixed fixed-width pair is a generation error. Comparisons order values by their mathematical value (an unsigned by its non-negative representative, a signed by its signed value).

**Exact intermediate bound.** `Nat` and `Int` are exact, but the evaluator is not unbounded. The generator evaluates every node and fails generation (`W0701`) if any `Nat`/`Int` value leaves `-(2^127-1)..=2^127-1`, so every value fits a signed 128-bit integer. A lane evaluator MUST compute `Nat`/`Int` nodes with **checked** operations (a product of two in-range values can need 254 bits before the check) and MUST report a violation as `ir-overflow`, never wrap or saturate.

The generator can evaluate only **subject-independent** nodes: those with no `subject_op` beneath them. For these, `ir-overflow` in a lane means the corpus is corrupt (exit 2), because by construction it never happens in an honest corpus. A node *below which a subject call sits* (possible only in a claim body or an `ensures` row) is evaluated on what the subject returned, which no generator can foresee: there `ir-overflow` is an ordinary **case failure** (exit 1, failure id `ir-overflow`), exactly like a false body. Either way a lane never continues with a wrapped exact value. This is the "overflow of a spec-declared width is a corpus error at generation time, so a lane never sees one" sentence of VER04i section 2.7, made concrete.

### 3.3 Frozen v0 documents: the divergence guard

A **v0 document's Rust harness** still evaluates `U8` as `Nat` (WD-013 keeps that frozen so v0 output stays byte-identical). A v2 corpus and that harness can therefore disagree on a case where a `U8` node wraps, and the evidence would split in two without anyone noticing. v2 does not paper over it:

1. The generator counts, per suite, the **subject-independent** fixed-width node evaluations whose wrapped result differs from the exact result (`wraps`, recorded in the suite row; the oracle reproduces it). The count is defined on the **unmemoised** evaluation: every evaluation of a node counts, a memo hit (section 10.4) counts what the recomputation would have counted, and the nodes counted are those of `admit`, of `expected`, of pure call-count guards and of every subject-independent argument of a `subject_op` (`power(a, e + 1u64)` contains one), each once per admitted case (`admit` once per product ordinal). A count that depends on cache state or on which suite ran first is not reproducible and is a defect (section 16.1). The count is taken on the *pure trace* (section 16.5 item 3), and **every pure node on that trace that may wrap or fail is evaluated by the generator**, not only the fixed-width ones: a subject-independent `Nat`/`Int` node leaving the `2^127` window, or a `concat` past `MAX_IR_VALUE`, is a generation error (`W0701`), so a lane never meets `ir-overflow` on a node the generator could have checked (section 3.2, section 4.2 rule 6).
2. For a **v0 document**, `wraps > 0` is a generation error: `W0701: U8 arithmetic wraps; v0 harness evaluates U8 as Nat`.
3. For a **v0 document**, a fixed-width `+ - *` or unary `-` node with a `subject_op` beneath it is refused **statically** with the same code. Counting cannot cover it: whether it wraps depends on what the subject returns, so no generation-time count can prove the two evaluators agree. (`mul(a,b) == mul(b,a)` is unaffected: it has no IR arithmetic node.) The rule also covers a `model_call` with a `subject_op` beneath it whose callee's call closure contains such a node (`model inc(x: U8) -> U8 = x + 1u8` called as `inc(subject(..))`). The Python oracle implements no rule 3 yet (S4 part 2).
4. The adopter that hits rule 2 or 3 migrates the document to v1 (where the Rust harness wraps as well) or reformulates the model. This is the erratum to VER04i section 2.7 stated in section 16.
5. For a v1 document, wrapping is legal everywhere and `wraps` is informational.

The guard is the decided resolution, not a default: WD-013 already records the v0 `U8`-as-`Nat` evaluator as an accepted residual that retires with the gf256 conversion to a v1 document (VER27 section 6 step 6), because fixing it changes every v0 harness byte. The backlog also records that gf256's models are written so that no `U8` expression wraps, so gf256 is expected to pass rule 2; rule 3 is checked in slice S8.

gf256 is the first adopter this touches; it must pass the guard or be migrated to a v1 document before slice S8.

### 3.4 Conformance cases that pin the rule (section 11)

`arith/u8-add-wrap` (`255u8 + 1u8 = 0u8`), `arith/u8-sub-wrap`, `arith/u8-mul-wrap` (`16u8 * 16u8 = 0u8`), `arith/u64-add-wrap`, `arith/u64-mul-wrap` (`MAX * MAX = 1`), `arith/i8-neg-min`, `arith/i64-sub-wrap`, `arith/nat-sub-is-int` (`3 - 5 = -2` as `Int`), `arith/nat-bound-generation-error` (a corpus whose `Nat` node exceeds `2^127` MUST be rejected by the oracle), `arith/v0-u8-wrap-guard`. They are specified in section 11; this document does not create the fixture files.

## 4. The model IR

### 4.1 Feature levels

Precomputed expected values fail on size and on relational claims, so the IR stays and grows in **feature levels**. A level is a closed set of tags and built-ins. Every corpus declares `requires` per suite (and per fault, and per op); every lane declares the levels it implements in its binding (`levels`).

| Level | Content | First needed by |
|---|---|---|
| `ir.core` | the tags of Appendix A (the 21 tags v1 emits, plus `neg`, `concat`, `take`, `drop` and `is_err`), and the wrapping rule of section 3 | ct-compare, gf256 |
| `ir.fixed` | integer built-ins: `wrap widen try_nat checked_add checked_sub checked_mul div rem shl shr sar rotl rotr bit_and bit_or bit_xor bit_not popcount clz ctz be_bytes le_bytes from_be from_le`, plus `at` | wasm-leb128, lzw, x25519 |
| `ir.data` | tuple, `project`, `match` (on variants, `Option` and `Result`), constructors and projection of records and variants, `some`/`none`, list literals, and `len`/`concat`/`take`/`drop`/`at` on lists | logic-gates, arithmetic |
| `ir.seq` | WX-D `fold map all any range ...` with a meter. **Reserved**: the level name is closed but has no content in this document; a suite, op or fault that needs it is `not-live` in every lane until an amendment to this spec defines the v2 meter (it depends on WD-D1 slice 2a) | zeroize's `all(result, is_zero)` postcondition |
| `ir.inout.records` | record-shaped `inout` host forms (section 7.6). **Reserved** in the same way | the 4004 flip-flops |

Semantics of the `ir.fixed` built-ins are those of VER04j (WX-B) and VER04k; slice S10 states each one a second time as conformance vectors (section 11) because eighteen evaluators will implement them. `ir.core` adds `concat`, `take`, `drop` and `is_err` relative to the v1 tag set (an erratum against memo-level plans, section 16): ct-compare's v1 faults (`ignore-last-byte`, `missing-error`) need byte surgery and a result test, and faults are IR from the first v2 slice (section 8).

A suite that needs a level the lane does not declare is recorded **`not-live`** in the manifest with its id and counted in `not_live`. So is a **control** whose fault needs such a level (section 8.2): its manifest row has the state `not-live` and names the deferring level, and it counts in `not_live` too. `not_live` is the number of not-live suites **plus** the number of not-live controls. Nothing is skipped silently (VER04j section 8, item 4), and a lane whose `not_live` is non-zero cannot be `release-ready` for that adopter: release-ready requires both counts to be zero.

### 4.2 Evaluation rules

1. Expressions are trees; there is no recursion, no loop and no mutable state. Models are pure and acyclic: model `m` may call only models declared *before* it, where "before" is the order of the `model_order` array of `corpus.json` (section 1.4), not the key order of `models`.
2. Arguments are evaluated left to right. `and`, `or`, `implies` and `if` **short-circuit**; an untaken branch is not evaluated and, for call counting, contributes no subject calls (section 4.4). A `match` evaluates exactly one arm.
3. `subject_op(name, args[, widths])` dispatches through the lane's binding to the real export. It is forbidden inside every model body and inside every fault expression. `model_call(name, args)` invokes a pure model. Built-in pure operators (`eq`, `lt`, `add`) are IR tags and never resolve through the binding table (VER04i section 2.5, unchanged).
4. Literals keep the v1 typed form `["lit", {"type": T, "value": V}]` with `V` per section 2.3; a Bool literal is `["lit", true]`. `T` is also allowed to be `Int` (section 2.1): a bare decimal literal is typed `Nat` in a `Nat` context and `Int` otherwise, and a suffixed literal (`1u8`) has the type of its suffix. The signed minimum (`-128i8`) is `neg` of a `lit` holding `-2^(w-1)` (Appendix A).
5. Equality is structural and type-directed (VER04 section 3): sequence order matters, errors are nominal, `Bool` is not `U8`, allocation identity is never compared.
6. A static **cost bound** per expression, computed before the first case; exceeding it is a corpus error. Because the IR has no loop and no recursion, this static bound is the whole fuel: no run-time step meter is needed at `ir.core`, `ir.fixed` or `ir.data`. (Amended 2026-11-16, section 16.1; the v1 profile keeps the wording of VER04i section 6.2.)

   **Cost model: worst path.** The cost of an expression is the largest number of node evaluations any single evaluation of it can perform under rule 2:

   | Node | Cost |
   |---|---|
   | `lit`, `var`, `err` | 1 |
   | `if` | 1 + cost(condition) + **max**(cost(then), cost(else)) |
   | `match` (`ir.data`) | 1 + cost(scrutinee) + **max** over the arms |
   | `model_call` | 1 + the sum of its arguments + the cost of the model's body |
   | `subject_op` | 1 + the sum of its arguments (the subject's own work is not IR work) |
   | every other tag, `and`/`or`/`implies` included | 1 + the sum of its operands (a short circuit is never credited) |

   A model body's cost is computed **once per model, in declaration order**, so the computation is linear in the size of the corpus IR. Memoisation (section 10.4) is never credited. The arithmetic is checked: a chain `m_k(x) = m_(k-1)(m_(k-1)(x))` doubles the cost per level, so an implementation stops and reports the violation as soon as a partial sum exceeds the ceiling below; it never wraps and never builds a big number. This is a sound bound and not an estimate, because rule 2 obliges every evaluator to evaluate exactly one branch of an `if` and one arm of a `match`: an evaluator that visits more nodes than the worst-path cost is nonconforming for a reason unrelated to cost. Charging both branches (the reading of VER04i section 6.2) is also sound but is not a bound on work, it is the size of the fully inlined tree, and it is exponential in model depth for the ordinary idiom of a bit-sliced model whose four branches each call the previous model once.

   **The bound: a default, a declared budget and a hard ceiling.**

   * `MAX_IR_COST = 2,000` is the **default** bound per expression (each `admit`, `body`, `expected` and fault expression, and each model body, separately).
   * A corpus whose models need more declares `max_ir_cost N` in `corpus.profile` (section 8.4) with `2,000 <= N <= MAX_IR_COST_CEILING = 65,536`. The compiler copies `N` into `corpus.json` as `max_ir_cost`; every consumer uses the recorded value, and a recorded value above the ceiling, or an expression above the recorded value, is a corpus error. The profile line is the **only** way to raise the bound: no compiler, oracle or lane has a command-line or environment override, because an override is a second place for the limit to live and the report could not show it.
   * Each suite row records `ir_cost`, the sum of the worst-path costs of its `admit`, its `body` and (for a `reference` suite) its `expected`. The compiler computes it, the oracle reproduces it (section 10.3 check 2), and a lane recomputes it before the first case and treats a difference as a corpus error. Three independent implementations therefore agree on the cost model, not only on the limit.
   * **Total IR work is bounded as well**, which is what keeps the resource guarantee when the per-expression bound rises. The IR work of a suite in a run is `ir_cost` times the number of product ordinals in the chunks that run executes; the sum over all suites, counted as `MAX_TOTAL_WORK_*` counts cases (section 14, control re-runs included), is at most `MAX_IR_WORK_SLIM = 64,000,000,000` for a `slim` run and `MAX_IR_WORK_FULL = 8,589,934,592,000` for a `full` run. These are exactly `32,000,000 x 2,000` and `2^32 x 2,000`, the products the case limits and the old cost bound already implied; the amendment states them instead of leaving them implicit, so a corpus may spend a larger cost on few cases or a small cost on many, never both. Checked before the first case, in checked arithmetic.

   Measured on gf256 with the S4 oracle (section 16.1): charging both branches, `xor8` costs 294,900 and `ginv` 92,014,191; on the worst path `gmul` costs 2,011 and `ginv` 26,204, the largest expression is 26,210, four of the 38 suites exceed 2,000, and the whole document needs under `2 x 10^9` node evaluations at `full`. gf256 therefore declares `max_ir_cost 32768`. ct-compare's largest expression costs 10 (the body of `eq_agrees`, of each `eq_fixed_agrees_wN` and of `eq_u64_agrees`: 1 + 3 + (1 + 2 + 3)) and declares nothing.
   A node count does not bound **value size**: `m1(x) = concat(x, x)` composed with itself through nine model levels (`m2(x) = m1(m1(x))`, and so on) costs about 1,500 nodes and multiplies a length by `2^256`. Therefore every `Bytes` or list value an evaluator produces is at most `MAX_IR_VALUE = 65,536` bytes or elements, checked **before** the allocation; the generator enforces it on subject-independent nodes (`W0701`), and a lane reports a violation as `ir-overflow` under the split of section 3.2 (corpus error when subject-independent, case failure otherwise). Exact integers are bounded by section 3.2 and shift counts by the operand width.
7. Unknown tags, unknown built-ins and any construct of an undeclared level are errors in every consumer. An unknown tag is never skipped.

### 4.3 Claims are predicates

A suite carries a Bool IR `body`, in which `subject_op` may appear **anywhere**, and an `oracle` field:

```json
"oracle": {"kind": "reference", "subject": <IR>, "expected": <IR>}
"oracle": {"kind": "predicate"}
```

* **`reference`** applies when `body` is `subject == pure`, in either order, with `expected` containing no `subject_op`. This is exactly the v1 shape. The compiler evaluates `expected` for every admitted case and commits `expected_sha256`; a wrong subject result cannot change it, so it must fail the comparison.
* **`predicate`** is every other well-typed Bool body: `mul(a,b) != 0u8`, `add(add(a,b),c) == add(a,add(b,c))`, `if b != 0 then mul(div(a,b),b) == a else true`. There is **no `expected_sha256`**, and it is *not* replaced by a hash of an all-true stream: nothing implementation-independent exists to hash. The suite is labelled `relational` (section 9.3), which the report must show, and the oracle verifies its inputs, case counts and call counts but cannot supply expected values.

A claim whose body calls no operation at all is refused with `W0701` (it would pass for any implementation, and its `expected_calls` would be zero).

Why not force every claim into `reference` form: `mul_commutes` has two subject sides and no model; refusing it would exclude the relational claims that find the most bugs. The price is honesty, paid in the label.

### 4.4 Call counts stay implementation-independent

`expected_calls` is the exact number of `subject_op` invocations in an unfaulted complete run (VER04i section 4.4). With several calls per case it is computed from the checked body as follows:

* A subject call may sit under a **guard** (the condition of `if`, the right operand of `and`/`or`/`implies`, a `match` arm) only if the guard's condition contains **no `subject_op`**, i.e. it is *pure*: it depends only on binders and models. Otherwise generation fails with `W0701`.
* Subject calls nested in the *arguments* of another subject call (`add(add(a,b),c)`) are unconditional and each counts once.
* For each admitted case the count is the number of `subject_op` nodes on the evaluation path under the pure guards. The compiler and the oracle each compute `expected_calls` from this pure trace, so the count never depends on what the subject returned. A lane's wrapper counts invocations and the manifest `calls` must equal the sum (`call-count-mismatch`).

For `add(add(a,b),c) == add(a,add(b,c))` the count is 4 per case. For gf256 `div_inverts` with a pure guard `b != 0` it is the number of admitted cases times the calls on the taken branch.

### 4.5 Admission

`admit` is a Bool IR over the binders (the `forall` refinements) and contains no `subject_op`. A tuple whose `admit` is false is **discarded**: it is counted, receives no case id, and costs no subject call (VER04i section 2.6). A suite that admits zero cases fails generation, with the code `W0706` (the v1 generator's code for an empty admitted domain).

## 5. Domains, enumeration order, chunks and depth

### 5.1 Resolved descriptor trees

A binder carries a **fully resolved descriptor tree**, emitted *after* the budget demotion of VER04h section 3 so that a lane never re-runs demotion (the single largest cause of silent drift between a generator and a consumer). The tree has these nodes; every node records its `size`:

| Node | JSON | Order of its values |
|---|---|---|
| leaf | `{"d":"bool"}`, `{"d":"u8_all"}`, `{"d":"u64_edge"}`, `{"d":"nat_0_64"}`, `{"d":"bytes_d1"}` | exactly VER04i section 2.6 |
| `bytes_n` | `{"d":"bytes_n","n":N}` | VER04i section 2.6, for any `N` in `0..=4096` |
| `fx_full` | `{"d":"fx_full","t":T}` for a fixed-width `T` of 8 or 16 bits other than `U8` (a `U8` binder is `u8_all` in every document version) | every value, numerically ascending (signed from `MIN` to `MAX`) |
| `fx_edge` | `{"d":"fx_edge","t":T}` for a fixed-width `T` of 32 or 64 bits other than `U64` (a `U64` binder is `u64_edge` in every document version) | for unsigned width `w`: `0..=255`; then `1<<i` for `i = 8..=w-1`; then `MAX`; then `MAX ^ (1<<i)` for `i = 0..=w-1`; for a signed type the same bit patterns reinterpreted in two's complement, in the same order (`u64`: 377 values, `u32`: 313) |
| `nat_count` | `{"d":"nat_count","k":K}` | `0..=K`, `K <= 4096` (`nat_0_64` is `K = 64`) |
| tuple / record | `{"d":"tuple","of":[...]}` | Cartesian product of the components, **last component varying fastest** |
| variant | `{"d":"variant","ctors":[[name,[desc..]]..]}` | concatenation, in declaration order, of each constructor's field product |
| option | `{"d":"option","of":D}` | `None`, then `Some(x)` for `x` in `D` |
| result | `{"d":"result","ok":D,"errs":[names]}` | `Ok(x)` for `x` in `D`, then each error case in declaration order |
| list | `{"d":"list","of":D}` | by length: length 0; length 1 and length 2 over `D'`, the first 16 values of `D`; length 3 over the first 4 values; within one length lexicographic by value index, last position fastest |

A descriptor tree is at most 16 levels deep. `size` is recomputed by the consumer and MUST equal the declared value (checked arithmetic; an overflow is an error, never zero). These are VER04j section 4.7 and VER04h section 3 written down **once**; lanes never re-describe them.

Every descriptor defines two total functions that the consumer implements: `size()` and `nth(i)`, the `i`-th value (0-based, `i < size`). `nth` is what makes chunk jumping possible (section 5.4); it is O(depth + value size), never O(i).

### 5.2 Product enumeration and case identity

The product of binders `b1..bk` has `P = size(b1) * ... * size(bk)` **product ordinals**, row-major, last binder fastest. Product ordinal `p` unranks to indices by mixed-radix division. A tuple is **admitted** if `admit` is true; the **admitted-case ordinal** `i` counts admitted tuples in product order. The case address `<suite id>@<i>` of VER04i section 2.4 is unchanged, and it remains stable under chunking (section 5.4).

### 5.3 Budget and demotion

The budget is a corpus constant read from `corpus.profile` (default `2^24 = 16,777,216` product ordinals per claim; the Rust bindings of the four large adopters already use it) and recorded as `budget` in `corpus.json`. `W0706`/`W0707` and the `DOMAIN_BYTES_BUDGET` of VER04h section 3 apply unchanged. A binder demoted to fit appears in the manifest with its demotion, so the label is honest (section 9.3). Claims have **at most 8 binders** (v0's 3 was a generator limit, not a language one); the product is computed in checked arithmetic.

### 5.4 Chunks

A suite's product space is cut into **chunks of `chunk_size` product ordinals** (65,536 unless `corpus.profile` lowers it; always a power of two in `2^10..=2^16`, one value for the whole corpus, recorded in `corpus.json`; a consumer uses the recorded value and MUST NOT assume the default). Chunk `c` covers product ordinals `[c*chunk_size, (c+1)*chunk_size)`; the number of chunks is `m = ceil(P / chunk_size)`, at most `MAX_CHUNKS = 4,096` (so the largest product, `2^28`, needs the default size, and a smaller size is only legal for a corpus whose every suite still fits 4,096 chunks). The upper limit of `2^16` is what keeps a slim run bounded: at most 32 chunks of at most 65,536 ordinals (section 5.6). Boundaries are on **product ordinals, not admitted ordinals**, so a lane can unrank `c*chunk_size`, enumerate from there, and never touch an earlier chunk. Each chunk row records:

| Field | Meaning |
|---|---|
| `cases` | admitted tuples in the chunk |
| `first_admitted` | the admitted-case ordinal of the chunk's first admitted case (equivalently, the count of admitted cases in earlier chunks) |
| `inputs_sha256` | digest of the chunk's input stream |
| `expected_sha256` | digest of the chunk's expected stream; present only for `reference` suites |
| `calls` | `expected_calls` contributed by this chunk |

A case's global address is `first_admitted + (position among the chunk's admitted cases)`, so a failure in chunk 200 is reported as `...@<i>` and reproduced in any lane or in Rust by index, with or without the earlier chunks.

**Chunk streams.** Each chunk has its own pair of SHA-256 streams. The prefix is the VER04i section 2.6 context (header, suite id, binders, result type) followed by the u32-LE chunk index; the body is the typed values of that chunk's admitted cases in order. Chunk digests are therefore independent and merge by *listing*, not by hashing a combined stream (the point VER04i section 3 required before any consumer accepts sharding).

**Suite digests.** `inputs_sha256` of a chunked suite is `SHA-256("warrant-chunks-v1\0" || u32-LE m || for each chunk, in order, its 32 raw inputs-digest bytes)`; `expected_sha256` likewise over the expected digests. This is the v2 suite commitment. The v1 whole-suite digests of section 1.2 are kept in the suite row as `v1_inputs_sha256`/`v1_expected_sha256` for suites that use the v1 header.

### 5.5 Chunk files

The chunk file of a suite is `chunks/<file>.txt`, where `<file>` is the suite's claim field with every `/` replaced by `.` (the synthesized `<op>/post` suite of section 7.6 becomes `<op>.post.txt`). A claim field is one or two names matching `[A-Za-z_][A-Za-z0-9_]{0,63}` joined by `/`, so `<file>` contains no path separator, no `..` and no leading dot; two suites whose `<file>` values are equal ignoring ASCII case are a generation error (a case-insensitive file system would merge them). A consumer **derives** the path from the validated claim field; `corpus.json` carries no path, so there is nothing for a hostile corpus to point elsewhere (section 14). The file has one chunk per line, exactly `m` lines: `<c> <cases> <first_admitted> <calls> <inputs_hex> [<expected_hex>]`, single spaces, decimal counts, 64-digit lowercase hex, LF. A 2^24-case suite has 256 chunks, about 35 KB. `corpus.json` records the file's SHA-256 (`chunks_sha256`), the chunk count and the suite digests; a consumer cross-checks the file against all three, and that `sum(cases)` equals the suite's `cases`, `first_admitted` is the running prefix sum, and the discarded count equals `P - sum(cases)`.

### 5.6 Depth: `full` and `slim` [D2]

A run has a **depth**:

* **`full`**: every chunk of every suite.
* **`slim`**: for each suite, the corpus-chosen subset recorded in `slim_chunks`. The subset is chosen **by the corpus, never by the lane**, so a lane cannot pick the chunks it likes.

**Slim selection (normative, computed by the compiler and verified by the oracle).** Let `m` be the chunk count. If `m <= 16`, `slim_chunks` is every chunk (the suite is `full` even in a slim run). Otherwise let `k = ceil(m / 14)` and `o = FNV-1a-64(suite id) mod k`, where FNV-1a-64 is taken over the UTF-8 bytes of the suite id with offset basis `0xcbf29ce484222325` and prime `0x100000001b3`, in wrapping 64-bit arithmetic. The **base set** is `{0, m-1} ∪ { o + j*k | j >= 0, o + j*k < m }`: the first and last chunk, plus every `k`-th chunk from an offset derived from the suite id (at most 16 chunks; a 2^24 suite has `m = 256`, `k = 19`).

**Witness chunks.** A stride cannot know where a fault becomes visible, so the corpus adds the chunks that prove its own controls are observable. For each control row, take the *candidate claims*: every claim that mentions the control's op. For each candidate that is a `reference` suite whose subject call takes the suite's binders directly as arguments, the compiler lets the reference stand in for the subject: it applies the fault (a `pre` fault re-evaluates `expected` at the rewritten binders; a `post` or `replace` fault rewrites `expected`'s value) and searches for the **lowest admitted case** at which the faulted value differs from `expected`. A candidate is then one of three things:

* **witnessed**: a case differs. That case is the claim's *witness*; its chunk is added to `slim_chunks`;
* **blind**: the stand-in exists and no admitted case differs, so the fault is unobservable by that claim at any depth [D8];
* **unproven**: there is no stand-in (a `predicate` suite, or a subject call whose arguments are not plain binders), so nothing can be said before the lane runs.

The control row lists the witnessed and unproven candidates, in suite order, as `required_claims`, and the blind candidates as `blind_claims`; `required_claims` therefore lists **the claims not proven blind** (the witnessed and the unproven), and a claim proven unable to catch a fault is never credited with catching it (section 8.3). Its `slim_witness` is a list with one entry per `required_claims` entry, in the same order, of exactly three shapes: `{"claim": C, "chunk": c}` for a witnessed claim; `{"claim": C, "unproven": true}` for an unproven one (no stand-in exists); and `{"claim": C, "deferred": "ir.fixed"}` (the level is the one named) for a claim that *has* a stand-in but whose fault the generator cannot yet evaluate because it has not implemented that level (section 8.2). A deferred entry is not unproven: S10's witness search replaces it with a `chunk` entry or moves the claim to `blind_claims`. An unproven claim fails closed as before: the lane's control either fails inside the slim set or is `uncaught-fault`, which fails the lane (section 8.3), and the remedy is a `slim_chunk <claim> <index>` line in `corpus.profile`, which can only **add** a chunk. **Generation fails only when every candidate claim is proven blind**: the fault is then unobservable by the whole corpus, and it is an error unless `accepted_blind_faults` names it. A control with no witnessed claim but at least one unproven (or deferred) one passes generation, and is caught or not at lane time (`uncaught-fault`, section 8.3); a blind-plus-unproven control is such a control. A claim that is blind for one fault is not an error: the claim may be there for other faults or for other ops.

*Worked example (ct-compare, verified against the v1 fixture and `ct-compare.warrant`).* Three of v1's eight controls list a claim that can never catch the fault. `eq_fixed/ignore-last-byte` lists `eq_fixed_agrees_w0`, whose binders are `Bytes<0>`: there is no last byte to ignore (the fault is guarded by `len(p) > 0`), so no case differs. `select_bytes/missing-error` lists `select_picks`, which admits only `len(a) == len(b)`: `select_bytes` never returns `Err` there, so there is no error to go missing. `select_bytes/swap-choice` lists `select_rejects_mismatch`, which admits only `len(a) != len(b)`: the result is `Err` whichever way `choice` is flipped. Under the rule above the rows become `eq_fixed/ignore-last-byte`: `required_claims` `w1 w2 w4 w16`, `blind_claims` `w0`; `select_bytes/missing-error`: required `select_rejects_mismatch`, blind `select_picks`; `select_bytes/swap-choice`: required `select_picks`, blind `select_rejects_mismatch`. Each control keeps at least one witnessed claim, so generation succeeds, and the three `required_claims` lists differ from v1's (control ids do not, and the section 1.2 gate concerns digests only). Read literally, the earlier wording (every `required_claims` entry must be witnessed unless `accepted_blind_faults` names the fault) would have made v2 generation of ct-compare fail, or put three faults that the corpus *does* catch on the accepted-blind list.

`slim_chunks` is the ascending, duplicate-free union of the base set, the witness chunks and the `slim_chunk` lines, at most `MAX_SLIM_CHUNKS = 32` per suite (more is a generation error), hence at most 2,097,152 cases. The list is written out in `corpus.json` (the lane never recomputes it) together with `slim_inputs_sha256` and (for a `reference` suite) `slim_expected_sha256`, the digests of the chosen chunks' inputs digests and expected digests under the same `warrant-chunks-v1` framing, parallel to the two suite digests of section 5.4 [D11b]; the oracle recomputes the list, base set and witnesses included. `corpus.profile` may set `slim_stride` to make the slim set denser, never sparser: a suite uses `min(profile stride, its automatic k)` (a stride larger than a suite's automatic `k` is **clamped** to it, per suite; the clamp is never an error and gives more chunks, never fewer, but the cap of 32 slim chunks above can still refuse a suite whose base set, witnesses and `slim_chunk` lines together exceed it), and the profile can never remove the first or last chunk. [Owner decision 2026-11-17, section 16.5.]

**What slim does and does not cover.** Product order is row-major, so a chunk is a contiguous run in which the *last* binders vary: binders whose joint size is at most `chunk_size` are enumerated completely in every slim chunk, boundary values included. What slim thins is the *leading* binders: for three `U8` binders it runs 16 of the 256 values of the first and all 65,536 pairs of the other two. Edge values of a leading binder other than its first and last (`2^63` in a leading `u64_edge`) are therefore usually **not** run at slim. That is a coverage reduction, not a soundness hole, for three reasons that the rest of this document enforces: the manifest and the report say `slim(n of m chunks)` and can never say more (section 9); every control must still be caught inside the slim set or the lane fails; and the missing chunks are exactly what the `full` depth runs. The offset `o` depends on the suite id, which a spec author chooses; an author could pick a name to steer the stride, but the author also writes the claims, the label is unaffected, and witness chunks do not depend on `o`.

**Policy.** PR CI runs `slim` for every lane. The nightly job runs `full` on the compiled lanes (rust, go, c, cpp, java, kotlin, csharp, fsharp, swift, haskell, ocaml, dart-native); interpreted lanes (python, ruby, elixir, lua, perl, typescript) run `slim` in PR CI and nightly `full` only where their wall time allows, and say so. A lane declares its depth in the binding (`depth full|slim`); the manifest records the depth actually run (section 9.1). **A slim run never claims `full`**, in any file a human or the report reads.

Negative controls always target the `slim` chunk set, in ascending chunk order, and stop at the first failure (a faulted run never writes a manifest, VER04i section 4.3).

## 6. Op descriptors

Each `ops.<name>` row in `corpus.json` is **language-neutral**: ops are bound by Warrant name only, and nothing about a host language appears in the corpus.

```json
"ops": {
  "multiply": {"params": [["a","U8"],["b","U8"]], "ret": "U8", "requires": ["ir.core"]},
  "zeroize":  {"params": [["buf","Bytes"]], "ret": "Bytes",
               "inout": ["buf"], "post_slot": "S1",
               "ensures": [["eq", ["len", ["var","result"]], ["len", ["var","buf"]]]],
               "requires": ["ir.core"]},
  "select_bytes": {"params": [["a","Bytes"],["b","Bytes"],["choice","Bool"]],
                   "ret": "Result<Bytes,SelectError>", "fresh": true,
                   "requires": ["ir.core"]}
}
```

| Field | Meaning |
|---|---|
| `params` | ordered `[name, type]` pairs |
| `width_params` | static widths, as v1 |
| `ret` | the declared Warrant result type |
| `fresh` | the result must not share storage with an argument (section 7.4) |
| `inout` | parameter names in parameter order; empty or absent for none. Not allowed with `fresh` |
| `post_slot` | `"S1"`..`"S4"`, the VER04k section 4.2 rule that locates the post-state in the result; required iff `inout` is non-empty |
| `admit` | the parameters' refinements as Bool IR (no `subject_op`) |
| `ensures` | rows: Bool IR over the parameters (for `inout` the **pre-state**) and the contextual name `result` (the declared return, post-state included). No `subject_op`, no quantifier; at most 32 rows |
| `requires` | the feature levels the op's `admit` and `ensures` need; always present, `["ir.core"]` at least (the rows of the example above all carry it) |

A corpus is **self-describing for lanes**: a lane needs no Warrant parser to know which parameter is `inout` and where the post-state sits in the result.

## 7. The lane binding contract

### 7.1 The binding file

The binding stays the data-only line file `evidence.bindings` of VER04i sections 5.1 and 5.2. v2 adds directives; every directive is data and no line contains an expression or code.

```text
lane / package / module / import / source / seed          (unchanged, VER04i section 5.1)
depth  full|slim
levels ir.core,ir.fixed,...
values Bool=.. U8=.. U16=.. U32=.. U64=.. I8=.. I16=.. I32=.. I64=.. Nat=.. Bytes=.. List=.. Option=.. Tuple=..
type   <WarrantName> host=<export> fields=a,b,c | ctors=A,B
op <name> export=<name> [args=<host,..>] [ret=<host>] [width=implicit|runtime]
          [errors=CASE:<disc>,..] [error=CASE] [err_any=CASE]
          [inout=<param>:<form>,..] [fresh=probe|immutable]
```

`depth` is the depth the lane's PR run uses. `levels` lists the IR levels the runner implements; a suite needing another is `not-live` (section 4.1). Each `values` entry names a host representation from that lane's **closed list** (section 7.2); a name outside the list is `BindingInvalid` at start-up, before any case runs.

### 7.2 Integers: one exact evaluator, host types at the boundary

The IR evaluator in every lane uses **one exact integer representation with a type tag and wraps per node** (section 3). It is never the host integer type of the library under test. Host types appear only at the boundary, where each value is **range-checked** on the way in and out. A model value the host type cannot hold is never truncated, wrapped or padded (VER04j section 14.4 rule). The case is still hashed into both streams, its subject calls are **not made**, and the suite row counts the case in `unreachable_by_type` and the calls it would have made in `unreachable_calls`. The call check becomes `calls + unreachable_calls == expected_calls`, so a skipped call is always accounted for and never passes as made. Because a lane could otherwise declare everything unreachable and call nothing, three things bind the count: it is zero for every type whose row below holds the full range (every `U`/`I` type in every lane, by bit pattern), so only `Nat` and composites containing it can be unreachable; the manifest checker recomputes it from the corpus and the binding's `values` line without running the subject; and a suite with a non-zero count is labelled `type-limited` and can never be `full` (section 9.3). The result check is exact: an integral value of the wrong representation (a float `1.0` for `U8`, a boxed object for a `long`) is `type-mismatch`.

| Lane | `U64` / `I64` at the boundary | narrow types (`U8..U32`, `I8..I32`) | `Nat` |
|---|---|---|---|
| rust, c, cpp, go, csharp, fsharp, swift | native 64-bit, unsigned / signed | native width | `u64`-sized where the host type holds it |
| kotlin | `ULong` / `Long` | `UByte..UInt` / `Byte..Int` | `ULong` |
| haskell | `Word64` / `Int64` | `Word8..Word32` / `Int8..Int32` | `Natural` or `Word64` (values past `Word64` are unreachable by type) |
| ocaml | `Int64` with unsigned operations (`Int64.unsigned_compare`, `unsigned_div`); `int` (63-bit) is **refused** | `int` is allowed only for widths of at most 32 bits | `Int64` |
| python, ruby, elixir | big integers, **range-checked** (not masked) at the boundary; Python results checked with `type(x) is int`, so a `bool` is refused; Elixir with `is_integer/1` and `===`, so `1.0` is refused; Ruby with `Integer` exactly | same | same |
| typescript, dart (web) | `bigint` / `BigInt`; `number` is **refused** | `number` is allowed at the boundary only, and only when integral, in range and not `-0` (`Object.is`); the evaluator stays exact | `bigint` |
| java | `long` carrying the bit pattern; unsigned compare/div/rem through `Long.compareUnsigned`, `Long.divideUnsigned`, `Long.remainderUnsigned`; `java.math.BigInteger` in the evaluator | `U8` as `byte` (bit pattern) or `int` (range-checked); `U16` as `short` or `int`; `U32` as `int` (bit pattern) or `long` (range-checked) | `long` (a value above `2^63-1` is unreachable by type) or `BigInteger` |
| lua | Lua 5.3+ integers; `U64` is the 64-bit pattern, compared with `math.ult`; the runner **refuses to start** if `math.type` or `math.ult` is absent (Lua 5.1, 5.2, LuaJIT) | integers in range; a float result is `type-mismatch` even if integral | integer `0..2^63-1` (larger is unreachable by type) |
| perl | native `UV`/`IV` only if `$Config{ivsize} >= 8 && $Config{uvsize} >= 8`, else `Math::BigInt`; **`use integer` is never used**; a result carrying a float flag (`NOK`) is `type-mismatch` | integer scalars in range | same as `U64` |

**Why Java, Lua, Perl and JavaScript need a table at all.** Their native integer type is signed 64-bit (Java, Lua), a double (JavaScript `number`) or either depending on the build (Perl). The corpus contains `u64_edge` values from `2^63` upward precisely so a lane that silently holds `U64` in a signed or floating type fails an enumerated case, not a code review. The runner's start-up checks (`math.type`, `ivsize`, `typeof 1n`) turn a wrong toolchain into exit 2 instead of a wrong verdict.

### 7.3 Bytes, lists, records and variants

`Bytes` and `Bytes<N>` use a **closed list of representations per lane**, checked by exact type (VER04i sections 5.1 and 5.2): `bytes` (Python), `Uint8Array` with an exact prototype and a non-detached buffer (TypeScript), `byte[]` (Java, C#, Kotlin), `[]byte` (Go), `uint8_t *` plus a length (C), a byte string or binary where the language has no byte array (Lua: a string; Perl: a byte string for which `utf8::downgrade` succeeds, so **wide characters are rejected**; Elixir: a binary). `Bytes<N>` additionally asserts the length equals `N` on the way in and out (a wrong-length result is a failure, never padded).

`List<T>` binds to the lane's sequence type named in `values` (`List=`) and a tuple to the host form named by `Tuple=` (a host tuple, a fixed-length array, or multiple return values where the language has them); a component or element type with no host mapping is `BindingInvalid`, and a returned tuple of the wrong arity is `type-mismatch`. Records and variants bind through `type` lines: `type Point host=Point fields=x,y` gives the exact host field names in the corpus field order, and `type Decoded host=Decoded ctors=Short,Done` gives the constructor names. Positional matching is never used for constructors; the name is the portable key (VER04j section 4.9). A lane with no sum types binds a result as a tagged struct, which is a binding concern.

### 7.4 Aliasing and freshness

1. Every mutable argument is a **fresh copy for each call**, and no two arguments of one call share storage. (Aliasing is legal in C, Java, C# and Go; the adapter is what makes it impossible.)
2. After the call, every argument that is **not** `inout` must equal its pre-call copy. A difference is the new failure id **`arg-mutated`**.
3. A `fresh` result must not share storage with any argument: the runner mutates the result and re-reads the arguments, then mutates an argument and re-reads the result (VER04i section 5 rule 4). `fresh=immutable` is accepted only where the type is immutable by construction (Elixir binaries, Python `bytes`).
4. An `inout` post-state is read from **the very object that was passed**, after the call, never from a copy taken before (VER04k section 4.6 rule 2). A fixed-length form must keep its length; a changed length is `type-mismatch`.

### 7.5 Error mapping

Mapping is **closed and matched exactly**. A host error discriminator is one of: a raised class (a built-in, or, new in v2, a class exported by the bound module and resolved at start-up), a Go sentinel (`errors.Is` on the exact value), a C status code, an Elixir tag, a `Left` constructor, or `panics` (only with a single-case `error=`).

* A raised class matches only when the raised object's class **is** the mapped class (`type(e) is cls`; `Object.getPrototypeOf(e) === cls.prototype`); a subclass, a duck-typed object and a thrown non-`Error` are `unexpected-error`. A bare `Error`/`Exception`/`BaseException` mapping is refused, so a binding cannot swallow arbitrary failures.
* `err_any=CASE` requires an error type with **exactly one case** (VER04j section 14.4): `Err(_)` does not say which error it was, so with two or more cases a wrong error would pass vacuously.
* For a multi-case error the runner **refuses a binding in which two cases of one op share a discriminator**; otherwise a wrong error would map to the same case.
* Anything unmapped is `unexpected-error`; a model `Err` answered with a value is `missing-error`.

Two shipped ports already force this: TypeScript `gf256` throws a plain `Error` (which the binding refuses) and Go `gf256` panics. Slice S8 gives TypeScript a dedicated error class and binds Go with `panics` plus a single-case `error=`.

### 7.6 `inout` and `ensures` in a lane

The corpus follows VER04k section 4.9: the result is encoded as its declared type and `ops.<op>` names `inout` and `post_slot`. The runner's adapter, **written once per lane**, (a) keeps the oracle's pre-state and gives the subject a fresh copy, (b) reads the post-state back from the object it passed, (c) assembles the Warrant result by the slot rule (S1 to S4), and (d) destructures the host return exactly: an extra or missing return component is `type-mismatch`.

The binding names a **host form per `inout` parameter**:

| Form | Meaning | Rust forms of VER04k section 4.6 it covers |
|---|---|---|
| `mutable` | a fixed-length mutable byte array, mutated in place | `slice`, `array` |
| `growable` | a growable sequence (Java `ByteArrayOutputStream`/`List<Byte>`, Go `*[]byte`, C buffer plus `size_t *len`) | `vec` |
| `cell` | a one-element holder, for languages with no reference to an integer or boolean | `int:<ht>`, `bool` |
| `returned` | the export returns the post-state as (part of) its result; immutable languages (Haskell, Elixir, OCaml, Python `bytes`) | `returned` |

The record forms (`struct:`, `slice-of:struct:`, `vec-of:struct:`, `codec:`) are reserved: a corpus using them is `not-live` (level `ir.inout.records`) until a later slice. **`ensures` rows** are IR. The runner's call wrapper evaluates them after every call whose arguments satisfy `admit`; a false row fails the case with the note `postcondition-violated: <op> row <i>`, and an unevaluable row fails with `postcondition-unevaluable`. The compiler emits the synthesized `warrant/<m>/exhaustive/<op>/post` suites as **predicate** suites (section 4.3), plus `postcondition_rows` and `monitored_calls`, which the lane manifest must reproduce.

`monitored_calls` must not depend on the subject, or a lane could not be checked against it. Whether a call is monitored depends on its arguments satisfying `admit`, and the arguments of an outer call in `add(add(a,b),c)` are subject results. So the corpus commits, per op and per depth (`{"slim": n, "full": n}`), the count of calls of that op whose arguments are **subject-independent** and admitted; the lane must reproduce the number for the depth it ran. Calls with subject-dependent arguments are monitored in exactly the same way (a false row still fails the case) and are reported in `monitored_dynamic`, which is informational and is never compared.

A row may use any built-in, including WX-D ones (VER04k section 4.2): `zeroize`'s `all(result, is_zero)` needs `ir.seq`. An op whose rows need a level the lane lacks, or a reserved level, has its `/post` suite and its monitoring recorded `not-live`; the rows are never partly evaluated.

**Honest limits.** A `returned` binding does not observe in-place mutation, and the report says so (`inout_form`, section 9.1). No lane gives evidence that a wipe survives the optimiser: `zeroize` shows the post-state is zero, not that the compiler kept the store (VER04i section 11).

## 8. Faults and sabotage controls

### 8.1 A fault is data

VER04i section 4.2 made each fault a per-lane wrapper with a name-keyed table (`faults.ts`, the Python equivalent). v2 moves the table into the corpus:

```json
{"fault": "generator-shift", "op": "multiply", "kind": "post",
 "expr": <pure IR over the op's params and result>, "requires": ["ir.core"]}
```

There are exactly three kinds:

| Kind | Effect |
|---|---|
| `pre` | rewrites parameters before the call: a `rewrites` list of `{"param": "a", "expr": ...}`, each expression over **all** the op's parameters as they were passed in, and all rewrites applied **simultaneously** (one rewrite never sees another's output). The row has `rewrites` instead of `expr`; a fault that rewrites every `Bytes` or every `U64` parameter (section 8.2) has one entry per such parameter [D11a] |
| `post` | rewrites the result: `expr` over params and `result` |
| `replace` | supplies the result without trusting the call's value (the call is still made and counted) |

`result` is the one reserved name for the call's result, in a fault expression and in an `ensures` row alike (section 6); an op with a parameter named `result` is a generation error (`W0701`) [D11a]. (The first draft wrote `r` here; two spellings would have been two reserved names.) A lane implements these three wrappers **once** and deletes its per-fault table. Fault expressions use only tags of their declared level and **never `subject_op`**.

### 8.2 Generic and adopter-specific faults

The portable names of VER04i section 4.2 become IR templates instantiated by signature; a few examples (full list in `corpus.json.faults`):

| Fault | Kind | IR (sketch) | Level |
|---|---|---|---|
| `invert` | post | `["not", result]` | core |
| `missing-error` | replace | `if is_err(result) then Ok(first Bytes arg) else result` | core |
| `swap-choice` | pre on `choice` | `["not", choice]` | core |
| `off-by-one` | post | `["add", result, ["lit", {type: T, value: "1"}]]` (wraps, section 3) | core |
| `truncate-last` | post | `take(result, len(result) - 1)` guarded by `len(result) > 0` | core |
| `ignore-last-byte` | pre on each `Bytes` param | `concat(take(p, len(p) - 1), b"00")` guarded by `len(p) > 0` | core |
| `xor-lsb` | post | `["bit_xor", result, 1]` | fixed |
| `ignore-high-bit` | pre on each `U64` param | `["bit_and", p, MAX >> 1]` | fixed |
| `ok-as-err`, `wrong-ctor`, `field-zero`, `drop-consumed`, `swap-components` | post | composite rewrites of VER04j section 4.8 | data |

**`ignore-high-bit` stays `ir.fixed` [D9].** ct-compare's `eq_u64` control uses it, and ct-compare is otherwise an `ir.core` document, so until slice S10 teaches the generator, the oracle and the lanes `ir.fixed`, that one control is recorded **`not-live`** in every lane (it is counted in `not_live`, never skipped and never an `uncaught-fault`), and the lane cannot be `release-ready` for ct-compare while it is (section 4.1). An `ir.core` rewriting of the same fault (`if p >= 2^63 then p - 2^63 else p`) was considered and declined by the owner: the table keeps one definition per fault, in the level whose built-in it names. Because a generator that has not implemented `ir.fixed` cannot evaluate this fault, it emits the fault row as data (`requires: ["ir.fixed"]`) and the control row with `requires: ["ir.fixed"]`, every candidate claim in `required_claims` and `slim_witness` entries `{"claim": C, "deferred": "ir.fixed"}` (section 5.6; these are not `unproven`, since `eq_u64_agrees` has a stand-in and only the evaluator is missing), and leaves the witness search to S10. The fail-closed rule that an unimplemented level refuses a *suite* (section 13, "Slice S3 in detail") does not apply to a fault whose level only gates a control, because no **suite or chunk digest** depends on the fault. The same is not true of the slim digests in general: a witness chunk feeds `slim_chunks` and so `slim_inputs_sha256`. For ct-compare nothing changes, because every suite has `m <= 16` and so is `full` in a slim run (section 5.6), but when S10 adds the witness search this control row changes (its `slim_witness` entries) and so can the slim sets of any suite with `m > 16`.

**Adopter-specific faults** (`alt-poly`, `exp-mod-256` in gf256) reference **fault-only models** emitted under `fault_models`: pure model bodies that may exist *only* for a fault and may never appear in a claim, so the corpus cannot be made to depend on them.

**WX-G faults** are all `post` rewrites over the result and the pre-state parameters: `stale-post` (result is the pre-state), `skip-last-write`, `drop-last-push`, `clobber-prefix`, `cursor-off-by-one`, with the applicability table of VER04k section 4.8. Two of those conditions name a host form (`drop-last-push` and `clobber-prefix` need a growable buffer, `cursor-off-by-one` an integer cell), which the language-neutral corpus cannot know. The corpus therefore lists the control with `applies_to_forms: [...]` in section 7.6 vocabulary; a lane whose `inout_form` for that parameter is not listed records the control as `not-applicable` with the form, and the manifest checker verifies that against the op row's `inout_form`. A lane cannot mark any other control not-applicable.

### 8.3 Controls, scoping and catching

VER04i section 4 is otherwise unchanged and applies to v2:

* Controls are per `(target suite kind, op, fault)`. Each perturbs **exactly one op**; a control passes iff the suite run with that single-op fault fails with a registered failure id. The scoping exists because a fault applied to every operation can cancel inside a relational claim: XOR-ing the low bit of every result turns `mul(a,b) == mul(b,a)` into `mul(a,b)^1 == mul(b,a)^1`, which is still true. This matters *more* in v2, where relational predicates are common.
* Each control row lists `required_claims`, the claims not proven blind (section 5.6), and `blind_claims`, the claims proven unable to witness the fault; the manifest's `caught_by` must be a non-empty subset of `required_claims`. An unrelated failure cannot credit a control, and a blind claim never appears in `required_claims`. A control no claim can catch is `uncaught-fault` and fails the lane unless the spec's `accepted_blind_faults` names it. A **not-live** control (section 4.1) is the one exception: the lane does not run it, its manifest row has `caught_by: []` and the state `not-live` with its `level`, and it is neither caught nor `uncaught-fault`; it does not fail the lane, but it counts in `not_live` and so blocks `release-ready`. A not-live row with a non-empty `caught_by`, or a control recorded `not-live` whose `requires` levels the lane declares, is rejected by the manifest checker. The control row is `{"id": "warrant/<module>/negctl/exhaustive/<op>/<fault>", "op", "fault", "target_kind", "requires", "required_claims", "blind_claims", "slim_witness"}` plus `applies_to_forms` for the WX-G faults of section 8.2 (a key that is not applicable is omitted, section 1.4). `requires` is the level of the fault's expression, and is what a lane that lacks it uses to record the control `not-live` (section 4.1).
* A faulted or filtered run **never writes a manifest**. The call counter (`calls == expected_calls`) and the fact that a faulted run must reach the real export remain the guards against a runner that never calls the library (VER04i section 4.4).
* In a `slim` run controls run against the `slim` chunk set and stop at the first failing case.

### 8.4 `corpus.profile`

Fault selection and the knobs of this document live in a **neutral line file**, `code/specs/fixtures/<module>-warrant-v2/corpus.profile`, read by the compiler and by the oracle's independent reader and by nothing else:

```text
budget 16777216
chunk_size 65536
slim_stride auto
max_ir_cost 32768
fault multiply generator-shift
fault multiply alt-poly
fault zeroize stale-post
```

`max_ir_cost` is optional (default 2,000, at most 65,536; section 4.2 rule 6).

The compiler checks that its fault names equal the Rust bindings' `faults=` lists for the same module, so the two cannot drift; the oracle checks the same file against `corpus.json`.

## 9. Manifest additions

### 9.1 Schema `warrant-lane-evidence-v2`

The v1 manifest of VER04i section 8, as a new closed key set, plus:

| Where | Field | Meaning |
|---|---|---|
| top level | `depth` | `"full"` iff every live suite ran every chunk, else `"slim"`; the depth of the *run*, set by the runner from what it executed, never copied from the binding or from a command-line flag. It says nothing about `not-live` suites, so every place that prints `depth` prints `not_live` beside it |
| top level | `levels`, `not_live` | the IR levels implemented, and the count of suites **and controls** recorded `not-live` (section 4.1) |
| suite row | `depth` | `"full"` or `"slim"` for this suite (a suite with `m <= 16` is `full` in any run) |
| suite row | `chunks_run`, `chunks_total`, `cases_run` | what actually executed |
| suite row | `chunks` | for each executed chunk, its recomputed `inputs_sha256`/`expected_sha256` (the sidecar-free place the checker compares) |
| suite row | `oracle` | `reference` or `relational` |
| suite row | `domain_label` | `full`, `bounded-set` or `sample` (VER04i section 8 sidecar, moved into the row: LP-012) |
| suite row | `unreachable_by_type`, `unreachable_calls` | cases a host type could not hold, and the subject calls not made for them (section 7.2) |
| suite row | `wraps` | informational (section 3.3) |
| top level | `conformance_sha256` | digest of the conformance fixture's `index.json` the runner passed (section 11) |
| op row | `inout_form` | the host form used per `inout` parameter, so a `returned` binding shows as such |
| `ensures` ops | `monitored_calls`, `postcondition_rows`, `monitored_dynamic` | the first two reproduced from the corpus for the depth run; the third informational (section 7.6) |
| control row | `applicable` | `true`, `not-applicable` with the host form (section 8.2), or `not-live` with `level`, the level the corpus control row's `requires` names and the lane lacks (sections 4.1, 8.2); a `not-live` control has an empty `caught_by` (section 8.3) |

The coverage-labels sidecar `domain-labels.json` of VER04i section 8 is **retired for v2**: its content is in the row, so one file cannot drift from the other.

### 9.2 What each depth may assert

| Depth | `inputs_sha256`/`expected_sha256` of the suite | Report may say |
|---|---|---|
| `full` | equals the corpus suite digests (and, for v1-header suites, the v1 whole-suite digests) | the label of section 9.3 |
| `slim` | equals the corpus `slim_inputs_sha256` and `slim_expected_sha256` over the chosen chunks; the suite digests cannot be recomputed | `slim(n of m chunks)`; never `full`, never `exhaustive` |

### 9.3 Honest labels [D6]

The three domain labels survive per suite and per lane: **`full`** (every binder enumerates its whole type: `bool`, `u8_all`, `fx_full`, `Bytes<0>`, `Bytes<1>`), **`bounded-set`** (a fixed strict subset chosen as the type's edge cases: `u64_edge`, `fx_edge`, `bytes_d1`, `nat_count`, and any composite or list over them) and **`sample`** (a small draw from a huge space: `bytes_n(N>=2)`, which is a few dozen of `256^N` values, and every demoted binder). The VER04i sidecar called `bytes_n(N>=2)` a `bounded-set`; VER04j section 14.4 later named it `sample(SIZE)` in the Rust manifest, and v2 follows the later and stricter word, so the ct-compare `eq_fixed` suites move from `bounded-set` to `sample` when it migrates (the case counts do not change, only the heading they are printed under). A suite with `unreachable_by_type > 0` carries the extra tag `type-limited`. A suite is as weak as its weakest binder. A **slim run, or a run with any case the host type cannot hold, can never be labelled `full`**; the effective strength of a suite is the minimum of its domain label, its depth and its oracle kind (`relational` is a separate tag, not a strength, but a relational suite never counts as an independent reference). Report headlines are computed **per label**, e.g. `full: 65,537; bounded-set: 506,572; slim(16 of 256): 1,048,576`; the word "exhaustive" is never a total (VER04i section 8, LP-012).

## 10. The independent oracle [D3]

### 10.1 Obligation

Today's oracle re-reads the `.warrant` source with its own reader and shares no code with the compiler (VER04i section 2.8). **That independence MUST survive**: an oracle that only re-evaluated the corpus IR would miss every lowering bug, which is the bug class v2 grows the most. The oracle is a **full independent `.warrant` source reader**, shares no code, data file or generated table with the Rust generator, and **gates each adopter**: an adopter's v2 corpus cannot merge until the oracle reads that adopter's spec and its three checks pass.

### 10.2 Shape

A stdlib-only package at **`code/packages/python/warrant-oracle/`** (distribution `coding-adventures-warrant-oracle`, import package `coding_adventures_warrant_oracle`) with five parts, each independently testable: the reader (`lexer`, `parser`, `ast`: source text to AST; fail-closed), `lower` (AST to IR), `ir_eval` (the IR evaluator with the section 3 rule), `domains` and `suites` (descriptor trees, `size`/`nth`, enumeration and chunking), `stream` (the section 2.4 encoders and SHA-256 framing). It is a package and not a directory under `code/scripts/` (the path this section named before the amendment of section 16.1) because the build tool discovers a `BUILD` file only under `code/packages/` and `code/programs/`: under `code/scripts/` its tests, its lint and the gate itself would not run in CI, and an oracle that CI does not run gates nothing. Living under `packages/` gives it no licence to depend on anything: it has no runtime dependency, imports no other package of this repository, and its `BUILD` runs the gate of section 1.2. The old `code/scripts/warrant_corpus_oracle.py` is the v1 oracle and stays until S13.

### 10.3 The three checks

1. **Lowering.** The oracle's own lowering of the spec equals the IR in `corpus.json`, compared as canonical JSON (the v1 canonical writer of VER04i section 9.1). Two independent implementations can only agree byte for byte if lowering is **syntax-directed and unoptimised**: one IR node per source construct, operands in source order, no constant folding, no reassociation, no common-subexpression sharing, no inlining of models, desugarings (`a != b`, `x.0`, refinements into `admit`) exactly as section 4 and Appendix A name them. Any normalisation the generator performs is a spec change here first. A generator that lowers `a - b` as `b - a` is caught here and nowhere else.
2. **Enumeration and evaluation.** The oracle's own enumeration and evaluation reproduce, for every suite and every chunk it verifies: `cases`, `discarded`, `first_admitted`, `calls`, `inputs_sha256`, and for `reference` suites `expected_sha256`, plus the suite digests, `slim_chunks`, `slim_inputs_sha256` and `slim_expected_sha256`, `expected_calls`, `wraps` and `ir_cost` (and the corpus `max_ir_cost` against `corpus.profile`).
3. **Faults and controls.** It re-derives every fault instantiation, `fault_models`, control row and `required_claims`, `blind_claims` and `slim_witness` set from the spec, the op signatures and `corpus.profile`, and compares them with `corpus.json`.

### 10.4 Scaling rule and cost

The reader grows with the feature levels (section 4.1): it refuses, with a named error, every construct of a level it does not implement in a suite, op or model; **an unknown construct is never skipped**. The one exception is a *fault row* (and its control) of a level the oracle lacks: check 3 must still re-derive it, so the oracle treats such a row as **data**, instantiating the template and comparing it without evaluating it (and expecting `deferred` witness entries, section 5.6). A naive Python pass over 51 million gf256 cases is, by estimate (not measured), far too slow for PR CI. Two measures are specified:

* **Memoisation.** Pure model calls are memoised on scalar arguments in a bounded table. gf256's models are functions of two `U8` values, so a three-binder claim becomes table lookups. The table is a cache only: a hit and a recomputation MUST agree, and a test samples both.
* **Parallel chunk verification.** A chunk is the unit of work, so verification parallelises across processes and CI jobs with no shared state.

**What is verified when.** "Gates each adopter" [D3] means no chunk digest reaches `main` unverified:

| Trigger | Oracle work |
|---|---|
| a PR that changes a spec, a `corpus.profile`, or any file of a v2 fixture directory | **every chunk of every suite** of that corpus, as a required check (sharded across jobs by chunk if one job is too slow), plus checks 1 and 3 |
| every other PR | checks 1 and 3; an **admission-only pass** over every chunk (no model evaluation, no hashing) that reproduces every `cases` and `first_admitted`; full digests for the `slim` chunks and for every chunk of suites with at most 2^20 product ordinals |
| nightly | every chunk of every suite, again |

The suite digests of section 5.4 are a framing over the chunk digests in the chunk file; the oracle always recomputes that framing, and it is only as verified as the chunk digests beneath it, which is why the first row is not optional. The cost of the first row for gf256 is not measured; if it does not fit CI even when sharded, that is a blocker for S8 to report, not a reason to merge an unverified corpus.

### 10.5 Trust boundary

The oracle reads the spec file from a PR and is therefore a parser of hostile input (section 14). It is also not infallible: a wrong claim is faithfully verified against (VER04i section 11, item 2) and the oracle only shrinks that gap.

## 11. Conformance: `warrant-corpus-conformance-v1` and the v2 cases

**Fixture family.** Every lane runner must pass `warrant-corpus-conformance-v1` (VER04i section 9.1, WD-100; delivered on branch `claude/warrant-lane-conformance` and possibly not yet on `main`): the strict JSON reader, canonical writer, `tagged` values and stream, containment and label digest categories. v2 **adds categories to the same fixture family** with the same layout (`index.json`, one JSON file per category, `{"schema": "warrant-corpus-conformance-v1", "category": C, "cases": [...]}`, `* -text`, per-category case counts compared against `index.json`, bytes as lowercase hex). The existing categories are not edited, so v1 lanes keep passing. This document **specifies** the cases; the fixture files are created by slice S5 (and extended by S10 to S12).

**New categories and the cases each must contain.**

| Category | Case fields | The lane runs | Expected |
|---|---|---|---|
| `v2-value` | `type`, `types`, `input_hex` | the section 2.3 decoder then the section 2.4 encoder | `{"outcome":"accept","stream_hex":X}` or `{"outcome":"reject","error":"value"}` |
| `v2-arith` | `type`, `op`, `lhs`, `rhs` (decimal strings) | the evaluator node | `{"value":S}` or `{"error":"ir-overflow"}` |
| `v2-ir` | `levels`, `ir`, `env` | the IR evaluator | `{"value":V,"calls":N}` or `{"error":"unknown-tag"}`, `{"error":"level-missing"}` |
| `v2-domain` | `descriptor`, `index` or `range` | `size()` and `nth()` | the `size` and the values (hex-stream) at the given indices |
| `v2-chunk` | `suite`, `chunk` | enumerate one chunk by unranking and stream it | `cases`, `first_admitted`, `inputs_sha256`, `expected_sha256` |
| `v2-fault` | `fault`, `args`, `result` | the pre/post/replace wrapper | the rewritten argument or result |
| `v2-binding` | `binding` text | the binding validator | accept, or `BindingInvalid` with a reason |

**Mandatory vectors** (a conformance PR may add more, never fewer):

* *Arithmetic* (`v2-arith`): the ten cases of section 3.4.
* *Values* (`v2-value`): a signed `I8` of `-1` and `-128`; `Bytes<N>` of the wrong length (reject); a decimal string with a leading zero, a `+`, a 21-digit value and `"-0"` (reject); `{"some":{"some":{"none":true}}}`; a record and a variant of every arity from 0 to 8; a variant with an undeclared constructor (reject); an `Err` case not in `E` (reject); nesting past the 64-container limit.
* *IR* (`v2-ir`): short-circuit with an untaken subject call (`calls` excludes it); a subject call under an impure guard (the generator must refuse; the oracle case is the corpus the lane must reject); nested subject calls; an unknown tag; a tag of an undeclared level.
* *Domains and chunks* (`v2-domain`, `v2-chunk`): every leaf descriptor at its first, second, last and `size-1` index; every composite (tuple, variant, option, result, list at the length-2/length-3 boundary); `fx_edge` for `U32`, `I32`, `U64`, `I64`; a chunk with zero admitted cases; the first, middle and last chunk of a 2^24 suite; a chunk whose boundary falls inside a refinement-discarded run; chunk `c` enumerated alone equals chunk `c` enumerated after `0..c-1`.
* *Negative cases*: a subject that returns a wrong value on a `reference` suite fails (the v1 separation case); a reordered enumeration fails `stream-mismatch`; a chunk list with `first_admitted` off by one fails; a `slim_chunks` list that omits the first or last chunk is rejected; a lane that claims `full` after a slim run is rejected by the manifest checker; a binding in which two cases of one op share a discriminator is `BindingInvalid`; `err_any` on a two-case error type is `BindingInvalid`.
* *Aliasing* (`v2-binding` plus a runner fixture): an adapter that passes the same buffer for two arguments; a mutating non-`inout` subject (`arg-mutated`); a `fresh` result sharing storage; an `inout` post-state read from a copy (`stale-post`).

* *Bounds and failure split* (`v2-ir`): a `Nat` product that leaves the 128-bit range in a subject-independent node (`ir-overflow`, corpus error) and the same below a subject call (`ir-overflow`, case failure); the doubling-`concat` model chain of section 4.2 (rejected before allocation); a v0 document with a fixed-width arithmetic node over a subject result (generation error).
* *Accounting* (manifest checker): `calls + unreachable_calls != expected_calls`; a non-zero `unreachable_by_type` for a type the lane's row holds in full; a control marked `not-applicable` whose form is listed in `applies_to_forms`; a `slim_chunks` list that omits the first or last chunk or a chunk of the recomputed base set; a witness chunk missing from `slim_chunks`.
* *Paths*: a claim field containing `..`, a separator beyond the one allowed, or two claims equal ignoring case (generation error); a symbolic link or an undeclared file in `chunks/` (corpus error).

**Gate [D7].** A lane runner runs the whole fixture family (the VER04i section 9.1 categories and the v2 categories of every level it declares) in its own test suite, and the lane's BUILD runs that before the evidence run; a runner that fails any conformance case does not write or check a manifest. The manifest records the fixture's `index.json` digest as `conformance_sha256`, and the manifest checker compares it with the committed fixture, so a lane cannot pass against an older or edited kit.

A new IR tag, built-in or stream row is not merged until it has vectors here and the oracle verifies them. That is what keeps eighteen evaluators aligned.

## 12. VER29 fit

VER29 contract ids carry no lane segment and resolve against the same lane's manifest. Three consequences, all intended:

1. **v2 lane rows keep the Rust claim ids exactly** (`warrant/<module>/exhaustive/<claim>`), so the VER29 section 5 join needs no mapping table.
2. **Strength propagation reads the labels of section 9.3.** VER29 section 6.2 decides `exhaustive` versus sampled from the binder types; v2 supplies the row-level `domain_label`, `depth` and `oracle`, and the rule is: a `slim` or `relational` row can never round up to `exhaustive`. VER29's free-text `domain` parsing (section 6.2, "falls down to `exhaustive-sampled`") is replaced, for v2 lanes, by a structured `binders` array in the row (follow-up F4).
3. **`corpus.json` copies the spec's assumed contract ids into `assumes`**, so a lane report can list its open links without a Warrant parser.

The VER29 section 5.1 dependency reader is Rust-only. Non-Rust lanes report **`lane-not-supported`** for an upper claim's link until a separate slice builds a reader; they never infer a discharge. The manifest path is section 1.3. The first VER29 lane link is the `arithmetic` adopter (slice S11).

## 13. Migration slices S1 to S13

**Rule.** Exactly three PRs touch shared crates: **S2** (before the work), **S9** (`ca-assurance-report`) and **S13** (after). Everything else lives in a new leaf crate or in lane packages that nothing depends on, so the roughly 560-package rebuild is not repeated per adopter. The three shared PRs are serialised with all other shared-crate work. The v2 generator is **new code** in a new leaf crate `rust/warrant-corpus-gen`. The v1 compiler (`corpus.rs`, `gen_corpus`) is not moved or edited: it stays in the shared `warrant-grammar` crate, byte-frozen, until S13 deletes it, so S3 touches no shared crate.

| # | Kind | Slice | Shared crate | Acceptance |
|---|---|---|---|---|
| S1 | spec | This document, plus the VER04i pointer and erratum | no | review; spec self-consistent with VER04i/h/j/k and VER29 |
| S2 | generator | `warrant-grammar`: publish the resolved-domain enumerator and the post-slot resolver as public API. No behaviour change. Lands after PR #19369 (WX-G) | **yes** | `gen_evidence --check` shows no drift for every adopter |
| S3 | generator | New leaf crate `rust/warrant-corpus-gen`: v2 emitter at `ir.core`, predicate claims, call counts, chunks, fault IR, `corpus.profile`. It can emit ct-compare v2; the committed fixture directory lands in S3d (below) [D10] | no | section 1.2: **the 18 v1 digests** (`inputs_sha256` and `expected_sha256` of all nine ct-compare suites) reproduced and pinned in the generator's own tests |
| S4 | oracle | `code/packages/python/warrant-oracle/` at `ir.core`, with chunk verification and memoisation. **Part 1** (done): reader, lowering, enumeration, `verify-v1`, `emit-v2`. **Part 2**: `verify-v2`, check 3, witness chunks, `corpus.profile` | no | part 2: the three checks of section 10.3 pass on ct-compare v2 and on a seeded-wrong corpus fail |
| S5 | fixture | the v2 categories of section 11 at `ir.core`. Prerequisite: `warrant-corpus-conformance-v1` (branch `claude/warrant-lane-conformance`) is on `main`. Independent of the ct-compare v2 fixture directory | no | oracle verifies; Python and TypeScript runners fail a deliberately wrong evaluator |
| S6 | lane | Python runner reads v2 as well as v1; `python/ct-compare` moves to v2 | no | manifest v2 `--check` green at slim and at full |
| S7 | lane | TypeScript runner, the same. **Go**: the in-flight Go lane finishes on v1 for ct-compare [D5]; it ports to v2 in its own PR in this slice or after, but before S13 | no | same |
| S8 | adopter | **gf256**: `gf256-warrant-v2`, then one PR each for `python/gf256`, `typescript/gf256` (adds the dedicated error class), `go/gf256` (panic mapping) | no | all lanes green at slim; nightly full on compiled lanes; the section 3.3 guard passed or the document migrated to v1 |
| S9 | report | Generic v2 lane validator in `ca-assurance-report`, joined to WD-004b1 | **yes** | falsification test per check, VER04i section 9 pattern |
| S10 | level | `ir.fixed` and tuples, **four PRs**: generator, oracle, fixture, both runners. Then **wasm-leb128**, **lzw**, **x25519** | no | fixture vectors for every built-in |
| S11 | level | `ir.data`, the same four PRs. Then **logic-gates**, then **arithmetic**, the first VER29 lane link | no | |
| S12 | feature | `inout` and `ensures`, the same four PRs. Then **zeroize**, whose `all(result, is_zero)` row needs the reserved `ir.seq` level: either the `ir.seq` amendment lands first or zeroize's `/post` suite is `not-live` and says so | no | `stale-post` and the other applicable WX-G faults caught |
| S13 | cleanup | Delete v1 `corpus.rs`, `gen_corpus`, the v1 oracle, the v1 fixture and the VER04i v1 text that v2 supersedes | **yes** | no consumer in any `consumers.json` still names the v1 profile |

**Slice S3 in detail (decided 2026-11-17, section 16.3).**

*Fixture timing [D10].* Section 10.1 is unchanged: a v2 corpus cannot merge until the oracle's three checks pass on it, and `verify-v2` is S4 part 2, which itself needs generator output. So S3 lands the **generator** with the 18 digests pinned in its own tests, and the committed `code/specs/fixtures/ct-compare-warrant-v2/` directory lands **with or after** the oracle's `verify-v2` (S4 part 2). Until then the evidence that the generator agrees with the oracle is the diff of the generator's chunk files against `emit-v2 --depth full` output, run by hand.

*Fail-closed rule.* The generator computes a digest for every suite it emits, so it cannot emit a suite it cannot evaluate: a document with any op, model, claim or example beyond `ir.core` is refused as a whole (`GenError` naming the level of section 4.1, or one of the feature names `inout`, `ensures`, `vectors`, `assume`, exit 2, nothing written), and a `match` over the grammar always ends in an explicit refusal, never a skip. **Fault rows are different:** a fault whose level is above the generator's is emitted as data with its `requires`, its control row carries `deferred` witnesses, and nothing is refused, because only a lane control, not a digest, depends on it (section 8.2).

*No shared-crate change.* `coding_adventures_warrant_grammar::codegen::Evaluator` is public, so S3 needs nothing from a shared crate. S3 evaluates **its own lowered `ir.core`** with its own evaluator, because the digests must depend on the IR that is written to `corpus.json` (a lowering bug has to reach `expected_sha256`), because the grammar evaluator evaluates `U8` as `Nat` in a v0 document (contradicting [D4]) and has no `wraps` or call-count accounting. The grammar `Evaluator` is used only as a **test-time differential**: on ct-compare and on wrap-free synthetic documents the two must agree on every case.

*PR breakdown.* Each PR is based on `main` after its predecessor merges (no stacked chain):

| PR | Content |
|---|---|
| S3-0 | this erratum (section 16.3), VER-BACKLOG row; no code |
| S3a | crate skeleton, workspace membership, canonical JSON writer, `corpus.profile` reader, lowering to `ir.core` with the level refusals, worst-path cost |
| S3b | `ir.core` evaluator, descriptors and enumeration, chunk streams, suite digests, the **18 digests**, the slim base set, and the **census** of section 16.2 (which adopters have a list of byte strings, `nat_count`, a signed `fx_edge` or `FxEdge(16)` binder) |
| S3c | fault IR and controls with blind claims, `corpus.json`, chunk files and `cases.tfv` emission, the `--check` mode |
| S3d | the committed ct-compare v2 fixture directory, with or after S4 part 2 [D10], plus the build-tool row below |

*Build-tool input row [D11h].* `code/specs/ct-compare.warrant`, the v2 fixture directory and `rust/ct-compare/evidence.bindings` lie outside the generator's crate, and the build tool maps a changed file to a package by path prefix, so a PR that edited only one of them would not run the generator's `--check`. A row in `code/programs/go/build-tool/repository_inputs.go` selects `rust/warrant-corpus-gen` for those paths. It is implemented in S3d, as its own reviewable change to the build tool; this document records the requirement. (The v1 `gen_corpus --check` has the same blind spot today.)

**First adopter after ct-compare is gf256.** It is a v0 document, so it needs no v1 built-ins or composite types, and it forces the four structural changes: predicate claims, several calls per case, chunking at 2^24, and portable faults. It exists in 17 of the 18 lanes. **zeroize is last**: besides WX-G it has ports only in c, cpp, csharp and fsharp, so its first lane needs a C or C# runner or a new port.

**Order and size.** Slices land in numeric order, except that S9 depends only on S5 and may land earlier than its number. Until S9 lands, a lane's own `check` is the only validator of a v2 manifest and no v2 lane row may be cited as report evidence. S7, S8, S10, S11 and S12 are each several PRs, listed in their rows; no single PR carries more than one of {generator, oracle, fixture, one lane runner, one adopter}. One exception to numeric order: S3d (the ct-compare v2 fixture directory) lands **after S3a to S3c and with or after S4 part 2** [D10]; "with" means merged together with S4 part 2's PR (adjacent merges, or a merge queue), **never in the same PR as the generator** and never before `verify-v2` exists.

**Go.** Per [D5] the Go lane finishes on v1 so it is not blocked, and **v1 is deleted in S13 only after every lane has ported**: S13's precondition is a mechanical one (no v1 consumer left), not a date.

## 14. Security considerations

A corpus, a chunk file, a binding and a `.warrant` spec are all inputs a PR author controls. Every consumer treats them as hostile.

**Resource bounds (all checked before the first case, in checked arithmetic, and a violation is a corpus error, exit 2).**

| Bound | Value |
|---|---|
| suite product ordinals (`MAX_SUITE_PRODUCT`) | `2^28` (the shipped v1 cap of 1,000,000 is lifted because chunks bound the *work*, not the product) |
| chunks per suite (`MAX_CHUNKS`); `chunk_size` | 4,096; a power of two in `2^10..=2^16` |
| chunks and cases run per suite at `slim` | 32 chunks (`MAX_SLIM_CHUNKS`); 2,097,152 cases |
| total cases per `slim` run (`MAX_TOTAL_WORK_SLIM`) | 32,000,000, including negative-control re-runs |
| total product ordinals per `full` run (`MAX_TOTAL_WORK_FULL`) | `2^32` (counted on product ordinals, which is stricter than admitted cases), checked for every suite together with the IR work before any suite is planned; the nightly job also has a wall-clock limit |
| binders per claim; tuple arity; record fields; variant constructors | 8; 8; 32; 64 |
| descriptor tree depth; type depth; IR depth | 16; 16; 64 |
| model-call nesting (active `model_call` frames) | 64 (VER04 section 2); a deeper chain is a corpus error |
| `MAX_IR_COST` per expression | 2,000 node evaluations by default on the **worst path** (one branch of an `if`, models at their body's worst-path cost); a corpus may declare `max_ir_cost` up to `MAX_IR_COST_CEILING = 65,536`; static, so it is the fuel (section 4.2) |
| total IR work per run (`MAX_IR_WORK_SLIM`, `MAX_IR_WORK_FULL`) | 64,000,000,000 and 8,589,934,592,000 node evaluations: the sum over suites of `ir_cost` times the product ordinals run (section 4.2) |
| `MAX_IR_VALUE`, any `Bytes` or list an evaluator produces | 65,536 bytes or elements, checked before allocation |
| `Bytes<N>`; a `Bytes` value in the corpus; list length in a value | `N <= 4096`; 4,096 bytes; 4,096 elements |
| decimal integer text | at most 20 digits (`Int` intermediates are bounded by `2^127` at generation, section 3.2) |
| `corpus.json`, `chunks/*.txt`, binding, spec file | size-capped before parse (corpus 16 MiB, chunk file 2 MiB, binding 64 KiB, spec per VER04a) |
| `DOMAIN_BYTES_BUDGET` | 64 MiB (VER04h), checked before any domain is materialised; chunk-wise enumeration never materialises a whole domain |

**Hostile corpus input.**

* **Parsing.** The strict reader of VER04i section 9.1: duplicate keys rejected after unescaping, safe-integer only, unpaired surrogates rejected, 256 nesting levels, prototype-free objects. Chunk files are parsed by a line grammar with fixed field counts, never by a regular expression with backtracking.
* **Verify before run.** The whole corpus (every IR node, every descriptor, every control) is compiled and validated before any case executes. A `model_call` cycle or forward reference is impossible by rule 4.2.1 and is rejected, not trusted. Unknown tags and unknown levels fail; they are never skipped.
* **Internal consistency, not just syntax.** Chunk `cases` sum to the suite `cases`; `first_admitted` is the running prefix sum; the chunk count equals `ceil(P/chunk_size)`; `slim_chunks` contains the first and last chunk and only valid indices in ascending order; a suite with zero admitted cases, a chunk past the product, or a duplicated suite or control id is rejected. A corpus that "passes" by declaring a tiny `slim` set is caught because the oracle recomputes `slim_chunks` independently (section 5.6).
* **Paths.** A consumer opens exactly the fixed file names of section 1.1 inside the fixture directory, and chunk files by the name it derives from a validated claim field (section 5.5). No path, file name or directory is ever read from `corpus.json`, a chunk file, `corpus.profile` or a binding, except the binding's `source`, which stays under the VER04i containment rule. Every opened file must be a regular file whose resolved path is inside the fixture directory (symbolic links are refused, not followed); the `chunks/` directory may contain only the derived names, and an extra file there is a corpus error. The manifest is written only to the lane's fixed path of section 1.3, through a temporary file in the same directory and a rename.
* **Digest binding.** `spec_sha256`, `corpus_sha256`, `chunks_sha256` and `corpus.profile`'s digest are re-derived from the bytes on disk; a stale or mismatched corpus is exit 2 (`StaleEvidence`).
* **Binding is data.** No expression, no code, no eval; exports are resolved by name at start-up under the `source` containment rule (VER04i sections 5.1 and 5.2, `source-mismatch`). Error classes exported by the bound module (section 7.5) are resolved at start-up and compared by identity; a class that is not found is `BindingInvalid`.
* **No reproducing material in public output.** A failure prints the suite id, the case index and the failure id; never input bytes (VER00 section 7.3, VER04i section 6.1).
* **Sabotage child processes** are launched with an argument vector, never through a shell string; their environment is the fixed `WARRANT_EVIDENCE_*` set; they have a wall-clock timeout and an output cap. (`cmd` on Windows is a quoting hazard; the portable `sabotage` command of VER04i section 6.2 already avoids it.)
* **The oracle's reader** parses an untrusted `.warrant` file: size cap, nesting cap, iterative or depth-bounded parsing, no `eval`, no `exec`, no `re` with unbounded backtracking on attacker text, no filesystem access beyond the spec and corpus paths.
* **Runtime capability sandbox.** The VER08b run of VER04i section 12 is unchanged: the same corpus, binding and wrapper under the sandbox, identical digests required, no manifest written.

## 15. Open questions

### 15.1 Decided by the owner (2026-11-16)

1. **Where does the `full` result live, and which manifest does the report read? Resolved 2026-11-16 (owner): option (b).** The committed `evidence/test-manifest.json` is the `slim` manifest, because PR CI must be able to reproduce it; the nightly `full` result exists only as a CI artifact. **Only `slim` is committed, and there are no bot commits**: no second file `evidence/test-manifest.full.json`, and no bot with write access to `main`. `ca-assurance-report` reads the nightly `full` manifest as a **digest-matched artifact** (`--full-manifest <artifact>`) and prints both depths. It prints `full` only when the artifact's `corpus_sha256`, `source_sha256` and binding digest equal the committed slim manifest's, and prints `slim(n of m chunks)` otherwise. Why not (a): a committed full manifest is stale the moment a source file changes and no PR job can re-derive it, the evidence-written-by-something-other-than-the-check pattern this family of specs exists to prevent. Every report derived from the committed file alone says `slim` for each suite with more than 16 chunks.

### 15.2 Resolved in review (2026-11-16), recorded so they are not reopened by accident

2. **v0 `U8` guard versus changing the v0 harness.** The guard (section 3.3). WD-013 already accepts the v0 residual and retires it with the gf256 conversion; the review added the static rule for subject-dependent nodes, without which the guard was incomplete.
3. **Is the slim selection a soundness hole?** No: section 5.6 states what slim thins (leading binders, not inner ones) and why the label, the controls and the `full` depth close the gap. It is a coverage reduction that is always labelled.
4. **Control observability in slim.** Proven by witness chunks for `reference` suites; `unproven` for predicate suites, where the lane's control fails closed (section 5.6). S8 reports how many gf256 controls are `unproven`.
5. **`ir.seq`.** A reserved level with no content here; users of it are `not-live` until an amendment defines the meter (section 4.1). Levels below it need no run-time meter (section 4.2 rule 6).
6. **Record-shaped `inout`.** Reserved level `ir.inout.records`; `not-live` until a slice after S12 defines the host forms (sections 4.1, 7.6).
7. **A `number` fast path in TypeScript.** Not allowed. The evaluator is exact (`bigint`) with no second code path; `number` exists only at the boundary for types of at most 32 bits (section 7.2). A lane that wants speed gets it from `slim`, not from a second evaluator that the fixture would have to cover twice.
8. **`ir.core` tag reconciliation.** Done in review against `warrant-grammar/src/corpus.rs` at `058f9498346`: the generator emits exactly the 21 tags of Appendix A (`lit var subject_op model_call len ok err eq ne lt gt le ge add sub mul and or implies not if`) and refuses numeric negation. S3 repeats the check mechanically as a test.
9. **Chunk size.** 65,536, lowerable per corpus to a power of two down to `2^10` and never raisable, with at most 4,096 chunks per suite (section 5.4). The upper bound is what bounds a slim run; a finer default waits for the S8 measurement and would be a new `chunk_size` in `corpus.json`, which every consumer already reads.
10. **VER29 section 5.2 path text.** Amended in this slice (F3, done).

## 16. Divergences, errata and follow-ups

**Erratum to VER04i section 2.7** (carried in VER04i by this slice). The sentence "the model IR's `+ - *` on `U64` wrap at 2^64 and on `Nat` are checked" is extended: fixed-width types of **every** width wrap (section 3.2), the evaluators of the shipped runners and the oracle must follow the same rule in the v2 profile, and a **v0 document** whose `U8` arithmetic wraps, or could wrap on a subject result, is refused at v2 generation because its Rust harness evaluates `U8` as `Nat` (section 3.3, WD-013). WD-013 itself is not reopened: it stays "done for v1 documents, v0 residual accepted". The v1 *profile* keeps its runners until slice S13; their refusal of overflow is harmless there because the only v1 corpus, ct-compare, has no model arithmetic at all. It would be wrong for any other adopter, which is why no second adopter may be added to the v1 profile.

**Differences from the design memo this spec encodes.**

* `concat`, `take`, `drop` and `is_err` are in `ir.core`, not `ir.fixed`/`ir.data`, because ct-compare's v1 faults need them at the first v2 slice (section 4.1).
* The v1 whole-suite digests are kept next to the chunk digests so the section 1.2 gate and a full-depth lane run stay checkable (the memo stated the gate but not how chunks coexist with it).
* The slim subset is written into the corpus as an explicit list and committed with its own digest (section 5.6), with the formula normative for the compiler and the oracle only.
* The manifest has a run-level `depth` and a per-suite `depth` (a suite with at most 16 chunks is `full` in every run).

**Changes made by the adversarial review of 2026-11-16.** Subject-dependent nodes: `ir-overflow` there is a case failure, and the v0 guard refuses them statically (sections 3.2, 3.3). `MAX_IR_VALUE` closes a memory bomb the node-count bound did not (section 4.2). Chunk count capped at 4,096 and chunk size at `2^16`, because the earlier limits allowed an 11 MiB chunk file against a 2 MiB cap and a 16-million-case "slim" suite (sections 5.4, 14). Chunk file names derived, never read (sections 5.5, 14). Witness chunks and the only-more-chunks rule for the stride (section 5.6). `unreachable_calls` and the recomputed `unreachable_by_type`, because skipped calls contradicted the call-count check and were a way to call nothing (section 7.2). `monitored_calls` restricted to subject-independent calls (section 7.6). Host-form applicability of WX-G controls (section 8.2). `bytes_n(N>=2)` is `sample`, resolving a contradiction between two lines of section 9.3. Unoptimised lowering and full oracle verification before merge (section 10). The v1 compiler is not moved in S3 (section 13). All ten open questions resolved (section 15; question 1 by the owner on 2026-11-16).

**Changes made after the review, 2026-11-16.** Open question 1 resolved as option (b) (section 15.1). Stream tags for tuple, record and variant moved from `60`/`62` to `64`/`65` to remove the collision with VER04l and VER04p (section 2.4; registry in VER04i section 2.6.1).

**Follow-ups outside this slice.**

* **F1** backlog rows for slices S2 to S13 in `VER-BACKLOG.d/portable-warrant.md` and a VER30 Track D line item (another branch edits both; this document does not).
* **F2** VER04h section 6's fault table is the ct-compare slice of the section 8.2 registry (as VER04i section 15 already says for v1).
* **F3** amend VER29 section 5.2 to the section 1.3 path. **Done in this slice.**
* **F4** structured `binders` array in the manifest row for VER29 section 6.2 (VER29's own follow-up, slice 2 of its section 13).
* **F5** when S13 lands, remove the v1 text of VER04i that this document supersedes and leave a tombstone pointing here.

### 16.1 Amendments of 2026-11-16 from slice S4 (oracle path, IR cost)

Building the oracle (slice S4) found two places where this document could not be implemented as written. Both are decided here.

**Oracle path (section 10.2, section 13 row S4).** `code/scripts/warrant_oracle/` becomes `code/packages/python/warrant-oracle/`. Reason: build discovery (section 10.2). Nothing else about the oracle changes; in particular section 10.1 is untouched, and the package shares no code, data file or generated table with the generator.

**IR cost (section 4.2 rule 6, sections 8.4, 10.3 and 14).** `MAX_IR_COST = 2,000` with both branches of an `if` charged could not hold the second adopter: it refuses 7 of gf256's 38 suites, and no constant would have fixed it, because the both-branches count of `ginv` is 92,014,191 while no evaluation of `ginv` visits more than 26,204 nodes. The decision has three parts, and each is needed:

1. *Worst-path costing* replaces both-branches costing. It is the tight sound bound on what an evaluator does; the old count measured the inlined tree.
2. *A declared budget under a hard ceiling* (`max_ir_cost` in `corpus.profile`, at most 65,536) replaces the single constant. Worst-path costing alone is not enough: `gmul` costs 2,011 and lzw's `units1024` costs 4,093. Raising the constant for everyone would hide the cost of a heavy document; a declared line is reviewed, recorded in `corpus.json` and printed.
3. *A total IR-work bound* (`MAX_IR_WORK_SLIM`, `MAX_IR_WORK_FULL`) keeps the guarantee the old pair of limits gave only implicitly. Without it a 65,536 ceiling would multiply the worst case of a slim run by about 32.

Rejected: a run-time step meter (eighteen evaluators would each need one and the count would become part of the conformance surface); costing the path actually taken per case (not static, so it is a meter by another name); rewriting gf256's models to fit 2,000 (the bit-sliced XOR is the honest v0 formulation, and a rewrite would change the frozen gf256 v0 harness for the sake of a counting rule). The S4 oracle's diagnostic flags `--max-ir-cost` and `--cost-model taken-path` are superseded: the model it calls `taken-path` is the worst-path model above and becomes the only one, the bound comes from `corpus.profile`, and the flags are removed (section 4.2 forbids an override).

**gf256 and the section 3.3 guard (WD-013).** Measured with the S4 oracle's evaluator, memoisation off, at full depth over every suite that evaluates a model or arithmetic: no fixed-width node wraps on any admitted case, and no arithmetic node has a `subject_op` beneath it. The three arithmetic nodes that sit in subject arguments (`e + 1u64`, `e + 255u64`, `e + e`) are refined to at most `2^32` and cannot wrap a `U64`. gf256 therefore passes rules 2 and 3 as a v0 document: exact (`Nat`) evaluation in the frozen Rust harness and wrapping evaluation in v2 agree on every case, the conversion changes no expected value, and WD-013 is not reopened. Two consequences are recorded so the result is not over-read. First, gf256 gives **no evidence that a lane wraps correctly**: the single rule of section 3.2 is exercised only by the conformance cases of section 3.4, which are therefore a hard prerequisite of every lane and cannot be traded against gf256 passing. Second, the measurement had to be made with memoisation off and by reading the subject arguments, because the S4 oracle's `wraps` counter skipped memo hits and never evaluated subject arguments; section 3.3 rule 1 now defines the count so that it cannot depend on either.

### 16.2 Divergences found by slice S2 (generator versus section 5.1), for S3/S10 to settle

Publishing the generator's resolved domains as `coding_adventures_warrant_grammar::domain` (S2) found five places where what the evidence generator enumerates is not what section 5.1 says. S2 changed none of them (the checked-in adopter evidence pins the generator's behaviour); each needs a decision before the v2 corpus relies on it, and the decision must be recorded here.

1. **Byte-string order in the Rust harness prelude.** The prelude's `alpha_strings` lists the length-2 and length-3 strings over `[00,01,80,ff]` first-byte-fastest. VER04i section 2.6 (and so section 5.1) and the v1 corpus compiler are lexicographic, first byte slowest. The S2 API follows the spec. For a bare `Bytes`/`Bytes<N>` binder the two enumerate the same *set*.
2. **That difference is a set difference for `List<T>`.** A list ranges over the first 16 (lengths 1, 2) and first 4 (length 3) values of `T`, so a different order of `T` is a different prefix: for `Bytes<2>` the first four values are `0000 0001 0080 00ff` (spec, S2 API) against `0000 0100 8000 ff00` (prelude). `List<Bytes<N>>`, `Option<List<..>>` and the like are different case sets in the corpus and in the Rust harness. Settle which is normative; if the corpus, the harness must be regenerated for every adopter that has such a binder (none of the current adopters does, to be confirmed by S3).
3. **`nat_count`.** Section 5.1 says `0..=K`. The generator's shift-count domain (VER04j 3.6) is `0..=64` plus `65 127 128 129 255 256 65535 65536`: 73 values, not `K+1`.
4. **Signed `fx_edge`.** Section 5.1 says the unsigned bit patterns reinterpreted in two's complement, in the same order. The generator sorts them ascending, adds `-1..=-255` and removes duplicates (VER04j 2.6: `i16` 527 values, `i32` 559, `i64` 623). The unsigned counts agree (377, 313).
5. **`fx_edge` at 16 bits and below (open question, needs an owner decision; found in S2).** Section 5.1 defines `fx_edge` for 32 and 64 bits only. The generator's budget demotion (VER04j 2.6) also produces edge sets for 16-bit binders (`FxEdge(U16|I16)`, 281 and 527 values), which the corpus must be able to describe, and the unsigned edge formula repeats values at 8 bits (`0..=255` already contains `MAX` and the `MAX ^ (1<<i)` patterns), so an 8-bit edge set has duplicates; the generator never produces one (an 8-bit binder is always `fx_full`). Decide whether `fx_edge` extends to 16 bits (and whether 8 bits is excluded or de-duplicated).

Also for S3: the descriptor depth limit of section 5.1 (16 levels) is the emitter's to enforce; the resolver allows the generator's 64 levels and 4,096 nodes.

**Settled for S3 (2026-11-17, section 16.3 item 11e):** ct-compare hits none of the five (its binders are `Bytes`, `Bytes<N>`, `U64` and `Bool`). S3 refuses, naming section 16.2, a descriptor of kind `nat_count`, a signed `fx_edge`, `FxEdge(16)` or a list whose element domain is a byte string; S10 decides items 1 to 5 with the census S3b reports in hand.

### 16.3 Owner decisions of 2026-11-17 (CV2-S3)

Planning slice S3 (the generator) found one place where this document could not be implemented as written and several where it was silent or contradicted what the independent oracle already does. The owner decided each. The normative sections (1.1, 1.4, 2.1, 2.5, 4.2, 5.6, 8.1 to 8.3, 9.2, 10.3, 13, 16.2) were amended in the same change, so they read correctly without this list.

**[D8] Blind pairs (section 5.6).** `required_claims` lists the claims that can witness the fault, in the sense of section 5.6: the claims not proven blind. A claim on which no enumerated case can witness the fault goes in a new control field `blind_claims`. Generation fails only when every candidate claim is proven blind. The motivating case is ct-compare's three blind pairs, each of which v1 lists in `required_claims`: `eq_fixed/ignore-last-byte` on `eq_fixed_agrees_w0` (`Bytes<0>` has no last byte), `select_bytes/missing-error` on `select_picks` (equal lengths never give `Err`) and `select_bytes/swap-choice` on `select_rejects_mismatch` (a length mismatch gives `Err` whichever way `choice` flips). Read literally, the earlier text made v2 generation of ct-compare fail.

**[D8 refinement, accepted by the owner]** There are three claim states, not two: *witnessed*, *blind* and *unproven* (no stand-in). Unproven claims stay in `required_claims`. A control with no witnessed claim passes generation, and is caught or not at lane time (`uncaught-fault`); the owner confirmed that this includes a control whose candidates are blind plus unproven, which now passes generation. (The author's refinement of the owner's wording "fails only when a control has no witnessed claim", which read literally would have failed every control whose claims are all predicate suites.)

**[D9] `ignore-high-bit` (section 8.2).** It stays `ir.fixed` (`bit_and`). At `ir.core` the ct-compare `eq_u64` control is reported `not-live`, honestly labelled, until S10. The owner declined the `ir.core` reformulation. **Consequence, confirmed by the owner:** a not-live control counts in `not_live` and blocks `release-ready` (section 4.1), so no ct-compare lane is release-ready until it implements `ir.fixed`.

**[D10] Fixture timing (sections 10.1, 13).** Section 10.1 is unchanged. S3 lands the generator with the 18 digests (`inputs_sha256` and `expected_sha256` of all nine v1 suites) pinned in its own tests; the committed ct-compare v2 fixture directory lands with or after the oracle's `verify-v2` (S4 part 2). The S3, S4 and S5 rows of section 13 say so.

**[D11] Eight format details**, accepted as proposed in the S3 plan:

* **(a)** A `pre` fault that rewrites several parameters has a `rewrites` list of `{param, expr}`, applied simultaneously; `result` (not `r`) is the reserved result name, for faults and for `ensures` (section 8.1).
* **(b)** Two slim digest fields, `slim_inputs_sha256` and `slim_expected_sha256` (the oracle's names), replace the single `slim_sha256` (sections 5.6, 9.2, 10.3, 1.4).
* **(c)** An explicit `model_order` array preserves declaration order, which rule 4.2.1 needs, under sorted-key canonical JSON (sections 1.4, 4.2).
* **(d)** `Int`-typed `lit` nodes are allowed, as the oracle emits them (section 2.1, 4.2 rule 4).
* **(e)** S3 refuses the section 16.2 divergent descriptors and S10 decides, using the census from S3b (section 16.2).
* **(f)** The chunk size stays 65,536 for ct-compare; the slim stride is exercised by synthetic tests at `chunk_size` 1,024, not by shrinking the real corpus (section 5.4; ct-compare's largest suite has at most four chunks, so it never exceeds the 16 that make slim differ from full).
* **(g)** `cases.tfv` holds examples only; "sabotage probes" is removed (section 2.5).
* **(h)** A build-tool `repository_inputs.go` row makes edits to the spec, the fixture or the binding run the generator's `--check`; implemented in S3d (section 13).

**[Section 1.4, provisional, ruled by the owner.]** The closed key set of section 1.4 is provisional until S4 part 2 reads it, and grows only by erratum.

**Author refinements (not owner decisions), made while applying the rulings above.** `slim_witness` is a list with three entry shapes (`chunk`, `unproven`, `deferred`, section 5.6); control rows carry `requires` and, for WX-G faults, `applies_to_forms`; the manifest control state `not-live` names its level and `not_live` counts suites plus controls (sections 4.1, 9.1); fault rows of a deferred level are data, for the generator and the oracle (sections 8.2, 10.4, 13); S3d merges with or after S4 part 2, never in the generator's PR (section 13); the manifest checker rejects a not-live control row with a non-empty `caught_by`, or one whose `requires` levels the lane declares (section 8.3).

**Findings recorded, not decided.** S3 needs no shared-crate change (section 13). The oracle's `emit-v2` output is its own `warrant-oracle-enumeration-v0` JSON, not a v2 `corpus.json`, so the key set of section 1.4 is defined by this document and not copied from the oracle; what the oracle's `emit-v2` does bind is the chunk-line format, the digests (suite, v1 and slim), `cases`, `discarded`, `product`, `expected_calls`, `demoted`, `domain_label`, `result_type`, `ir_cost`, `wraps`, the descriptor JSON (`{"d":..,"size":..}`) and its lowering rules. Its `slim_chunks` is the **base set only**: witness chunks wait for S4 part 2, so the two will differ on any suite with `m > 16` and a witnessed control until then.

**Follow-ups outside this slice.** The oracle README (`warrant-oracle/README.md`) still says "`slim_sha256` is emitted as separate inputs and expected digests"; it is accurate for the oracle and needs no change, but S4 part 2 must read `blind_claims` and the `rewrites` shape when it implements check 3. VER04i has no field named `slim_sha256` or `blind_claims`; VER04i sections 4 and 15 (`accepted_blind_faults`, `required_claims`) describe the v1 profile, which keeps `required_claims` as written until S13, and need no edit. VER04n and VER04p mention `accepted_blind_faults` only for v1-style controls and are unaffected. VER29 does not mention these fields.

### 16.4 Divergences and clarifications found by S3a

Slice S3a (lowering and cost, `rust/warrant-corpus-gen` 0.1.0) agreed byte for byte with the independent oracle on ct-compare, gf256 and two synthetic documents, and found the following. Items marked **OPEN** are questions for the owner and are not decided here.

1. **Section 4.2 rule 6.** ct-compare's largest expression costs **10**, not 9: the body of `eq_agrees`, of each `eq_fixed_agrees_wN` and of `eq_u64_agrees` is `1 + 3 + (1 + 2 + 3)`; `select_picks`' body is 9. The oracle computes the same. Corrected in place.
2. **Section 4.2 rule 6.** The bound applies per expression **including each model body** (the oracle already did). Corrected in place.
3. **Section 13, fail-closed rule.** A refusal names a level of section 4.1 or one of the feature names `inout`, `ensures`, `vectors`, `assume`. Corrected in place.
4. **Interim, `assume`.** S3 refuses any non-`none` `assume platform` / `assume environment` row. **OPEN:** the shape of `assumes` (section 12 item 3) must be settled before the first adopter with such a row.
5. **Sections 6 and 1.4, op rows.** `requires` is always present (at least `["ir.core"]`); `admit`, `width_params` and `fresh` are conditional. Applied in place in section 6 (the example rows and the `requires` row of the table). **OPEN:** a claim that calls an op outside that op's `admit`: refused at generation, or the author's problem?
6. **Section 4.2 rule 4 / Appendix A `neg`.** The signed minimum literal lowers to `neg` of a `lit` holding `-2^(w-1)`; `neg` wraps it to itself (section 3.2) and counts in `wraps`. Applied in place in section 4.2 rule 4 (Appendix A already said it).
7. **Section 8.4.** Repeated `fault` or `slim_chunk` lines, an empty token, `budget 0` and `slim_stride 0` are profile errors (the Rust reader in S3b, the oracle's in S4 part 2). S3a's reader mirrors the oracle's and still accepts them.
8. **Section 14.** Model-call nesting is limited to 64 frames (VER04 section 2). Applied in place (a row of the section 14 table).
9. **Section 3.3 rule 3** also covers a `model_call` over a subject result whose callee's closure wraps. Applied in place; the oracle implements no rule 3 (S4 part 2).
10. Type aliases (`model Word = Bytes<4>`) are resolved away and never appear in a corpus; `is_err` has no source form (Appendix A). An example must be one subject call compared with a pure value (the section 1.4 `examples` row); S3a refuses any other example with `W0701`.

### 16.5 Divergences and clarifications found by S3b

Slice S3b (the evaluator, enumeration, the 18 digests and the census; `rust/warrant-corpus-gen` 0.2.0) agreed with the independent oracle on every count, digest, descriptor and chunk line of ct-compare and two synthetic documents (`tests/enumeration_parity.rs`), reproduced all eighteen v1 digests, and found the following. Items marked **OPEN** are questions for the owner and are not decided here.

1. **Section 4.3, in place.** A claim whose body calls no operation is `W0701` (S3a review gap L5). Applied in section 4.3.
2. **Section 4.5.** A suite that admits zero cases is refused with the code `W0706` (the v1 generator's code for an empty admitted domain). Recorded here, not yet applied in place.
3. **Section 3.3 rule 1, precision.** The `wraps` count is taken on the *pure trace*: a pure guard (the condition of an `if` with a subject in a branch, the left operand of a short circuit whose right operand calls a subject) is evaluated and its wraps counted and the taken branch followed; the subject-independent arguments of a `subject_op` and the `expected` side are evaluated; a node with a subject beneath it is never evaluated; an operand that runs only under a short circuit whose guard depends on a subject result is skipped; the nodes of a `model_call` body count on every call. `admit` counts once per product ordinal (the generator evaluates it once per prefix tuple of the binders it reads and multiplies). Both implementations agree on all synthetic suites (a wrapping `admit`, a wrapping subject argument, a pure guard). Recorded here, not yet applied in place.
4. **Section 2.3 and the streams.** A stream value must have its declared type: a `Nat` or unsigned value outside `0..=2^64-1` (for example the `Int` result of `Nat - Nat` when the subject side is a `Nat`) cannot be encoded and is `W0701` at generation. Recorded here.
5. **Section 3.3 and WD-013, correction of the record.** The `warrant-grammar` `Evaluator` *wraps* an `EvalValue::U8` (and `U64`) in a v0 document too; the v0 divergence of WD-013 appears where the harness hands a `U8` over as a `Nat`, when `x + 1u8` is the exact sum. S3 therefore uses that evaluator only as a differential on sampled cases and pins both behaviours in a test. The v0 guard rules 2 and 3 are unchanged.
6. **Section 5.1, descriptor names (owner decision 2026-11-17, applied in place).** `u8_all` and `u64_edge` are the canonical names of `U8` and `U64` domains in every document version; `fx_full`/`fx_edge` are used only for the other widths (an `I8` is `fx_full` with `t` `I8`). Generator and oracle already agree. (Original finding:) A v1 document resolves a `U8`/`U64` binder to `Fx(U8)`/`Fx(U64)`; the generator and the oracle name them `u8_all`/`u64_edge` whatever the document version (the value sets are identical), so `fx_full`/`fx_edge` name only the other widths. Section 5.1 now says which name each takes.
7. **Section 5.6, profile lines.** `slim_stride` is ignored for a suite of at most 16 chunks (the slim set is every chunk); for a larger suite it is **clamped** to the suite's automatic stride, `min(profile stride, automatic)`, and is never an error (owner decision 2026-11-17, applied in place in section 5.6 and in the generator; the oracle's `emit` takes no stride yet, so S4 part 2 must implement the clamp); a `slim_chunk` line must name a declared claim and a chunk below `m`. Applied in the generator; recorded here.
8. **Section 16.2, S3 refusals.** A refused 16.2 descriptor is reported with the generator code `domain` naming the item (`nat_count` 3, signed `fx_edge` 4, `FxEdge` at 16 bits 5, a list of byte strings 1/2).
9. **Section 16.2, the census (input to S10).** Over the eleven adopters that bind `gen_evidence` (355 claims; `max_ir_cost 65536` so cost blocks nobody): `ct-compare` (9 of 9) and `gf256` (38 of 38) are emittable at `ir.core`, 89 claims in all; the rest are blocked by `ir.data` (154 claims), `ir.fixed` (101), `inout` (9) or `W0701` (2: a subject call under an impure guard in `wasm-leb128`). **No binder touches items 1 to 3** (no list of byte strings, no `nat_count`); **item 4 (signed `fx_edge`, `I64`) occurs in 7 `wasm-leb128` claims and item 5 (`FxEdge` at 16 bits) in 2 `lzw` claims**, each already blocked by `ir.fixed` or `ir.data`. Items 4 and 5 therefore need deciding before S10 lifts those levels; items 1 to 3 can wait for the first adopter that has one.
10. **Section 8.4, in the generator.** The profile reader refuses a repeated `fault` or `slim_chunk` line, an empty token, `budget 0` and `slim_stride 0` (16.4 item 7); the oracle's reader still accepts them (S4 part 2).
11. **Resource bounds, checked before the first case (corrected after review).** The total IR work of a full run (`MAX_IR_WORK_FULL`) and the total product ordinals (`2^32`) are charged for **every** suite after the domains are resolved and before any suite is planned (planning builds the admission table, up to `2^28` `admit` evaluations); the ordinal bound is on product ordinals, stricter than cases; a claim of more than 8 binders, a product past `2^28` and more than 4,096 chunks are refused by the planner.
12. **Review fixes in S3b.** The generator evaluates every pure node on the trace that may wrap or fail (section 3.3 rule 1, applied in place), so a pure subject argument that leaves the exact window or builds a `Bytes` past `MAX_IR_VALUE` is refused at generation. **For S4 part 2:** the oracle's `may_wrap` (`ir_eval.py`) is true for any `add sub mul neg`, including `Nat`/`Int`, but evaluates no pure `concat` on the trace, so a `concat` past `MAX_IR_VALUE` in a subject argument still generates there; the oracle must evaluate it too. The generator's stream encoder now also rejects a fixed-width value outside its type and a `Bytes<N>` value of another length. The evaluator is iterative (an explicit work stack): the bounds of section 14 run on a default 2 MiB thread in a debug build. The work stack's bound (`MAX_EVAL_TASKS`, 65,536) is not a new limit: the pending tasks of an evaluation are at most the cost of the worst path, which is at most `max_ir_cost`, which is at most 65,536 (section 4.2 rule 6).

### 16.6 Decisions and findings of S3c (fault IR, controls, emission, `--check`)

Slice S3c (`rust/warrant-corpus-gen` 0.3.0) emits ct-compare as `corpus.json`, nine chunk files and `cases.tfv`, with the eight control rows of section 8.3. **No owner decision is recorded here.** Everything under "Author refinements" is the author's reading where this document is silent, written so the oracle's `verify-v2` (S4 part 2) can implement or contest it; each is an erratum candidate, not a ruling. Items under **OPEN** are questions for the owner and are not decided.

**Result for ct-compare.** The three blind pairs of section 5.6 come out exactly as predicted (`eq_fixed/ignore-last-byte`: `required_claims` w1 w2 w4 w16, `blind_claims` w0; `select_bytes/missing-error`: required `select_rejects_mismatch`, blind `select_picks`; `select_bytes/swap-choice`: required `select_picks`, blind `select_rejects_mismatch`), confirmed by a brute-force search written separately in the tests. `eq_u64/ignore-high-bit` is emitted as data with `requires: ["ir.fixed"]` and the single witness entry `{"claim": ..., "deferred": "ir.fixed"}`; the other seven controls are witnessed at chunk 0 of every candidate. All suites have `m <= 16`, so no witness chunk changes a slim set for ct-compare; a synthetic suite at `chunk_size 1024` (64 chunks, witness at chunk 32 outside the stride) tests the merge.

**Author refinements.**

1. **Claims are named by suite id** (`warrant/<module>/exhaustive/<claim>`, as in v1) in `required_claims`, `blind_claims` and every `slim_witness` `claim`; "claim" in sections 5.6 and 8.3 is read that way.
2. **Order.** `faults` and `controls` are sorted by `(op, fault)` by bytes, independent of the order of `fault` lines in `corpus.profile`; the three claim lists keep suite declaration order. `levels` is the duplicate-free union of every `requires` (suites, ops, faults, controls) sorted by bytes (`["ir.core","ir.fixed"]` for ct-compare).
3. **Fault row.** `{"fault","op","kind","requires"}` plus `expr` (`post`, `replace`) or `rewrites` (`pre`). `requires` is a one-element list holding the level of the expression; the control row repeats it. Rewrites are listed in parameter order.
4. **Registry.** The portable names of section 8.2 are instantiated by signature: `invert` (Bool result), `missing-error` (`Result<Bytes,E>` result and a `Bytes` parameter; `replace`), `swap-choice` (a `Bool` parameter named `choice`), `off-by-one` (`Nat` or fixed-width result), `truncate-last` (`Bytes` result), `ignore-last-byte` (one rewrite per `Bytes` parameter, each `if len(p) > 0 then concat(take(p, len(p) - 1), b"00") else p`), and at `ir.fixed` `xor-lsb` and `ignore-high-bit` (`bit_and p 2^63-1`, one rewrite per `U64` parameter). An unknown name, or a template the signature cannot take, is the generator code `fault`; nothing is skipped. An op with a parameter named `result` is `W0701` (section 8.1) whether or not it is faulted.
5. **Candidate claims and the stand-in.** A candidate is a claim whose body calls the fault's op. It has a stand-in iff it is a `reference` suite whose subject side is exactly `op(b1..bk)` with every argument a plain binder. A binder passed at two positions must be rewritten at both or at neither (a `pre` fault), otherwise there is no single rewritten binder and the claim is **unproven**. A claim with a computed argument (`g(a, id(b))`) or a predicate claim is unproven. A `pre` fault re-evaluates `expected` at the rewritten binders; `post` and `replace` evaluate the fault expression with `result` bound to `expected`; the rewrites of a `pre` fault all read the original parameters.
6. **Witness.** The lowest admitted **product ordinal** at which the faulted value differs; its chunk is `ordinal / chunk_size`. A fault expression that fails to evaluate is a generation error for that claim and fault, not a blind claim. One enumeration pass serves every fault of a suite, so the cost is one extra `admit`-filtered pass per suite with a stand-in (bounded by the budget); a blind fault scans the whole suite.
7. **Generation failures.** A faulted op that no claim calls, and a fault whose every candidate is blind, are `fault` errors (section 5.6, and v1's `W0701` "no claim target"). A fault whose candidates are blind plus unproven passes, as the owner confirmed.
8. **Not-live and deferred.** A fault whose level the generator has not implemented is data (section 8.2): its candidates with a stand-in are `deferred`, those without are `unproven`; no digest depends on it.
9. **Other keys.** `fault_models` is `{}`, `assumes` is `[]` (S3 refuses any non-`none` assume row, section 16.4 item 4), `accepted_blind_faults` is `[]` (OPEN 1). An example's `width` is omitted when the call has no static width (never `null`, section 1.4); every other example key is always present. The `models` rows have exactly `params` and `body`. A suite row's `claim` is the claim name, from which `chunks/<claim>.txt` derives (section 5.5).
10. **Profile and bindings.** A `fault <op> <name>` line must name a declared op. The CLI's optional `--bindings <evidence.bindings>` reads the `faults=` list of every `op` line and the generator compares the sets with the profile's per op (`drift`, section 8.4); the profile is the authority and an op with no `faults=` and no `fault` line simply has no controls.
11. **Files.** Chunk files are the chunk lines joined by LF with one final LF. Writing validates every path first (`corpus.json`, `cases.tfv`, `chunks/<name>.txt`, names of ASCII letters, digits, `_` and `.`, no leading dot, no `..`), refuses a symbolic link at `chunks/` or at a target, refuses a `chunks/` that holds a file the corpus does not own (it never deletes one), writes each file to a temporary name and renames it, and renames `corpus.json` last. `--check` compares `corpus.json`, `cases.tfv` and every chunk file byte for byte and the exact file set of `chunks/`, and reports each difference with file, line, column and byte offset (never content); the other files of a fixture directory (`corpus.profile`, `consumers.json`, README) are not the generator's and are not compared. `--warrant` must be a repo-relative path because it is recorded as `spec`.
12. **Codes.** `GenError.code` gains `fault`, `drift` (an on-disk corpus or the bindings differ) and `io` (a refused path or failed file operation); `unimplemented` is gone.
13. **Parity with the oracle.** The chunk files equal the oracle's `chunk_lines` for all nine ct-compare suites (checked in the generator's tests against the committed oracle snapshot). The oracle's `slim_chunks` is the base set only, so the two differ on any suite with `m > 16` and a witnessed control until S4 part 2.

**OPEN (not decided here).**

1. **`accepted_blind_faults`: where is it declared?** Sections 5.6 and 8.3 say "the spec's `accepted_blind_faults`", but a `.warrant` file has no such declaration and `corpus.profile` (section 8.4) has no such line. S3c writes `[]` and treats an all-blind fault as an error with no escape. A profile line (`accept_blind <op> <fault>`) is the obvious home; it needs a decision, and VER04p (which forbids some faults from the list) needs the same.
2. **The JSON of `ir.fixed` data nodes.** Appendix A lists the 26 `ir.core` tags only. The generator writes `["bit_and", e, {"lit"..}]` and `["bit_xor", e, {"lit"..}]` with a `U64`/fixed-typed `lit` for the two data faults; the node shapes of `ir.fixed` should be fixed by the S10 erratum, and S4 part 2 must expect these two forms until then.
3. **`required_claims` as suite ids versus claim names** (refinement 1). The manifest's `caught_by` is a subset of `required_claims`, so the choice also fixes the manifest; v1 used ids.
4. **Should `--check` also compare the authored files of a fixture directory?** Today a stale `corpus.profile` is caught only through `profile_sha256` in `corpus.json`, not by comparing the file; S3d, which commits the directory, should decide whether the BUILD line also hashes it.

## Appendix A. `ir.core` tags

| Tag | Form | Notes |
|---|---|---|
| `lit` | `["lit", true]`, `["lit", {"type":T,"value":V}]` | `V` per section 2.3; Bytes literals are hex |
| `var` | `["var", name]` | a binder or model parameter |
| `subject_op` | `["subject_op", name, [args][, [widths]]]` | claim bodies only |
| `model_call` | `["model_call", name, [args]]` | |
| `len` | `["len", e]` | `Nat` |
| `ok`, `err` | `["ok", e]`, `["err", "CASE"]` | `err` takes the case name |
| `eq ne lt gt le ge` | two operands | same-type, or Nat/Int widening |
| `add sub mul` | two operands | section 3 |
| `and or implies` | two Bool operands | short-circuit |
| `not` | one Bool operand | |
| `if` | `["if", c, t, e]` | |
| `concat take drop` | on `Bytes` | total: `take`/`drop` clamp, a negative count is `0` |
| `is_err` | one `Result` operand | Bool (no source form: only fault templates write it) |
| `neg` | one operand | wraps (section 3); the v0 generator refuses it, the v2 generator does not. The signed minimum literal (`-128i8`) lowers to `neg` of a `lit` holding `-2^(w-1)`; `neg` wraps it to itself and counts in `wraps` |

`neg`, `concat`, `take`, `drop` and `is_err` are the five v2 additions to the 21 tags v1 emits today: `lit var subject_op model_call len ok err eq ne lt gt le ge add sub mul and or not implies if`. The list was reconciled with the generator source in review (section 15.2, item 8).
