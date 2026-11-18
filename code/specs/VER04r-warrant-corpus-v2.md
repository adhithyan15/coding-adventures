# VER04r — Warrant Corpus v2 (`warrant-corpus-v2`)

**Parent:** [VER04i](VER04i-polyglot-warrant-evidence.md) (the v1 corpus, lanes, bindings and manifests; this document amends it), [VER04h](VER04h-warrant-derived-evidence.md) (domains, budgets, negative controls), [VER04j](VER04j-warrant-v1-extensions.md) §4.7 and §14.4 (composite domains, `Bytes<N>`, `err_any=`), [VER04k](VER04k-warrant-v1-wave4.md) §4 (WX-G `inout` and `ensures`), [VER04](VER04-warrant-language.md) §3 and §4 (value and arithmetic semantics), [VER29](VER29-layer-contract-linking.md) (layer contracts and strength propagation), [VER30](VER30-warrant-expansion-roadmap.md) §4.4 Track D.

**Status:** slice S1 (this specification) is followed by implemented slices S2, S3a to S3d (the generator `rust/warrant-corpus-gen` and the committed ct-compare v2 fixture) and S4 parts 1 and 2 (the oracle `python/warrant-oracle`, `verify-v2`); S3d is done on its branch (section 13). Everything from S5 on (conformance fixtures, lane runners, adopters, `ir.fixed` and later levels, the report validator, v1 deletion) is not implemented. The v1 corpus, runners and oracle keep working unchanged until slice S13. Owner decisions of 2026-11-16 are encoded in sections 1, 3, 5, 9, 10 and 13 and are marked **[D1]** (in-lane enumeration plus model IR as a new closed profile), **[D2]** (slim in PR CI, full nightly), **[D3]** (full independent oracle), **[D4]** (one wrapping rule), **[D5]** (Go finishes on v1; v1 deleted in S13) **[D6]** (honest labels preserved) and **[D7]** (every lane runner passes the conformance kit of section 11 before it may write a manifest) where they bind. Owner decisions of 2026-11-17, taken at the start of slice S3, are marked **[D8]** (blind claims), **[D9]** (`ignore-high-bit` stays `ir.fixed`), **[D10]** (when the fixture may be committed) and **[D11]** (eight format details) and are recorded in section 16.3. Owner decisions of 2026-11-17, taken at the start of slice S3d, are marked **[D12]** (claims are named by suite id), **[D13]** (the generator name and version are pinned by the build), **[D14]** (edits to `corpus.profile` are caught) and **[D15]** (`accept_blind`) and are recorded in section 16.8. The normative sections below already read as amended by them.

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

`corpus.json` is a **closed** key set: a reader rejects an unknown key, and a writer emits every key below (an empty object or list where there is nothing to say), so that two generators cannot differ by omission. The exceptions are the **conditional keys**, each marked "only" below: when the condition does not hold the key is **omitted**, never written as `null` or an empty string. **Provisional [D11]:** the owner ruled the key set provisional until the oracle's `verify-v2` (S4 part 2) reads it (it does, section 16.7: the key set above is what `verify-v2` expects, and no key was added); it grows only by erratum to this section (for example the `applies_to_forms` key of the control row, section 8.3, the top-level `omitted_examples` key of section 16.10 [D22], and S12's `monitored_calls`). It is rendered with the VER04i section 9.1 canonical writer: keys sorted by code point, two-space indent, LF, one final LF, non-ASCII escaped. Sorting loses declaration order, and one rule needs it (a model may call only models declared *before* it, section 4.2 rule 1), so the order is carried as data in `model_order` [D11c]. Counts are JSON integers and every value is a string (section 2.3).

Top level: `profile` (`"warrant-corpus-v2"`), `schema_version` (2), `module`, `document` (`"v0"` or `"v1"`), `spec` (its path), `spec_sha256`, `profile_sha256` (the digest of `corpus.profile`), `generator` (`{"name":..,"version":..}`), `budget`, `chunk_size`, `max_ir_cost`, `levels` (the union of every `requires`), `types`, `ops`, `models`, `model_order`, `fault_models`, `suites`, `examples`, `faults`, `controls`, `accepted_blind_faults`, `omitted_examples`, `assumes`.

* `models` is `{name: {"params": [[name, type]..], "body": IR}}` and `model_order` the array of the same names in declaration order; a consumer validates every `model_call` against that order and rejects a name missing from either.
* `ops` is the shape of section 6.
* `examples` is a list of `{"id", "op", "args", "expected", "result_type", "width"}` with the values untagged and decoded by declared type (section 2.3).
* `faults` rows are those of section 8.1 and `controls` rows those of section 8.3 (`applies_to_forms` is present only on a WX-G control).
* `omitted_examples` **[D22, D34]** (only) is the list of `{"id": <example id>, "reason": <reason>}` rows, sorted by `id` (bytes, no duplicates), for the examples that `omit_example` lines of `corpus.profile` removed from `examples` (section 8.4). `reason` is **computed by the tool**, never written in the profile, and is one of the closed set `value-over-limit`, `not-op-eq-pure` (section 8.4). The key is a **conditional key**: it is written **only** when at least one example is omitted, and omitted otherwise, so no committed fixture moves a byte. **A consumer rejects a present `omitted_examples` that is `[]`**, one that is unsorted or has a duplicate id, a row with a key other than `id` and `reason`, and an unknown `reason` string (author refinement, section 16.10 R4: an always-present empty list would change every committed `corpus.json` and force the atomic PR of [D29] a second time). Implemented in slice S10-1 (section 13), oracle first: until then neither tool writes or accepts the key.

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
2. For a **v0 document**, `wraps > 0` is a generation error: `W0701: U8 arithmetic wraps; v0 harness evaluates U8 as Nat`. The rule applies to every evaluation the generator performs for the document, so it covers an `example` as well as a claim **[D18]**: in a v0 document an example whose arguments or pure side wrap (`f(255u8 + 2u8) == 1u8`) fails generation with the same code, the error attributed to the example; in a v1 document the same example is accepted (rule 5). The same holds for a wrap while evaluating a callee's `admit` for the check of section 4.5, in a claim or an example; those wraps are refused in v0 but never added to `wraps` (section 16.9 refinement 6). Each implementation counts the wraps of an example's evaluation exactly as it counts a suite's, so the two agree on it.
3. For a **v0 document**, a fixed-width `+ - *` or unary `-` node with a `subject_op` beneath it is refused **statically** with the same code. Counting cannot cover it: whether it wraps depends on what the subject returns, so no generation-time count can prove the two evaluators agree. (`mul(a,b) == mul(b,a)` is unaffected: it has no IR arithmetic node.) The rule also covers a `model_call` with a `subject_op` beneath it whose callee's call closure contains such a node (`model inc(x: U8) -> U8 = x + 1u8` called as `inc(subject(..))`). The Python oracle implements rules 2 and 3 since S4 part 2 (section 16.7).
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
| `ir.fixed` | exactly the 35 names of Appendix A.1 **[D24]**: `bit_and bit_or bit_xor bit_not shl shr sar rotl rotr popcount clz ctz bit widen wrap try to_int to_nat max_value min_value checked_add checked_sub checked_mul div rem div_trunc rem_trunc imin imax be_bytes le_bytes from_be from_le at unwrap_or` (corrected 2026-11-17: the earlier list named `try_nat`, which does not exist, and omitted ten names the checker already classifies as `ir.fixed`). `Option<T>` is admitted as an **IR-intermediate type only** [D23] | x25519, lzw, wasm-leb128 |
| `ir.tuple` | **[D20, D35]** tuple types `(T1,..,Tn)`, the tuple literal, `project`, tuple values, structural `eq`/`ne` on them, the stream tag `64` for tuples, and **`match` on `Result`** (patterns `ok`, `err`, `err_any`, `tuple`, `bind`, `wild`; Appendix A.1). **`match` is `ir.tuple` exactly when the static type of its scrutinee is `Result<X,E>`**; a `match` on a scrutinee of any other type (a bare tuple, a variant, a record, an `Option`) is `ir.data` | wasm-leb128, x25519 (faults inside `Ok`), logic-gates, hash-functions |
| `ir.data` | `match` whose scrutinee is **not** a `Result` (a bare tuple, a variant, a record or an `Option`; a `match` on a `Result` is `ir.tuple`), constructors and projection of records and variants, `some`/`none`, `Option` anywhere other than the intermediate use of `ir.fixed` (section 16.10 D23), list literals, and `len`/`concat`/`take`/`drop`/`at` on lists | logic-gates, arithmetic |
| `ir.seq` | WX-D `fold map all any range ...` with a meter. **Reserved**: the level name is closed but has no content in this document; a suite, op or fault that needs it is `not-live` in every lane until an amendment to this spec defines the v2 meter (it depends on WD-D1 slice 2a) | zeroize's `all(result, is_zero)` postcondition |
| `ir.inout.records` | record-shaped `inout` host forms (section 7.6). **Reserved** in the same way | the 4004 flip-flops |

**Level lattice [author refinement, section 16.10 R1].** The levels are ordered by implementation, not by a linear order of content: `ir.core` is the base; `ir.fixed` and `ir.tuple` each depend on `ir.core` only and **not on one another** (a tuple of `Bool` or `Bytes`, and a `match` on a `Result` of them, use no fixed-width built-in, and a corpus that has only `ir.fixed` pays nothing for tuples); `ir.data` **implies `ir.tuple`** (a lane that declares `ir.data` must declare `ir.tuple`, because `match` on variants and `Option` has tuple patterns and the data constructors compose with tuples). **`ir.data` does not imply `ir.fixed`**: a document that uses a list `at` (which returns `Option<U8>`, `ir.data`) consumed by `unwrap_or` (`ir.fixed`) simply requires both levels, through the computed `requires` (section 16.10 R9), and a lane that declares `ir.data` without `ir.fixed` records that construct `not-live` like any other. `requires` lists the exact levels a construct uses and is not closed under implication; the implication is a rule on a lane's declared `levels` only. **Enforcement is the lane runners' (S6 and S7, via S10e/f) and the report validator's (S9)**; the generator and the oracle never see a lane's declared levels. The `ir.tuple` level appears in a `requires` when a tag of it is used **or** when the **static type of any sub-expression** is a tuple type (section 16.10 R9). Implemented from slice S10t-1 (the lattice itself is documentation; the `requires` computation is S10t-1/2).

Semantics of the `ir.fixed` built-ins are those of VER04j (WX-B) and VER04k; slice S10 states each one a second time as conformance vectors (section 11) because eighteen evaluators will implement them. `ir.core` adds `concat`, `take`, `drop` and `is_err` relative to the v1 tag set (an erratum against memo-level plans, section 16): ct-compare's v1 faults (`ignore-last-byte`, `missing-error`) need byte surgery and a result test, and faults are IR from the first v2 slice (section 8).

A suite that needs a level the lane does not declare is recorded **`not-live`** in the manifest with its id and counted in `not_live`. So is a **control** whose fault needs such a level (section 8.2): its manifest row has the state `not-live` and names the deferring level, and it counts in `not_live` too. `not_live` is the number of not-live suites **plus** the number of not-live controls. Nothing is skipped silently (VER04j section 8, item 4), and a lane whose `not_live` is non-zero cannot be `release-ready` for that adopter: release-ready requires both counts to be zero. **A corpus or lane with any accepted-blind control (section 8.3) is not `release-ready` either [D17]**: `release-ready` requires zero not-live suites, zero not-live controls **and** zero accepted-blind controls, because an accepted-blind control is a fault no claim can observe, and evidence that cannot catch a seeded fault must not be released as complete. An **omitted example** (section 8.4, [D34]) does **not** block `release-ready`; reports show their count beside `not_live` and the S9 validator prints their ids. (Lanes, `ca-assurance-report` and the lane validator enforce this in S6, S7 and S9; until then it is a rule of this document that no check implements.)

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

**A call outside the callee's `admit` is a generation error [D19].** A claim's `admit` and an op's `admit` (section 6) are independent. While enumerating, for **every admitted case** of a suite the generator and the oracle evaluate, for each `subject_op` on the pure trace of the body (section 4.4: a call in an untaken branch is not on it), the call's arguments against the callee op's `admit` row; if any admitted case calls an op outside its `admit`, generation fails with `W0701` naming the claim and the op, and the message carries the admitted-case ordinal of the **first** such case in product order (the `<suite id>@<i>` address of section 5.4). It is a refusal, never a silent discard: the case is not dropped from the suite, the document is not generated. The same check applies to an `example`: an example whose call is outside the op's `admit` fails generation, attributed to the example. A claim whose binder refinements are at least as strong as the op's `admit` is accepted. The check runs over the whole product at any depth (a hole at case 2^20 is a defect of the document, not of the slim set) and costs nothing for a body that calls no op with an `admit`; it is charged to the IR-work meter (section 16.9 refinement 4). A call whose op `admit` **reads a parameter whose argument contains a `subject_op`** cannot be decided at generation (the generator never learns what the inner call returns): it is refused **statically**, with `W0701`, for every such call in the body whether or not a case reaches it. An argument that depends on a subject result for a parameter the `admit` does not read is not an input of the check and is accepted (gf256's `power(add(a, b), 2u64)`: the `admit` is `exp <= 4294967295u64`); the other arguments are still checked. (The last two sentences, the static refusal and the narrowing to the parameters the `admit` reads, are author refinements recorded in section 16.9, not owner decisions. Section 7.6 has **no** dynamic `admit` refusal: a runner evaluates `ensures` rows only after calls whose arguments satisfy `admit`, so a call outside it is simply not monitored. An exemption from the static refusal would therefore mean *unchecked*, which is why section 16.9 recommends keeping it.)

## 5. Domains, enumeration order, chunks and depth

### 5.1 Resolved descriptor trees

A binder carries a **fully resolved descriptor tree**, emitted *after* the budget demotion of VER04h section 3 so that a lane never re-runs demotion (the single largest cause of silent drift between a generator and a consumer). The tree has these nodes; every node records its `size` (`S` below), including every leaf:

| Node | JSON | Order of its values |
|---|---|---|
| leaf | `{"d":"bool","size":S}`, `{"d":"u8_all","size":S}`, `{"d":"u64_edge","size":S}`, `{"d":"nat_0_64","size":S}`, `{"d":"bytes_d1","size":S}` | exactly VER04i section 2.6 |
| `bytes_n` | `{"d":"bytes_n","n":N,"size":S}` | VER04i section 2.6, for any `N` in `0..=4096` |
| `fx_full` | `{"d":"fx_full","t":T,"size":S}` for a fixed-width `T` of 8 or 16 bits other than `U8` (a `U8` binder is `u8_all` in every document version) | every value, numerically ascending (signed from `MIN` to `MAX`) |
| `fx_edge` | `{"d":"fx_edge","t":T,"size":S}` for a fixed-width `T` of 16, 32 or 64 bits other than `U64` (a `U64` binder is `u64_edge` in every document version). **A 16-bit `fx_edge` is produced only by budget demotion** (section 5.3; `U16` 281 values, `I16` 527) and **8-bit is illegal [D25]**: a consumer rejects `fx_edge` with `t` `U8` or `I8` | for unsigned width `w`: `0..=255`; then `1<<i` for `i = 8..=w-1`; then `MAX`; then `MAX ^ (1<<i)` for `i = 0..=w-1` (`u64`: 377 values, `u32`: 313, `u16`: 281). **For a signed type [D21, amended 2026-11-17]** the set is `wrap_S(E_U(w)) ∪ {-255..=-1}` (the unsigned set's bit patterns reinterpreted in two's complement, plus the small negatives), de-duplicated and in **ascending numeric order** (`i16`: 527 values, `i32`: 559, `i64`: 623). The earlier text (the same bit patterns in the unsigned order, 377 values for `i64`) is withdrawn; the generator already did this and the oracle follows it from slice S10c-1 |
| `nat_count` | `{"d":"nat_count","k":K,"size":S}` | `0..=K`, `K <= 4096` (`nat_0_64` is `K = 64`). Unchanged [D26] |
| `nat_shift` | `{"d":"nat_shift","size":73}` **[D26]** (a leaf, no parameter; `S` is always 73) | `0..=64`, then `65 127 128 129 255 256 65535 65536` (73 values, label `bounded-set`), chosen by the VER04j section 3.6 rule: a `Nat` binder that **is directly** the count argument of `shl shr sar rotl rotr` or `bit` in the claim body. The binder must be the count operand itself (`shl(x, n)`): a count that is an expression of the binder (`n + 1`), a count passed through a `model_call` argument, and a binder that reaches a count only through an `admit` (or appears as a count only in `admit`) **do not** select it; such a binder keeps the domain its type gives it (`nat_count` or the budget's demotion) [author refinement, section 16.10 R10] Implemented in S10c-1 (oracle) and S10c-2 (generator) |
| tuple / record | `{"d":"tuple","of":[...],"size":S}` | Cartesian product of the components, **last component varying fastest** |
| variant | `{"d":"variant","ctors":[[name,[desc..]]..],"size":S}` | concatenation, in declaration order, of each constructor's field product |
| option | `{"d":"option","of":D,"size":S}` | `None`, then `Some(x)` for `x` in `D` |
| result | `{"d":"result","ok":D,"errs":[names],"size":S}` | `Ok(x)` for `x` in `D`, then each error case in declaration order |
| list | `{"d":"list","of":D,"size":S}` | by length: length 0; length 1 and length 2 over `D'`, the first 16 values of `D`; length 3 over the first 4 values; within one length lexicographic by value index, last position fastest |

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

The chunk file of a suite is `chunks/<file>.txt`, where `<file>` is the suite's claim field with every `/` replaced by `.` (the synthesized `<op>/post` suite of section 7.6 becomes `<op>.post.txt`). A claim field is one or two names matching `[A-Za-z_][A-Za-z0-9_]{0,63}` joined by `/`, so `<file>` contains no path separator, no `..` and no leading dot; two suites whose `<file>` values are equal ignoring ASCII case are a generation error (a case-insensitive file system would merge them). So are a `<file>` any of whose dot-separated segments is a Windows reserved device name (`con`, `prn`, `aux`, `nul`, `com1`-`com9`, `lpt1`-`lpt9`, case-insensitive; such a file cannot be created on Windows), and a `<file>` longer than 133 bytes with its `.txt` (two 64-character names, the `.`, and the extension is the longest legal one). A consumer **derives** the path from the validated claim field; `corpus.json` carries no path, so there is nothing for a hostile corpus to point elsewhere (section 14). The file has one chunk per line, exactly `m` lines: `<c> <cases> <first_admitted> <calls> <inputs_hex> [<expected_hex>]`, single spaces, decimal counts, 64-digit lowercase hex, LF. A 2^24-case suite has 256 chunks, about 35 KB. `corpus.json` records the file's SHA-256 (`chunks_sha256`), the chunk count and the suite digests; a consumer cross-checks the file against all three, and that `sum(cases)` equals the suite's `cases`, `first_admitted` is the running prefix sum, and the discarded count equals `P - sum(cases)`.

### 5.6 Depth: `full` and `slim` [D2]

A run has a **depth**:

* **`full`**: every chunk of every suite.
* **`slim`**: for each suite, the corpus-chosen subset recorded in `slim_chunks`. The subset is chosen **by the corpus, never by the lane**, so a lane cannot pick the chunks it likes.

**Slim selection (normative, computed by the compiler and verified by the oracle).** Let `m` be the chunk count. If `m <= 16`, `slim_chunks` is every chunk (the suite is `full` even in a slim run). Otherwise let `k = ceil(m / 14)` and `o = FNV-1a-64(suite id) mod k`, where FNV-1a-64 is taken over the UTF-8 bytes of the suite id with offset basis `0xcbf29ce484222325` and prime `0x100000001b3`, in wrapping 64-bit arithmetic. The **base set** is `{0, m-1} ∪ { o + j*k | j >= 0, o + j*k < m }`: the first and last chunk, plus every `k`-th chunk from an offset derived from the suite id (at most 16 chunks; a 2^24 suite has `m = 256`, `k = 19`).

**Witness chunks.** A stride cannot know where a fault becomes visible, so the corpus adds the chunks that prove its own controls are observable. For each control row, take the *candidate claims*: every claim that mentions the control's op. For each candidate that is a `reference` suite whose subject call takes the suite's binders directly as arguments, the compiler lets the reference stand in for the subject: it applies the fault (a `pre` fault re-evaluates `expected` at the rewritten binders; a `post` or `replace` fault rewrites `expected`'s value) and searches for the **lowest admitted case** at which the faulted value differs from `expected`. (A `pre` rewrite can move a tuple outside `admit`, where `expected` is not known to model the op; the stand-in is therefore a heuristic about observability, and any mis-classification still fails closed: a wrongly *blind* claim can only make generation fail or leave a control to the other claims, a wrongly *witnessed* one is caught at lane time as `uncaught-fault`.) A candidate is then one of three things:

* **witnessed**: a case differs. That case is the claim's *witness*; its chunk is added to `slim_chunks`;
* **blind**: the stand-in exists and no admitted case differs, so the fault is unobservable by that claim at any depth [D8];
* **unproven**: there is no stand-in (a `predicate` suite, or a subject call whose arguments are not plain binders), so nothing can be said before the lane runs.

The control row lists the witnessed and unproven candidates, in suite order, as `required_claims`, and the blind candidates as `blind_claims` (claims are named by suite id, `warrant/<module>/exhaustive/<claim>`, as in v1); `required_claims` therefore lists **the claims not proven blind** (the witnessed and the unproven), and a claim proven unable to catch a fault is never credited with catching it (section 8.3). Its `slim_witness` is a list with one entry per `required_claims` entry, in the same order, of exactly three shapes: `{"claim": C, "chunk": c}` for a witnessed claim; `{"claim": C, "unproven": true}` for an unproven one (no stand-in exists); and `{"claim": C, "deferred": "ir.fixed"}` (the level is the one named) for a claim that *has* a stand-in but whose fault the generator cannot yet evaluate because it has not implemented that level (section 8.2). A deferred entry is not unproven: S10's witness search replaces it with a `chunk` entry or moves the claim to `blind_claims`. An unproven claim fails closed as before: the lane's control either fails inside the slim set or is `uncaught-fault`, which fails the lane (section 8.3), and the remedy is a `slim_chunk <claim> <index>` line in `corpus.profile`, which can only **add** a chunk. **Generation fails only when every candidate claim is proven blind**: the fault is then unobservable by the whole corpus, and it is an error unless the profile accepts it with an `accept_blind <op> <fault>` line (section 8.4, [D15]), in which case the control row is emitted without `required_claims` and `accepted_blind_faults` records its control id (section 16.8). A control with no witnessed claim but at least one unproven (or deferred) one passes generation, and is caught or not at lane time (`uncaught-fault`, section 8.3); a blind-plus-unproven control is such a control. A claim that is blind for one fault is not an error: the claim may be there for other faults or for other ops.

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
| `admit` | the parameters' refinements as Bool IR (no `subject_op`); present iff some parameter has a refinement; several refinements are `and`-chained in source order, **folded left** (`and(and(r1, r2), r3)`), exactly as a claim's `admit` (section 4.5). A claim or example that calls the op outside this `admit` fails generation (section 4.5, **[D19]**) |
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
| `off-by-one` | post | `["add", result, ["lit", {type: T, value: "1"}]]` (wraps, section 3; `T` is the result type, `Nat` included) | core |
| `truncate-last` | post | `if len(result) > 0 then take(result, len(result) - 1) else result` | core |
| `ignore-last-byte` | pre on each `Bytes` param | `concat(take(p, len(p) - 1), b"00")` guarded by `len(p) > 0` | core |
| `xor-lsb` | post | `["bit_xor", ["var","result"], ["lit", {"type": T, "value": "1"}]]` (`T` the result type) | fixed |
| `ignore-high-bit` | pre on each `U64` param | `["bit_and", ["var", p], ["lit", {"type":"U64","value":"9223372036854775807"}]]` | fixed |
| `flip-result-bit` **[D27]** | post | exact JSON in the block below the table | fixed (+ tuple inside `Ok`) |
| `drop-last-result-byte` **[D27]** | post | exact JSON in the block below the table | core (+ tuple inside `Ok`) |
| `ok-as-err`, `drop-consumed`, `swap-components` | post | composite rewrites of VER04j section 4.8 over `Result` and tuples, written with `match` on `Result` and the tuple literal | tuple [D35] |
| `wrong-ctor`, `field-zero` | post | composite rewrites of VER04j section 4.8 over variants and records | data |

**`ignore-high-bit` stays `ir.fixed` [D9].** ct-compare's `eq_u64` control uses it, and ct-compare is otherwise an `ir.core` document, so until slice S10 teaches the generator, the oracle and the lanes `ir.fixed`, that one control is recorded **`not-live`** in every lane (it is counted in `not_live`, never skipped and never an `uncaught-fault`), and the lane cannot be `release-ready` for ct-compare while it is (section 4.1). An `ir.core` rewriting of the same fault (`if p >= 2^63 then p - 2^63 else p`) was considered and declined by the owner: the table keeps one definition per fault, in the level whose built-in it names. Because a generator that has not implemented `ir.fixed` cannot evaluate this fault, it emits the fault row as data (`requires: ["ir.fixed"]`) and the control row with `requires: ["ir.fixed"]`, every candidate claim in `required_claims` and `slim_witness` entries `{"claim": C, "deferred": "ir.fixed"}` (section 5.6; these are not `unproven`, since `eq_u64_agrees` has a stand-in and only the evaluator is missing), and leaves the witness search to S10. The fail-closed rule that an unimplemented level refuses a *suite* (section 13, "Slice S3 in detail") does not apply to a fault whose level only gates a control, because no **suite or chunk digest** depends on the fault. The same is not true of the slim digests in general: a witness chunk feeds `slim_chunks` and so `slim_inputs_sha256`. For ct-compare nothing changes, because every suite has `m <= 16` and so is `full` in a slim run (section 5.6), but when S10 adds the witness search this control row changes (its `slim_witness` entries) and so can the slim sets of any suite with `m > 16`.

**`flip-result-bit` and `drop-last-result-byte`: exact forms [D27, D35; normative from S10c-1 (oracle) and S10c-2 (generator)].** `R` is `["var","result"]`, `N0` is `["lit",{"type":"Nat","value":"0"}]` and `N1` is the same with `"1"` (the `Nat` literal form of the committed ct-compare faults), `U1` is `["lit",{"type":"U8","value":"1"}]`, `U0` the same with `"0"`, and `B01`, `B00` are `["lit",{"type":"Bytes","value":"01"}]` and `...,"value":"00"}]` (a `Bytes` literal is lowercase hex, section 2.3; the empty-result values are the Rust harness's `01` and `00`, which the x25519 adopter PR confirms against the bindings before its fixture is committed). Let `GT` be `["gt",["len",R],N0]`.

| Fault and result type | `expr` (kind `post`) | `requires` |
|---|---|---|
| `flip-result-bit`, fixed-width result `T` | `["bit_xor", R, ["lit",{"type":"T","value":"1"}]]` (the `xor-lsb` form; `T` is the type's name) | `["ir.fixed"]` |
| `flip-result-bit`, `Bytes` **or `Bytes<N>`** (one template; the `if` wrapper is emitted for `Bytes<N>` too, whatever `N`) | `["if", GT, ["concat", ["be_bytes", ["bit_xor", ["unwrap_or", ["at", R, N0], U0], U1]], ["drop", R, N1]], B01]` | `["ir.fixed"]` |
| `drop-last-result-byte`, `Bytes` or `Bytes<N>` (a fixed-width result is not a template: the generator code `fault`) | `["if", GT, ["take", R, ["sub", ["len", R], N1]], B00]` | `["ir.core"]` |
| either, result `Result<X,E>` with `X` as above | `["match", R, [[["ok",["bind","ok_payload"]], ["ok", F]], [["err_any"], R]]]` where `F` is the `expr` of the row above with `["var","ok_payload"]` in place of `R` throughout | the union of the row's level and `ir.tuple` |

Typing: `concat(be_bytes(..), drop(..))` has the type `Bytes` (dynamic) and so does the `if`; for a `Bytes<N>` result the faulted value is therefore typed `Bytes` (inside `["ok", F]`, for a `Result<Bytes<N>,E>`, the payload of the faulted `Ok` is likewise `Bytes`) and a consumer compares and streams it as `Bytes` (the flip keeps its length; the drop deliberately does not, which is the fault). `be_bytes` of a `U8` is `Bytes<1>` (Appendix A.1); the collision rule applies only to the `Result` fault forms that introduce the `ok_payload` binder: there, an op parameter or binder named `ok_payload` is a generator `fault` error, as one named `result` is `W0701` (author refinement, S10t-1 review). A result `Result<X,E>` with `X` a tuple, a variant or anything not in the table has no template (the generator code `fault`, nothing skipped). Because `match` on `Result` is `ir.tuple` [D35], these rows are **live** from the S10c flip and never `deferred` (the plan's deferral to S11 is superseded, section 16.10 D27). A fault row's `requires` may therefore hold more than one level (section 16.6 item 3 said one; the sorted union of R9 applies). The registry keeps `xor-lsb` and `truncate-last`; the two new names are the Rust bindings' spellings, which section 8.4 requires the profile to equal.

**Adopter-specific faults** (`alt-poly`, `exp-mod-256` in gf256) reference **fault-only models** emitted under `fault_models`: pure model bodies that may exist *only* for a fault and may never appear in a claim, so the corpus cannot be made to depend on them.

**WX-G faults** are all `post` rewrites over the result and the pre-state parameters: `stale-post` (result is the pre-state), `skip-last-write`, `drop-last-push`, `clobber-prefix`, `cursor-off-by-one`, with the applicability table of VER04k section 4.8. Two of those conditions name a host form (`drop-last-push` and `clobber-prefix` need a growable buffer, `cursor-off-by-one` an integer cell), which the language-neutral corpus cannot know. The corpus therefore lists the control with `applies_to_forms: [...]` in section 7.6 vocabulary; a lane whose `inout_form` for that parameter is not listed records the control as `not-applicable` with the form, and the manifest checker verifies that against the op row's `inout_form`. A lane cannot mark any other control not-applicable.

### 8.3 Controls, scoping and catching

VER04i section 4 is otherwise unchanged and applies to v2:

* Controls are per `(target suite kind, op, fault)`. Each perturbs **exactly one op**; a control passes iff the suite run with that single-op fault fails with a registered failure id. The scoping exists because a fault applied to every operation can cancel inside a relational claim: XOR-ing the low bit of every result turns `mul(a,b) == mul(b,a)` into `mul(a,b)^1 == mul(b,a)^1`, which is still true. This matters *more* in v2, where relational predicates are common.
* Each control row lists `required_claims`, the claims (by suite id) not proven blind (section 5.6), and `blind_claims`, the claims proven unable to witness the fault; the manifest's `caught_by` must be a non-empty subset of `required_claims`. An unrelated failure cannot credit a control, and a blind claim never appears in `required_claims`. A control no claim can catch is `uncaught-fault` and fails the lane unless the corpus's `accepted_blind_faults` names its control id (a fault is accepted by an `accept_blind <op> <fault>` line of `corpus.profile`, section 8.4, [D15]). A **not-live** control (section 4.1) is the one exception: the lane does not run it, its manifest row has `caught_by: []` and the state `not-live` with its `level`, and it is neither caught nor `uncaught-fault`; it does not fail the lane, but it counts in `not_live` and so blocks `release-ready`. A not-live row with a non-empty `caught_by`, or a control recorded `not-live` whose `requires` levels the lane declares, is rejected by the manifest checker. The control row is `{"id": "warrant/<module>/negctl/exhaustive/<op>/<fault>", "op", "fault", "target_kind", "requires", "required_claims", "blind_claims", "slim_witness"}` plus `applies_to_forms` for the WX-G faults of section 8.2 (a key that is not applicable is omitted, section 1.4). `requires` is the level of the fault's expression, and is what a lane that lacks it uses to record the control `not-live` (section 4.1).

**Accepted-blind controls must be visible [normative, S3d review].** An accepted control appears in `corpus.json` only through the top-level `accepted_blind_faults` list (its control row has an empty `required_claims`), so every consumer must surface it, never fold it into a pass: (a) a lane manifest records it with the distinct control state `accepted-blind` and `caught_by: []`, never `uncaught-fault` and never caught; (b) the report counts such controls and shows the count like `not_live`; (c) the report validator (S9) rejects an `accepted_blind_faults` id that has no control row, or whose control row has a non-empty `required_claims`, and rejects a control with empty `required_claims` whose id is not listed. **A non-empty `accepted_blind_faults` blocks `release-ready` [D17]**, like a not-live control (sections 4.1 and 9.1).

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

`omit_example <id>` **[D22, D34]** removes one example from `corpus.json` `examples`, **but only when the tool can decide that the example is inexpressible** for one of two named reasons. `<id>` is the example's full id as `examples[].id` spells it (`warrant/<module>/example/<name>`). The profile line carries **only the id**; the author never asserts the reason, the tool computes it, and records it in the `omitted_examples` row `{"id": ..., "reason": ...}` (section 1.4), sorted by id. The closed reason vocabulary:

* `not-op-eq-pure`: the example is not of the shape `op(args) == pure` (section 1.4 `examples`; the generator's refusal for any other shape is `W0701`). Example: lzw's two `match`-shaped bomb examples.
* `value-over-limit`: a value the example needs (an argument, or the pure side) exceeds `MAX_IR_VALUE` (section 4.2) or the 4,096-byte `Bytes` value limit of section 2.3, found by evaluating the example's argument and pure expressions with the size checked **before** the allocation (as rule 6 requires). Example: lzw's `bomb512`/`bomb1024` arguments of about 142,848 and 285,696 bytes.

**Shape (author refinement, S10-1 review).** `op(args) == pure` is defined exactly as the generator's `lower_example` (`W0701`) defines it: **exactly one subject call, its arguments free of subject calls, on one side of an equality, the other side pure** (no subject call). A body that is not an equality, has no subject call, has subject calls on both sides, has a subject call that is not the whole of its side, or has a subject call inside an argument (`echo(echo(x)) == y`) is `not-op-eq-pure`. **First error (author refinement, S10-1 review, ruling 2):** "first error" in the order below means evaluation **stops** at the first error: a size error in an earlier argument qualifies even if a later argument would fail with a different error. **Levels (ruling 3):** an omitted example is still lowered and must be within the tool's implemented levels; a level refusal refuses the document even when the example has an `omit_example` line. **Evaluation order (ruling 1):** arguments, then the pure side, then the [D19] check, then the [D18] check; the generator's S10-1 PR adopts this order. If both apply the reason is `not-op-eq-pure` (the shape is tested first; author refinement, section 16.10 R4). **Normative order of the `value-over-limit` decision [author refinement, section 16.10 R4; S10-1, both tools, which must follow it identically even though the generator evaluates per argument and the oracle evaluates all arguments first]:** (1) evaluate the arguments left to right, then the pure side; (2) any evaluation error **other than** a size error refuses generation; (3) a size error qualifies as `value-over-limit` **only if it is the first error** met in that order; (4) if the evaluation completes, run the D18 and D19 checks first and refuse on failure; (5) only after that does a final value over the 4,096-byte limit of section 2.3 qualify. An example that fails (2) or (4) is never rescued by the line. **Nothing else qualifies.** A v0 wrap (section 3.3 rule 2, [D18]), a call outside the callee's `admit` ([D19]) and any other check failure still refuse generation for that example: the line never rescues them, and an omitted example is simply not evaluated, not written to `cases.tfv`, not stream-encoded and not checked against an `admit`. Error cases, in the style of `accept_blind` below (generator code `profile`, oracle code `corpus`): a malformed line (wrong token count, an empty token) is an error; an `<id>` that names no example of the spec is an error; a second `omit_example` line with the same id is an error; a line of whitespace alone is an error like any other; and **a line for an example that is emittable is an error** (an omission that excuses nothing is a stale profile, [D15] refinement 2), with the message naming the id and never echoing example values. A document with zero examples left is accepted. An omitted example does **not** block `release-ready`; every report shows the count beside `not_live` and the S9 validator prints the ids [D34] (implemented by the lane runners, S6 and S7, and S9; no check enforces it before). The parsing and the reason are implemented in slice S10-1 (section 13), oracle first; until then both readers keep refusing the unknown line.

`accept_blind <op> <fault>` **[D15]** accepts a fault that every candidate claim is blind to (section 5.6). It must repeat a `fault` line of the same file, may not repeat itself, and names a fault that really is all-blind: a line for a fault some claim can see (witnessed, unproven or deferred) is a profile error, because an acceptance that excuses nothing is a stale profile. The accepted faults are recorded in `accepted_blind_faults` as **control ids** (`warrant/<module>/negctl/exhaustive/<op>/<fault>`), sorted by bytes (section 16.8). Only an empty line is skipped: a line of whitespace alone is a profile error.

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

**`release-ready` [D17].** A lane is `release-ready` for an adopter only when it has zero not-live suites, zero not-live controls (`not_live` is 0) **and zero accepted-blind controls** (the report's count of controls in the state `accepted-blind`, section 8.3). The report shows that count beside `not_live` wherever it prints `depth`. This slice states the rule; the lane runners (S6, S7) and the report validator (S9) enforce it.

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

**Status (S4 part 2).** `verify-v2 <corpus-dir> --warrant P --profile P [--bindings P]` implements all three checks over a whole fixture directory (`corpus.json`, `chunks/*.txt`, `cases.tfv`), byte for byte where this document fixes bytes and structurally where section 1.4 is provisional; section 16.7 records what it decided where this document is silent.

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
| S4 | oracle | `code/packages/python/warrant-oracle/` at `ir.core`, with chunk verification and memoisation. **Part 1** (done): reader, lowering, enumeration, `verify-v1`, `emit-v2`. **Part 2** (done, section 16.7): `verify-v2`, check 3, witness chunks, `corpus.profile` | no | part 2: the three checks of section 10.3 pass on ct-compare v2 and on a seeded-wrong corpus fail |
| S5 | fixture | the v2 categories of section 11 at `ir.core`. Prerequisite: `warrant-corpus-conformance-v1` (branch `claude/warrant-lane-conformance`) is on `main`. Independent of the ct-compare v2 fixture directory | no | oracle verifies; Python and TypeScript runners fail a deliberately wrong evaluator |
| S6 | lane | Python runner reads v2 as well as v1; `python/ct-compare` moves to v2 | no | manifest v2 `--check` green at slim and at full |
| S7 | lane | TypeScript runner, the same. **Go**: the in-flight Go lane finishes on v1 for ct-compare [D5]; it ports to v2 in its own PR in this slice or after, but before S13 | no | same |
| S8 | adopter | **gf256**: `gf256-warrant-v2`, then one PR each for `python/gf256`, `typescript/gf256` (adds the dedicated error class), `go/gf256` (panic mapping) | no | all lanes green at slim; nightly full on compiled lanes; the section 3.3 guard passed or the document migrated to v1 |
| S9 | report | Generic v2 lane validator in `ca-assurance-report`, joined to WD-004b1 | **yes** | falsification test per check, VER04i section 9 pattern |
| S10 | level | `ir.fixed` and `ir.tuple` **[D20, D35]** (`ir.tuple` includes `match` on `Result`), the slices of "Slice S10 in detail" below (oracle before generator at each layer, then fixture and runners behind S5 to S7). Adopters: **x25519**, then **lzw**, then **wasm-leb128** | no | fixture vectors for every built-in |
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
| S3d | the committed ct-compare v2 fixture directory, with or after S4 part 2 [D10], plus the build-tool row below, and the owner decisions of section 16.8 (**done on branch** `claude/warrant-cv2-s3d`: `warrant-corpus-gen` 0.4.0, `warrant-oracle` 0.3.0) |
| S3e | generator, oracle | the owner decisions of 2026-11-17, second batch (section 16.9): v0 rule 2 on examples [D18], a call outside an op's `admit` refused [D19], the `release-ready` rule [D17] (**done on branch** `claude/warrant-cv2-s3e`: `warrant-corpus-gen` 0.5.0, `warrant-oracle` 0.4.0) | no | both implementations refuse and accept the same documents; no emitted byte changes |

*Build-tool input row [D11h].* `code/specs/ct-compare.warrant`, the v2 fixture directory and `rust/ct-compare/evidence.bindings` lie outside the generator's crate, and the build tool maps a changed file to a package by path prefix, so a PR that edited only one of them would not run the generator's `--check`. A row in `code/programs/go/build-tool/repository_inputs.go` selects `rust/warrant-corpus-gen` for those paths. It is implemented in S3d (done on branch), as its own commit: the rows `rust/warrant-corpus-gen` and `python/warrant-oracle` (the oracle's existing row gains the fixture directory and the bindings), and a test that proves both are selected (section 16.8). (The v1 `gen_corpus --check` has the same blind spot today.)

**First adopter after ct-compare is gf256.** It is a v0 document, so it needs no v1 built-ins or composite types, and it forces the four structural changes: predicate claims, several calls per case, chunking at 2^24, and portable faults. It exists in 17 of the 18 lanes. **zeroize is last**: besides WX-G it has ports only in c, cpp, csharp and fsharp, so its first lane needs a C or C# runner or a new port.

**Order and size.** Slices land in numeric order, except that S9 depends only on S5 and may land earlier than its number. Until S9 lands, a lane's own `check` is the only validator of a v2 manifest and no v2 lane row may be cited as report evidence. S7, S8, S10, S11 and S12 are each several PRs, listed in their rows; no single PR carries more than one of {generator, oracle, fixture, one lane runner, one adopter}. One exception to numeric order: S3d (the ct-compare v2 fixture directory) lands **after S3a to S3c and with or after S4 part 2** [D10]; "with" means merged together with S4 part 2's PR (adjacent merges, or a merge queue), **never in the same PR as the generator** and never before `verify-v2` exists.

**Next level slices [D16].** After S3e the next generator, oracle and fixture work is **S10 (`ir.fixed`, with `ir.tuple` [D20]) and then S11 (`ir.data`)**, ahead of the lane runners S6 and S7 in effort; section 13's numbering is unchanged (a planning note, not a renumbering). `ir.fixed` first because ct-compare's `eq_u64/ignore-high-bit` control is `not-live` until it lands (section 8.2, [D9]).

**Slice S10 in detail (decided 2026-11-17, section 16.10).** S5, S6 and S7 are not on `main`, so the conformance vectors and the lane runners cannot land yet [D30]: S10's generator, oracle and corpus fixtures can land now, and **S10 produces no lane evidence until S5 to S7 land**. Order: oracle before generator at each layer, so every generator PR lands with byte parity against a committed oracle dump. No stacked chain: each PR is based on `main` after its predecessor.

| PR | Package(s) | Content | Blocked on |
|---|---|---|---|
| S10-0 | `code/specs` | this erratum (section 16.10); the VER-BACKLOG row is added by a **separate docs PR**, not by this commit; no code | owner (done) |
| S10-1 | `python/warrant-oracle`, then `rust/warrant-corpus-gen` (two PRs, oracle first) | `omit_example` lines, the computed `reason` and the conditional `omitted_examples` key [D22, D34]; may land any time after S10-0 and before the lzw fixture; no emitted byte of an existing fixture changes (the key is written only when an example is omitted), so the pair needs no atomic PR and bumps no version | S10-0 |
| S10a | `python/warrant-oracle` | lowering and typing of the 35 built-ins of Appendix A.1, `requires` inference (`ir_levels`), cost, committed lowering dump; evaluation still refuses (`level ir.fixed`) | S10-0 |
| S10b | `rust/warrant-corpus-gen` | the same for the generator, byte parity with S10a's dump, census updated and pinned; `generate` stays a fail-closed stub for `ir.fixed` documents | S10a |
| S10c-1 | `python/warrant-oracle` | evaluator, domains ([D21], [D25], [D26]), value and TFV paths, fault templates (gated), `emit-v2` on synthetic `ir.fixed` documents, enumeration dump | S10b |
| S10c-2 | `rust/warrant-corpus-gen` | evaluator, domains, fault templates (gated), generate enabled, parity with S10c-1's dumps, a generator-written synthetic corpus the oracle's `verify-v2` accepts | S10c-1 |
| S10t-1, S10t-2 | oracle, then generator | `ir.tuple` [D20, D35]: type, literal, `project`, values, stream tag `64`, TFV, descriptor `tuple`, **and lowering, typing, cost and evaluation of `match` on `Result`** (patterns `ok err err_any tuple bind wild`) in both tools; the `ir.tuple` fault templates (`ok-as-err`, `drop-consumed`, `swap-components`, the tuple-payload `missing-error`, the `Ok`-form of `flip-result-bit` and `drop-last-result-byte`); may be developed beside S10c-1 and S10c-2 but **must precede the flip** | S10b |
| S10c (flip) | both packages and the ct-compare fixture, **one atomic PR** [D29] | turn the two fault-evaluation gates on for `ir.fixed` and `ir.tuple`, bump `warrant-corpus-gen` and `warrant-oracle` to their next minor versions (0.6.0 and 0.5.0 if nothing else has bumped them; the S10 plan's 0.5.0 and 0.4.0 predate S3e, which already used them), regenerate the ct-compare fixture and the oracle's `accept-blind-v2` and `synth-v2` fixtures, and follow every step of the ct-compare fixture README's "Bumping the generator version" checklist (the Cargo files, the three regenerations, the `--expect-generator` pins in the oracle's `BUILD` and `BUILD_windows` for ct-compare **and** synth-v2, the pin in `test_verify_v2.py`, the README hash table; [D13] counted six places, the list has had seven since S3e) | S10c-2, S10t-2 |
| x25519 adopter | fixture (`x25519-warrant-v2`) plus the table-driven build-tool rows [D32] | `corpus.profile` hand-written, BUILD lines in both packages; its faults inside `Ok` are live at `ir.fixed` plus `ir.tuple` | S10c (flip) |
| lzw adopter | fixture (`lzw-warrant-v2`) | `omit_example` for the six bomb examples [D22, D34] **(acceptance conditions, author refinement, S10-1 review: the six examples of `code/specs/lzw.warrant` (`bomb_*`, about lines 231 to 245) must omit with their expected reasons. The two `match decompress(..) with Ok/Err` examples are `not-op-eq-pure`, which needs S10t to lower `match` on `Result` in example bodies (an omitted example is still lowered, section 8.4); the four eq-shaped ones are `value-over-limit`, which needs S10c to evaluate `be_bytes`. The generator must defer its non-`op == pure` refusal at `lower_example` (`W0701`) to the profile decision of section 8.4, so that an `omit_example` line can excuse it.)**; oracle verification sharded by chunk if it does not fit one job [D33] | S10c (flip), S10-1 |
| wasm-leb128 adopter | fixture (`wasm-leb128-warrant-v2`) and the spec change of [D28] | **split the two `W0701` claims** (42 to 44 claims, Rust evidence regenerated) in this PR; signed `fx_edge` [D21] (7 claims); tuples and `match` on `Result`; the composite faults at `ir.tuple` | S10c (flip), after the x25519 and lzw adopters. Note: a claim shaped `match decode(..) { Ok(..) => <another subject call> }` is `W0701` under section 4.4 (a subject call under an impure guard), like the [D28] claims, so the census of "4 match+tuple" claims must be re-checked in this PR |
| S10d | conformance fixture | `v2-ir`, `v2-arith`, `v2-domain`, `v2-fault` vectors at `ir.fixed` and `ir.tuple` (including `match` on `Result`) | **S5** |
| S10e, S10f | `python/warrant-corpus`, `typescript/warrant-corpus` | `ir.fixed` and `ir.tuple` in each lane runner; the ct-compare lanes drop `not_live` to 0 for the `eq_u64` control | **S6, S7**, S10d |

The flip is the one **exception to "no PR carries more than one of {generator, oracle, fixture}"** [D29]: the generator's BUILD runs `--check` on the committed fixture and the oracle's BUILD runs `verify-v2` on it, so a generator that emits a witness `chunk` against an oracle that expects `deferred` (or the reverse) is red in CI. Before the flip, the generator and the oracle implement `ir.fixed` suites, models and examples but keep **fault evaluation gated at `ir.core`** in the shipped path (the witness search over `ir.fixed` faults is reachable only through a library parameter used by tests), and neither bumps a version the fixture pins. After the flip the oracle **rejects** a `{"deferred": "ir.fixed"}` or `{"deferred": "ir.tuple"}` entry (levels it implements); a lane records the control `not-live` until its runner declares `ir.fixed` (and `ir.tuple` for a fault that needs it). Also in S10c-1 and S10c-2: the witness search for `ir.fixed` faults changes the `slim_witness` of `eq_u64/ignore-high-bit` in the ct-compare fixture (`{"claim": "warrant/ct_compare/exhaustive/eq_u64_agrees", "chunk": 0}`, expected witness `a = 0, b = 2^63`, to be confirmed by both tools), and nothing else in the fixture moves (every suite has `m <= 16`).

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
4. **Signed `fx_edge`.** Section 5.1 said the unsigned bit patterns reinterpreted in two's complement, in the same order. The generator sorts them ascending, adds `-1..=-255` and removes duplicates (VER04j 2.6: `i16` 527 values, `i32` 559, `i64` 623). The unsigned counts agree (377, 313). **Decided 2026-11-17 ([D21], section 16.10): the spec follows the generator.** Section 5.1 now says `wrap_S(E_U(w)) ∪ {-255..=-1}`, de-duplicated, ascending. The oracle's `leaf_fx_edge` changes to match (S10c-1); no shared-crate PR is needed, since `warrant-grammar` already behaves this way. *(The text of this item before the decision is kept above for the record.)*
5. **`fx_edge` at 16 bits and below (open question, needs an owner decision; found in S2).** Section 5.1 defines `fx_edge` for 32 and 64 bits only. The generator's budget demotion (VER04j 2.6) also produces edge sets for 16-bit binders (`FxEdge(U16|I16)`, 281 and 527 values), which the corpus must be able to describe, and the unsigned edge formula repeats values at 8 bits (`0..=255` already contains `MAX` and the `MAX ^ (1<<i)` patterns), so an 8-bit edge set has duplicates; the generator never produces one (an 8-bit binder is always `fx_full`). Decide whether `fx_edge` extends to 16 bits (and whether 8 bits is excluded or de-duplicated).

**Decided 2026-11-17 (section 16.10):** item 3 by [D26] (new leaf `nat_shift`; `nat_count` stays `0..=K`), item 4 by [D21], item 5 by [D25] (16-bit `fx_edge` by demotion only; 8-bit illegal). Items 1 and 2 (lists of byte strings) stay refused; lists are `ir.data` (S11).

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

**Follow-ups outside this slice.** The oracle README (`warrant-oracle/README.md`) still says "`slim_sha256` is emitted as separate inputs and expected digests"; it is accurate for the oracle and needs no change, and S4 part 2 reads `blind_claims` and the `rewrites` shape in check 3 (section 16.7). VER04i has no field named `slim_sha256` or `blind_claims`; VER04i sections 4 and 15 (`accepted_blind_faults`, `required_claims`) describe the v1 profile, which keeps `required_claims` as written until S13, and need no edit. VER04n and VER04p mention `accepted_blind_faults` only for v1-style controls and are unaffected. VER29 does not mention these fields.

### 16.4 Divergences and clarifications found by S3a

Slice S3a (lowering and cost, `rust/warrant-corpus-gen` 0.1.0) agreed byte for byte with the independent oracle on ct-compare, gf256 and two synthetic documents, and found the following. Items marked **OPEN** are questions for the owner and are not decided here.

1. **Section 4.2 rule 6.** ct-compare's largest expression costs **10**, not 9: the body of `eq_agrees`, of each `eq_fixed_agrees_wN` and of `eq_u64_agrees` is `1 + 3 + (1 + 2 + 3)`; `select_picks`' body is 9. The oracle computes the same. Corrected in place.
2. **Section 4.2 rule 6.** The bound applies per expression **including each model body** (the oracle already did). Corrected in place.
3. **Section 13, fail-closed rule.** A refusal names a level of section 4.1 or one of the feature names `inout`, `ensures`, `vectors`, `assume`. Corrected in place.
4. **Interim, `assume`.** S3 refuses any non-`none` `assume platform` / `assume environment` row. **OPEN:** the shape of `assumes` (section 12 item 3) must be settled before the first adopter with such a row.
5. **Sections 6 and 1.4, op rows.** `requires` is always present (at least `["ir.core"]`); `admit`, `width_params` and `fresh` are conditional. Applied in place in section 6 (the example rows and the `requires` row of the table). *(Decided 2026-11-17, [D19]: refused at generation; section 4.5 and 16.9.)*
6. **Section 4.2 rule 4 / Appendix A `neg`.** The signed minimum literal lowers to `neg` of a `lit` holding `-2^(w-1)`; `neg` wraps it to itself (section 3.2) and counts in `wraps`. Applied in place in section 4.2 rule 4 (Appendix A already said it).
7. **Section 8.4.** Repeated `fault` or `slim_chunk` lines, an empty token, `budget 0` and `slim_stride 0` are profile errors (the Rust reader in S3b, the oracle's in S4 part 2). S3a's reader mirrors the oracle's and still accepts them. (Both readers refuse them since S4 part 2 and S3b, section 16.7.)
8. **Section 14.** Model-call nesting is limited to 64 frames (VER04 section 2). Applied in place (a row of the section 14 table).
9. **Section 3.3 rule 3** also covers a `model_call` over a subject result whose callee's closure wraps. Applied in place; the oracle implements rule 3 since S4 part 2 (section 16.7).
10. Type aliases (`model Word = Bytes<4>`) are resolved away and never appear in a corpus; `is_err` has no source form (Appendix A). An example must be one subject call compared with a pure value (the section 1.4 `examples` row); S3a refuses any other example with `W0701`.

### 16.5 Divergences and clarifications found by S3b

Slice S3b (the evaluator, enumeration, the 18 digests and the census; `rust/warrant-corpus-gen` 0.2.0) agreed with the independent oracle on every count, digest, descriptor and chunk line of ct-compare and two synthetic documents (`tests/enumeration_parity.rs`), reproduced all eighteen v1 digests, and found the following. Items marked **OPEN** are questions for the owner and are not decided here.

1. **Section 4.3, in place.** A claim whose body calls no operation is `W0701` (S3a review gap L5). Applied in section 4.3.
2. **Section 4.5.** A suite that admits zero cases is refused with the code `W0706` (the v1 generator's code for an empty admitted domain). Recorded here, not yet applied in place.
3. **Section 3.3 rule 1, precision.** The `wraps` count is taken on the *pure trace*: a pure guard (the condition of an `if` with a subject in a branch, the left operand of a short circuit whose right operand calls a subject) is evaluated and its wraps counted and the taken branch followed; the subject-independent arguments of a `subject_op` and the `expected` side are evaluated; a node with a subject beneath it is never evaluated; an operand that runs only under a short circuit whose guard depends on a subject result is skipped; the nodes of a `model_call` body count on every call. `admit` counts once per product ordinal (the generator evaluates it once per prefix tuple of the binders it reads and multiplies). Both implementations agree on all synthetic suites (a wrapping `admit`, a wrapping subject argument, a pure guard). Recorded here, not yet applied in place.
4. **Section 2.3 and the streams.** A stream value must have its declared type: a `Nat` or unsigned value outside `0..=2^64-1` (for example the `Int` result of `Nat - Nat` when the subject side is a `Nat`) cannot be encoded and is `W0701` at generation. Recorded here.
5. **Section 3.3 and WD-013, correction of the record.** The `warrant-grammar` `Evaluator` *wraps* an `EvalValue::U8` (and `U64`) in a v0 document too; the v0 divergence of WD-013 appears where the harness hands a `U8` over as a `Nat`, when `x + 1u8` is the exact sum. S3 therefore uses that evaluator only as a differential on sampled cases and pins both behaviours in a test. The v0 guard rules 2 and 3 are unchanged.
6. **Section 5.1, descriptor names (owner decision 2026-11-17, applied in place).** `u8_all` and `u64_edge` are the canonical names of `U8` and `U64` domains in every document version; `fx_full`/`fx_edge` are used only for the other widths (an `I8` is `fx_full` with `t` `I8`). Generator and oracle already agree. (Original finding:) A v1 document resolves a `U8`/`U64` binder to `Fx(U8)`/`Fx(U64)`; the generator and the oracle name them `u8_all`/`u64_edge` whatever the document version (the value sets are identical), so `fx_full`/`fx_edge` name only the other widths. Section 5.1 now says which name each takes.
7. **Section 5.6, profile lines.** `slim_stride` is ignored for a suite of at most 16 chunks (the slim set is every chunk); for a larger suite it is **clamped** to the suite's automatic stride, `min(profile stride, automatic)`, and is never an error (owner decision 2026-11-17, applied in place in section 5.6 and in the generator; the oracle implements the clamp since S4 part 2 (section 16.7)); a `slim_chunk` line must name a declared claim and a chunk below `m`. Applied in the generator; recorded here.
8. **Section 16.2, S3 refusals.** A refused 16.2 descriptor is reported with the generator code `domain` naming the item (`nat_count` 3, signed `fx_edge` 4, `FxEdge` at 16 bits 5, a list of byte strings 1/2).
9. **Section 16.2, the census (input to S10).** Over the eleven adopters that bind `gen_evidence` (355 claims; `max_ir_cost 65536` so cost blocks nobody): `ct-compare` (9 of 9) and `gf256` (38 of 38) are emittable at `ir.core`, 89 claims in all; the rest are blocked by `ir.data` (154 claims), `ir.fixed` (101), `inout` (9) or `W0701` (2: a subject call under an impure guard in `wasm-leb128`). **No binder touches items 1 to 3** (no list of byte strings, no `nat_count`); **item 4 (signed `fx_edge`, `I64`) occurs in 7 `wasm-leb128` claims and item 5 (`FxEdge` at 16 bits) in 2 `lzw` claims**, each already blocked by `ir.fixed` or `ir.data`. Items 4 and 5 therefore need deciding before S10 lifts those levels; items 1 to 3 can wait for the first adopter that has one.
10. **Section 8.4, in the generator.** The profile reader refuses a repeated `fault` or `slim_chunk` line, an empty token, `budget 0` and `slim_stride 0` (16.4 item 7); the oracle's reader refuses them since S4 part 2 (section 16.7).
11. **Resource bounds, checked before the first case (corrected after review).** The total IR work of a full run (`MAX_IR_WORK_FULL`) and the total product ordinals (`2^32`) are charged for **every** suite after the domains are resolved and before any suite is planned (planning builds the admission table, up to `2^28` `admit` evaluations); the ordinal bound is on product ordinals, stricter than cases; a claim of more than 8 binders, a product past `2^28` and more than 4,096 chunks are refused by the planner.
12. **Review fixes in S3b.** The generator evaluates every pure node on the trace that may wrap or fail (section 3.3 rule 1, applied in place), so a pure subject argument that leaves the exact window or builds a `Bytes` past `MAX_IR_VALUE` is refused at generation. **For S4 part 2:** the oracle's `may_wrap` (`ir_eval.py`) is true for any `add sub mul neg`, including `Nat`/`Int`, but evaluates no pure `concat` on the trace, so a `concat` past `MAX_IR_VALUE` in a subject argument still generates there; the oracle must evaluate it too (done in S4 part 2, section 16.7). The generator's stream encoder now also rejects a fixed-width value outside its type and a `Bytes<N>` value of another length. The evaluator is iterative (an explicit work stack): the bounds of section 14 run on a default 2 MiB thread in a debug build. The work stack's bound (`MAX_EVAL_TASKS`, 65,536) is not a new limit: the pending tasks of an evaluation are at most the cost of the worst path, which is at most `max_ir_cost`, which is at most 65,536 (section 4.2 rule 6).

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
9. **Other keys.** `fault_models` is `{}`, `assumes` is `[]` (S3 refuses any non-`none` assume row, section 16.4 item 4), `accepted_blind_faults` is `[]` in S3c (OPEN 1, decided by [D15] and implemented in S3d: it lists the control ids of the profile's `accept_blind` lines, section 16.8). An example's `width` is omitted when the call has no static width (never `null`, section 1.4); every other example key is always present. The `models` rows have exactly `params` and `body`. A `pre` fault's stand-in assumes uniform rewrites per parameter (true of the registry; S10 faults must revisit it, and a `debug_assert` marks the spot). A suite row's `claim` is the claim name, from which `chunks/<claim>.txt` derives (section 5.5).
10. **Profile and bindings.** A `fault <op> <name>` line must name a declared op. The CLI's optional `--bindings <evidence.bindings>` reads the `faults=` list of every `op` line and the generator compares the sets with the profile's per op (`drift`, section 8.4); the profile is the authority and an op with no `faults=` and no `fault` line simply has no controls.
11. **Files.** Chunk files are the chunk lines joined by LF with one final LF. Writing validates every path first (`corpus.json`, `cases.tfv`, `chunks/<name>.txt`, names of ASCII letters, digits, `_` and `.`, no leading dot, no `..`), refuses a symbolic link at `chunks/` or at a target, refuses a `chunks/` that holds a file the corpus does not own (it never deletes one) (a leftover `.<name>.tmp` of its own from an interrupted run does not count and is overwritten), writes each file to a temporary name and renames it, and renames `corpus.json` last. `--check` compares `corpus.json`, `cases.tfv` and every chunk file byte for byte and the exact file set of `chunks/`, and reports each difference with file, line, column and byte offset (never content); the other files of a fixture directory (`corpus.profile`, `consumers.json`, README) are not the generator's and are not compared. `--warrant` must be a repo-relative path because it is recorded as `spec`. Hardening, all in `fsio`: the temporary file is unlinked and then created with `create_new` (a planted link, hard link or FIFO at its name is never followed, written through or opened); `--check` and the CLI read only regular files, through a handle, capped (the expected length plus one byte; 4 MiB for an input); a symbolic link present at `chunks/` is refused by write and reported by `--check` (a symlinked `--out` itself is the caller's choice); names found on disk are escaped before they are printed; chunk file names may be up to 133 bytes (two 64-character claim names, `.` and `.txt`) and a claim whose chunk file would be a Windows reserved device name (`con`, `prn`, `aux`, `nul`, `com1`-`com9`, `lpt1`-`lpt9`, in any segment, any case) is `W0701`. Concurrent modification of the output directory is unsupported. Without `--bindings` the CLI prints a note that the profile's faults were not cross-checked.
12. **Codes.** `GenError.code` gains `fault`, `drift` (an on-disk corpus or the bindings differ) and `io` (a refused path or failed file operation); `unimplemented` is gone.
13. **Parity with the oracle.** The chunk files equal the oracle's `chunk_lines` for all nine ct-compare suites (checked in the generator's tests against the committed oracle snapshot). The oracle's `slim_chunks` is the base set only, so the two differ on any suite with `m > 16` and a witnessed control until S4 part 2.

**Classification of the review (Fable, 2026-11-17).** For item 3 below the erratum is applied (sections 5.6 and 8.3: claims are named by suite id); the owner acknowledged it on 2026-11-17 ([D12], section 16.8); item 1 (`accepted_blind_faults`) and item 4 (the authored files) were decided by [D15] and [D14] (section 16.8); item 2 is an erratum for S10 and stays open.

**OPEN (not decided here).**

1. **`accepted_blind_faults`: where is it declared?** *(Decided 2026-11-17, [D15]: a profile line `accept_blind <op> <fault>`; section 16.8.)* Sections 5.6 and 8.3 say "the spec's `accepted_blind_faults`", but a `.warrant` file has no such declaration and `corpus.profile` (section 8.4) has no such line. S3c writes `[]` and treats an all-blind fault as an error with no escape. The owner chose the recommendation, a profile line `accept_blind <op> <fault>` (VER04p, which forbids some faults from the list, needs the same: see 16.8 follow-ups).
2. **The JSON of `ir.fixed` data nodes** (erratum for S10, not a ruling). Appendix A lists the 26 `ir.core` tags only. The emitted form for `ignore-high-bit` is `["bit_and", ["var", p], ["lit", {"type":"U64","value":"9223372036854775807"}]]` (and `["bit_xor", ["var","result"], ["lit", {"type": T, "value":"1"}]]` for `xor-lsb`). *(Decided 2026-11-17, [D24]: the shapes of Appendix A.1; section 16.10.)* Recommendation: `verify-v2` treats an `expr` of a level it has not implemented as **opaque data**, compared as canonical JSON and never evaluated (section 10.4); S10 fixes the node shapes in Appendix A.
3. *(Settled, not open.)* Suite ids as claim names in `required_claims`, `blind_claims` and `slim_witness`; the manifest's `caught_by` uses the same ids. Sections 5.6 and 8.3 now say so.
4. **Should `--check` also compare the authored files of a fixture directory?** *(Decided 2026-11-17, [D14]: no; the BUILD passes `--profile <fixture>/corpus.profile` and `profile_sha256` covers it; section 16.8.)* Today a stale `corpus.profile` is caught only through `profile_sha256` in `corpus.json`.

### 16.7 Decisions and findings of S4 part 2 (`verify-v2`)

Slice S4 part 2 (`python/warrant-oracle` 0.2.0) implements `verify-v2` and closes the oracle gaps the sections above listed under "S4 part 2". **No owner decision is recorded here.** Everything under "Author refinements" is the author's reading where this document is silent, written so the generator's owners and the lane runners can contest it; each is an erratum candidate, not a ruling. Items marked **OPEN** are questions for the owner.

**Result.** On ct-compare v2 as generated by the merged Rust CLI (S3c, `warrant-corpus-gen` 0.3.0; fixture directory not committed, [D10]) and on a synthetic 10-suite document with a 64- to 236-chunk slim set, a witness outside the stride, blind, unproven and deferred controls, a predicate suite, a v2-stream suite, an operation `admit`, a static width and `truncate-last`/`Nat` `off-by-one`, **the oracle derived every value of `corpus.json`, every chunk file and `cases.tfv` byte for byte as the generator wrote them**: no divergence between the two independent implementations was found. (The generator's `--check` and the oracle therefore agree on the section 1.4 key set as emitted; no key was added or removed.)

**The three checks, as implemented.**

1. *Lowering:* `types ops models model_order examples`, each suite's `id kind claim ops requires admit body oracle result_type`, each binder's name and type, and `cases.tfv` come from the oracle's own lowering and TFV writer.
2. *Enumeration and evaluation:* the descriptor trees, `demoted domain_label stream product cases discarded expected_calls wraps ir_cost chunks`, every chunk line, `chunks_sha256`, the suite, v1 and slim digests, `slim_chunks`, `budget`, `chunk_size` and `max_ir_cost`.
3. *Faults and controls:* `faults`, `fault_models`, `controls` (including `required_claims`, `blind_claims`, `slim_witness`) and `accepted_blind_faults`; the witness chunks feed `slim_chunks`.

**Author refinements (not owner decisions).**

1. **Comparison.** The oracle derives the whole fixture and compares it: type-strict (`true` is not `1`), closed (an unknown or missing key is a difference), with the canonical writer's bytes checked last (a reordered key or a changed indent is a `bytes` difference). Chunk files and `cases.tfv` are compared as bytes. Differences are grouped `lowering`, `enumeration`, `faults`, `controls`, `header`, `files`, `bytes`, `drift`; at most 12 are reported per field and no input bytes are printed.
2. **`spec` and `generator`.** `spec` must **equal** the repo-relative POSIX path of `--warrant`, taken from the nearest ancestor directory that contains a `code/` directory or from `--repo-root`. `generator` is `{name, version}`, each matching `[A-Za-z0-9._+-]{1,64}` (so no escape, lone surrogate or U+2028 can reach a report); the oracle cannot derive either value, so it checks the shape only. *(Decided 2026-11-17, [D13]: the build pins them; `--expect-generator`, section 16.8.)*
3. **Profile reader (closing 16.4 item 7 and 16.5 item 10).** An empty line is skipped; a line that is only whitespace, a token that is empty or padded, a repeated `fault` or `slim_chunk` line, `budget 0` and `slim_stride 0` are errors. A `fault` line must name a declared operation; a `slim_chunk` line must name a declared **claim** (not a suite id) and a chunk below `m`; a suite's slim set is capped at 32 (`MAX_SLIM_CHUNKS`) after the union with witness chunks. **Erratum (generator), closed in S3d (16.8):** the generator's reader accepted whitespace-only lines (`" "`, `"\t"`, `"\x0b"`) and emitted; it now refuses them like the oracle does. Integers above 2^64 - 1 are refused by the oracle as by the generator.
4. **Slim stride (closing 16.5 item 7).** For a suite of more than 16 chunks the effective stride is `k = min(profile stride, ceil(m / 14))` and the offset is `o = FNV-1a-64(suite id) mod k` with that effective `k`. Section 5.6 writes `o = ... mod k` before it introduces the clamp; the effective reading is the one the generator implements (verified on `slim_stride 12` over suites of 64, 118 and 236 chunks).
5. **Fail-closed above `ir.core`.** `verify-v2` refuses the whole document, naming the level, if any model, operation refinement, claim or example needs a level above `ir.core`, if the document has `ensures` rows or `inout` parameters, or a non-`none` `assume` row (16.4 item 4 is still OPEN); fault rows above `ir.core` are data (section 10.4), not refusals.
6. **Operation `admit`.** `ops.<name>.admit` is present iff a parameter has a refinement; it is the `and`-chain of the refinements in source order, folded left, exactly as a claim's `admit` (section 4.5). The generator agrees on one- and three-refinement operations (tested against the generator); the left fold is now stated in section 6.
7. **Fault registry.** The oracle instantiates the eight portable names by signature as section 16.6 item 4 reads them. Where section 8.2 is a sketch the oracle wrote: `truncate-last` is `if len(result) > 0 then take(result, len(result) - 1) else result`; `off-by-one` adds a `lit` of the result type (`Nat` included) holding `1`; `xor-lsb` takes any fixed-width result; `missing-error`'s `Ok(first Bytes arg)` is the first `Bytes` parameter in parameter order. All eight agree with the generator (`ignore-high-bit` and `xor-lsb` as opaque data); `truncate-last`'s `else` branch and `off-by-one`'s `Nat` case are applied to section 8.2 as an erratum.
8. **Witness search.** As section 16.6 items 5 to 8; the oracle walks every admitted tuple in product order once per suite for all its pending faults and stops a fault at its first difference. A fault expression that fails to evaluate is a generation error. A binder passed at two positions that a `pre` fault rewrites to two different values is a generation error (it cannot happen with the registry).
9. **v0 rules 2 and 3 (section 3.3).** Rule 3 is checked on a claim's `body`: a `+ - *` or unary `-` whose operand type is fixed-width with a `subject_op` beneath it, or a `model_call` with a `subject_op` among its arguments whose callee's call closure (transitively, through `model_call`) holds such a node; `Nat` and `Int` arithmetic is exempt. Rule 2 is checked per suite after counting `wraps`. *(Decided 2026-11-17, [D18]: rule 2 applies to example evaluation, so `f(255u8 + 2u8) == 1u8` is refused in a v0 document; section 3.3 and 16.9.)*
10. **IR depth (closing the off-by-one).** The bound counts levels, the root being level 1: a tree of 64 levels is accepted and one of 65 is refused. S4 part 1 admitted 65.
11. **`concat` on the trace (closing 16.5 item 12).** The oracle evaluates every pure `concat` on the trace as it does arithmetic, so a result past `MAX_IR_VALUE` in a subject argument or on the expected side is `ir-overflow` at generation.
12. **Bindings (optional `--bindings`).** The oracle reads only the `op <name> ... faults=a,b` lines of an `evidence.bindings` and compares the set per operation with the profile's `fault` lines (`drift`); an operation bound twice is an error. It is the same check section 8.4 gives the compiler.
13. **Files.** `chunks/` must hold exactly the derived names; every file is opened by derived name, must be a regular file (symbolic links are refused, also for `chunks/` itself), and is size-capped before parsing (corpus 16 MiB, chunk file 2 MiB, `cases.tfv` 4 MiB, profile and bindings 64 KiB). A FIFO or other non-regular file is refused after an `lstat` and before any open (it would block). A `corpus.profile` inside the fixture directory must equal the `--profile` argument; its absence is not an error. The `files` group covers `chunks/` and the three named files (`corpus.json`, `cases.tfv`, an optional `corpus.profile`) only: the fixture root is not closed, and an extra file beside `corpus.json` passes (section 16.6 item 11). The spec file's effective size cap is the lexer's 1 MiB.
14. **`emit-v2`** applies a profile's `slim_stride` but still emits the base set only; `verify-v2` computes the full slim set (base set, witness chunks, `slim_chunk` lines).

**Measured.** `derive` on gf256 (38 suites, 51.47 million cases, `max_ir_cost 65536`) takes about 9 s on one core; `emit-v2 --depth full` 9.8 s (review measurement, memoisation on).

**S8 blocker.** gf256's faults (`generator-shift`, `alt-poly`, `exp-mod-256`) and its `fault_models` are not in `faults.REGISTRY`, so `verify-v2` exits 2 ("an unknown fault name") on a real gf256 profile until S8 adds them. The generator does not have them either (`warrant-corpus-gen` names them only in profile-reader tests), so S8 needs both sides.

**Further findings, recorded not decided.** The generator's front-end checker (W0602) refuses a claim body at 63 to 64 IR levels and an `admit` at 64 while accepting a 64-level model body; the oracle accepts 64 levels everywhere and refuses 65. `accepted_blind_faults` was decided by [D15] (16.8). **OPEN (owner):** `assumes`: the oracle expects `[]` (16.4 item 4).

### 16.8 Owner decisions of 2026-11-17 (CV2-S3d)

Slice S3d (`rust/warrant-corpus-gen` 0.4.0, `python/warrant-oracle` 0.3.0) commits the ct-compare v2 fixture, wires it into both packages' `BUILD` and `BUILD_windows` and into the build tool, and implements four owner decisions taken at its start. The normative sections (1.4, 5.6, 8.3, 8.4, 13) were amended in the same change. Items under "Author refinements" are the author's reading where the decision is silent; they are erratum candidates, not rulings.

**[D12] Claims are named by suite id.** `required_claims`, `blind_claims` and every `slim_witness` `claim` hold suite ids (`warrant/<module>/exhaustive/<claim>`, as in v1), and the manifest's `caught_by` uses the same ids. The owner confirmed the erratum of 16.6 item 1 (sections 5.6 and 8.3); "owner acknowledgement pending" is withdrawn.

**[D13] The generator is pinned by the build.** `corpus.json` records `generator` (`{"name": "warrant-corpus-gen", "version": "0.4.0"}`). The generator's `--check` already compares the bytes of `corpus.json`, `generator` included, so a version bump of the crate fails `--check` until the fixture is regenerated and the diff reviewed. That is the pin; nothing was added to the generator. The oracle cannot derive the generator, so `verify-v2` gained an optional `--expect-generator name@version` (each part `[A-Za-z0-9._+-]{1,64}`; anything else is exit 2): a recorded pair that differs is a `header` mismatch at `generator`, and the message never echoes either value. The oracle's `BUILD` and `BUILD_windows` pass `--expect-generator warrant-corpus-gen@0.4.0`, so the pin is checked in both packages and **a generator version bump touches at least six places** (**seven since S3e**: the `tests/fixtures/synth-v2` regeneration, whose `generator.version` the oracle's BUILD lines pin with `--expect-generator`, is the seventh; the checklist "Bumping the generator version" in the ct-compare fixture README, steps 1 to 5, is the authority and is what a flip PR follows): the crate `Cargo.toml` and `Cargo.lock`, the ct-compare fixture (regenerated), the oracle's `tests/fixtures/accept-blind-v2/corpus/corpus.json` (the generator's test compares its bytes, `generator.version` included), the oracle's `BUILD` and `BUILD_windows` (`--expect-generator`), the version pinned in the oracle's `test_verify_v2.py`, and the fixture README's hash table. The fixture README has the checklist.

**[D14] Edits to the hand-written `corpus.profile` are caught.** `profile_sha256` in `corpus.json` is the digest of the `--profile` file. The `BUILD` lines of both packages pass `--profile code/specs/fixtures/ct-compare-warrant-v2/corpus.profile`, so an edit to the committed profile that is not followed by a regeneration fails the generator's `--check` (`corpus.json: differs ...` at the digest) and the oracle's `verify-v2` (`header profile_sha256`). `--check` itself still does not compare the authored files of a fixture directory (16.6 item 11 stands). The oracle additionally requires a `corpus.profile` inside the fixture directory to equal `--profile` (16.7 item 13). Verified by editing a copy of the profile: both tools fail.

**[D15] `accepted_blind_faults` is declared by `accept_blind <op> <fault>` lines of `corpus.profile`** (section 8.4), implemented in both readers and emitters (`rust/warrant-corpus-gen/src/{profile,emit}.rs`, `python/warrant-oracle/{profile,faults,corpus_v2}.py`). A fault whose every candidate claim is blind passes generation when a line names it; it is recorded in `accepted_blind_faults`, sorted, canonical. ct-compare has no all-blind fault, so its list stays `[]`.

**Author refinements (not owner decisions).**

1. **The entry is the control id.** `accepted_blind_faults` is an array of control ids (`warrant/<module>/negctl/exhaustive/<op>/<fault>`) sorted by bytes, so a lane manifest, which keys controls by id, can join on it directly. The control row of an accepted fault is still emitted: `required_claims` `[]`, `blind_claims` every candidate in suite order, `slim_witness` `[]`; its fault row is listed as for any fault.
2. **Fail closed in both directions.** An `accept_blind` line is a profile error (generator code `profile`, oracle code `corpus`) when it has no `fault` line of the same file, when it repeats an earlier `accept_blind` line, when it is malformed (wrong token count, an empty token), and, once the document is read, when it names a fault that is **not** all-blind (a claim can see it, or the claim is unproven or deferred): an acceptance that excuses nothing is a stale profile, and silently ignoring it would let a corpus claim a blindness it no longer has. An all-blind fault without a line is still the `fault` error of 5.6, whose message now points at `accept_blind`. A fault no claim calls stays a `fault` error even with a line: "no candidate" is not "every candidate blind".
3. **Reader errata.** Both readers skip only an empty line. The generator's skipping of whitespace-only lines (16.7 item 3 erratum) is closed: `" "`, `"\t"`, `"\x0b"` (and a form feed, a no-break space) are profile errors.
4. **Fixtures for the line.** `python/warrant-oracle/tests/fixtures/accept-blind-v2/` is a small synthetic document with two all-blind faults accepted (listed in reverse order in the profile to test the sort) and two witnessed faults; it was written by the generator, `verify-v2` accepts it, and the generator's tests regenerate it byte for byte, so the two implementations agree on the line in both directions. The `synth-v2` fixture is unchanged (its profile has no `accept_blind` line; it still records generator `0.3.0`, which `verify-v2` checks for shape only).
5. **Where the fixture's authored profile lives.** The ct-compare profile moved from `rust/warrant-corpus-gen/tests/fixtures/ct-compare.corpus.profile` to `code/specs/fixtures/ct-compare-warrant-v2/corpus.profile` (section 8.4: the profile belongs to the fixture directory). The generator's tests read it there. `.gitattributes` in the directory is `* -text`, because every file is hashed or compared byte for byte; the repository's `* text=auto eol=lf` is not applied.
6. **BUILD.** The generator's `--check` line runs from the repository root (`--warrant` is recorded as `spec`) and is one `cd` plus one `cargo run` invocation, so `BUILD_windows` carries the identical line under `cmd /C` (the existing sabotage line stays Unix-only: it needs `mktemp` and `sed`, and a cargo test covers it on Windows). The oracle's two lines are `verify-v2` on the new synthetic fixture and on the committed ct-compare fixture.
7. **Build tool.** `repository_inputs.go` gains the row `rust/warrant-corpus-gen` (inputs: `code/specs/ct-compare.warrant`, the fixture directory, `code/packages/rust/ct-compare/evidence.bindings`, and the oracle's accept-blind fixture directory, which the generator's tests regenerate and which lies in the oracle's own directory), and the existing `python/warrant-oracle` row gains the fixture directory and the bindings. Directories match by slash-delimited prefix, so `ct-compare-warrant-v20/` is not the fixture. `TestWarrantCorpusV2FixtureSelectsGeneratorAndOracle` proves that every fixture file and the bindings select exactly those two packages, that a language-filtered run keeps one, that lookalikes select neither, and that a consumer missing from discovery is a hard error. **Not covered:** the generator's tests also read the v1 `corpus.json` and `cases.tfv` (the 18 digests); adding them would put the generator in every v1-corpus selection and change the expected sets of existing selection tests, so it is left as a follow-up.

**Result.** `code/specs/fixtures/ct-compare-warrant-v2/` holds `corpus.json` (43,790 bytes, SHA-256 `4a61bfb6561b902a7f91db67638bb4cf449635c321794ff7d5d6179d14654c45`), `cases.tfv` (1,318), nine chunk files (138 to 596 bytes), the authored `corpus.profile` (209 bytes, `profile_sha256` `848056c69a04716c92117d75e7eb034ecf47710787ec81afd425336302338cee`) and a README. 572,109 cases across nine suites, exactly the v1 fixture's count. The generator's `--check` and the oracle's `verify-v2` (with `--bindings` and `--expect-generator`) pass on it; `verify-v1` on the v1 fixture still passes. The generated bytes are those of S3c except the generator version.

**Follow-ups, not decided.**

* **Float faults.** VER04p forbids `drift` and `rel-drift` in `accepted_blind_faults` (VER04i 4). Refusing an `accept_blind` line that names them, in both readers, is a **hard precondition of the slice that adds those faults**.
* **v1 inputs of the generator.** The generator's tests read the v1 `corpus.json` and `cases.tfv` (the 18 digests), which are not in its build-tool consumer row: a v1 edit reaches the generator's tests late (fail-late). Tracked; adding them changes the expected sets of existing selection tests.
* The lane runners (S6, S7) and the report validator (S9) read `accepted_blind_faults` as control ids, and surface an accepted control as specified in 8.3 (an accepted-blind control state).

**Still OPEN (owner), unchanged by this slice.** The shape of `assumes` (16.4 item 4); rule 2 of section 3.3 inside an example in a v0 document (16.7 item 9); a claim that calls an op outside that op's `admit` (16.4 item 5); the JSON of the `ir.fixed` data nodes (16.6 OPEN 2, an erratum for S10; *decided 2026-11-17, [D24], section 16.10*); whether a non-empty `accepted_blind_faults` blocks `release-ready` (8.3). *(Updated 2026-11-17 by section 16.9: all but `assumes` and the `ir.fixed` JSON are decided there.)* **Still OPEN, from 16.9:** whether a call whose op `admit` reads a subject-dependent argument stays statically refused (16.9 refinement 1). **Author recommendation: keep the static refusal, no exemption.** Section 7.6 has no dynamic `admit` refusal, so an exemption would leave such a call unchecked. If a real adopter needs one, add a per-claim, visibly recorded opt-in in `corpus.json` together with a new runner rule that fails the case on an out-of-admit call.

### 16.9 Owner decisions of 2026-11-17 (second batch)

Slice S3e (`rust/warrant-corpus-gen` 0.5.0, `python/warrant-oracle` 0.4.0) implements the second batch of owner decisions of 2026-11-17 and closes three OPEN items. The normative sections (3.3, 4.1, 4.5, 6, 8.3, 9.1, 13) were amended in the same change. Items under "Author refinements" are the author's reading where the decision is silent; they are erratum candidates, not rulings.

**[D16] Next slices: S10 (`ir.fixed`), then S11 (`ir.data`).** Recorded only, as a planning note in section 13. No numbering changes.

**[D17] An accepted-blind control blocks `release-ready`.** A corpus or lane with any accepted-blind control is not `release-ready`, exactly like a not-live suite or control. Sections 4.1 and 9.1 now define `release-ready` as zero not-live suites, zero not-live controls and zero accepted-blind controls; the OPEN item of section 8.3 (and of 16.8) is closed. **Spec only:** no generator or oracle check enforces it (`accepted_blind_faults` is already emitted and verified); the lanes and the report validator enforce it in S6, S7 and S9.

**[D18] v0 rule 2 applies to example evaluation.** In a v0 document an example whose arguments or pure side wrap (`f(255u8 + 2u8) == 1u8`) fails generation, with the code and the message of a claim's rule-2 refusal (`W0701: U8 arithmetic wraps; v0 harness evaluates U8 as Nat`), attributed to the example. A v1 document accepts the same example (the argument is `1u8`). Implemented in both implementations: the generator's `tfv::examples` counts the wraps of each example's arguments and pure side and calls `check_v0_wraps`; the oracle's `_examples` counts the wraps of the same two evaluations (the dedicated counting mode) and refuses in a v0 document. Tests in both, including the exact `255u8 + 2u8` example and the v1 acceptance (`tests/admit_calls.rs`, `tests/test_admit_calls.py`).

**[D19] A call outside the callee's `admit` is a generation error.** Section 4.5 states the rule. Implemented identically in both: the generator's `Program::trace` (with `Program::with_ops`) and the oracle's `compile_admit_check` walk the pure trace of each admitted case and evaluate each admit-bearing subject call's arguments against the callee's `admit` row; examples go through the same evaluation. The first offender is `W0701` naming the claim, the op and the admitted-case ordinal. **Evidence:** thirteen documents (the D18 and D19 cases of both test files) were generated by the Rust CLI and derived by the oracle: the seven refusals carry the same code and, where it applies, the same op and ordinal (case 20 for a weaker refinement and for a weaker refinement with a subject-dependent argument the `admit` does not read, case 2560 for an unrefined `U8` binder, case 2560 for a guard that takes the call one value too far), and the six accepted documents are accepted by both (the oracle's `verify-v2` accepts the generator's output for them). The oracle's gf256 tests (`power(add(a, b), 2u64)`) cross-check the narrowed static refusal. **Nothing changes in emitted bytes:** the ct-compare ops have no `admit`, so the committed fixture, the synth-v2 corpus and the accept-blind corpus differ from their previous versions only in `generator.version` (the D13 checklist was followed: only that field, the pins and the fixture README hash changed). **Cost:** none for a body that calls no op with an `admit` (ct-compare: 0.42 s before and after in release; synth-v2: 0.40 s; the generator's trace is skipped, the oracle's pass is not built); for a 65,536-case suite that calls an admit op, 0.15 s before and after in the generator and within noise (0.4 to 0.55 s) in the oracle's `verify-v2`.

*Author refinements.*

1. **An `admit` that reads a subject-dependent argument is refused.** The check needs the argument values; a nested call's result is unknown at generation. Refusing it statically (for every such call in the body, reached or not) is the conservative reading of "never a silent discard", and the same for both implementations. It rejects `op_lo(op_add(a, b), b)` when `op_lo`'s `admit` reads its first parameter; a document that needs it either restates the inner call as a pure model in the reference side or drops the op's refinement. A first draft refused every subject-dependent argument of an admit-bearing op; the oracle's own gf256 tests caught it (`power_frobenius_is_additive` and `power_cube_is_multiplicative` call `power(add(a, b), 2u64)`, whose `admit` reads only `exp`), so the rule was narrowed to the parameters the `admit` reads and gf256's S8 conversion is not blocked. **OPEN (owner):** whether the remaining refusal should instead be exempt. Section 7.6 has no dynamic `admit` check (a runner evaluates `ensures` rows only after calls that satisfy `admit`), so an exemption would let a corpus contain a call that nothing checks. Recommendation: keep the static refusal; if a real adopter needs the exemption, add a per-claim opt-in that is visibly recorded in `corpus.json` and a new runner rule that fails the case on an out-of-admit call. Also listed in section 16.8.
2. **The check covers the pure trace only.** A call in an untaken branch of a pure guard is not checked (it is not made), which is the same trace section 4.4 uses for `expected_calls`.
3. **Static widths.** The call's static widths are substituted into the op's parameter types before its `admit` is typed in the oracle; the generator's evaluator is untyped, and an `admit` cannot name a static width (its scope is the parameters), so the two agree.
4. **The check is IR work, and is metered.** Per suite, both implementations charge to the IR-work meter, before any case runs and on every product ordinal (conservatively: the check runs only on admitted cases), the sum over the body's `subject_op` nodes whose op has an `admit` of the worst-path cost of that `admit` plus the cost of each argument that does not call the subject (a static sum, a call in an untaken branch is counted). The generator charges it in pass 1 with the suite's `ir_cost`; the oracle charges it when the suite is planned. An over-budget document is refused with the code `limit` (`the total IR work of a run exceeds its bound`) in both. The **recorded `ir_cost` of the suite row does not change** (the check is not one of `admit`, `body` and `expected`), so no committed byte moves. The oracle's pass over the admitted cases is dedicated and not charged to its analysis-walk meter. Test: 128 calls of an op whose `admit` costs about 500 over 2^28 product ordinals is refused in both, while its recorded `ir_cost` alone is far inside the bound. **Consequence, recorded after the S3e review (2026-11-17):** a `slim` run can now return `limit` where it previously passed, roughly when product x admit-check cost exceeds the slim IR budget (`MAX_IR_WORK_SLIM`), because the D19 pass covers the **whole product at every depth** (section 4.5) while the slim run executes only the chosen chunks. It is intended (a hole at a case outside the slim set is still a defect of the document), and it is the reason the memoisation follow-up below matters.
5. **Examples** use the generator's and the oracle's own argument evaluation, outside the claim machinery.
6. **Wraps in callee-`admit` evaluation are refused in v0 and not recorded.** Section 3.3 rule 2 applies to every evaluation the generator performs, so a fixed-width wrap while evaluating a callee's `admit` for the D19 check (`op op_m(a: U8 | add8(a, 1u8) < 5u8, ...)` at `a == 255u8`), in a claim or in an example, is `W0701` in a v0 document, naming the claim (or example), the op and, for a claim, the admitted-case ordinal; a v1 document accepts it. Those wraps are **not** added to the suite's recorded `wraps` (they are not nodes of `admit`, `expected` or the pure trace), so no committed byte moves. Both implementations use the same message.
7. **Which call is named.** When two calls offend in the same first admitted case, the **leftmost** (outer call before its arguments, arguments left to right) is the one named, in both implementations.

*Follow-ups from the S3e review (2026-11-17), not decided and not implemented.*

* **Oracle `lower_op_admit` fail-closed parity.** The oracle's `lower_op_admit` should refuse an op `admit` that calls a subject op **even when no claim calls that op**, as the generator's front end does with `W0301` (a callee `admit` is pure and may never contain `subject_op`). Today the oracle notices only when a claim reaches the op, so the two tools can differ on a document with an unused op.
* **Oracle memoisation of callee-`admit` compilation.** The oracle should memoise the compilation of a callee `admit` per `(op, widths)`, so the D19 pass compiles each admit row once per distinct static-width tuple rather than once per call site.
* **Generator `--check` for the synth-v2 fixture.** `python/warrant-oracle/tests/fixtures/synth-v2` is at generator 0.5.0 since S3e and the oracle's `BUILD` pins it with `--expect-generator`, so the version is checked. What is still missing is a generator `--check` of its bytes: a generator change that would move them is not caught by the generator's build. It needs a generator `--check` line, and with it a build-tool row selecting `rust/warrant-corpus-gen` for the fixture directory (section 13, [D32]).

### 16.10 S10 owner decisions (2026-11-17)

Slice S10-0 (this erratum, spec text only; no code and no emitted byte changes) records the owner's decisions on the S10 plan (`ir.fixed`, now with `ir.tuple`). The normative sections (1.4, 4.1, 5.1, 8.2, 8.4, 13, 16.2, 16.6, 16.8, 16.9 and Appendix A.1) were amended in the same change; each new rule names the slice that implements it. **Owner decisions** are [D20] to [D35]; the author's readings where the decisions are silent are listed separately as **Author refinements** R1 to R10 and are erratum candidates, not rulings. [D16] (S10, then S11) is carried in section 13. A review of the first version of this section (2026-11-17) changed M1 (`omit_example`, [D34]) and M2 (`match` on `Result`, [D35]) by owner decision, and the text below is the amended one.

**[D20] Tuples are in S10, through a new IR level `ir.tuple`.** This goes against the plan's recommendation (tuples to S11). `ir.tuple` has the tuple type, tuple literal, `project`, tuple values, structural `eq`/`ne` and stream tag `64` (Appendix A.1, section 4.1), and, by [D35], `match` on `Result`. Implemented by S10t-1 (oracle) and S10t-2 (generator), before the flip (section 13); the lane side is S10e/f; vectors are S10d.

**[D21] The spec follows the generator for signed `fx_edge` (section 16.2 item 4).** The signed set is `wrap_S(E_U(w)) ∪ {-255..=-1}`, de-duplicated, ascending (`i16` 527, `i32` 559, `i64` 623); descriptor unchanged. The oracle's `leaf_fx_edge` changes to match; **no shared-crate PR**. Sections 5.1 and 16.2 amended. Implemented in S10c-1 (oracle); the generator already behaves so.

**[D22] `omit_example <id>` profile line and the `omitted_examples` key**, for lzw's six bomb examples; **restricted by [D34]**. Section 8.4 states the line and its error cases, section 1.4 the key. Implemented in S10-1 (oracle, then generator), before the lzw fixture.

**[D23] `Option<T>` is an IR-only intermediate, and `unwrap_or` is in `ir.fixed`.** The nine Option-returning built-ins (`try checked_add checked_sub checked_mul div rem div_trunc rem_trunc at`) produce it and `unwrap_or` consumes it; like `Int` (section 2.1) it never appears in an op or model signature, binder, example or stream. **Who may consume an `Option` (author refinement R8, which makes the decision exact): `unwrap_or` is the only consumer, and only as its first operand and directly**, i.e. the first operand of `unwrap_or` is a node of one of the nine producers. An `Option`-typed operand of any other node (`eq`, `ne`, `if` branches, a model argument or return, `ok`, `tuple`, ...) is refused with the named level `ir.data`; `some`/`none`, `Option` in a signature, binder, example or stream, and `match` on an `Option` stay `ir.data`. Both lowerers refuse each form (negative tests required). Implemented in S10a (oracle) and S10b (generator).

**[D24] The Appendix A.1 shapes and the corrected 35-name list.** One tag per source built-in, same spelling, positional operands, a static type argument as the canonical type name (a JSON string) in position 1. The `ir.fixed` row of section 4.1 lists exactly the 35 names (`try`, not `try_nat`; plus ten names the earlier list omitted). Section 8.2's two fault forms are replaced by the byte-exact shapes already emitted (the ct-compare fault row does not change). This settles section 16.6 OPEN 2. Implemented in S10a and S10b.

**[D25] `fx_edge` at 16 bits exists by demotion only; 8-bit is illegal.** Section 16.2 item 5; a consumer rejects `fx_edge` with `t` `U8` or `I8`. Implemented in S10c-1 and S10c-2.

**[D26] New leaf `nat_shift` (73 values); `nat_count` stays `0..=K`.** Section 5.1 and 16.2 item 3; chosen when a `Nat` binder is directly the count argument of `shl shr sar rotl rotr bit` (R10 for what does not select it). Implemented in S10c-1 and S10c-2.

**[D27] Faults inside `Ok(..)`.** The owner's decision was to emit them as deferred `ir.data` rows until S11. **[D35] supersedes the deferral:** their `match` on `Result` is `ir.tuple`, so the rows are live from the S10c flip (section 8.2 has their exact JSON), the registry gains `flip-result-bit` and `drop-last-result-byte` (the Rust bindings' names) and keeps `xor-lsb` and `truncate-last`. No deferred `ir.data` fault row exists after S10. Implemented in S10c-1, S10c-2 and S10t-1/2.

**[D28] wasm-leb128's two `W0701` claims are split.** `encode_*_at_most_ten_bytes` is `len(f(v)) >= 1 and len(f(v)) <= 10`, a subject call under an impure guard, `W0701` forever under section 4.4 (which stays as written, so call counts stay implementation-independent). Each is split into two claims in `wasm-leb128.warrant` (`len(f(v)) >= 1` and `len(f(v)) <= 10`), Rust evidence regenerated, 42 to 44 claims. A change to a frozen adopter spec and its evidence, **made in the wasm-leb128 adopter PR, which is now an S10 PR (section 13), not an S11 one** [D35].

**[D29] An atomic flip PR is the exception to one-kind-per-PR.** The S10c flip changes the generator's and the oracle's fault-evaluation gates, bumps their versions and regenerates the ct-compare fixture in one PR (section 13), because the generator's `--check` and the oracle's `verify-v2` both read the fixture; there is no transitional oracle flag. Everything else lands and is tested behind a library parameter beforehand.

**[D30] Proceed now with the generator, the oracle and x25519; the runners queue behind S5 to S7.** S10 produces no lane evidence until S5, S6 and S7 land (section 13).

**[D31] `wraps` counts only `add`, `sub`, `mul` and `neg`.** `wrap`, `shl`, `bit_not` and every other `ir.fixed` built-in are explicit truncations and are never counted (section 3.3 rule 1 is unchanged). Implemented in S10a and S10b.

**[D32] Build-tool rows are table-driven.** `code/programs/go/build-tool/repository_inputs.go` replaces the hard-coded ct-compare rows (`warrantCorpusV2Fixture`, `warrantCorpusV2Bindings`) with a `{module, spec, bindings}` table, one entry per adopter, with the existing selection test extended; it lands **with the x25519 fixture PR**. The synth-v2 `--check` follow-up (section 16.9) needs a row in the same table.

**[D33] If full-depth oracle verification of lzw does not fit in one CI job, shard it by chunk** (section 10.4). The measurement is made in S10c-1, before the lzw fixture PR; a miss is a blocker to report, never a reason to merge unverified.

**[D34] `omit_example` is restricted, not blocking (amends [D22]).** An `omit_example <id>` line is a profile error unless the example is inexpressible for a reason **the tool can decide**; the closed vocabulary is `value-over-limit` (a value over `MAX_IR_VALUE` or the section 2.3 limit; the lzw bombs) and `not-op-eq-pure` (a shape other than `op(args) == pure`). A D18 or D19 refusal, and any other check failure, does **not** qualify and still refuses generation. The corpus records `omitted_examples: [{"id", "reason"}]` sorted by id; the tool computes the reason and the profile line carries only the id; an empty list is rejected. The error cases follow `accept_blind`: a line for an emittable example is an error (an omission that excuses nothing is a stale profile), as are unknown ids, duplicates and malformed lines. An omitted example **does not block `release-ready`**; reports show the count beside `not_live` and S9 prints the ids (this settles the OPEN of R4 of the first version). Sections 1.4, 4.1 and 8.4; implemented in S10-1 (the reason and the line), and in the lane runners and S9 (the reporting).

**[D35] `ir.tuple` includes `match` on `Result` (the plan's Q1 option A).** `ir.tuple` = tuple types and literals, `project`, tuple values, structural `eq`/`ne`, **and `match` on `Result`**. `match` on a scrutinee of any other type (a bare tuple, a variant, a record, an `Option`) stays `ir.data` (the plan has no other `match` scrutinee in S10: its section 1.5 puts only `Result` and tuple patterns in the tuple level and its section 1.1 keeps `Some`/`None`/`match` on `Option` in `ir.data`). **wasm-leb128 therefore becomes an S10 adopter**, after x25519 and lzw; its claim split [D28] is made in that PR; the S10t rows of section 13 carry `match` lowering and evaluation in both tools. Consequences: the composite faults `ok-as-err`, `drop-consumed` and `swap-components` are `ir.tuple` (`wrong-ctor` and `field-zero` stay `ir.data`, section 8.2), and the faults inside `Ok` are live ([D27]). The plan's census note that wasm-leb128 "reaches 40 of 42 and still needs Q9 and the composite faults" applies, with the claim split done in the adopter PR: the rest is what the adopter PR must show (the census is a regex estimate and is pinned in S10b).

**Author refinements (not owner decisions).**

* **R1. Where `ir.tuple` sits (the owner left this to the author).** `ir.core` is the base; `ir.fixed` and `ir.tuple` are **siblings** (each depends on `ir.core` only); `ir.data` implies `ir.tuple` and **does not** imply `ir.fixed`. Rationale: the plan's level structure makes `ir.fixed` a set of built-ins over scalar fixed-width types with `Option` as an intermediate, none of which mentions a tuple, while a tuple's components may be `Bool`, `Bytes` or `Nat`, so neither level needs the other; the adopters that need `ir.fixed` alone (x25519's claims, lzw) must not be forced to implement tuples; wasm-leb128, logic-gates and hash-functions need `ir.fixed` and `ir.tuple` (and, for logic-gates, `ir.data`), which is why `ir.data` implies `ir.tuple`. A list `at` returns `Option<U8>` (`ir.data`) and `unwrap_or` is `ir.fixed`, so that idiom requires both levels, through the computed `requires`; `ir.data` still does not imply `ir.fixed`. Section 4.1 states it. **Who enforces the declared-levels implication:** the lane runners (S6 and S7, via S10e/f) and the S9 validator, never the generator or the oracle, which do not see a lane's declared levels.
* **R2. Tuple and `match` shapes.** Taken from the plan, section 1.5, not the owner: type name `(U64,Nat)` (no spaces, 2 to 8 components, section 2.1), literal `["tuple", [e1, .., en]]`, `["project", e, i]` with `i` a 0-based JSON integer, value a JSON array, stream tag `64` with a u32-LE count, TFV `L<n> v1..vn`, equality component-wise, binder domain `{"d":"tuple","of":[..]}` (section 5.1 already), cost `1 + sum of operands` (section 4.2 rule 6); `match` as in Appendix A.1. **Provisional until S10t-1 reads them**, as section 1.4 was.
* **R3. `missing-error` with a tuple payload (re-decided under [D35]).** It is a `replace` fault at **`ir.tuple`** (it needs the tuple literal and `is_err`, not `match`): `if is_err(result) then Ok(["tuple", [c1, .., cn]]) else result`, where `ci` is the first parameter, in parameter order, of exactly the type of the `i`-th component if one exists, and otherwise the type's zero (`0` of that type, `false`, or `Bytes` of `N` zero bytes, `b""` for `Bytes`). It is the tuple analogue of "`Ok(first Bytes arg)`" (section 8.2) and is **provisional**: the wasm-leb128 adopter PR pins it against that adopter's bindings.
* **R4. `omit_example` details (the author's reading where [D22] and [D34] are silent).** The precedence between the two reasons (`not-op-eq-pure` first); that an omitted example is neither evaluated nor stream-encoded nor checked against the op `admit`; the message rule (an id, never a value); and that `omitted_examples` is a **conditional** key (written only when non-empty) so that no committed fixture moves a byte and the oracle-first/generator-second order needs no atomic PR. The test for `value-over-limit` is the size check of section 4.2 rule 6 applied to the example's own argument and pure expressions.
* **R5. Versions of the flip.** The plan's 0.5.0 and 0.4.0 were computed from S3d; S3e already used them. The flip bumps each crate by one minor version (section 13), and follows the ct-compare fixture README's checklist rather than a count of places.
* **R6. The exact fault forms and the Rust harness semantics.** Section 8.2 fixes the JSON; the empty-result values `01` and `00`, and the typing of the result of the `concat`/`take` on a `Bytes<N>` result as dynamic `Bytes`, are the author's reading of "Rust harness semantics" in the plan, which cites no code, so the x25519 adopter PR must confirm them against the bindings before its fixture is committed; the oracle's committed dump of S10c-1 is the reference if the two differ (erratum then).
* **R7. `div`/`rem` typing is the grammar checker's** (Appendix A.1, normative). The first version left `Nat`/`Nat` open; the checker answers it (`Option<Int>`).
* **R8. The exact consumer rule of `Option`.** Part of [D23] above. A fault expression at `ir.fixed` may use the same `unwrap_or(at(..), d)` form.
* **R9. Computed `requires`.** A suite's, op's or fault's `requires` is the sorted union of the levels of every tag in its expressions and in the call closure of every `model_call` (plan section 1.3; a model has no `requires` of its own, section 1.4), **plus `ir.tuple` whenever the static type of any sub-expression, binder, op or model parameter or result, or example value **is or contains a tuple type**. The rule is on the **type**, not on a tag: a suite that calls an op returning a tuple, or a `Result<(U64,Nat),E>`, and compares it with `eq`, with no tuple tag and no tuple binder, requires `ir.tuple`, because a consumer that does not implement tuples cannot hold the value. Implemented as `ir::levels` (generator) and `ir_levels` (oracle) in S10a and S10b (the tag part) and S10t-1/2 (the type part); today both emit a constant. For a fault row, `requires` is the sorted union of the levels of the fault template's own tags and the levels of the op's parameter and result types; `ir.core` appears only when the template itself uses a core tag (so `drop-consumed` is `["ir.tuple"]`, `flip-result-bit` under `Ok` is `["ir.fixed","ir.tuple"]`, `invert` on an op with a tuple parameter is `["ir.core","ir.tuple"]`). The S10t-1 committed faults dump pins the bytes (author refinement, S10t-1 review).
* **R10. What selects `nat_shift`.** [D26] says a `Nat` binder that is directly a count argument. The author's reading: only the count operand itself selects it; a count that is an expression of the binder, one passed through a `model_call` argument, and a binder that reaches a count only in an `admit` do not (section 5.1). This keeps the rule decidable from the claim body without inlining models.

* **R11. Composite and tuple faults have no exact JSON here.** For `ok-as-err`, `drop-consumed`, `swap-components` and the tuple-payload `missing-error` (R3), **the committed dump of S10t-1 is normative** and these rows are provisional until then, exactly as R6 says for the faults of section 8.2 whose JSON is not fixed by that section.

* **R12. Ordering rules for parity.** When a control needs two or more levels the tool does not implement, its `deferred` witness names the first of them in sorted order, and the control row's `requires` carries the full list. Among `match` corpus errors, unreachable-arm errors are reported (in arm order) before fall-through. Both tools must follow both rules so that refusal messages and deferred rows agree (author refinement, S10t-1 review).

**Items of sections 16.2 and 16.6 settled by this section.** 16.2 items 3, 4 and 5 ([D26], [D21], [D25]); 16.6 OPEN 2 ([D24]). Still open and unchanged: `assumes` (16.4 item 4; wasm-leb128 has `none` rows only), whether the static refusal of an op `admit` that reads a subject-dependent argument stays (16.9 refinement 1), lists of byte strings (16.2 items 1 and 2, S11).

**Findings in the S10 plan that contradict the decisions or each other, resolved here.** (1) The plan's sections 1.1, 1.5, Q1 and the census note assumed tuples (and wasm-leb128) in S11; superseded by [D20] and [D35]. (2) The plan's S10-0 row says "VER04r 16.9"; 16.9 is already the S3e batch, so this erratum is 16.10. (3) The plan's flip versions (0.5.0, 0.4.0) were already taken by S3e (R5). (4) The plan said "no version bump until the flip" and also scheduled `omitted_examples`; resolved by making the key conditional (R4). (5) The plan counts 35 names while describing the grammar as 34 plus `unwrap_or`; both are right. (6) The plan's `div`/`rem` row leaves `Nat`/`Nat` typed ambiguously; the checker settles it (R7). (7) `flip-result-bit` on a fixed-width result and the older `xor-lsb` are the same expression; both stay in the registry because bindings use one and not the other. (8) The plan's `at` and list forms: `at` is listed in `ir.fixed` for `Bytes` and in `ir.data` for lists (section 4.1 did the same); kept.


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


### A.1 `ir.fixed` and `ir.tuple` tags [D24, D20; implemented in S10a to S10c-2 and S10t-1, S10t-2]

One tag per source built-in, same spelling, positional operands; a static type argument is the canonical type name as a JSON string in position 1. The shapes of the two fault rows are kept byte for byte (`["bit_and", ["var", p], ["lit", {"type":"U64","value":"9223372036854775807"}]]`). `T` is a fixed-width type of width `w`, `S` a signed one, `U` an unsigned one.

| Tag | JSON form | Operand types | Result | Semantics (VER04j 2.3, 3.2, 3.3) |
|---|---|---|---|---|
| `bit_and bit_or bit_xor` | `[tag, a, b]` | `T, T` | `T` | per bit of the `w`-bit two's-complement pattern, re-read in `T` |
| `bit_not` | `["bit_not", a]` | `T` | `T` | flip `w` bits (`bit_not(0i8) = -1i8`) |
| `shl` | `["shl", a, n]` | `T, Nat` | `T` | zero fill; `n >= w` gives 0 |
| `shr` | `["shr", a, n]` | `T, Nat` | `T` | **logical** on the pattern, then re-read (`shr(-1i8,1) = 127i8`); `n >= w` gives 0 |
| `sar` | `["sar", a, n]` | `S, Nat` | `S` | sign fill; `n >= w` gives `-1` if `a < 0` else `0` |
| `rotl rotr` | `[tag, a, n]` | `T, Nat` | `T` | rotate by `n mod w` |
| `popcount clz ctz` | `[tag, a]` | `T` | `Nat` | on the `w`-bit pattern; `clz`/`ctz` of 0 is `w` |
| `bit` | `["bit", a, i]` | `T, Nat` | `Bool` | bit `i`, LSB 0; `false` for `i >= w` |
| `widen` | `["widen", "U32", e]` | fixed, range inside the target | target | value preserved |
| `wrap` | `["wrap", "I8", e]` | fixed, `Int` or `Nat` | target | value mod `2^w`, re-read in the target |
| `try` | `["try", "U8", e]` | fixed, `Int` or `Nat` | `Option<target>` | `Some` iff in range |
| `to_int` `to_nat` | `[tag, e]` | fixed / `U` | `Int` / `Nat` | mathematical value |
| `max_value min_value` | `[tag, "I64"]` | none | the type | not folded to `lit` (section 10, unoptimised lowering) |
| `checked_add checked_sub checked_mul` | `[tag, a, b]` | `T, T` | `Option<T>` | `None` iff the exact result leaves `T` |
| `div rem` | `[tag, a, b]` | `U, U` or `Int`/`Nat` | `Option<U>` / `Option<Int>` | `None` iff `b == 0`; `Int`/`Nat`: floor, `rem` has the divisor's sign. **Signed fixed-width operands are refused** (use `div_trunc`/`rem_trunc`) and two `Nat` operands give `Option<Int>` |
| `div_trunc rem_trunc` | `[tag, a, b]` | `S, S` | `Option<S>` | toward zero; `None` iff `b == 0` or (`a == MIN` and `b == -1`) |
| `imin imax` | `[tag, a, b]` | `Int`/`Nat` | `Int` | exact |
| `be_bytes le_bytes` | `[tag, e]` | `T` | `Bytes<w/8>` | two's-complement bytes |
| `from_be from_le` | `[tag, "U32", e]` | `Bytes<w/8>` | target | a runtime length other than `w/8` is a corpus error, never a truncation |
| `at` | `["at", b, i]` | `Bytes`/`Bytes<N>`, `Nat` or `Int` | `Option<U8>` | `None` for `i < 0` or `i >= len` (the list form is `ir.data`) |
| `unwrap_or` | `["unwrap_or", o, d]` | `Option<T>, T` | `T` | both operands evaluated (no short circuit; cost is the sum) |

Notes. Shift counts are exact `Nat` (bounded only by the `2^127` window): an evaluator compares `n >= w` **before** any host shift, and `rotl`/`rotr` reduce `n mod w` first. `wraps` counts `add sub mul neg` only [D31]. Every tag costs `1 +` the sum of its operands (section 4.2 rule 6); the `ceil(w/8)` work units of VER04j 3.5 belong to the Rust harness meter, not to `ir_cost`. Typing is inferred, as for `add`: a consumer types operands from `lit`, `var` and model signatures, and a mismatch (mixed widths, `sar` on an unsigned type, a count that is not `Nat`) is a corpus error. **`div`/`rem` typing is normative [D24, R7]**: it is the typing of the grammar checker, `code/packages/rust/warrant-grammar/src/checker.rs`, the `"div" | "rem"` arm (line 3231 in the tree this erratum is based on; `Option<Int>` for `Int`/`Nat` operands, `Option<U>` for matching unsigned fixed operands, a signed pair refused). **`Option<T>` is an intermediate type only [D23]: its single consumer is `unwrap_or`, as the first operand and directly** (section 16.10 D23). **The static width of a `from_be`/`from_le` operand** (the `Bytes<w/8>` it must have) is computed by the grammar checker's rule, `checker.rs` (`Literal::Bytes`, the `concat` arm and `Type::common_type`): a bytes literal is `Bytes<len>`, a `concat` of two statically sized operands is `Bytes<sum>`, an `if` whose two branches have the same static width keeps it (otherwise the branches unify to plain `Bytes`), and `take`/`drop` give plain `Bytes`. A consumer must implement the same rule (S10b included), so that `from_le<U32>(concat(t, b"000000"))` with `t: Bytes<1>` is accepted by every tool and a plain `Bytes` operand is refused by every tool. Fixed-width types and literals, `+ - *`, unary `-` and comparisons on them are `ir.core` (section 3.2) and unchanged.

`ir.tuple` (provisional until S10t-1 reads it, section 16.10 R2):

| Tag | JSON form | Result |
|---|---|---|
| tuple literal | `["tuple", [e1, .., en]]`, 2 to 8 operands | `(T1,..,Tn)` |
| `project` | `["project", e, i]`, `i` a 0-based JSON integer | `Ti` |

`ir.tuple` `match` on `Result` (the plan's shape, section 16.10 R2; provisional until S10t-1): `["match", s, [[pat, arm], ..]]` with `s` of type `Result<X,E>` and patterns `["ok", pat]`, `["err", "CASE"]`, `["err_any"]`, `["tuple", [pats]]`, `["bind", name]`, `["wild"]`. The **patterns** are tried in order and **exactly one arm body is evaluated**, the arm of the first pattern that matches (section 4.2 rule 2). Author refinements (section 16.10 R2; the plan is silent), each a corpus error with the generator code `ir` (oracle `corpus`), refused by both tools in S10t-1/2: a `match` that can fall through (not exhaustive over `ok`/`err` cases); **an arm that no value can reach** (every value it matches is matched by an earlier arm, decided by the usual usefulness check over the patterns); the **same `bind` name twice in one pattern**; and a `bind` name that **shadows** a binder, a parameter, `result` or an outer `bind` (there is no shadowing); arms must have one type. Cost: `1 +` cost(scrutinee) `+` the maximum over the arms (section 4.2 rule 6). A `match` whose scrutinee is a bare tuple, variant, record or `Option` is `ir.data`; the same JSON, a different level (the scrutinee's static type decides). All arm bodies of a `match` must have one type; a mismatch is a typing error (code `type` at source, `ir` in the IR typer), like mismatched `if` branches, not a `corpus` error (author refinement, S10t-1 review).
