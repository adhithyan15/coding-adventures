# VER04l — Warrant Text (WX-T) and Alphabet Domains (WX-AD)

**Status:** Specification only. Nothing here is implemented. It is the normative
text for backlog item WX-T0 and for the shared alphabet-domain built-in `within`
(the "WX-I" of the design memo, registered under the unique id **WX-AD**: the id
WX-I already names recursive sums in VER-BACKLOG and VER27). The implementation
slice is WX-T1 (section 12), which the owner has ordered after WX-G, WX-D2, WX-F,
LP-012 and the stateful slice S1 of VER04m (section 12.1).

**Parent:** [VER04 §2–§8](VER04-warrant-language.md),
[VER04d](VER04d-warrant-type-system.md),
[VER04h](VER04h-warrant-derived-evidence.md) (domains, budget, controls),
[VER04i](VER04i-polyglot-warrant-evidence.md) (corpus encoding, bindings, lane
runners), [VER04j](VER04j-warrant-v1-extensions.md) (header, fixed-width types,
records, WX-E, WX-H), [VER04k](VER04k-warrant-v1-wave4.md) (WX-D sequence
combinators, WX-F drivers, WX-G `inout`),
[VER08b](VER08b-runtime-capability-conformance.md) (the jail),
[TF01 §2](TF01-bootstrap-harness.md) (the TFV `S` value), and
[VER-BACKLOG](VER-BACKLOG.md) LP-012 (honest case-count kinds).

**Section references.** As in VER04k, where this document needs a corpus or
binding rule it states a requirement on VER04i; it does not edit VER04i, VER04j,
VER04k or VER29. A sibling specification, VER04m (stateful objects), is written
in parallel; the two share only `within` (section 4) and the order of section
12.1, and neither edits the other.

## 0. Why this exists

Warrant v0 and v1 have no text. A package whose contract is about text
(`caesar-cipher`, `url-parser`, the text-bearing result fields of `http1`) has to
be described as bytes, which hides exactly the bugs that matter: a decoder that
accepts a surrogate, a lossy conversion at a language boundary, an observer that
compares with canonical equivalence, a length that counts UTF-16 units. The
inventory counts 133 crates blocked on text by their signatures alone.

This document adds one value type and one conversion discipline, not a text
library:

| Added | Where |
|---|---|
| the types `Text` and `Char`, the literals `t"..."` and `c"."` | 3, 5 |
| six new built-ins `utf8 from_utf8 chars text code char_of`, `concat` extended to `Text`, and `within` | 3.5, 4 |
| the rule that a bare `len` of `Text` is a type error | 3.4 |
| a per-lane, per-parameter declaration for host text that is not a `Text` | 3.7 |
| the domains `char_full`, `char_edge`, `text_d1`, `text_over(A, K)`, labelled by LP-012 kind | 4, 6 |
| the corpus encoding (TFV `S<hex>`, two stream tags), mandatory boundary rows, an ill-formed probe set | 7 |
| five text faults, the proof status (`not-live` at first) and the bound rules | 9 |
| a per-lane hazard table for fourteen language families | 10 |
| the header `v2`, which gates all of the above | 2 |

Everything else about text is a model over `chars(t)` or `utf8(t)` using WX-D.

## 1. Decisions of record (owner, 2026-11-14)

| # | Decision | Where applied |
|---|---|---|
| a | Text is a sequence of Unicode scalar values. No normalisation, no case mapping, no graphemes, no locale. Normalisation and case mapping may come later as explicit, version-pinned functions (never as silent behaviour of `==`). | 3.1, 3.3, 3.6 |
| b | A bare `len(t)` on `Text` is a type error (W0301). The unit is always written: `len(utf8(t))` or `len(chars(t))`. | 3.4 |
| c | Ill-formed host text uses a per-lane declared policy `unreachable \| rejects \| unspecified`. `unspecified` is allowed, but the assurance report always prints it as an open exemption. | 3.7, 9.4 |
| d | Every other recommendation of the design memo is adopted as the default: the header `v2`, starting with WX-T; `within` alphabet domains labelled `bounded-set` and never `exhaustive` for now; the corpus encoding of section 7; a phase-A text warm-up per lane in the jail; proofs `not-live` initially. | 2, 4.4, 7, 11, 9.5 |
| e | Order: text implementation (T1) comes after WX-G, WX-D2, WX-F, LP-012 and stateful S1. | 12.1 |

These are not open questions; section 15 lists what remains open.

## 2. Versioning and compatibility

### 2.1 Header

A document is a **v2 document** exactly when its header is `warrant name v2`.
`v2` means: everything in v1 (VER04j §1.1, VER04k) plus the elements of this
document. The checker today refuses any second NAME other than `v1` (VER04j
§1.1); the T1 slice changes that to accept `v1` and `v2`. A v2 construct in a v0
or v1 document is W0101 ("v2 construct in a v1 document"), never ignored. A
document that is v1 today keeps its header and its meaning.

Why a new header and not "v1, evolving": the earlier waves kept the rule "no new
reserved words and no new tokens" (VER04j §1.2, VER04k §1.2) so that every
existing document regenerates byte-identically. Text is the first wave that
cannot keep it (a new type name, a literal prefix, reserved function names).
A header makes the byte-identity gate a one-line condition. Other specifications
may add to `v2` through their own change records; this one defines only its own
elements.

### 2.2 Every new surface element and its treatment

| Element | Kind | In a v0 or v1 document | In a v2 document |
|---|---|---|---|
| `Text`, `Char` | type names | ordinary identifiers (an error type or alias named `Text` keeps meaning itself) | reserved: a declaration, parameter or binder of that name is W0201/W0203 |
| `utf8 from_utf8 chars text code char_of within` | built-in function names | ordinary identifiers; an undeclared call is W0202 as for any other gated name | reserved, all seven, from T1-S1 on (the break happens once, as in VER04k S0 E7) |
| `concat` on `Text` | extended built-in | `concat` keeps its v1 meaning on `List` and `Bytes`; on `Text` it is W0301 | also accepts `(Text, Text) -> Text` |
| `t"..."`, `c"."` | literals | **W0101** ("text literal in a document older than v2"); `b"..."` is unaffected | literals (section 5.1) |
| `ill_formed=`, `ascii_only=`, `proofs_not_live=text` | binding schema keys | unreachable (no `Text` type can exist) | required or allowed as section 3.7 and 9 say |

Adding reserved names is a breaking change only for a document that opts in by
changing its header to `v2`. The in-repository v1 documents are checked
mechanically at T1-S0 for any model, op, binder or constructor named like one of
the nine reserved names (a v1 model called `text` or `code` must be renamed
before its header moves to v2).

### 2.3 What "means the same" is tested by

1. Every v0 and v1 byte-identity test of VER04j §1.3 and VER04k §1.3 still passes.
2. Every new prelude part (the `Text` value conversion, the text domains, the
   text faults, the probe suites) is emitted only for a document that uses text,
   like the `comp`/`obs` parts of VER04j §14.
3. The checker's `Debug` output for the checked-in v0 documents is unchanged; no
   slice adds a field to `CheckedWarrantModule` (the WX-A lesson).
4. The ground evaluator's v0 and v1 behaviour is unchanged; the text forms are
   keyed on the document version.

### 2.4 Diagnostics

No new code is allocated. Existing codes are reused so that rejections stay in
classes the tooling already tests:

| ID | Uses in this document |
|---|---|
| W0101 | a text or char literal in a pre-v2 document; a bad escape; a surrogate or out-of-range `\u{...}`; a raw character outside the allowed raw ranges (5.1); a `Char` literal that is not exactly one scalar; a v2 construct in a v1 document |
| W0201, W0203 | a v2 declaration or binder named like a reserved name |
| W0202 | one of the seven built-ins called in a pre-v2 document that does not define it |
| W0301 | bare `len` of `Text`; `<` on `Text` or `Char`; `Text` where `Bytes` is required and the reverse; a duplicate member in a `within` alphabet; a `within` whose operand types disagree; a non-ground alphabet or non-literal `K` where the domain rule needs one |
| W0501 | a malformed `ill_formed=`, `ascii_only=` or probe entry in a binding file |
| W0601 | a `Result`/`Option` arm with a non-trivial body never taken in the exhaustive suite (14.2) |
| W0602 | a text literal above 4,096 scalars; a `within` alphabet above 256 members; the caps of 15 |
| W0701 | a text fault that does not apply to the operation's shape; a lane with no strict text conversion (the target is `not-live`) |
| W0703 | ground-evaluator exhaustion on a text value or on `chars(t)` |
| W0704 | a host form that cannot carry `Char` (a UTF-16 unit, a signed 8-bit `char`) or `Text` |
| W0706 | a `within` domain, a `char_full` demotion or a product over the exhaustive budget |

## 3. Value model and semantics

### 3.1 Types

`Char` is one Unicode scalar value: an integer in `0..=0xD7FF` or
`0xE000..=0x10FFFF`, 1,112,064 values. `Text` is a finite sequence of `Char`.
There are no other text types, no encoding tag, no "invalid" member, and no
locale. (Decision a.)

Why scalar sequences and not bytes plus a validity predicate, UTF-16 units or
graphemes:

| Candidate | Problem |
|---|---|
| `Bytes` with `is_utf8(b)` as a refinement | every Rust `&str` contract would carry a refinement the host type already guarantees; the generator would filter random bytes, which are almost never valid past ASCII; non-byte lanes (JavaScript, Java, C#, Python) would treat a string as bytes, which is the per-lane drift VER04i exists to prevent |
| UTF-16 code units | not what Rust, Go, C, Python or Haskell hold; admits lone surrogates as values |
| grapheme clusters | depend on Unicode version tables; not a fixed mathematical object |
| **scalar sequence** | the largest set every lane can represent exactly; a fixed mathematical object; in bijection with well-formed UTF-8 |

### 3.2 Equality

Equality of `Text` is sequence equality of scalars. It coincides with byte
equality of the UTF-8 encodings (`utf8` is injective, 3.5). It is **not**
canonical equivalence: `t"\u{E9}"` and `t"e\u{301}"` are different values. A lane
whose native `==` is not scalar equality (Swift) must compare through
`Array(s.utf8)` (10). Equality of `Char` is equality of scalars. Both are
structural inside `Option`, `Result`, records and lists, as everywhere (VER04 §3).

### 3.3 No normalisation, no case mapping

Warrant never normalises, case-folds, trims or re-encodes. A package whose
contract is "NFC-normalises its input" states that with a WX-E table of known
answers (VER04j §5), not with a built-in. `to_upper`, `to_lower`, case folding,
normalisation forms, grapheme segmentation, collation, width and locale
behaviour are **deliberately absent**: each depends on a Unicode version or the
locale, and Rust's `char::to_lowercase`, Python's `str.lower` and JavaScript's
`toLowerCase` do not agree across versions. A contract that needs them is either
restricted to ASCII by a refinement or pinned by a WX-E table that names the
Unicode version. Normalisation and case mapping may later arrive as explicit
functions whose names carry a Unicode version; that is a future decision with its
own header, not an extension of this one.

### 3.4 Ordering and length units

**Ordering.** `Char` is compared through `code`: `code(a) < code(b)`. There is no
`<` on `Char` or `Text` in T1 (W0301), as `Bytes` has none. The canonical order is
recorded now so a later slice cannot choose differently: `Text` is ordered
lexicographically by scalar value, which equals the bytewise order of the UTF-8
encodings and **differs from UTF-16 code-unit order** (in UTF-16, U+FFFF sorts
after U+10000 because 0xFFFF > 0xD800). A lane that sorts with a native
comparison (Java `compareTo`, JavaScript `<`, C# ordinal comparison) is therefore
wrong for this order, which is what the boundary row of 7.3 detects. (Decision
recorded as open question T-6 of the memo, resolved: not in T1, definition fixed.)

**Length.** `len(t)` with `t: Text` is **W0301** with the message "length of
Text needs a unit: write len(utf8(t)) or len(chars(t))". (Decision b.) The host
notions differ (bytes in Rust, Go and C; UTF-16 units in JavaScript, Java and C#;
graphemes in Swift and Elixir) and a bare `len` would silently pick one.

| Written | Type | Meaning |
|---|---|---|
| `len(utf8(t))` | `Nat` | number of UTF-8 bytes |
| `len(chars(t))` | `Nat` | number of scalars |

For `labels` rows, `t.length` means the UTF-8 byte length (what a timing channel
leaks). For WX-H `output` rows a `Text` result is measured in UTF-8 bytes, and a
`bound` or `output` expression may mention a `Text` parameter only through `utf8`
or `chars`. The capped allocation observer measures bytes.

### 3.5 Built-ins

All are total, v2-gated and pure.

| Form | Type | Meaning |
|---|---|---|
| `utf8(t)` | `Text -> Bytes` | the UTF-8 encoding; injective |
| `from_utf8(b)` | `Bytes -> Option<Text>` | strict decode: `None` on overlong forms, surrogates, values above U+10FFFF, truncated sequences and stray continuation bytes. This is the strictness TF01 already requires of the TFV `S` value; NUL and the noncharacters U+FFFE and U+FFFF are valid |
| `chars(t)` | `Text -> List<Char>` | the scalars in order |
| `text(cs)` | `List<Char> -> Text` | inverse of `chars` |
| `code(c)` | `Char -> U32` | the scalar value |
| `char_of(u)` | `U32 -> Option<Char>` | `None` for surrogates and values above U+10FFFF |
| `concat(a, b)` | `(Text, Text) -> Text` | concatenation; the existing `concat` extended |
| `within(xs, a, k)` | section 4 | alphabet predicate |

Laws the three-way differential tests (9.1) check: `from_utf8(utf8(t)) == Some(t)`;
`text(chars(t)) == t`; `chars(text(cs)) == cs`; `char_of(code(c)) == Some(c)`;
`len(utf8(concat(a, b))) == len(utf8(a)) + len(utf8(b))`.

`take`, `drop`, `slice`, `map`, `all`, `fold` and `count` (WX-D) apply to
`chars(t)`; no text-specific sequence library is added. ASCII classification is a
three-line model, for example `code(c) >= 65u32 and code(c) <= 90u32`. A `Text`
value counts four times its scalar count against the sequence-element cap of the
ground evaluator and its UTF-8 length against the 64 MiB data budget (VER04k
2.7); `chars(t)` is therefore subject to W0703 like any list.

### 3.6 Boundaries of the model

Not provided, on purpose: indexing by scalar, substring search, trimming,
splitting, formatting, parsing numbers. They are models (WX-D2) or they are the
subject under test. The model language stays first-order, total and
non-recursive (VER04 §4).

### 3.7 How ill-formed host text crosses the boundary

The Warrant value is never ill-formed. What varies is whether the *host parameter
type* can hold something that is not a `Text`.

| Lane type | Can hold a non-`Text`? | What |
|---|---|---|
| Rust `&str`, `String`, `char`; Swift `String`; Haskell `Data.Text` | no | unreachable by type |
| Python `str`; Haskell `String` | yes | lone surrogate code points |
| JavaScript/TypeScript, Java, Kotlin, C#, F#, Dart | yes | lone surrogate UTF-16 units |
| Go `string`, C and C++ byte strings, Lua, Perl byte strings, Ruby, Elixir binaries | yes | any invalid UTF-8 |
| C `const char*` (NUL-terminated) | yes, and **cannot hold U+0000** | truncation at NUL |

**Rule.** For every `Text` and `Char` parameter of an operation the lane binding
(VER04i §5) declares `ill_formed`, one of:

* `unreachable`: only where the host type guarantees it. Counted as
  `unreachable_by_type` (VER04j §4.6). A lane may not declare `unreachable` for a
  host type that can hold an ill-formed value; the runner checks the declared
  host type against the table above (W0501).
* `rejects:<CASE>`: the lane runner feeds the fixed **ill-formed probe set** of
  7.4 through the lane's lossless host-injection path and requires `Err(E.CASE)`
  for each probe. `<CASE>` must be a case of the operation's error type; the
  operation's result must be a `Result` (W0501 otherwise). This is lane-native
  evidence: the probes are not Warrant values and are not in the stream digests.
* `unspecified`: allowed. The lane's report prints an **open exemption** for it
  (9.4). It is never silent and never counted as a pass. (Decision c.)

**Results.** An adapter converts a host string to `Text` **strictly**. A host
result that is not well-formed fails the instance with the note
`host-text-ill-formed`. A lossy conversion in an adapter (`from_utf8_lossy`,
`TextEncoder`, `errors='replace'`, Java `getBytes`) is a binding defect and is
what the `lossy-utf8` fault models (9.2).

**Bytes in, text out.** A package whose public input really is bytes
(`http1::parse_request_head(&[u8])`) keeps a `Bytes` parameter, and its contract
states the text condition itself, for example
`rejects from_utf8(slice(...)) == None with E.INVALID_HEAD_ENCODING`. Only its
result fields are `Text`.

**`Char` is a scalar, never a UTF-16 unit.** A lane maps `Char` to a type that
can hold 0x10FFFF and checks it (Rust `char`, Python one-code-point `str`, Java
`int` code point, C# `Rune`, Swift `Unicode.Scalar`, Go `rune` checked against
the scalar set). Mapping `Char` to Java/Kotlin/C#/Dart `char` is W0704.

## 4. WX-AD: alphabet domains (`within`)

### 4.1 Problem

Both text and (in VER04m) call sequences have interesting members that no
type-generic domain contains: URL strings are made of `:/?#@%`, a framer's calls
are `feed("\r\n")` and `frames`. A refinement is a filter (VER04h §4: 100
retries) and cannot introduce a domain; binding-side hints (WX-F `lengths`) are
per lane, whereas a domain must be language-neutral so VER04i digests agree.

### 4.2 The predicate

One built-in predicate, no grammar:

```text
within(xs, alphabet, k) : Bool
  xs: Text,    alphabet: Text    -- every scalar of xs occurs in alphabet, len(chars(xs)) <= k
  xs: Bytes,   alphabet: Bytes   -- every byte of xs occurs in alphabet,   len(xs) <= k
  xs: List<T>, alphabet: List<T> -- every element of xs equals a member,   len(xs) <= k
```

`k` is `Nat`. As a predicate it is total and has its ordinary first-order meaning
anywhere an expression is allowed. The checker rejects a duplicate member in a
literal alphabet (W0301), and an alphabet above 256 members (W0602).

### 4.3 Domain rule

When a binder's refinement has a **top-level conjunct** `within(binder, A, K)`
where `A` is ground (a literal, or a model call with no free binder) and `K` is a
literal, the binder's exhaustive domain is exactly `A^{<=K}`:

* order: length ascending, then lexicographic in the **positional order of `A`**
  (not scalar order), so `text_over(t"ba", 2)` begins `""`, `b`, `a`, `bb`, `ba`,
  `ab`, `aa`;
* size `sum_{i<=K} |A|^i`, computed with checked arithmetic and counted against
  the budget `B = 400,000` and the 64 MiB domain-byte budget before anything is
  materialised (W0706 over budget);
* the quantifier keeps its meaning: the refinement is an ordinary conjunct and
  the binder simply ranges over fewer values. This is the precedent of VER04k
  2.9's `split(m)` domain, a domain inferred from a recognised use.

The budget applies to the product of binder domains **before** refinement
(VER04h §3). A `within` binder contributes `|A^{<=K}|`, not `|D(T)|`; other
refined binders still contribute their whole type, so a document uses `Bool`
binders or model-built values where `U8 | x < 16u8` would blow the product.

Descriptor names for VER04i: `text_over(<alphabet sha256/8>, K)`,
`bytes_over(...)`, `list_over(...)`, where the hash is the SHA-256 of the
alphabet's canonical stream encoding (7.2) and `corpus.json` carries the
alphabet itself (as decimal scalars for `Text`).

Property and fuzz suites draw sequences over `A` of length `0..=K` directly, so
the refinement holds by construction and nothing is filtered. Longer random
sequences are a second claim with a larger `K` whose exhaustive suite is over
budget and therefore declared random-only; there is no separate knob.

Sizes worth knowing: 6 symbols to length 3 is 259; 5 symbols to length 4 is 781;
11 symbols to length 5 is 177,156; 12 symbols to length 5 is 271,453; 16 symbols
to length 4 is 69,905; 8 calls to depth 5 is 37,449; 7 calls to depth 6 is
137,257.

### 4.4 Label

The kind of a `within` domain is **`bounded-set`**, printed
`text_over(12 symbols, <=5) = 271,453`. The enumeration is complete for the claim
as written, but the claim is a bounded one. A `within` claim is **never
`exhaustive`** for now (decision d; memo question I-1): VER29 §6.1 reserves
`exhaustive` for a complete domain, and a length bound is not complete. If a
later specification shows a bound that is also a proof (for example a
small-model theorem for a regular grammar), it must say so in its own text; this
one does not.

## 5. Surface syntax

Additions: the type names `Text` and `Char`; the literals of 5.1; the nine
reserved names of 2.2. No new declaration form, no new keyword. Projections,
records, `Option`, `Result`, `match`, `labels`, `input`, `bound` are unchanged and
accept `Text` wherever they accept `Bytes`. `fresh` applies to `Text` results as
to `Bytes`.

### 5.1 Literals

```text
text_literal = "t" '"' { text_char | escape } '"' ;
char_literal = "c" '"' ( text_char | escape ) '"' ;
escape       = "\" ( "\" | '"' | "n" | "r" | "t" | "0" | "u{" HEX {HEX} "}" ) ;
```

* `\u{H..H}` has 1 to 6 hexadecimal digits (either case) and must name a scalar
  value; a surrogate or a value above 0x10FFFF is W0101.
* The source is already UTF-8 without a BOM (VER04 §2), so non-ASCII may be
  written directly, **but only within the raw ranges `0x20..=0x7E` (except `"` and `\`, which
  are escaped) and `0xA1..=0x24F`** (printable ASCII, and Latin letters with
  diacritics). Everything else is written with `\u{}` and a raw occurrence is
  W0101. This covers controls, NBSP (U+00A0), bidi controls, zero-width and
  format characters, combining marks and every character the lexer's table does
  not know, so that a reviewed document cannot hide what it says. (Memo question
  T-5, resolved by the single simple rule.)
* A `Char` literal is exactly one scalar (W0101 otherwise).
* A text literal is at most 4,096 scalars (W0602), the list-literal cap.
* `t"\0"` is the single scalar U+0000.
* The existing `b"hex"` literal is unaffected; the new prefixes extend the same
  token class (a letter immediately followed by `"`), so the token budgets of
  VER04 §2 apply unchanged.

**Generated code never embeds a Text literal as a host string literal.** A lane's
generated evidence emits the literal as UTF-8 hex decoded through the lane's
strict decoder (7.2), so no host source-encoding rule, escape syntax or
surrogate representation can change a value.

### 5.2 Typing summary

* `Text == Text`, `Char == Char`, equality inside composites: allowed.
* `<`, `<=`, `>`, `>=` on `Text` or `Char`: W0301.
* `len(Text)`: W0301 (3.4). `len(chars(t))`, `len(utf8(t))`: `Nat`.
* `Text` and `Bytes` never convert implicitly; `utf8` and `from_utf8` are the
  only bridges.
* `Char` and `U32` never convert implicitly; `code` and `char_of` are the only
  bridges.
* `within` types as 4.2.

### 5.3 Example sketches

The worked, counted versions are in section 13; these are the shapes.

```text
warrant caesar_cipher v2
  model caesar(t: Text, k: U32) -> Text = text(map(chars(t), shift_char(k)))
  op encrypt(input: Text, shift: I32) -> Text fresh
  claim letters_every_shift:
    forall t: Text | within(t, t"azAZ m", 3), s: I32. encrypt(t, s) == caesar(t, k_of(s))
```

The pattern for parsers is a *constructive* claim over structured binders plus a
*total* claim over `Sigma^{<=k}` (13.3).

## 6. Domains and honest labelling

### 6.1 Kinds (LP-012)

Backlog LP-012 splits case counts into `full`, `sampled` and `bounded-set`, and
only `full` may be shown as exhaustive. This document assigns text domains to
kinds and states how a claim's kind is combined; LP-012 is a prerequisite of T1
(12.1) and this section is the text input to it.

| Domain | Kind | Manifest text |
|---|---|---|
| `char_full` | `full` | `c: Char full(1112064)` |
| `char_edge` | `sampled` | `c: Char edge(151)` |
| `text_d1` | `bounded-set` | `t: Text set(418)` |
| `text_over(A, K)` | `bounded-set` | `t: Text over(12, <=5) = 271453` |
| WX-F `lengths` hints on `Text` | `sampled` | `+lengths(255,256,2048)` |

**Claim kind** is the weakest binder kind: if any binder is `sampled` (including
`edge` of `U16`, `I32`, ...), the claim is `sampled`; otherwise if any is
`bounded-set`, the claim is `bounded-set`; otherwise `full`. A refinement or a
demotion never improves a kind. Only a claim of kind `full` may be printed as
exhaustive. (LP-012 may refine the combination rule; the invariant that must
survive is that no claim containing a `text_d1`, `text_over` or `edge` binder is
reported as exhaustive.)

### 6.2 `char_full` and `char_edge`

* `char_full`: all 1,112,064 scalars ascending, skipping the surrogate gap. It
  needs `exhaustive_budget` raised to at least 1,112,064 by the package, as gf256
  does for `2^24`. Memory: each element costs `VALUE_OVERHEAD = 32` plus 4
  payload bytes, 40,034,304 bytes in all, inside the 64 MiB domain-byte budget.
* `char_edge` (default, 151 values): all of ASCII `0x00..=0x7F` (128); then,
  ascending, U+0080, U+0085, U+00DF, U+00E9, U+00FF, U+0130, U+0131, U+017F,
  U+0301, U+07FF, U+0800, U+200D, U+2028, U+202E, U+212A, U+D7FF, U+E000, U+FEFF,
  U+FFFD, U+FFFF, U+10000, U+1F600, U+10FFFF (23). Kind `sampled`.
* **Demotion** extends VER04j §2.6 rule 2: every `Char` binder starts `full` and,
  while the product exceeds `B`, the leftmost binder that is currently `full` and
  of type `U16`, `I16` or `Char` is demoted to `edge`. The manifest prints
  `c: Char edge(151)` and `demoted: ["c"]`. W0706 if still over budget.

### 6.3 `text_d1` (418 values)

Modelled on `bytes_d1`; order is fixed:

1. `t""` (1);
2. every length-1 text over `char_edge`, ascending (151);
3. every text of length 2 and then 3 over `A_t = [a, Z, 0, space, U+00E9,
   U+1F600]` in lexicographic order of that listing: 36 + 216 = 252;
4. for each length `L = 4..=8`, the fill of `a` then the fill of U+00E9 (10);
5. the four multi-scalar boundary rows of 7.3 in table order: `t"a\0b"`,
   `t"e\u{301}"`, `t"\u{FEFF}a"`, `t"\u{FFFF}\u{10000}"` (4).

1 + 151 + 252 + 10 + 4 = 418. Kind `bounded-set`. Two `Text` binders give 174,724
pairs, inside `B`.

*Reconciliation with the memo.* The memo's `text_d1` had 414 values and no fifth
group. With 414 values none of the three non-ASCII faults that need a multi-scalar
input (`truncate-at-nul`, `compose-e-acute`, and the NFC/BOM hazards of 7.3) could
fire on the default domain, so the memo's own vacuity guard (its T-4) would fail
every document that relies on the default. Group 5 restores catchability at a cost
of four cases.

### 6.4 Constructive domains

Texts built by models from small binders (13.3). Kind is that of the binders.
This is the idiom `arithmetic.warrant` uses for bit vectors, and for the same
reason: a filtered random string almost never parses.

### 6.5 The explosion problem, stated plainly

There is no "exhaustive over strings": 12 symbols reach length 5, 16 reach length
4. Depth-first bugs (a 2,048-byte host, a 64-label name) are out of reach of
enumeration and are covered by WX-F-style `lengths` hints applied to `Text`
(fills of a declared character at declared byte lengths), the property
generator, fuzzing and WX-E tables. The manifest says which.

### 6.6 Property generator, fuzz decoding, shrinking

* **Property generator.** Length `0..=64` scalars. Per scalar: 1/2 from the claim's
  *document dictionary* (every scalar that occurs in a text literal of the
  document, a fuzzing dictionary for free), 1/4 printable ASCII, 1/8 `char_edge`,
  1/8 uniform over all scalars. For two `Text` binders the relation bias of
  VER04h §4 adds: identical, one scalar changed, ASCII case of one letter
  flipped, one scalar appended or removed.
* **Shrinking.** Shorter first, then each scalar toward `a`.
* **Fuzz decoding.** A mode byte selects "decode the stream as UTF-8, replacing
  each invalid byte by a dictionary scalar" or "each byte indexes the dictionary".
  Never lossy toward U+FFFD only, which would collapse the input space.

### 6.7 Manifest fields

Per `Text` binder: `kind`, `size`, `non_ascii_members`, `astral_members`. A
binder whose domain has none of either is printed `ascii-only`; that is allowed
when the refinement says so and is a visible fact otherwise. Per claim: the claim
kind of 6.1 and, for claims that match on a `Result` or `Option`, the per-arm
instance counts of 14.2.

## 7. Corpus encoding (VER04i requirements)

### 7.1 TFV (`cases.tfv`)

| Warrant type | TFV value | Notes |
|---|---|---|
| `Text` | `S<hex of UTF-8>` | TF01 already defines `S` as strict UTF-8 text in hex; NUL allowed; may be empty |
| `Char` | `I<scalar>` | decimal, range-checked against the scalar set: `I55296` (U+D800) in a `Char` field is `corpus-malformed` |
| variant names | unchanged | constructor names stay `S<VARIANT>` and are disambiguated by the record schema (VER04j §2.8 rule 1: the schema carries the type) |

The decoder is strict: an invalid `S` or an out-of-set `I` in a `Char` field is
`corpus-malformed`, never repaired.

### 7.2 Stream (`warrant-stream-v1`)

| Value | Bytes |
|---|---|
| `Text` | `60`, u32-LE byte length, the UTF-8 bytes |
| `Char` | `61`, 4 bytes little-endian scalar |

`60` and `61` are the next free tags after those of VER04i §2.6 (`00 01 10 20 30
40 41 50 51`). T1-S0 confirms they are still free against whatever tags the
record and variant encoding of VER04j §12a has assigned by then, and takes the
next two free values if not; the numbers are fixed in the VER04i text before any
code lands. The type name in the stream context prefix is `Text` or `Char`.
Both streams (`inputs`, `expected`) are unchanged in structure.

A lane decodes the corpus hex through its strict decoder (3.5) and compares the
re-encoded bytes with the corpus bytes; a lane whose decoder repairs anything
fails the conformance fixtures of 16.

### 7.3 Mandatory boundary rows

Every document that uses `Text` carries these rows. Each exists to make one lane
hazard fail a case. Rows 1 to 12 are single scalars or single bytes already in
`char_edge` or `text_d1`; rows 13 to 16 are the multi-scalar rows of `text_d1`
group 5.

| # | Row | Catches |
|---|---|---|
| 1 | `t""` | empty-string special cases |
| 2 | `t"\0"` | NUL-terminated transport (C) |
| 3 | U+007F, U+0080 | ASCII/Latin-1 boundary, signed `char` |
| 4 | U+07FF, U+0800, U+FFFF, U+10000, U+10FFFF | encoder length-class boundaries (1, 2, 3, 4 bytes) |
| 5 | U+D7FF, U+E000 | off-by-one around the surrogate gap |
| 6 | `t"\u{E9}"` | the composed form (see 15) |
| 7 | `t"\u{130}"`, `t"\u{131}"`, `t"\u{212A}"`, `t"\u{DF}"` | locale or Unicode case mapping used where ASCII was meant |
| 8 | `t"\u{1F600}"` | astral plane, surrogate pair, 4-byte form |
| 9 | `t"\u{2028}"`, `t"\u{85}"` | Unicode line and whitespace separators used by host `split`/`trim` |
| 10 | U+FEFF alone | a BOM character as data |
| 11 | U+0301 alone | a bare combining mark |
| 12 | U+202E | a bidi control carried as data |
| 13 | `t"a\0b"` | NUL in the middle: truncation at NUL (C) |
| 14 | `t"e\u{301}"` against row 6 | an observer that compares with canonical equivalence (Swift `==`) or normalises |
| 15 | `t"\u{FEFF}a"` | BOM stripping by a decoder |
| 16 | `t"\u{FFFF}\u{10000}"` (3 UTF-16 units, 7 UTF-8 bytes); the conformance fixtures also compare it with `t"\u{10000}\u{FFFF}"` in scalar order | UTF-16 code-unit ordering and length (the pair is 3 units) |

Each lane also runs the **round-trip obligation** `observe(construct(t)) == t`
(VER04 §6) over `text_d1` and `char_edge` with no subject call, so a lane whose
conversion repairs a row fails before any operation is blamed.

### 7.4 Ill-formed probe set

Used only by lanes that declare `rejects:<CASE>` (3.7). Not Warrant values; not in
the digests. Three families; a lane runs the families its representation can
hold.

| Family | Probes | Lanes |
|---|---|---|
| B (bytes) | `80`, `c080`, `c1bf`, `e282`, `eda080`, `edbfbf`, `f4908080`, `f8888080`, `ff` | byte-string lanes; Python through its lossless byte-injection path |
| U (UTF-16 units) | `D800`, `DC00`, `DC00 D800` | Java, Kotlin, C#, F#, JavaScript, TypeScript, Dart |
| S (scalars) | `D800`, `DFFF`, `110000` where the host integer can hold them | Python, Haskell `String`, Perl character strings, Go `rune` |

Meaning of the byte probes: `80` stray continuation; `c080` and `c1bf` overlong;
`e282` truncated three-byte form; `eda080` and `edbfbf` UTF-8-encoded surrogates;
`f4908080` above U+10FFFF; `f8888080` five-byte form; `ff` never valid.

## 8. Oracles and reference models

A text reference model is a pure Warrant model over `chars(t)` or `utf8(t)`. Three
shapes cover the targets:

| Shape | Example | Needs |
|---|---|---|
| per-scalar transform | caesar, atbash, rot13, scytale-style ciphers | WX-D2 `map` |
| byte-level codec stated on `utf8` | percent coding, base64 and hex text renderings | WX-D2 `fold` |
| parser | url-parser, `http-core` `HttpVersion::parse`, route patterns | constructive round trips, total claims over `Sigma^{<=k}`, `rejects` rows, WX-E tables (WHATWG or RFC 3986 test vectors) |

A full parser model is possible only for regular, bounded-lookahead grammars
written as a `fold` state machine, and it tends to be a transliteration of the
implementation (risk 14.4). **Recommendation, not requirement:** for parsers
require (a) a printer model and the round trip `parse(print(v)) == Ok(v)` over a
structured domain, (b) totality and output-shape claims over `Sigma^{<=k}`,
(c) rejection rows for each error case with at least one example each, and
(d) an external table where a public test suite exists.

**Dependency on WX-D2.** Without `map` and `fold`, text evidence is round trips and
known answers only. T1 does not ship before WX-D2 (12.1).

Layering needs nothing new: a `Text`-returning claim is a functional contract id
like any other (VER29 §3.1). Error messages returned as `String`
(`Result<T, String>`) are normalised to one error case through the existing
`error=` binding; message text is not contract.

## 9. Evidence and proof impact

### 9.1 Generator, per lane

Value conversion `Text <-> host`, strict both ways; the descriptors of section 6
in the generated prelude and in `warrant-corpus`; text literals in evidence bodies
(5.1); the nine names in the ground evaluator, the generated model code and the
lane IR evaluator, with the three-way differential test of VER04j §10.3 (the
`from_utf8` strictness table is the part most likely to drift: it is tested
against the TF01 strictness cases and the probe bytes of 7.4). The IR gains
`["text", "<hex>"]`, `["char", n]` and the call tags of the new built-ins; a lane
evaluator rejects an unknown tag.

### 9.2 Faults (VER04h §6; adapter rewrites)

| Fault | Rewrite of a `Text` result | Bug modelled |
|---|---|---|
| `lossy-utf8` | every non-ASCII scalar becomes U+FFFD | lossy conversion at a boundary |
| `truncate-at-nul` | cut at the first U+0000 | C-string transport |
| `drop-last-char` | remove the last scalar | off-by-one, or a split surrogate pair |
| `ascii-case-flip` | flip the case of the first ASCII letter | case-insensitive comparison used for equality |
| `compose-e-acute` | replace `e` U+0301 by U+00E9 | silent normalisation |

Controls are per (suite kind, operation, fault) as in VER04h §6: the control
passes iff some suite of that kind, run with the fault, fails with a registered
failure id, and the manifest records `caught_by`. `lossy-utf8`,
`truncate-at-nul` and `compose-e-acute` can fire only if some claim's domain
yields a result containing the relevant scalar. They are therefore **mandatory**
for every operation with a `Text` or `Char` result, with one exception: an
operation declared `ascii_only=true` in its (reviewed) binding line. For such an
operation the three faults are `not-applicable`, the report prints
`ascii-only contract`, and the runner checks every `Text` result it observes: a
non-ASCII scalar in a result of an `ascii_only` operation fails the instance with
`ascii-contract-violated`, so the declaration cannot hide a non-ASCII bug. (Memo
question T-4, resolved.) A `Text` fault on an operation whose result has no
`Text` leaf is W0701.

### 9.3 Properties and fuzz

Section 6.6. Property suites report the claim kind of 6.1 as `sampled`.

### 9.4 Reports

The assurance report (VER04h §9, `ca-assurance-report`) gains, per lane and per
operation with a text parameter or result: the `ill_formed` policy, the number of
probes run and their families (for `rejects`), `unreachable_by_type` counts, the
faults caught and any `ascii-only contract`. For `ill_formed = unspecified` it
prints, in the lane's header block and again in the headline, the line

```text
open exemption: <lane>/<package>.<op>.<param>: behaviour on ill-formed host strings not covered
```

A lane with any `unspecified` parameter is labelled `live-with-exemption`, never
plain `live`, for the text feature. The report fails closed if a `Text` parameter
has no declared policy.

### 9.5 Proofs: not-live in the first slice (decision d)

The WIR front end and the two provers (constant-time, step bound) work on byte
slices with length shapes. To lower `&str` code they would need `&str` as a
pointer-length pair carrying the validity invariant, the `chars()` decoder state
machine (a data-dependent loop of 1 to 4 bytes per step), `String::push` and
`push_str`, `char` ranges in `match`, and `format!` (used by `percent_encode`),
which is the hardest. Therefore an operation with a `Text` parameter and a
`labels ... secret` row or a `bound steps` row **must carry `proofs_not_live=text`**
in the bindings (the WX-F `proofs_not_live=driver` precedent of VER04k 3.4), so the
gap is acknowledged in a reviewed file (refused as W0501 otherwise). When it goes live the label is `bounded(K)` in UTF-8
byte length, never `proved`, exactly as for `Bytes`.

### 9.6 Bounds (CWE-400)

Text adds one amplification trap: byte length versus scalar count. A bound
written in scalars under-counts by up to four times. Rules: `bound` and `output`
expressions mention a `Text` parameter only through `utf8` or `chars` (3.4); the
allocation observer measures bytes; caps are in section 15.

## 10. Per-lane hazard table

"Strict conversion" is the check an adapter must perform before it hands a host
string to the Warrant side; "Permitted policies" are the `ill_formed` values 3.7
allows for a `Text` parameter of that lane. A lane that cannot convert strictly
marks the dependent targets `not-live` (VER04j §8 item 4).

| Lane | Host representation | What ill-formed input is there | Native traps | Strict conversion | Permitted policies |
|---|---|---|---|---|---|
| Rust | `&str`/`String` UTF-8, validated; `char` is a scalar | none: unreachable by type (`from_utf8_lossy` in an adapter is a defect) | `len` is bytes; `chars().count()` scalars | `std::str::from_utf8`, never `_lossy` | `unreachable` |
| Python | `str` of code points | lone surrogates (`'\ud800'`), also produced by `surrogateescape` | `len` is code points; `encode('utf-8')` raises on surrogates; `lower()` is Unicode-version dependent | `s.encode('utf-8')` (strict) and `bytes.decode('utf-8')` (strict) | `rejects:<CASE>` (S probes), `unspecified` |
| JavaScript / TypeScript | UTF-16 `string` | lone surrogates | `length` and indexing are UTF-16 units; `TextEncoder` silently replaces lone surrogates; `<` and `localeCompare` are not scalar order | `isWellFormed()` (or a manual scan) before encoding; iterate by code point; decode with `new TextDecoder('utf-8', {fatal: true})` | `rejects`, `unspecified` |
| Go | `string` is bytes | any invalid UTF-8; encoded surrogates | `range` yields U+FFFD for invalid input; `[]rune(s)` is lossy; `rune` is an `int32` that can hold surrogates and values above 0x10FFFF | `utf8.ValidString` on inputs and results; check each `rune` against the scalar set | `rejects`, `unspecified` |
| Java / Kotlin | UTF-16 `String`; `char` is a UTF-16 unit | lone surrogates | `getBytes(UTF_8)` / `toByteArray()` replace lone surrogates with `?`; `compareTo` is code-unit order; Kotlin `Char` is a unit | `CharsetEncoder`/`CharsetDecoder` with `REPORT`; `Char` maps to an `int` code point; compare via UTF-8 bytes | `rejects`, `unspecified` |
| C# / F# | UTF-16 `string`; `char` is a unit; `Rune` is a scalar | lone surrogates | `Encoding.UTF8.GetBytes` replaces lone surrogates with U+FFFD unless `new UTF8Encoding(false, true)`; default `string.Compare` is culture-sensitive; `Length` is units | `UTF8Encoding(false, true)`; `Rune` for `Char`; `string.CompareOrdinal` only after confirming it matches 3.4 or compare UTF-8 bytes | `rejects`, `unspecified` |
| Ruby | encoding-tagged byte strings | invalid byte sequences; wrong encoding tag | `length` counts chars of the current encoding; `upcase` is Unicode-version dependent | `force_encoding('UTF-8')` then `valid_encoding?`; `unpack('U*')` | `rejects`, `unspecified` |
| Elixir | UTF-8 binaries | any invalid binary | `String.length` counts graphemes; `String.upcase` is Unicode-version dependent | `String.valid?/1`; compare binaries; `String.to_charlist` only on valid input | `rejects`, `unspecified` |
| C / C++ | `char*` plus length, or NUL-terminated; `char8_t`; `std::u32string` | any bytes; `std::u32string` can hold surrogates; NUL-terminated strings cannot hold U+0000 | `char` signedness (`c >= 'a'` on a negative byte); `wchar_t` is 16-bit on Windows; locale-dependent `toupper`/`strcoll` | pointer-plus-length binding form; a vetted strict validator in `warrant-corpus`; the `cstr` form declares U+0000 `unreachable_by_type` and counts it | `rejects`, `unspecified` (`unreachable` for none) |
| Swift | `String` always valid; `Character` is a grapheme; `Unicode.Scalar` is a scalar | none in a `String`, but decoding repairs (`String(decoding:as:)` replaces with U+FFFD) | `==` is canonical equivalence; `count` is graphemes; `<` follows canonical ordering | `String(validating:as:)` (strict); compare `Array(s.utf8)` only; `Char` is `Unicode.Scalar` | `unreachable` for parameters; `rejects` at the byte-decode boundary |
| Haskell | `Data.Text` validated; `String` is `[Char]` including surrogate code points | `String` can hold surrogates; `Data.Text.pack` replaces them with U+FFFD | `length` of `Text` is code points; `T.toUpper` is Unicode-version dependent | `Data.Text.Encoding.decodeUtf8'` (returns `Either`); reject surrogates in a `String` before `pack` | `unreachable` for `Text`; `rejects`, `unspecified` for `String` |
| Lua | byte strings; embedded NUL allowed | any bytes | `#s` is bytes; 5.1 and LuaJIT have no `utf8` library; `utf8.char` can build values above 0x10FFFF | `utf8.len(s)` in strict (non-lax) mode on 5.3 and later; a vetted validator in `warrant-corpus` elsewhere | `rejects`, `unspecified` |
| Perl | character strings and byte strings (the `utf8` flag) | surrogate and above-0x10FFFF characters exist in character strings; invalid UTF-8 in byte strings | `length` counts characters or bytes by flag; lax `utf8` encoding accepts what strict `UTF-8` rejects | `Encode::decode('UTF-8', $bytes, Encode::FB_CROAK)`, never the lax `utf8` name | `rejects`, `unspecified` |
| Dart | UTF-16 `String`; `runes` iterate code points | lone surrogates | `length` is units; `utf8.encode` replaces lone surrogates with U+FFFD; on the web `int` is a double | scan for lone surrogates before encoding; `utf8.decode` with `allowMalformed: false`; `Char` as an `int` code point | `rejects`, `unspecified` |

Two cross-lane rules follow from the table. First, `Char` is never a native
16-bit or 8-bit character type (3.7). Second, comparison for `==` must be the
lane's scalar or UTF-8-byte equality, never a locale, collation or
normalisation-aware comparison; the row-14 and row-16 boundary cases detect the
difference.

## 11. Runtime jail (VER08b)

Text is pure computation, but three things change in practice.

1. **Lazy loading after arming.** Codec tables, ICU data and encoding modules are
   loaded on first use in several runtimes (Python `unicodedata`, .NET
   globalization and ICU, Ruby transcoders, Java charset providers, Perl `Encode`
   tables). A first text call in phase B would `openat` and be killed. Each lane's
   harness performs a **text warm-up in phase A** that does not call the library:
   it round-trips the rows of 7.3 through the lane's strict conversion and
   exercises the lane's collation-free comparison, so that every table the
   conversions need is loaded before arming. The CAP-RT-1xx lane rows (VER04i §12)
   state the warm-up for the lane.
2. **Locale as ambient input.** `setlocale`, `toupper`, `strcoll` and
   `String#upcase(:turkic)` read `LANG` and `LC_*`. The empty launch environment
   already removes them; this document adds one **negative control**: run the
   suite twice with different `LC_ALL` values injected in phase A and require
   identical TAP output. A library that asks for `env:read` to see the locale has a
   contract that is not a function of its arguments, and that is a finding.
3. **No new syscalls expected.** The six-entry baseline `rcg-baseline-1` should hold
   for Rust. This is **measured on the first pilot, not assumed**; if a text
   workload needs `mremap` or `madvise`, the baseline is bumped (VER08b §4) by a
   separate decision and not by this document.

## 12. The minimal slice T1

### 12.1 Order (decision e)

| Step | Work | Depends on |
|---|---|---|
| 0 | implement WX-G, WX-D2, WX-F; implement LP-012's case-count split | specified already |
| 1 | stateful S1 (explicit state) of VER04m | WX-G |
| 2 | **WX-T1** (this document) and WX-AD | step 0, S1 landed, header decision of section 2 |
| 3 | later slices that need text (protocol objects of VER04m S2) | T1, WX-F, WX-D2, WX-AD |

WX-T1 therefore starts after all of WX-G, WX-D2, WX-F, LP-012 and stateful S1 have
merged. T1-S0 may begin earlier as a paper review.

### 12.2 Scope

In T1: `Text`, `Char`, the literals, the nine names, `text_d1`, `char_edge`,
`char_full`, `within`, the TFV and stream encodings, boundary rows, the five
faults, the `ill_formed` policies, the Rust lane and the Python lane (the
independent oracle is extended in the same slice, VER04i §2.8), and the header
`v2`. **Out of T1:** proofs (`not-live`), an ordering on `Text`, normalisation or
case-mapping functions, lanes other than Rust and Python, ill-formed probes in
lanes that do not declare `rejects:`.

### 12.3 Sub-slices

| Slice | Content | Acceptance |
|---|---|---|
| T1-S0 | errata pass on this spec against the then-current VER04j/k/i; confirm stream tags; mechanical check of in-repo v1 documents against the nine names; accept `v2` in the header parser | spec change record updated; checker accepts `v2`, refuses `v3` |
| T1-S1 | checker and ground evaluator: types, literals, built-ins, `within` predicate, `len` refusal; model IR tags; three-way differential tests | section 16 groups A, B, E |
| T1-S2 | domains (`char_full`, `char_edge`, `text_d1`, `text_over`), corpus encoding, independent Python oracle, conformance fixtures | groups C, D; `gen_corpus --check` and the oracle agree |
| T1-S3 | Rust lane evidence: strict conversion, faults, probes (`unreachable`), manifest and report fields; pilot P1 | group F; P1 acceptance |
| T1-S4 | Python lane: strict conversion, `rejects` probes, warm-up; pilots P2 and P3 | group F; P2, P3 acceptance |

### 12.4 Pilots

| Pilot | Why | Ports |
|---|---|---|
| P1 `caesar-cipher` (+ `atbash-cipher`) | smallest complete model; exercises `Char full`; 16 ports each, so also the cross-lane hazard test | 16 |
| P2 `url-parser` | the highest-value text crate in the inventory (45 direct dependents); records of `Text` and `Option<Text>`, `rejects`, `within`, WX-H output bounds | 14 |
| P3 `http1` | bytes in, text in the result: `from_utf8` in `rejects` rows and the text-as-output path most HTTP and TLS crates share | 12 |

Not pilots, and why: `huffman-tree` (returns `HashMap<u16, String>` and hides a
recursive tree: needs maps and an immutable-object binding), `wasm-module-parser`
(the result is a recursive `WasmModule`), `http-core` (borrowed iterators and
lifetime-carrying views). The inventory's "text unblocks 133" counts signatures;
each adopter still needs a model or tables, so yield per week is bounded by
specification effort, not by the type.

## 13. Worked examples for the pilots

Every Warrant fragment here is a **sketch**: none has been run through the checker
(the T1 slices are not implemented). Counts are computed by hand and re-derivable
from sections 4 and 6; where a claim depends on what a crate actually does, it is
marked **[crate-dependent]** and is pinned against the crate at the pilot's own
slice, not here. "Domain" counts are before refinement and below the default
`B = 400,000` unless stated.

### 13.1 P1: `caesar-cipher`

```text
warrant caesar_cipher v2
  model is_up(c: Char) -> Bool = code(c) >= 65u32 and code(c) <= 90u32
  model is_lo(c: Char) -> Bool = code(c) >= 97u32 and code(c) <= 122u32
  -- floor remainder: the shift as a number in 0..=25, for every I32 including MIN
  model k_of(shift: I32) -> U32 = wrap<U32>(unwrap_or(rem(to_int(shift), 26), 0))
  model rot(c: Char, base: U32, k: U32) -> Char =
    unwrap_or(char_of(base + unwrap_or(rem(code(c) - base + k, 26u32), 0u32)), c)
  model shift_char(k: U32, c: Char) -> Char =
    if is_up(c) then rot(c, 65u32, k) else (if is_lo(c) then rot(c, 97u32, k) else c)
  model caesar(t: Text, k: U32) -> Text = text(map(chars(t), shift_char(k)))

  op encrypt(input: Text, shift: I32) -> Text fresh
  op decrypt(input: Text, shift: I32) -> Text fresh
  op rot13(input: Text) -> Text fresh

  example hello:
    encrypt(t"Hello, World!", 3i32) == t"Khoor, Zruog!"
  claim encrypt_is_the_model:
    forall t: Text, s: I32. encrypt(t, s) == caesar(t, k_of(s))
  claim decrypt_inverts_encrypt:
    forall t: Text, s: I32. decrypt(encrypt(t, s), s) == t
  claim rot13_is_shift_13:
    forall t: Text. rot13(t) == caesar(t, 13u32)
  claim letters_every_shift:
    forall t: Text | within(t, t"azAZ m", 3), s: I32. encrypt(t, s) == caesar(t, k_of(s))
  claim non_ascii_untouched:
    forall t: Text | within(t, t"\u{E9}\u{DF}\u{130}\u{212A}\u{1F600}", 3), s: I32.
      encrypt(t, s) == t
```

Internal checks: `rot` is only called on a letter, so `code(c) - base` is in
`0..=25` and `U32` subtraction never wraps; `k_of` is in `0..=25` for every `I32`
(`rem` is floor modulo, so a negative shift is reduced into range, and the
`wrap<U32>` of a value already in `0..=25` is exact); `char_of` of a value in
`65..=90` or `97..=122` is always `Some`, so the `unwrap_or` default is never
used. `example hello` agrees with the model: `H`(72) + 3 gives `K`, `e`(101) + 3
gives `h`, `W`(87) + 3 gives `Z`, `d`(100) + 3 gives `g`; `,`, space and `!` are
unchanged.

| Claim | Binders | Cases | Kind |
|---|---|---|---|
| `encrypt_is_the_model` | `Text` `text_d1` 418 x `I32` edge 559 | 233,662 | `sampled` (the `I32` edge binder) |
| `decrypt_inverts_encrypt` | same | 233,662 | `sampled` |
| `rot13_is_shift_13` | `Text` `text_d1` | 418 | `bounded-set` |
| `letters_every_shift` | `text_over(6 symbols, <=3)` = 259 x 559 | 144,781 | `sampled` |
| `non_ascii_untouched` | `text_over(5 symbols, <=3)` = 156 x 559 | 87,204 | `sampled` |

The `I32` edge domain has 559 values (VER04j §2.6); every `I32` claim is therefore
`sampled`, whatever the `Text` side is. `letters_every_shift` has no non-ASCII
symbol, so it supports no non-ASCII claim; `non_ascii_untouched` supplies the
non-ASCII members (a `non_ascii_members` count of 5, `astral_members` 1, from the
manifest fields of 6.7), and its alphabet includes U+0130 and U+212A, which a
`char::to_lowercase`-based implementation would fold into ASCII letters.

*Expected finding (unverified by execution).* `I32` edge contains `MIN`, and
`decrypt` is `encrypt` with `-shift`. From reading, that negation overflows for
`i32::MIN` (a panic in a debug build; in release the wrapped value makes `decrypt`
apply the same shift again, so `decrypt_inverts_encrypt` would fail). This is the
kind of result the feature exists to produce; the document does not assert it.

### 13.2 P1: `atbash-cipher`, a complete `Char` domain

```text
warrant atbash_cipher v2
  model flip(c: Char) -> Char =
    if code(c) >= 65u32 and code(c) <= 90u32 then unwrap_or(char_of(155u32 - code(c)), c)
    else (if code(c) >= 97u32 and code(c) <= 122u32 then unwrap_or(char_of(219u32 - code(c)), c)
    else c)
  model flip_text(t: Text) -> Text = text(map(chars(t), flip))
  op atbash_char(ch: Char) -> Char
  op atbash(input: Text) -> Text fresh

  claim atbash_char_is_the_model:
    forall c: Char. atbash_char(c) == flip(c)
  claim atbash_is_the_model:
    forall t: Text. atbash(t) == flip_text(t)
  claim atbash_is_an_involution:
    forall t: Text. atbash(atbash(t)) == t
```

Checks: `155 - 65 = 90` (`A` to `Z`), `155 - 90 = 65`; `219 - 97 = 122`
(`a` to `z`), `219 - 122 = 97`; hence `flip` is an involution on letters and the
identity elsewhere. `atbash_char_is_the_model` has one `Char` binder: with the
package's `exhaustive_budget` raised to at least 1,112,064 it is **`full`**,
1,112,064 cases, 40,034,304 domain bytes (6.2); at the default `B = 400,000` the
rule of 6.2 demotes the binder to `char_edge` (151 cases, `sampled`) and the
manifest prints `demoted: ["c"]`. The other two claims are `bounded-set`, 418
cases each. This is the one place a text claim is honestly `full`: every scalar
value, by a stateless function.

### 13.3 P2: `url-parser`

```text
warrant url_parser v2
  error UrlError = MISSING_SCHEME | INVALID_SCHEME | INVALID_PORT
                 | INVALID_PERCENT_ENCODING | EMPTY_HOST | RELATIVE_WITHOUT_BASE
  record Url(scheme: Text, userinfo: Option<Text>, host: Option<Text>, port: Option<U16>,
             path: Text, query: Option<Text>, fragment: Option<Text>)

  model unreserved(b: U8) -> Bool = (
    (b >= 65u8 and b <= 90u8) or (b >= 97u8 and b <= 122u8) or (b >= 48u8 and b <= 57u8)
    or b == 45u8 or b == 46u8 or b == 95u8 or b == 126u8 or b == 47u8)
  model hexdig(n: U8) -> U8 = if n < 10u8 then 48u8 + n else 55u8 + n          -- upper case
  model enc_byte(b: U8) -> List<U8> =
    if unreserved(b) then [b] else [37u8, hexdig(shr(b, 4)), hexdig(bit_and(b, 15u8))]
  model enc_step(acc: List<U8>, b: U8) -> List<U8> = concat(acc, enc_byte(b))
  model enc_bytes(b: Bytes) -> Bytes = to_bytes(fold(b, [], enc_step))

  op percent_encode(input: Text) -> Text fresh
  op percent_decode(input: Text) -> Result<Text, UrlError> fresh
  op parse(input: Text) -> Result<Url, UrlError>

  example encode_mixed:                                  -- [crate-dependent: the set of unreserved bytes]
    percent_encode(t"a b/\u{E9}") == t"a%20b/%C3%A9"
  example decoded_bytes_must_be_utf8:
    percent_decode(t"%FF") == Err(UrlError.INVALID_PERCENT_ENCODING)
  example truncated_escape:
    percent_decode(t"%4") == Err(UrlError.INVALID_PERCENT_ENCODING)

  claim encode_is_the_model:
    forall t: Text. utf8(percent_encode(t)) == enc_bytes(utf8(t))
  claim decode_inverts_encode:
    forall t: Text. percent_decode(percent_encode(t)) == Ok(t)
  claim decode_never_longer:
    forall t: Text | within(t, t"%4fF1g\u{E9}", 6).
      match percent_decode(t) with Ok(r) -> len(utf8(r)) <= len(utf8(t)) | Err(e) -> true

  -- parse: structured round trip. The texts are BUILT from small binders, so the claim is
  -- total and the generators are never starved by a filter.
  model scheme_of(i: Bool) -> Text = if i then t"http" else t"x+y.z-1"
  model host_of(j: Bool) -> Text = if j then t"a.example" else t"h"
  claim parse_scheme_host_path:                    -- [crate-dependent: the right-hand side is a normal form]
    forall i: Bool, j: Bool, p: Text | within(p, t"/ab-2", 4).
      parse(concat(scheme_of(i), concat(t"://", concat(host_of(j), concat(t"/", p))))) ==
        Ok(Url(scheme_of(i), None, Some(host_of(j)), None, concat(t"/", p), None, None))
  claim parse_small_alphabet_total:
    forall t: Text | within(t, t"ah:/?#@%.[1", 5).
      match parse(t) with Ok(u) -> len(utf8(u.scheme)) >= 1 | Err(e) -> true

  bound percent_encode:
    output <= 3 * len(utf8(input))
    steps <= 64 * len(utf8(input)) + 512
    heap <= 8 * len(utf8(input)) + 4096
    depth <= 8
```

Checks. `encode_mixed`: `a` and `b` are unreserved; space is 32, so `%` then
`hexdig(2)` = `2` and `hexdig(0)` = `0`; `/` is in the model's unreserved set;
U+00E9 is the bytes `C3 A9`, so `hexdig(12)` = `C` (55 + 12 = 67), `hexdig(3)` =
`3`, `hexdig(10)` = `A`, `hexdig(9)` = `9`, giving `%C3%A9`. The bound
`3 * len(utf8(input))` holds because every byte becomes at most three. In
`decode_never_longer` each `%XX` shrinks by two bytes and other bytes pass
through, so the length bound is true whenever the result is `Ok`; the claim is a
no-panic and shape claim, not an ASCII claim (the memo's name for it said
"ascii_or_error", which it does not state; renamed).

`parse_scheme_host_path` uses the alphabet `/ab-2` (no `%` and no `.`) so that
the path never contains a truncated percent escape or a dot segment that the
crate may legitimately reject or normalise; those behaviours are covered by their
own examples at P2. Its right-hand side is a guess at the crate's normal form;
writing it is exactly the specification work an adopter does.

| Claim | Binders | Cases | Kind |
|---|---|---|---|
| `encode_is_the_model` | `Text` `text_d1` | 418 | `bounded-set` |
| `decode_inverts_encode` | same | 418 | `bounded-set` |
| `decode_never_longer` | `text_over(6 symbols, <=6)`: (6^7 - 1)/5 | 55,987 | `bounded-set` |
| `parse_scheme_host_path` | `Bool` 2 x `Bool` 2 x `text_over(5 symbols, <=4)` 781 | 3,124 | `bounded-set` |
| `parse_small_alphabet_total` | `text_over(11 symbols, <=5)`: (11^6 - 1)/10 | 177,156 | `bounded-set` |

The `parse_small_alphabet_total` alphabet has 11 symbols (`a h : / ? # @ % . [ 1`),
not the 12 of the memo's table of sizes; its Ok arm has a non-trivial body, so the
per-arm rule of 14.2 requires at least one string of the 177,156 to parse
(`a:` or `ah:/` are candidates **[crate-dependent]**); if none does, the claim is
W0601 and the document must widen the alphabet or add a constructive claim.

### 13.4 P3: `http1`

The operation takes bytes; text appears only in the result. Reading the crate:
`parse_request_head(&[u8])` validates the start line and each header line with
`std::str::from_utf8` and reports `InvalidHeadEncoding` otherwise; it splits the
start line with `split_whitespace`, which separates on every Unicode
White_Space scalar, not only SP.

```text
warrant http1 v2
  error Http1Error = INCOMPLETE_HEAD | INVALID_HEAD_ENCODING | INVALID_START_LINE
                   | INVALID_HEADER | INVALID_VERSION | INVALID_STATUS
                   | INVALID_CONTENT_LENGTH
  record Header(name: Text, value: Text)
  -- projection of ParsedRequestHead: version, body_offset and body_kind are bound under
  -- VER04j 4.6 at P3 and elided here
  record Head(method: Text, target: Text, headers: List<Header>)
  op parse_request_head(input: Bytes) -> Result<Head, Http1Error>

  model req(p: Bytes) -> Bytes =                  -- GET /<p> HTTP/1.1 CR LF CR LF
    concat(b"474554202f", concat(p, b"20485454502f312e310d0a0d0a"))

  example bad_start_line_encoding:
    parse_request_head(b"ff0d0a0d0a") == Err(Http1Error.INVALID_HEAD_ENCODING)
  example surrogate_in_target:
    parse_request_head(b"474554202feda08020485454502f312e310d0a0d0a")
      == Err(Http1Error.INVALID_HEAD_ENCODING)
  example nel_is_not_a_separator:                       -- [crate-dependent; expected to surface a finding]
    parse_request_head(b"474554c2852f20485454502f312e310d0a0d0a")
      == Err(Http1Error.INVALID_START_LINE)

  claim target_text_follows_utf8:                       -- 156 cases: 146 take the Err arm, 10 the Ok arm
    forall p: Bytes | within(p, b"41c3a980ff", 3).
      match from_utf8(p) with
        None    -> parse_request_head(req(p)) == Err(Http1Error.INVALID_HEAD_ENCODING)
      | Some(s) -> parse_request_head(req(p)) == Ok(Head(t"GET", concat(t"/", s), []))
```

Checks. `ff0d0a0d0a` is the byte `FF` then CR LF CR LF: the empty-line scan
reaches the first blank line, the single line is `FF`, and the start-line
`from_utf8` fails, hence `INVALID_HEAD_ENCODING`. `474554202f` `eda080`
`2048545450` `2f312e31` `0d0a0d0a` is `GET /` + the encoded surrogate +
` HTTP/1.1` + CR LF CR LF, so the start line is not UTF-8. For
`nel_is_not_a_separator`, the bytes are `GET`, U+0085 (`c285`), `/`, SP,
`HTTP/1.1`, CR LF CR LF; the crate's `split_whitespace` treats U+0085 as a
separator, so it parses `GET`, `/`, `HTTP/1.1` and returns `Ok`, while an
RFC 9112 reader that splits only on SP sees two parts and rejects. **This is a
reading of the source, not an execution.** The pilot owner decides whether the
contract is "SP only" (then the example exposes a defect and the crate changes) or
"Unicode whitespace" (then the example is rewritten to say so); either way the
text feature has made the choice visible.

`target_text_follows_utf8`: the alphabet `41 c3 a9 80 ff` has 5 symbols, so
1 + 5 + 25 + 125 = 156 byte strings of length at most 3. Of these 10 are valid
UTF-8 (`""`; `41`; `4141`, `c380`, `c3a9`; `414141`, `41c380`, `41c3a9`,
`c38041`, `c3a941`) and 146 are not. Both arms have non-trivial bodies and both
are taken, so the per-arm rule of 14.2 is satisfied by construction. No alphabet
byte is SP, CR, LF, TAB or part of a Unicode separator, so the request target is
exactly `/` followed by the decoded text. The claim kind is `bounded-set`.

Lane policy for P3: the input is `Bytes`, so the `ill_formed` declaration applies
to no parameter; the `Text` results of the Rust lane are `unreachable`, and every
lane with a `String`-typed result field declares its strict result conversion.

## 14. Risks and vacuity guards

1. **ASCII evidence presented as text evidence.** Prevented by: `text_d1` and
   `char_edge` always contain non-ASCII and astral members; the manifest prints
   `ascii-only` per binder; the three non-ASCII faults fail their controls on an
   ASCII-only suite unless the operation declares `ascii_only=true`, which the
   runner then verifies (9.2).
2. **Error-path-only coverage.** A claim `match parse(t) with Ok(u) -> P | Err(e)
   -> true` passes trivially if no enumerated string parses. Prevention: for every
   claim whose body matches on a `Result` or `Option` that depends on a binder, the
   manifest records per-arm instance counts, and an arm whose body is not the
   literal `true` and that is never taken in the **exhaustive** (full or
   bounded-set) suite is **W0601** (the VER04k "driver not exercised" precedent).
   The constructive claim form exists so the `Ok` arm is reachable by construction.
3. **An observer or adapter that repairs text.** Strict conversion both ways;
   `lossy-utf8`, `compose-e-acute` and `truncate-at-nul` controls; the NFC/NFD, BOM
   and NUL rows; the round-trip obligation of 7.3.
4. **The model is the implementation.** A parser model copied from the Rust source
   proves agreement with itself. Prevention is procedural and partial: prefer
   printer-plus-round-trip, external tables, and state it in the contract document
   (the AR01 "oracles" paragraph is the template). Tooling cannot detect it.
5. **`bounded-set` counts inflating the headline.** A `Sigma^{<=5}` suite is 271,453
   cases. LP-012's split must land before or with T1 (12.1), or the report would
   show a text parser as "271,453 exhaustive cases".
6. **Unicode-version drift.** Excluded by construction: no built-in depends on a
   Unicode version. Residual: the *subject* may call `to_lowercase`; the Kelvin-sign
   and dotted-I rows exist to surface it.
7. **Lane digests agreeing on a wrong decode.** All lanes share the strictness
   table in `warrant-corpus`; the independent Python oracle is the second
   implementation. A shared misreading of UTF-8 in both is unlikely but is trusted
   code: it is recorded in `tcb`.
8. **`unspecified` becoming the default.** A lane could declare `unspecified` for
   every parameter. The report prints each as an open exemption and labels the lane
   `live-with-exemption`; a count of exemptions is part of the headline. It is
   permitted, never hidden.
9. **A `within` claim read as a proof.** `bounded-set` and the bound `K` are printed
   in the manifest text; 4.4 forbids `exhaustive` for such a claim.
10. **Inventory optimism.** See 12.4.

## 15. Resource caps and security considerations

| Cap | Value | Diagnostic |
|---|---|---|
| text literal | 4,096 scalars | W0602 |
| `within` alphabet | 256 members | W0602 |
| `Text` against the sequence-element cap | 4 x scalars | W0703 |
| `Text` against the data budget | its UTF-8 length, inside 64 MiB | W0703 |
| `text_over` and `char_full` domains | `B = 400,000` cases and 64 MiB domain bytes, checked arithmetic | W0706 |
| identifier length | 128 bytes (existing) | W0602 |

Threat notes. A document is untrusted input. A literal or alphabet that is large,
contains confusable or invisible characters, or encodes an unexpected scalar is
bounded by the caps and by the raw-range rule of 5.1. The corpus decoders are
strict (7.1) so a crafted `S` or `I` cannot smuggle a surrogate into a lane. The
generated prelude repeats the budget checks before materialising any domain
(VER04h §3). Adapters must not log or echo raw `Text` into diagnostics unescaped;
diagnostics print literals with `\u{}` for everything outside the raw ranges.

## 16. Conformance and test plan

An implementation conforms when all of the following hold. Tests ship with their
slice.

**A. Syntax and types (T1-S1).** (1) `warrant x v2` accepted, `v3` and
`v2 extra` refused. (2) `t"..."`/`c"."` in a v1 document: W0101. (3) A model,
op or binder named like each of the nine reserved names in a v2 document:
W0201/W0203; in a v1 document: accepted. (4) Each of `t"\u{D800}"`,
`t"\u{110000}"`, `t"\u{}"`, `t"\u{1234567}"`, `c""`, `c"ab"`, a raw U+00A0, a raw
U+0301, a raw U+202E, a raw U+007F in a literal: W0101. (5) `len(t)` with
`t: Text`: W0301 with the message of 3.4. (6) `t < u`, `c < d`: W0301.
(7) Text above 4,096 scalars: W0602. (8) `within` with a duplicate alphabet
member, mismatched operand types, or more than 256 members: W0301/W0602.

**B. Evaluator.** (9) The five laws of 3.5 over `text_d1` and `char_full`. (10)
`from_utf8` over the 9 probe byte strings of 7.4 and over 1,000 generated
overlong, truncated, surrogate and above-range sequences: `None`; over every valid
encoding length class: `Some`. (11) The three-way differential (ground evaluator,
generated Rust, lane IR evaluator) agrees on every built-in.

**C. Domains (T1-S2).** (12) `char_edge` has 151 members in the stated order;
`text_d1` has 418; `text_over(t"ba", 2)` enumerates `"" b a bb ba ab aa`.
(13) Sizes of 4.3 (259, 781, 55,987, 177,156, 271,453) reproduce. (14) Budget:
`text_over(17 symbols, <=6)` is W0706; a `Char` binder demotes per 6.2.
(15) Kinds: a claim with a `U16 edge` binder and a `text_d1` binder is `sampled`;
`Char full` alone is `full`; a `within` claim is never `exhaustive`.

**D. Corpus.** (16) TFV and stream round trip for every row of 7.3 and every
`char_edge` scalar; `I55296` in a `Char` field and an `S` of each B probe are
`corpus-malformed`. (17) The independent Python oracle re-derives `cases`,
`inputs_sha256` and `expected_sha256` for the P1 documents. (18) The conformance
fixtures `code/specs/fixtures/warrant-corpus-conformance-v1/` gain strictness
cases covering the boundary rows and probes; every lane's `warrant-corpus` passes
them before any Text evidence is accepted from that lane.

**E. Compatibility.** (19) Every v0 and v1 byte-identity test still passes
(2.3).

**F. Evidence (T1-S3, T1-S4).** (20) Each of the five faults fails its control on
P1 and P2; an `ascii_only=true` declaration on `encrypt` makes three faults
`not-applicable` and an injected non-ASCII result fails with
`ascii-contract-violated`. (21) A lane declaring `unreachable` for a host type that
can hold ill-formed text is refused (W0501). (22) A `rejects:<CASE>` lane fails when
one probe returns a value; a lane with `unspecified` prints the open-exemption line
and the `live-with-exemption` label. (23) An operation with a `Text` parameter and a
`bound steps` row without `proofs_not_live=text` is refused. (24) The two-run
`LC_ALL` control (11.2) passes for the pilots. (25) P2's `parse_small_alphabet_total`
with an alphabet that never parses is W0601. (26) The per-arm counts of P3's
`target_text_follows_utf8` are 146 and 10. (27) Round-trip obligation over
`text_d1` passes in both lanes; a lossy adapter fails it.

## 17. Open questions

| # | Question | Position taken here |
|---|---|---|
| T-9 | Which later explicit functions provide normalisation and case mapping, and under which header? | none now (3.3); a Unicode-version-named family, decided with its first adopter |
| T-10 | May a bounded `within` claim ever be `exhaustive` (memo I-1)? | no (4.4); revisit only with a proof of small-model sufficiency for a stated grammar class |
| T-11 | Should the probe sets of 7.4 move into VER04i proper once T1 lands? | yes, at T1-S0 as a requirement on VER04i |
| T-12 | `http1` `split_whitespace` on U+0085 and other separators (13.4): contract is SP-only or Unicode whitespace? | the pilot owner decides at P3; this spec records the question |

## 18. Out of scope, and why

Text proofs (9.5), an ordering on `Text` (3.4), normalisation and case mapping
(3.3), localisation, regular expressions, number formatting and parsing,
grapheme and word segmentation, maps keyed by text (`huffman-tree`), recursive
text-carrying types (`wasm-module-parser`), and the stateful objects of VER04m.
Each needs a language feature or a Unicode-version commitment this slice does not
make.

## 19. Specification change record

- Initial VER04l (2026-11-14): the value model, `within`, header `v2`, domains and
  labelling, corpus encoding, the five faults, the lane hazard table, the jail
  additions, slice T1 and pilots, worked examples.
- Departures from the design memo, each resolved here:
  1. `text_d1` has **418** values, not 414: the four multi-scalar boundary rows are
     added as group 5 (6.3), because with 414 the memo's own vacuity guard for the
     non-ASCII faults could not be met by the default domain.
  2. The memo says "eight built-ins" in one place and "seven built-in names" in
     another. There are nine reserved names: `Text`, `Char` and seven functions
     (`utf8 from_utf8 chars text code char_of within`); `concat` is an existing name
     extended, not reserved anew (2.2, 3.5).
  3. The memo's `url-parser` sketch put `%` and `.` in the `parse_scheme_host_path`
     alphabet and called a claim `..._is_ascii_or_error` that does not state that;
     the alphabet and name are corrected (13.3), and the `parse_small_alphabet_total`
     alphabet is 11 symbols (177,156), not 12.
  4. The memo's `atbash` involution claim used an undeclared `encrypt`; the example
     declares `atbash` (13.2).
  5. The id "WX-I" already means recursive sums in VER-BACKLOG and VER27; the
     alphabet-domain item is registered as **WX-AD**.
  6. The `rejects` policy is written `rejects:<CASE>` so the lane names the error
     variant (3.7), the form the memo used; the owner's decision names the class.
