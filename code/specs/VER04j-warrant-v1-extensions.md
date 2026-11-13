# VER04j — Warrant v1 Extensions (WX-B, WX-C, WX-A, WX-E, WX-H; sketches WX-F, WX-D)

**Status:** WX-B, WX-C, WX-A, WX-E and WX-H are **implemented** in
`rust/warrant-grammar` (section 14 lists exactly where the implementation
deviates from, or sharpens, the text below). The sketches WX-F and WX-D are
specification only. Each extension is a separately landable slice; none
changes the meaning of a v0 document (section 1).

**Parent:** [VER04 §3–§8](VER04-warrant-language.md),
[VER04c](VER04c-guarded-warrant-parser.md), [VER04d](VER04d-warrant-type-system.md),
[VER04e](VER04e-warrant-codegen.md), [VER04f](VER04f-claims-to-properties.md),
[VER04h](VER04h-warrant-derived-evidence.md), [TF01 §2](TF01-bootstrap-harness.md)
(TFV). **Corpus encoding:** the polyglot evidence corpus format is owned by
[VER04i](VER04i-polyglot-warrant-evidence.md) (backlog WD-002; a Draft on this
branch). VER04i owns the corpus encodings. Where this document needs a corpus
rule it states a requirement; VER04i must satisfy that requirement. Conflicts
and gaps between the two documents are listed in section 12a, not silently
resolved in either.

## 0. Why this exists

The leaf-library survey (Rust first, 25 pure candidates) found that Warrant v0
can state known answers, algebraic laws *between* operations, round trips and
rejection contracts, but not "operation X equals this algorithm". The first
conversion that needed a real model, `gf256` (MA01), forced three workarounds
that are the motivating evidence for this document:

| Workaround in `gf256.warrant` | Cost | Removed by |
|---|---|---|
| `xor1`..`xor8`: an 8-layer XOR built from comparisons and `+`/`-` | unreadable, and only correct because no sub-expression wraps | WX-C `bit_xor` |
| `power(base: U8, exp: U64 \| exp <= 4294967295u64)`: a `U64` plus refinement standing in for Rust's `u32` | the harness narrows and panics above the range (a harness bug, never a claim result); no claim ranges over the true `u32` domain | WX-B `U32` |
| `U8` arithmetic in the generated evaluator is non-wrapping (`V::Nat`), so every model branch had to be written so nothing wraps (`a - 128u8` only when `a >= 128u8`) | the document's meaning silently depends on an evaluator deviation from VER04 §3 | section 1.4 (conformance fix) and the WX-B differential tests |

The survey also ranks the unblocked packages (x25519, the four hashes, des,
wasm-leb128, der-tlv, protobuf, lzw/lzss/lz77/lz78, zeroize, bitset). This
specification fixes the language surface they need, in the order
**WX-B, WX-C, WX-A, WX-E, WX-H**, with design sketches for WX-F and WX-D.

| Ext | Adds | Main packages unlocked | Size | Risk |
|---|---|---|---|---|
| WX-B | `U16 U32 I8 I16 I32 I64`, explicit conversions, checked ops | gf256 (`U32` exponent, `Field::new(u16)`), wasm-leb128 (`u32`, `i64`), lz* (`u16` offsets), x25519/sha/md5 word types | S | low-medium |
| WX-C | bitwise ops, shifts, rotates, popcount/clz/ctz/bit, endian conversion | gf256 model, des, x25519 clamp, leb128 model, hash round functions | S | low |
| WX-A | tuples, records, non-recursive sum types, patterns | wasm-leb128 `(value, consumed)`, der-tlv, protobuf, resp (encode), range-coder, rans, lz* token APIs | M | medium-high |
| WX-E | external pinned known-answer tables | sha1, md5, sha512, blake2b, x25519, des, gf256 table, der/cbor reuse | M | medium |
| WX-H | `output` bound metric, parameterised bounds, allocation-capped observation | lzw, lzss, lz77, lz78 (CWE-409, CWE-789) | S-M | medium |
| WX-F | binding-level sequence drivers (sketch) | streaming digests, rans, range-coder, resp decoder | M | medium |
| WX-D | bounded fold/map/count over lists and bytes (sketch) | bitset, heap, hash reference models, leb128 general model | M | medium-high |

## 1. Versioning and compatibility (applies to every extension)

### 1.1 Version header

```text
file = "warrant" NAME [ NAME ] NEWLINE INDENT { declaration } DEDENT EOF ;
```

A document with one name after `warrant` is a **v0 document** and means exactly
what VER04 says. A document whose header is `warrant name v1` is a **v1
document**: the second NAME must be exactly `v1` (any other text is W0101) and
every extension below is enabled. A v1 construct in a v0 document is W0101
("v1 construct in a v0 document"), never ignored. The header is part of the
spec identity (VER04 §2), so adding `v1` invalidates evidence for the old
source, as any edit does.

### 1.2 Lexical and grammar compatibility

* **No new reserved words.** `record`, `variant`, `vectors`, `source`, `sha256`,
  `rows`, `column`, `holds`, `output` are *contextual* NAME tokens recognised
  only at their declaration/row position, like the metric names `steps`/`heap`.
  The reserved-word list in VER04 §2 and the `keywords:` block of
  `warrant.tokens` are unchanged. A v0 document may therefore keep a model
  called `record`.
* **New tokens never matched a valid v0 prefix.** The new literal tokens
  (`5u16`, `0x1d`, `"path"`) lexed in v0 as `INT NAME` or an unterminated quote,
  which no v0 production accepts; so no valid v0 token stream changes. This is
  checked mechanically (section 10.1).
* **No new multi-character operator tokens.** `>>` would break
  `List<List<U8>>`; `&`, `^`, `|` collide with refinements and match arms. All
  bitwise operations are named functions (WX-C).
* **Built-in names are version scoped.** The new built-ins (`widen`, `wrap`,
  `try`, `bit_xor`, `shl`, `rem`, `unwrap_or`, ...) are in scope, and reserved
  per VER04 §2, only in v1 documents. A v0 document may define a model named
  `rem` or `shl`; it keeps meaning its own definition.
* **`_` is a wildcard pattern only in v1.** In v0 `Some(_)` binds a variable
  named `_` (legal, if odd); that stays so.
* **Diagnostics** keep their IDs and meanings. New IDs are W0708–W0710
  (sections 5.4 and 10.4).

### 1.3 What "means the same" is tested by

(1) every case of `warrant-syntax-v1` keeps its verdict and witnesses; (2) the
type checker's output for `ct-compare.warrant` and `gf256.warrant` (v0
spelling) is byte-identical; (3) regenerating their evidence (examples,
exhaustive, property, fuzz, negctl, manifests) is byte-identical except where
section 1.4 fixes a documented defect, in which case the diff is reviewed.

### 1.4 Evaluator conformance fix (not an extension)

VER04 §3 already says `U8`/`U64` arithmetic wraps. The Rust evidence generator
evaluates `U8` as an exact integer (`V::Nat`), i.e. non-wrapping, which is why
`gf256.warrant` is written to avoid wrapping. That is a defect against the v0
text, fixed in the first slice (WX-B) and covered by the differential tests of
section 10.3. A v0 document written to avoid wrapping means the same under the
fixed evaluator; a v0 document that does wrap (none exists in the repository)
meant what VER04 said all along.

*Implementation note (section 14, item 1):* the fix is carried by the v1
document type. In a v1 document every fixed-width value, `U8` and `U64`
included, is a typed `V::Fixed` and wraps. The generated harness of a **v0**
document keeps its `V::Nat` emission so that the checked-in evidence of
`ct-compare` and `gf256` stays byte-identical (the instruction for the WX-B
slice); the v0 oracle is therefore still non-wrapping, and a v0 `U8`
expression that would wrap panics ("model arithmetic overflow") or silently
exceeds 255 rather than wrapping. The ground evaluator used for examples
(`codegen::Evaluator`) already wrapped `U8` and `U64`.

## 2. WX-B — fixed-width integers

### 2.1 Motivation

* **wasm-leb128**: `max_bits` is `u32`, `encode_signed(i64)`, ten-byte U64
  groups; signed LEB128 sign-extension claims need `I64` and `wrap`.
* **gf256**: `power(a, exp: u32)` (replaces the refinement hack),
  `Field::new(poly: u16)`.
* **lz\***: `u16` offsets/lengths in token streams, `u32` header counts (the
  CWE-789 header word in lz77/lz78 is a `u32`).
* **x25519, sha1/md5/sha512/blake2b, des**: `u32`/`u64` word arithmetic with
  wrap-around (CWE-190 is the bug class: a ported `+` that traps or saturates).

Newly expressible: `wrap<U8>(v)` is the low byte of `v`; `try<U32>(x)` is
`Some` exactly for in-range values; `max_value<U32>() + 1u32 == 0u32`; a
subject's `u32` length counter equals the wrapped `Nat` length.

### 2.2 Types and literals

New primitive types (ordinary NAMEs, so the grammar's `type` rule is unchanged):

| Type | Domain | Arithmetic |
|---|---|---|
| `U8`, `U16`, `U32`, `U64` | `0 .. 2^w - 1` | modulo `2^w` |
| `I8`, `I16`, `I32`, `I64` | `-2^(w-1) .. 2^(w-1) - 1` | two's complement, modulo `2^w`, result re-read as signed |

Literal tokens (added to `warrant.tokens` after `BYTES_LITERAL` and before
`INT`; the `u8` and `u64` ones already exist):

```text
U16_LITERAL = /(?:0|[1-9][0-9]*)u16/      U32_LITERAL = /(?:0|[1-9][0-9]*)u32/
I8_LITERAL  = /(?:0|[1-9][0-9]*)i8/       I16_LITERAL = /(?:0|[1-9][0-9]*)i16/
I32_LITERAL = /(?:0|[1-9][0-9]*)i32/      I64_LITERAL = /(?:0|[1-9][0-9]*)i64/
HEX_LITERAL = /0x[0-9a-f]+(?:u8|u16|u32|u64|i8|i16|i32|i64)?/
```

Rules: a suffixed literal must be in range of its type (`256u8`, `32768i16`
are W0101; a suffix never reduces). Negative signed values are written with
unary minus; the single exception is the minimum: a signed literal whose
magnitude is exactly `2^(w-1)` (`128i8`) is accepted *only* as the direct
operand of unary minus, so `-128i8` is legal and `128i8` alone is not. A hex
literal is lowercase, has at least one digit, may have leading zeros (width is
visible in hex), and without a suffix has type Int (Nat-widenable, like a
decimal literal); with a suffix it is range checked like decimal.

### 2.3 Typing rules

Let `F` be the eight fixed-width types, `U` the unsigned ones, `S` the signed ones.

| Form | Operands | Result | Semantics |
|---|---|---|---|
| `a + b`, `a - b`, `a * b` | `T, T` with `T ∈ F` | `T` | **wrapping**: exact result mod `2^w`, signed re-read as two's complement. Never traps, never saturates. |
| `- a` | `S` | `S` | wrapping negation; `-min_value<S>() == min_value<S>()` |
| `- a` | `U` | rejected (W0301) | write `0uN - a`; unary minus on an unsigned is almost always a bug |
| `<`, `<=`, `>`, `>=` | `T, T` | Bool | unsigned by value, signed by signed value |
| `==`, `!=` | same `T` | Bool | structural |
| mixing `T1 != T2`, or `T` with Int/Nat | | W0301 | **no implicit conversion in either direction**: Nat/Int never widen to a fixed type, fixed types never widen to Int |

**Wrapping is the language semantics; checked is explicit.** This matches v0
(`U8`/`U64` already wrap) and makes a model's `a + b` equal Rust's
`a.wrapping_add(b)`, C unsigned arithmetic and Java `int` arithmetic. A subject
that panics on overflow (Rust debug), saturates, or traps is *detected* by a
claim `add32(a, b) == a + b`, which is the point. Where the specification wants
overflow to be an error, it uses the checked forms below and a `Result` op.

Conversion and utility built-ins, all total. `T` is a *static type argument*
written in the existing `name<...>(...)` reference form (the grammar already
accepts a NAME in the angle brackets); only `T ∈ F` is admitted:

| Form | Operands | Result and total semantics |
|---|---|---|
| `widen<T>(x)` | `x: F`, range(typeof x) ⊆ range(T) | `T`, value preserved. `widen<U32>(x: U8)` ok; `widen<U32>(x: I8)` is W0301 (range not contained) |
| `wrap<T>(x)` | `x: F`, Int or Nat | `T`; mathematical value reduced mod `2^w`, then read in T's range (`wrap<I8>(255u8) == -1i8`, `wrap<U8>(-1) == 255u8`). The only truncating/reinterpreting conversion |
| `try<T>(x)` | `x: F`, Int or Nat | `Option<T>`; `Some` exactly when the mathematical value is in T's range |
| `to_int(x)` | `x: F` | `Int`, the mathematical value |
| `to_nat(x)` | `x: U` | `Nat`, the mathematical value (a signed value reaches Nat only through `try`/`wrap`, never implicitly) |
| `max_value<T>()`, `min_value<T>()` | | `T`; `min_value<U>() == 0uN` |
| `checked_add(a,b)`, `checked_sub`, `checked_mul` | `T, T` | `Option<T>`; `None` iff the exact result is outside range |
| `div(a, b)`, `rem(a, b)` | `U, U` | `Option<U>`; `None` iff `b == 0` |
| `div_trunc(a, b)`, `rem_trunc(a, b)` | `S, S` | `Option<S>`, truncating toward zero; `None` iff `b == 0`, or `b == -1 and a == min_value<S>()` (the one overflowing case; checked-style) |
| `rem(a, b)` | Int/Nat | `Option<Int>`; floor modulo (sign of the divisor, nonnegative for `b > 0`); `None` iff `b == 0` |
| `div(a, b)` | Int/Nat | unchanged from v0 (`Option<Int>`, floor) |
| `div`, `rem` | `S` | rejected (W0301): floor versus truncation is a real portability trap (Python/Ruby floor; C/Rust/Go/Java truncate), so a signed model must say which it means |
| `unwrap_or(o, d)` | `Option<T>, T` | `T`; `d` (pure) is returned for `None`. Needed by bound expressions (WX-H) |

`wrap<T>`/`try<T>` from Int make the old `x - (x div 128) * 128` idiom
unnecessary. `Int` arithmetic stays exact and unbounded; an evaluator that
cannot represent a result within its budget reports W0703, never a wrapped value.

Quantifier binders may have any type in `F`.

### 2.4 Grammar additions

No production changes beyond the literal tokens above. `type` already accepts
NAME; `reference = qualified_name [ LT widths GT ] [ LPAREN ... RPAREN ]` with
`width = INT | NAME`, so `widen<U32>(x)` and `max_value<I8>()` parse today as a
static application. The checker, not the parser, resolves the static argument
as a type in these built-ins (and as a Nat everywhere else, as in v0). The
`f<T>(x)` versus comparison ambiguity is resolved exactly as for `eq_fixed<N>`
in v0: a NAME followed by `<` is a static application only when it names a
callable that takes static arguments; comparisons cannot chain, so no valid v0
text changes meaning.

### 2.5 Evaluation semantics

The ground evaluator's `Value` gains `Fixed(type, i128)` holding the canonical
in-range representative (so equality is integer equality; `U64` and `I64` both
fit `i128`).

* add/sub: compute in `i128`, reduce `rem_euclid(2^w)`, map to the signed range for `S`.
* mul: for `w = 64`, multiply the two's-complement bit patterns with
  `u128::wrapping_mul` and mask (an `i128` product of two `U64` overflows);
  narrower types multiply exactly first.
* `Int` operations use checked `i128`; overflow is W0703 (incomplete checking),
  never a wrapped value (`to_int(U64) * to_int(U64)` can reach this).
* Each arithmetic/conversion node charges 1 work unit (VER04e §2.3).

Generated Rust model code uses the native `wrapping_*` method of the matching
Rust type and never `+` on a fixed-width native type (debug builds would
panic, release would wrap, and the two must not differ). Other lanes: section 2.8.

### 2.6 Evidence-generation impact (VER04h)

**Generators (VER04f §3 extended).** `U16 → Gen::u16_gen()` and so on; each new
type draws 3/4 uniformly over the full range and 1/4 uniformly from its edge
domain, so boundaries are reached without exhaustive enumeration. The v0
`U8`/`U64` generators are unchanged (byte-identical output).

**Shrinkers.** Unsigned: toward `0` by bisection then linear step. Signed:
toward `0` from both sides (try `-x`, then bisect `|x|`, then step).

**Domain descriptors** (extends the VER04h §3 table). A type has `full` (every
value, only where small), `edge`, or both:

| Binder type | `full` | `edge` |
|---|---|---|
| `U8` (v0, unchanged) | all 256 | n/a (never demoted) |
| `U64` (v0, unchanged) | n/a | the 377-value set of VER04h §3 |
| `U16` | all **65,536** | 281 values |
| `U32` | n/a (4,294,967,296 is never enumerated) | 313 values |
| `I8` | all 256 | n/a (full is small) |
| `I16` | all **65,536** | 527 values |
| `I32` | n/a | 559 values |
| `I64` | n/a | 623 values |

For unsigned width `w`: `E_U(w) = {0..=255 ∩ range} ∪ {1<<i : 8 <= i < w} ∪ {MAX}
∪ {MAX ^ (1<<i) : 0 <= i < w}` (for `w = 64` this is exactly v0's 377 values;
`U16` is 256+8+1+16 = 281; `U32` is 256+24+1+32 = 313). For signed width `w`:
`E_S(w) = wrap_S(E_U(w)) ∪ {-1 ..= -255 ∩ range}`, deduplicated, ascending
numeric order (`I16` 527, `I32` 559, `I64` 623). Orders are fixed so output is
byte-stable.

**Budget with full domains.** One `U16` binder is 65,536 cases; two are `2^32`.
The VER04h budget `B = EXHAUSTIVE_BUDGET` (400,000 in the reference profile; a
package may raise it through `EvidenceOptions::exhaustive_budget`, as the gf256
document does for its `2^24`-case `U8^3` claims) is applied with the same
checked arithmetic by this deterministic rule:

1. every binder starts at `full` where it has one, otherwise `edge`;
2. while the product (including `Bytes`/`Nat`/other binders) exceeds `B`, demote
   the leftmost binder that is currently `full` and of type `U16` or `I16` to its
   `edge` domain;
3. if the product (or the byte budget) still exceeds `B`, generation fails with
   W0706 exactly as in v0. No silent sampling; `U8` binders are never demoted, so
   v0 behaviour is unchanged.

The manifest `domain` text states the outcome per binder, for example
`a: U16 edge(281); b: U16 full(65536)`, plus `demoted: ["a"]` (the rule demotes
the *leftmost* full `U16`/`I16` binder first, so a `U16 x U16` claim at the
raised budget `B = 2^26` demotes only `a`, and at the reference `B = 400,000`
demotes `a` and then `b`: `65536*65536 -> 281*65536 -> 281*281 = 78,961`). The
binder kinds are `full` (every value), `edge` (the biased boundary set),
`count` (the shift-count domain of section 3.6) and `set` (the fixed finite
sets of `Bool`, `Nat`, `Bytes`). The honest statement stays "exhaustive over
<domain>", never "all inputs".

### 2.7 Sabotage implications (VER04h §6)

New faults are injected in the adapter (the crate is never modified); a control
passes iff some suite for the operation fails under it:

| Fault | Applies to | Models |
|---|---|---|
| `saturate-add` `saturate-sub` `saturate-mul` | `+ - *` shaped ops on `T ∈ F` | saturating instead of wrapping arithmetic |
| `trap-as-zero-add` `-sub` `-mul` | same | overflow returns 0 |
| `narrow-width` | ops whose native type is `U32`/`U16`/... | the adapter computes one width narrower |
| `sign-confuse` | comparison/order ops on `S` | signed value compared as unsigned |
| `off-by-one-range` | `try`-shaped ops | accepts `max+1` / `min-1` |

The fault names carry the operator because an adapter is told what the
operation *is* only through its binding (an adapter cannot know whether an
export is an addition or a subtraction); `saturate` and `trap-as-zero` without a
suffix are not faults.

The edge domains guarantee `0`, `1`, `max`, `min` and `-1` (signed), and
`max-1` (unsigned), are present. (An earlier draft also promised `max-1` and
`min+1` for signed types; the normative sets do not contain them, and
`wrap_S(E_U(w))` cannot.) The `off-by-one-range` fault needs the *source*
domain of a `try<T>` to contain `T.max + 1` and `T.min - 1`; every wider source
of the same family does (`I32` holds `256` and `-1` for `try<U8>`, `I64` holds
`2^32` and `-1` for `try<U32>`), so each fault above is catchable by the
bounded-exhaustive suite by construction. The property and fuzz suites reach a
boundary only with probability about `1/4 * 2/|edge|` per draw, so a
`try`-shaped claim should carry a refinement that confines the source to the
neighbourhood of the range (`| x >= -300i16 and x <= 300i16`) if its negative
control must be caught by every suite kind.

### 2.8 Cross-language portability (VER04i corpus encoding)

The corpus transport already carries every value: TFV `I<int>` is arbitrary
precision to 1000 digits and negatives are `I-128`; no new TFV token is needed.
Requirements on VER04i for fixed-width fields:

1. **The record schema, not the value, carries the type.** A TFV integer is
   untyped; the corpus record declares each field's Warrant type, and a decoder
   range-checks (`I256` in a `U8` field is `corpus-malformed`, not a wrap).
2. **Mandatory boundary rows** for every fixed-width type a document uses:
   `0, 1, MAX-1, MAX` (unsigned); `MIN, MIN+1, -1, 0, 1, MAX-1, MAX` (signed);
   and for types wider than 32 bits also `2^53 - 1, 2^53, 2^53 + 1` (double
   precision loss) and `2^63 - 1, 2^63, 2^63 + 1` (signedness flip).
3. **Lane hazards**; each lane's decoder and adapter must pass the boundary rows:

| Lane | Hazard | Required handling |
|---|---|---|
| JavaScript/TypeScript | Number exact only to 2^53; bitwise ops are signed 32-bit | `U64`/`I64` as `BigInt`; `U32` via `>>> 0`; `I32` via `\| 0` |
| Dart | on the web target `int` is a double | `BigInt`/`Int64` for 64-bit types, or `not-live` for 64-bit |
| Lua | 5.3+ integers are signed 64-bit wrapping; 5.1/5.2/LuaJIT are doubles | `U64` compare via `math.ult`; refuse or emulate on double-only runtimes |
| OCaml | native `int` is 63-bit | `Int32`/`Int64` modules; unsigned via `Int64.unsigned_*` |
| Java | no unsigned primitives | `Integer/Long.compareUnsigned/divideUnsigned/remainderUnsigned`; `U8`/`U16` held wider and masked |
| C/C++ | signed overflow and over-wide shifts are UB | arithmetic on unsigned types with explicit conversion to signed; shifts guarded by the WX-C rule |
| Python/Ruby/Elixir/Perl | unbounded (or IV-width) integers | explicit masking after every operation |

A lane that cannot represent a type exactly marks the dependent targets
`not-live`; it never substitutes a narrower type.

### 2.9 Compatibility

`U8`/`U64` semantics are unchanged. The names `U16 U32 I8 I16 I32 I64` are
builtin types only in v1 documents; in v0 they remain unbound names (W0202).

### 2.10 Test plan (WX-B)

* **Syntax corpus** (section 10.1): accept each suffix and `0x`; reject `256u8`,
  `65536u16`, `128i8` alone, `-129i8`, `01u16`, `0xU8`, uppercase hex, `1_000`,
  suffix `u128`.
* **Type-checker negatives:** `1u8 + 1u16`; `x + 1` with `x: U32`; `-a` for
  `a: U8`; `widen<U8>(x: U16)`; `widen<U32>(x: I8)`; `div(a: I32, b: I32)`;
  `try<Int>(x)`; `max_value<Nat>()`; `to_nat(x: I8)`; `I8` compared with `U8`.
* **Evaluator differential** (section 10.3).

## 3. WX-C — bitwise operations

### 3.1 Motivation

`add = XOR` in gf256, the `0x1D` reduction (`xtime`), x25519 scalar clamping
(`a[0] & 248`, `a[31] & 127 | 64`), des parity-bit insensitivity and the
complementation property, LEB128 7-bit groups and continuation bits, and every
hash round function (`rotl`, `bit_xor`, `bit_and`, `bit_not`). Newly
expressible: "operation equals this bit-level model" with a two-line model
instead of an 8-layer XOR, and `x25519(clamp(a), u) == x25519(a, u)`.

`gf256.warrant`'s `xor8` and `xtime` become:

```text
model xtime(x: U8) -> U8 = if x >= 128u8 then bit_xor(shl(x, 1), 0x1du8) else shl(x, 1)
claim add_is_xor:
  forall a: U8, b: U8. add(a, b) == bit_xor(a, b)
```

### 3.2 Built-ins (all total; `T ∈ F`; named functions, no new operator tokens)

| Form | Operands | Result | Semantics |
|---|---|---|---|
| `bit_and(a, b)`, `bit_or(a, b)`, `bit_xor(a, b)` | `T, T` | `T` | per bit of the width-`w` two's-complement pattern |
| `bit_not(a)` | `T` | `T` | flip all `w` bits; `bit_not(0u8) == 255u8`, `bit_not(0i8) == -1i8` |
| `shl(a, n)` | `T, Nat` | `T` | pattern shifted left, zero fill, high bits dropped; **`n >= w` gives `0`** |
| `shr(a, n)` | `T, Nat` | `T` | **logical** right shift of the pattern, zero fill; `n >= w` gives `0`. On `S` the pattern is shifted, then re-read signed (`shr(-1i8, 1) == 127i8`) |
| `sar(a, n)` | `S, Nat` | `S` | **arithmetic** right shift, sign fill; `n >= w` gives `-1` if `a < 0` else `0`. Rejected on `U` (W0301): a logical shift is spelled `shr` |
| `rotl(a, n)`, `rotr(a, n)` | `T, Nat` | `T` | rotate by `n mod w` (`rotl(a, w) == a`); `rotr(a, n) == rotl(a, (w - n mod w) mod w)` |
| `popcount(a)` | `T` | `Nat` | number of 1 bits of the pattern; `popcount(-1i8) == 8` |
| `clz(a)`, `ctz(a)` | `T` | `Nat` | leading/trailing zero bits of the pattern; both `w` for zero |
| `bit(a, i)` | `T, Nat` | `Bool` | bit `i` (LSB = 0); `false` for `i >= w` |

**Total semantics for shift counts.** A shift count is a mathematical `Nat`
(not a fixed-width value), so there is no count type to overflow, and the
result for every `n >= w` is defined without reference to the host. This
*differs on purpose* from hardware masking (x86 masks the count modulo the
width; C leaves it undefined; Rust's `<<` panics in debug builds while
`wrapping_shl` masks). A subject using any of those is checked against the
total model, and a mismatch at `n >= w` is a finding (the classic bug in
decoders such as LEB128, where a hostile continuation chain drives a shift past
the width). A specification that *wants* the masked behaviour says so:
`shl(a, unwrap_or(rem(n, 32), 0))`.

Truth table (`w = 8`): `shl(0x81u8, 1) = 0x02`, `shl(1u8, 7) = 128`,
`shl(1u8, 8) = 0`, `shr(0x81u8, 1) = 0x40`, `sar(-128i8, 1) = -64i8`,
`sar(-128i8, 8) = -1i8`, `sar(100i8, 9) = 0i8`, `rotl(0x81u8, 1) = 0x03`,
`rotl(0x81u8, 9) = 0x03`, `rotr(0x03u8, 1) = 0x81`, `popcount(0xffu8) = 8`,
`clz(0u8) = 8`, `ctz(0x80u8) = 7`, `bit(5u8, 8) = false`.

### 3.3 Endian conversion

Needed by every hash and by LEB128/DER length fields; placed here because it is
bit-pattern work:

| Form | Type | Semantics |
|---|---|---|
| `be_bytes(x)`, `le_bytes(x)` | `x: T ∈ F` → `Bytes<w/8>` | big/little-endian two's-complement bytes (`be_bytes(0x01020304u32) == b"01020304"`, `le_bytes(-2i16) == b"feff"`) |
| `from_be<T>(b)`, `from_le<T>(b)` | `b: Bytes<w/8>` → `T` | inverse. A dynamic `Bytes` is W0301 (use `fixed<N>` first), so there is no implicit truncation |

### 3.4 Grammar

None beyond WX-B: every form is a call. Operand-type errors (`bit_xor` of `U8`
and `U16`; `shl(a, 1u8)` because the count must be `Nat`; `sar` on `U`;
`popcount` on `Nat`) are W0301.

### 3.5 Evaluation semantics

Operate on the canonical `u128` bit pattern masked to `w` bits. `shl`/`shr`
with `n >= w` short-circuit to the result above *before* any host shift (a host
shift by at least the host word width panics or is undefined in some lanes and
must never be reached). `popcount` is `count_ones` of the masked pattern. Each
node charges 1 work unit (shifts and `popcount` charge `ceil(w/8)`).

### 3.6 Evidence-generation impact

* **Count-aware `Nat` domain.** A `Nat` binder that flows *directly* as the
  count argument of `shl shr sar rotl rotr bit` gets the exhaustive domain
  `0..=64 ∪ {65, 127, 128, 129, 255, 256, 65535, 65536}` instead of `0..=64`, so
  `n = w` and `n = w + 1` are present for every width including 64 (v0's
  `0..=64` stops at `w` for 64-bit types). The property generator draws such a
  `Nat` from `0..=130` with probability 1/2. The manifest `domain` text records it.
* Relation-biased pairs (VER04h §4) already include "one bit flipped"; for these
  ops the bit-flip branch applies to every width in `F`.

### 3.7 Sabotage implications

| Fault | Applies to | Models |
|---|---|---|
| `shift-mask` | `shl shr sar` ops | count reduced modulo `w` (x86 / `wrapping_shl`) |
| `shift-ub` | same | count `>= w` returns the value unshifted (a C UB outcome) |
| `rotate-as-shift-left` `rotate-as-shift-right` | `rotl` / `rotr` | zero fill |
| `sar-as-shr` | `sar` | zero fill on negatives |
| `popcount-off` | `popcount` | counts only the low `w-1` bits |
| `endian-swap` | `be_bytes le_bytes from_*` | opposite byte order |

### 3.8 Cross-language portability

The adapter must implement the total rule and never rely on the host:
JavaScript `<<` masks the count modulo 32; Lua `<<` yields 0 for counts `>= 64`
and reverses direction for negative counts (counts here are `Nat`, so never
negative); C/C++ undefined beyond width and on negative operands; Java masks the
count (`& 31`/`& 63`); Python/Ruby/Elixir are arbitrary precision (mask the
result); OCaml `lsl` on the 63-bit `int` is not a 64-bit shift. Rotation is
composed from guarded shifts in lanes without an intrinsic. The corpus requires,
per width, count rows `{0, 1, w-1, w, w+1, 2w, 255}`. `popcount` of a negative
signed value operates on the width-`w` pattern, not the host's sign-extended
word (Java `Long.bitCount` of a sign-extended `byte` is the classic mistake;
the corpus row `popcount(-1i8) == 8` catches it).

### 3.9 Compatibility

Pure addition of built-ins, in scope only in v1 documents; the names avoid the
Boolean keywords `and or not`.

### 3.10 Test plan (WX-C)

* **Evaluator differential:** all ops over all of `U8`/`I8` and every shift
  count `0..=300`; `E x E` plus random pairs for wider types; versus Rust native
  (`rotate_left`, `count_ones`, `leading_zeros`, `checked_shl` normalised) and
  the Python reference. Metamorphic identities: `rotl(rotr(a,n),n) == a`,
  `bit_not(bit_not(a)) == a`, De Morgan, `from_be<T>(be_bytes(x)) == x`,
  `popcount(bit_xor(a,b)) mod 2 == (popcount(a) + popcount(b)) mod 2`.
* **Type negatives:** listed in 3.4.
* **Re-conversion check:** `gf256.warrant` rewritten with `bit_xor`/`shl` keeps
  every example and claim verdict, and its negative controls still fail.

## 4. WX-A — tuples, records and non-recursive sum types

### 4.1 Motivation

* **wasm-leb128**: `decode_unsigned(data, offset) -> Result<(U64, Nat), Leb128Error>`;
  claims `decode_unsigned(encode_unsigned(v), 0) == Ok((v, len(encode_unsigned(v))))`,
  canonical-form uniqueness, overlong/truncated rejection, and `consumed` pinned
  on every success.
* **der-tlv**: an element record (tag class, constructed flag, tag number, value
  bytes) and a limits record parameter; `decode_exact` result.
* **protobuf** (varint pair), **resp** (encode side), **range-coder**/**rans**
  (probability/state pairs), **lz\*** token APIs (`Token(offset, length, next_char)`).

### 4.2 Types

```text
type  += "(" type "," type { "," type } ")"        -- tuple, arity 2..8
record_decl  = "record" NAME "(" field { "," field } ")" NEWLINE ;
variant_decl = "variant" NAME "=" ctor "|" ctor { "|" ctor } NEWLINE ;
ctor   = NAME [ "(" field { "," field } ")" ] ;
field  = NAME ":" type ;
```

* **Tuple** `(T1, ..., Tn)`: structural, `2 <= n <= 8`; two tuple types are equal
  iff componentwise equal. No unit and no 1-tuple (`(e)` stays parenthesisation).
* **Record** `record Name(f1: T1, ..., fn: Tn)`: nominal, `1 <= n <= 32`, field
  names unique. It is sugar for a variant with the single constructor `Name`.
* **Variant** `variant Name = C1(fields) | C2 | ...`: nominal sum with at least
  two and at most 256 constructors. Constructor names are module-level, unique
  across all variants and records and the callable namespace (VER04 §2); a binder
  may not equal a constructor name (W0203). Error types (`error`) are unchanged:
  finite, payload-free, qualified `E.CASE`.
* **No recursion.** The relation "type mentions type" must be acyclic through
  tuples, records, variants, `List`, `Option` and `Result` alike (W0203 on a
  cycle). This keeps domains finite per size and evaluation total; a recursive
  type (a tree, `CborValue`) is out of scope (section 9).
* The 64-node nesting path limit and 4,096-declaration limit of VER04 §2 apply.

`Result<T, E>` keeps its v0 rule: `E` is a declared payload-free `error` type;
`T` may now be a tuple, record or variant. Error *data* (DER's byte offset) is
exposed by a second observer op (`op error_offset(input: Bytes) -> Option<Nat>`);
allowing payload-carrying errors is an open question (section 12).

### 4.3 Expressions

```text
primary   = atom { PROJ } ;                         -- postfix projection
atom     += tuple_literal ;
tuple_literal = LPAREN expression COMMA expression { COMMA expression } RPAREN ;
PROJ      = DOT_ADJ ( INT | NAME ) ;                -- a DOT with no whitespace before the INT/NAME
pattern   = "_" | NAME | "true" | "false" | tuple_pattern | ctor_pattern ;
tuple_pattern = LPAREN pattern COMMA pattern { COMMA pattern } RPAREN ;
ctor_pattern  = ( "Some" | "Ok" | "Err" | "None" | NAME | QNAME ) [ LPAREN pattern { COMMA pattern } RPAREN ] ;
match_arm = pattern ARROW expression ;              -- nested patterns allowed, depth <= 8
```

* **Construction** is positional: `Tlv(0u8, false, 2u32, b"00")`; a nullary
  constructor is a bare name. No named arguments, defaults or record update.
* **Projection.** A tuple component is `t.0`, `t.1`, ...; a record field is
  `r.field`. `x.f` with a binder head lexes as the existing `QNAME`; the checker
  resolves a QNAME whose head is a binder as projection (and an error-constructor
  head otherwise). `f(x).0` and `f(x).field` use the adjacent-dot postfix `PROJ`.
  The quantifier separator `.` must be followed by whitespace (VER04 §5), which
  keeps the two apart, so `forall a: T. e` is unaffected. Projecting a variant
  that is not a record is W0301; use `match`.
* **Equality** is structural and type directed: same nominal type, same
  constructor, equal fields; tuples componentwise; `Bytes` by content. Ordering
  comparisons are not defined on these types.
* **Patterns.** `_` binds nothing and may repeat. A NAME that resolves to a
  constructor is a constructor pattern, any other NAME a fresh binder (VER04 §2
  shadowing rules apply). `Some(NAME)` generalises to `Some(pattern)`, so
  `Ok((v, n))` works. There are no integer or bytes literal patterns (`if` and
  equality cover them, and it avoids a literal exhaustiveness theory).
* **Exhaustiveness** is the standard pattern-matrix usefulness check over finite
  constructor sets (Bool, Option, Result, variants, tuples, records). A
  non-exhaustive match, and an arm made redundant by earlier arms, are both
  W0301. The check is charged work (arms x constructors) and bounded to 256 arms
  per match and pattern depth 8 (W0602 on exhaustion).

### 4.4 Typing

`(e1, ..., en) : (T1, ..., Tn)`; `Name(e...) : Name` with positional,
type-checked arguments (arity mismatch W0301; `Nat <= Int` and
`Bytes<N> <= Bytes` apply per argument as in v0); `t.i : Ti` for a literal `i`
in range; `r.f : T_f`; `match` arms need a common type under the v0 least upper
bound. Subtyping is **not** lifted through composites: `(Nat, Nat)` is not a
subtype of `(Int, Int)`; widen components explicitly.

### 4.5 Evaluation semantics

`Value` gains `Tuple(Vec<Value>)` and `Con(type, ctor_index, Vec<Value>)`
(records are `Con` with index 0). Construction, projection and match charge 1
unit plus one per field touched; values count toward the 64 MiB owned-data
budget. Matching tries arms top to bottom; the exhaustiveness check guarantees
one matches.

### 4.6 Bindings (VER04d §5 schema additions; normative requirements)

The closed binding schema gains, per composite type in an `op` signature, a
`host_type` and how each constructor/field is built from and observed into the
host (a Rust `(u64, usize)` tuple, a struct with named fields, an enum). VER04
§6 extends verbatim: every bound host composite needs construction/observation
round trips, distinct generated values and an observer that distinguishes them,
so a constant generator or a `drop-field` adapter fails a control. A `Nat`
component bound to a host `usize`/`u32` declares its host width; the generator
never produces a value the host cannot represent (such values are counted as
*unreachable by type*, not narrowed; this is the gf256 `u32` harness-panic
lesson). Labels extend to field paths (`labels decode_exact: limits.max_len public`;
QNAME paths already lex); `input op.param` stays per parameter and its
refinements may mention fields.

### 4.7 Evidence-generation impact

**Domain descriptors** (VER04h §3 extended), all deterministic:

| Type | Domain |
|---|---|
| tuple / record | Cartesian product of component domains, lexicographic in component order |
| variant | concatenation, in declaration order, of the product of each constructor's field domains |
| `Option<T>` | `None` then `Some(x)` for `x ∈ D(T)`; size `1 + |D(T)|` |
| `Result<T,E>` binder | `Ok(x)` for `x ∈ D(T)`, then each `E` case; size `|D(T)| + |E|` |
| `List<T>` binder | every list of length 0..=2 over `D'(T)` (the first 16 elements of `D(T)`), plus length-3 lists over its first 4 elements; recorded in the manifest |

Sizes multiply into `B` and the 64 MiB domain-byte budget (a record with a
`Bytes` field costs the payload per element). A binder whose domain alone exceeds
`B` is W0706. Example: `(U64, Nat)` is `377 x 65 = 24,505` elements.

**Generators.** Components are drawn independently, constructors uniformly (so
rare constructors are reached), with VER04h §4 relation bias acting on one *leaf*
component ("one component changed", "one bit flipped") when two binders share a
type. Refinements are joint filters with `max_retries = 100`.

**Shrinkers.** Component-wise with each component's own shrinker; variants shrink
to an earlier constructor with minimal fields, then field-wise; `Option` toward
`None`; `List` as in v0.

**Manifests.** `domain` text gains `tuple(...)`, `record Name(...)` and
`variant Name[k ctors]`; `unreachable_by_type` counts appear for narrower host types.

### 4.8 Sabotage implications

| Fault | Applies to | Models |
|---|---|---|
| `drop-consumed` | tuple results | second component replaced by `0` |
| `swap-components` | homogeneous tuples | components exchanged |
| `field-zero` | record results | first numeric field replaced by `0` |
| `wrong-ctor` | variant results | constructor replaced by its predecessor |
| `ok-as-err` | `Result` of a composite | success reported as the first error case |
| `off-by-one-consumed` | `(value, consumed)` pairs | `consumed + 1` / `- 1` |

The last is the principal LEB128 bug class: right value, wrong bytes consumed.

### 4.9 Cross-language portability (VER04i corpus encoding)

TFV already has the needed shapes. The encoding below is a requirement on
VER04i, which owns the corpus encoding (VER04i section 2.7). VER04i may rename
but not change the information content:

* **tuple**, **record**: `L<n> v1 ... vn`, components in declaration order (field
  names are not transported; the corpus record schema carries them);
* **variant**: `L2 S<hex ctor name> L<k> f1 ... fk`, by constructor *name* so
  reordering declarations cannot change a vector;
* **Option**: `L0` or `L1 x` (agrees with VER04i section 2.7). **Result**:
  `L2 S<hex "Ok"|"Err"> v` **conflicts with VER04i section 2.7**, which fixes
  `Result` as an `M` with key `ok` or `err` (see section 12a, item 1);
* depth: nesting counts against TFV's 64-container limit and is rejected at
  generation (W0709).

Lanes without tuples or sum types bind a result as a struct, out-parameters or a
tagged struct; that is a binding concern. The constructor *name* is the portable key.

### 4.10 Compatibility

Pure addition: the new productions appear only in v1 documents. `match` on
`Option`/`Result` with `Some(x)`/`Ok(x)` in a v0 document parses and means the
same under the generalised pattern rule (a bare NAME is still a binder).

### 4.11 Test plan (WX-A)

* **Syntax corpus:** tuple type/literal, nested tuple, record/variant
  declarations, `t.0`, `f(x).0`, `r.f`, nested patterns, `_`; rejects: 1-tuple,
  empty tuple, arity 9, `t.00`, `t. 0`, `variant V = A` (one constructor),
  duplicate constructor.
* **Type negatives:** arity mismatch; wrong field type; `t.2` of a pair;
  projecting a variant; recursive record (W0203); non-exhaustive and redundant
  match; constructor name equal to a binder; `(Nat, Nat)` where `(Int, Int)` is
  expected; tuple compared with `<`.
* **Exhaustiveness differential:** the matrix checker versus brute-force
  enumeration over small finite types (Bool x Option<Bool> x 3-constructor
  variant) for every generated pattern set up to four arms.
* **Domain tests:** sizes equal the formulas; first and last element of each
  ordering pinned; every constructor appears.
* **Pilot:** a wasm-leb128 document with the `off-by-one-consumed` control failing.

## 5. WX-E — external known-answer tables

### 5.1 Motivation

Hashes have no v0 reference model (no recursion; WX-D is later) so their evidence
is known answers. NIST CAVS/SHAVS files have thousands of vectors, DES has NIST
SP 800-17, x25519 has RFC 7748. A thousand `example` blocks bloat a specification
and invite transcription error. A table keeps the *data* in a TFV fixture while
the *specification* pins its identity and states what each row must satisfy.

### 5.2 Declaration

```text
vectors_decl = "vectors" NAME ":" NEWLINE INDENT
               "source" PATH_LITERAL NEWLINE
               "sha256" BYTES_LITERAL NEWLINE
               "rows" INT NEWLINE
               column { column }
               "holds" expression NEWLINE DEDENT ;
column       = "column" NAME ":" type NEWLINE ;
PATH_LITERAL = /"[A-Za-z0-9_.\/-]+"/ ;
```

`PATH_LITERAL` exists *only* in this rule (no string type reaches expressions)
and has no v0 antecedent. Example:

```text
vectors sha256_cavs_short:
  source "sha256-cavs-v1/short.tfv"
  sha256 b"<64 lowercase hex digits: SHA-256 of the file>"
  rows 65
  column msg: Bytes
  column md: Bytes<32>
  holds sha256_digest(msg) == md
```

Rules:

* **Path** is relative to `code/specs/fixtures/`, at most 256 bytes, no leading
  `/`, no `.` or `..` segment, no empty segment, and must resolve to a regular
  file (no symlink), at most 16 MiB (W0708). Documents contain no host paths
  (VER04 §6); the fixtures root is a tool-profile constant.
* **Pin.** `sha256` is the SHA-256 of the file's exact bytes as a `Bytes<32>`
  literal. The tool hashes the file *before* parsing it; a mismatch is W0708 and
  no row is used. Changing the data therefore requires editing the warrant,
  which changes `spec_sha256` and invalidates evidence (VER04h §9).
* **Format** is TF01 TFV: one record per row, id `[A-Za-z0-9._/-]+`, one field per
  `column` (same names), values decoded by the column's Warrant type (`I<int>`
  range-checked, `X<hex>` for `Bytes`, lists for tuples/records per 4.9). The
  record count must equal `rows`; no field may be missing or extra (W0708/W0710).
  Legacy fixtures (`sha512-v1/cases.txt`) are projected to TFV by a stdlib-only
  script with a drift check, as TF01 does for JSON corpora.
* **`holds`** is a Bool expression whose free variables are exactly the columns,
  evaluated per row with columns bound as ground values. It may call operations
  and models and must call at least one operation (a table that never touches the
  subject is vacuous, W0601). `Result`/`Option`/tuple columns let expected errors
  be data.
* **Budget.** Rows are data, not generated instances, and do not count toward the
  10,000-instance document limit; each row runs under the per-case work/stack
  budgets of VER04. At most 100,000 rows per table and 250,000 per document
  (tool-profile constants; exhaustion is W0602).
* Table names share the test-name namespace with `example` and `claim`.
* Provenance (source URL, licence, generation procedure) is a fixture README
  concern; the trust anchor is the pin plus review of the fixture.

### 5.3 Evidence-generation consumption

1. **Materialisation.** The generator copies the verified fixture into the
   package as `tests/generated/tables/<name>.tfv` (byte-identical; a crate must
   not `include_bytes!` outside its own tree or it cannot be published) and emits
   its SHA-256 as a constant. The generated test hashes the copy at start and
   fails `corpus-malformed` on mismatch. Regeneration must reproduce the copy.
2. **Execution.** One L0 case per row, id `vectors/<name>/<row id>`, calling the
   real export through the adapter (panic normalisation as for examples). Rows are
   read with the lane's TF01 TFV parser; no new reader.
3. **Manifest.** One entry per table, not per row: `kind: "table"`,
   `id: warrant/<module>/table/<name>`, `cases`, `table_sha256`, `rows_pinned`,
   `verdict`, and on failure `failed_ids` (first 16; redacted in CI mode per
   VER00 §7.3). The report counts table rows as examples.
4. **Negative controls.** Each applicable VER04h §6 fault runs against the table;
   the control passes iff at least one row fails. A table that no fault can fail
   is W0601 ("table is not discriminating"), checked at generation time.
5. Properties, exhaustive and fuzz suites do not apply to a table; its rows may
   seed the fuzz corpus of the same operation (optional, recorded).

### 5.4 Diagnostics

| ID | Trigger |
|---|---|
| W0708 | Table integrity: bad/absent path, not a regular file, size limit, digest mismatch, row-count mismatch, malformed TFV |
| W0709 | Unsupported or unrepresentable shape for a target (for example TFV nesting beyond 64) |
| W0710 | Column/type mismatch: missing or extra field, value outside the column type's range, undecodable type |

### 5.5 What this is not

It is not an oracle: a table cannot *derive* an answer, only pin data authored
independently. A second implementation used as a differential oracle is a
different construct with a larger trusted base and is not specified here.

### 5.6 Compatibility and portability

Pure addition; `vectors` is a contextual declaration head. The fixture is TFV,
which every lane already parses; integer columns follow 2.8, `Bytes` columns are
`X<hex>`; one fixture file is shared by all lanes (VER04i's consumers registry
lists which lanes execute which table).

### 5.7 Test plan (WX-E)

* **Syntax:** valid table; rejects: missing `sha256`, `sha256` not 32 bytes,
  `source` with `..`, backslash, uppercase or empty; non-integer `rows`; no
  `column`; missing `holds`.
* **Integrity negatives:** one flipped byte in the fixture (W0708), row count off
  by one each way, a duplicate row id, odd-length hex, `I256` in a `U8` column
  (W0710), a missing field, a symlinked fixture, a 17 MiB file, `holds` with no
  operation call (W0601).
* **Sabotage:** every fault fails at least one row of the shipped `sha256`
  table; a table of constant-output vectors fails the discriminating check.
* **Differential:** the runner's verdict on the shipped NIST vectors versus a
  Python `hashlib` run over the same rows (proves the pinned data agrees with an
  independent implementation; the fixture is checked-in data, not generated by either).

## 6. WX-H — output-size bounds and parameterised bounds

### 6.1 Motivation

Decompressors cannot state `bound heap <= f(len(a))`: a 9-byte input may
legitimately request gigabytes, and the survey found, by reading,
`lz77::deserialise_tokens` and `lz78::deserialise_tokens` sizing
`Vec::with_capacity` from a raw header `u32`, and `lzss::decode` from
`original_length` (CWE-409, CWE-789). v0 can mention parameters in bounds
(VER04d §4.3) but cannot bound **what the operation returns**, and cannot make a
harness cap what the subject allocates.

### 6.2 Language additions

```text
bound_row = ( "steps" | "heap" | "depth" | "output" ) LE expression NEWLINE ;
```

* **`output <= e`** (new contextual metric) bounds the *size of the successful
  result*: `Bytes`/`Bytes<N>` byte length, `List<T>` element count, and for
  `Result<X, E>` the size of the `Ok` payload (an `Err` has size 0). Tuples,
  records and variants are W0301 (bound a component through a model of the op).
  `e` is an Int/Nat expression over the op's parameters, `len`, `to_int`,
  `unwrap_or` and the new total Int helpers `imin(a, b)`, `imax(a, b)`, nonnegative
  on the admitted domain.
* **Parameterised bounds.** Bound expressions already may mention parameters;
  WX-B types make them usable (`to_int(max_output)` for a `U32`). Because `div`
  returns an Option, bounds divide with `unwrap_or(div(n, k), 0)`.

```text
op decompress(data: Bytes, max_output: U32) -> Result<Bytes, LzError> fresh
bound decompress:
  output <= to_int(max_output)
  steps  <= 32 * (len(data) + to_int(max_output)) + 4096
  heap   <= 2 * to_int(max_output) + 8 * len(data) + 4096
  depth  <= 16
```

An API with *no* output parameter can only state a ratio bound such as
`output <= 255 * len(data) + 1024`. Writing it is itself the finding: for
LZ-family decoders no sound ratio exists, so the document cannot be satisfied
without an API that takes `max_output`, which is the change the survey predicted
(breaking across 17 ports; section 12).

### 6.3 Evaluation and evidence semantics

`output` is a *generated property*, not a prover obligation: it is the claim
`forall <params>. match op(<params>) with Ok(r) -> size(r) <= e | Err(_) -> true`,
registered as `exhaustive/`, `property/` and `fuzz/` suites named
`<op>/output-bound`, so it reuses VER04h domains, generators, shrinkers and
controls. Because `max_output` is a parameter, relation bias draws it from
`{0, 1, len(data) - 1, len(data), 2 * len(data), 255, 256, 65535, MAX}` as well as
uniformly, so the limit's boundary is crossed.

**Observed heap and the allocation cap.** For every op with a `heap` or `output`
row, *every* test instance the suites run on it (examples, vectors, exhaustive,
property, fuzz) executes under a lane-supplied **allocation-capped observer**.
Rust uses a counting `#[global_allocator]` in the generated test binary with a
per-test cap (default 64 MiB, a tool-profile value) that makes an over-cap
allocation *fail*, so a bomb is recorded as a failed instance
(`bound-heap-exceeded`, with the measured peak), not a process abort or CI OOM.
The observer is subject to VER04 §7's controls: a subject whose result is
unchanged but whose allocation grows must be distinguished. Measured peak is
compared with the bound evaluated at the instance's arguments; the manifest
records `max_observed_heap`, `bound_at_max` and `instances_observed`. A lane
without an allocation observer records `heap` and `output` as `not-live`: a bomb
cannot be contained there, so the target is blocked, never run uncapped.

**Hostile-header coverage.** The generator cannot invent format-specific bombs.
The document supplies them as ordinary `example`s (a nine-byte stream declaring
`0xFFFFFFFF` output) and through the byte alphabets of the exhaustive domain;
every example also runs under the observer, so a bomb example demonstrates the
CWE-789 finding or its fix.

### 6.4 Sabotage implications

| Fault | Models |
|---|---|
| `ignore-limit` | `max_output` replaced by `MAX` before the call |
| `off-by-one-limit` | limit `+ 1` |
| `limit-after-alloc` | adapter allocates `max_output` bytes before the call (the observer must flag heap above the bound) |
| `truncate-silently` | returns `Ok` of the first `max_output` bytes instead of `Err(OUTPUT_LIMIT)` (an `input ... rejects` claim must catch it) |

### 6.5 Portability

`output` is value-level and lane independent. Allocation observation is per lane
(Rust counting allocator; Go `runtime.MemStats`/`debug.SetMemoryLimit`; a
memory-limited child JVM/Node process; Python `tracemalloc`/`resource`); each
lane declares its observer revision and negative control, and a lane without one
is `not-live` for heap/output. `U32` limits are exact on all lanes after 2.8.

### 6.6 Compatibility

`output` is a contextual metric; a v0 `bound` with `output` is W0101. `imin` and
`imax` are v1 built-ins.

### 6.7 Test plan (WX-H)

* **Syntax:** `output` accepted in v1, W0101 in v0; an unknown metric stays W0401.
* **Type negatives:** `output` on a tuple/record result; an Option-typed bound
  expression; a bound negative on its domain (`output <= to_int(n) - 1` at `n = 0`);
  a bound mentioning a secret-labelled value is unchanged (W0402).
* **Observer controls:** equal results with 10x allocation are distinguished; a
  subject allocating only on bomb inputs is caught by the bomb example; a
  cap-less observer fails its own control.
* **Pilot:** an `lzw` byte-API document, then `lz77`, that declares the bound,
  observably fails on header-driven allocation, then passes after the capped fix.

## 7. Design sketches (not normative)

### 7.1 WX-F — binding-level sequence drivers

Streaming digests, `rans`/`range-coder` encoders and decoders, the RESP decoder
and `bitset`/`heap` operation sequences are stateful objects. The document stays
pure; the *binding* declares a **driver**: a host construction, a step method
driven by the elements of a `List` parameter, and a finalising observer.

```text
op digest_chunks(chunks: List<Bytes>) -> Bytes<20>         -- an ordinary op to Warrant
claim split_independent:
  forall a: Bytes, b: Bytes. digest_chunks([a, b]) == digest_chunks([concat(a, b)])
claim one_shot_agrees:
  forall m: Bytes. digest_chunks([m]) == sha1(m)
```

and in the binding (schema addition, no Warrant syntax change):
`"drivers": {"digest_chunks": {"new": "Digest::new", "step": {"method": "update", "arg": "chunk"}, "finish": "sum1"}}`.
Obligations: every named method exists; the driver runs with empty lists and
empty chunks; controls `drop-last-chunk`, `reorder-chunks`, `double-finish`.
Needs `List<Bytes>` binders (4.7) and, for split-point sweeps, `slice` from WX-D.
Risk: medium (schema and control design, not language).

### 7.2 WX-D — bounded fold/map/count

First-order, no closures: the higher-order forms take the *name* of a declared
model function of the required arity.

```text
fold(xs, init, step)     -- xs: List<T> | Bytes; step(acc: A, x: T) -> A; result A
map(xs, f)               -- f(x: T) -> U; result List<U>
count(xs, pred)  all(xs, pred)  any(xs, pred)    -- pred(x: T) -> Bool
take(xs, n)  drop(xs, n)  slice(xs, start, len)  replicate(x, n)  range(n)
```

Meaning is structural recursion over a finite value, so termination is by
construction. Cost: one unit per element plus the callee's own charge, so the
1,000,000-unit budget bounds a `fold` over 64 KiB; `replicate`/`range` are
charged `n` and capped by the 64 MiB data budget before allocation. Quantifier
semantics are unchanged; symbolic discharge of a `fold` over a symbolic length is
`bounded(K)` by unrolling, never `proved`. Unlocks `bitset` popcount/permutation
models, hash reference models (80-round schedules), the general LEB128 model and
`gmul` without unrolling (`fold` over `range(8)`). The checker must reject
non-model arguments and recursion through the step function (VER04's acyclic-model
rule). Cost accounting and quantifier interplay need a design pass with VER10
before this is specified normatively.

## 8. Cross-cutting evidence and portability rules

1. **One semantics, several implementations.** The ground evaluator (examples),
   the generated claim-model code and the lane adapters implement the same
   semantics separately; section 1.4 is what happens when they drift. v1 requires
   the differential tests of 10.3 *between the evaluator and the generated model
   code* for every built-in, not only against the host.
2. **Manifest additions** (VER04h §8; all optional fields, so v0 manifests are
   unchanged): `domain` per binder with `full`/`edge`/`demoted`, `tables`,
   `max_observed_heap`, `unreachable_by_type`, `warrant_version: "v1"`.
3. **Corpus (VER04i).** Key every field to a Warrant type; range-check decode;
   carry the boundary rows of 2.8/3.8; use constructor names for variants; list
   tables and which lanes run them.
4. **Not-live honesty.** A lane missing a type, observer or driver marks the
   dependent target `not-live`; it never runs a narrower type.

## 9. Out of scope, and why

| Item | Why it stays out |
|---|---|
| **Strings/Text** (Unicode) | needs a text type with an encoding model, normalisation questions and a corpus encoding (TFV `S` is bytes); it is a language of its own, and bytes cover the leaf cryptography and codec surface |
| **Floating point** | NaN payloads, rounding modes, `-0`, per-lane FPU differences (TFV `D` carries bits but semantics differ); fenwick/polynomial/trig are numerically, not bit-exactly, specified |
| **Maps and sets** | iteration order, key hashing and equality; model values need canonical ordering rules; collections (graph, trees, heap) are stateful Tier 3 |
| **Recursive types** | unbounded structures and non-finite domains with no domain descriptor; canonical-cbor, resp and json need them plus structural recursion with a measure (a WX-D follow-up) |
| **Stateful models** | reached indirectly through WX-F drivers; a stateful *model* language needs transitions and invariants and a different evidence story |
| **In-place/postcondition parameters** (zeroize) | needs `ensures post(a)` over an in/out parameter and a compiler-barrier proof story; 5 packages |
| **Cross-package claims, imports** | protobuf-versus-leb128 equality; modules cannot import (VER04 §2) |
| **Payload-carrying errors, closures, user recursion, overloading, defaults, implicit conversions** | would break the typed, total, first-order core that makes evidence meaningful (errors: open question 5) |

## 10. Test plan (all slices)

### 10.1 Syntax corpus

`code/specs/fixtures/warrant-syntax-v1/cases.json` (57 cases; features
`contracts declarations layout lexical operators precedence quantifiers types
values`; limits 128 cases / 2 MiB) is **not edited**. A new corpus
`warrant-syntax-v2` (format string `warrant-syntax-v2`, limits raised to 512 cases
/ 4 MiB) contains the 57 v0 cases *byte-identical, in the same order, under the
same ids, with the same verdicts and witnesses* (run with the v0 header),
followed by new feature groups `fixedwidth`, `bitwise`, `composite`, `patterns`,
`vectors`, `bounds-v1`, `versioning`. Required `versioning` cases: every new
construct is rejected in a v0 document (W0101) and the identical text with `v1`
is accepted; `warrant x v2` and `warrant x v1 extra` are rejected; `model record`,
`model rem` and `model shl` in a v0 document are accepted; `List<List<U8>>` still
parses; `_` as a v0 binder still parses. The `warrant.tokens` diff must be
additive and reviewed, and the compiled grammar for the v0-only token set must
regenerate byte-identically. Each new case is authored with an independent
expected verdict and witnesses (never derived from the parser), and the
deliberately-wrong-precedence and missing-token controls of VER04 §11 are re-run.

### 10.2 Type-checker negative corpus

One case per row of every "negative" list above, each pinning phase, diagnostic
ID and offending span, plus: shadowing a v1 built-in in a v1 document
(W0201/W0203) while a v0 name collision stays legal in v0; the conversion matrix
`F x F` exhaustively (64 pairs for each of `widen`, `wrap`, `try`, with verdicts
checked against an independent table of ranges); arity and static-argument errors
for every built-in; and cycle detection through tuples, `List` and `Option`. The
checker must fail deliberate corruptions of what it claims to validate (VER04
§9): a mutant that drops the "range contained" test of `widen`, or lets `div`
through on `S`, must fail a case.

### 10.3 Differential tests of the evaluator

Three-way equality, deterministic seeds, committed expected-output digests:

1. an independent unbounded-integer **reference evaluator** (Python), written from
   this document, not from the Rust code;
2. the Rust **ground evaluator** (examples) and the **generated model code**
   (claims), each against (1) and against each other;
3. **native Rust** (`wrapping_*`, `checked_*`, `rotate_*`, `count_ones`,
   `leading_zeros`, `to_be_bytes`) for the ops that have one.

Domains: `U8`/`I8` exhaustive for unary and binary ops, every shift/rotate count
`0..=300`; `U16`/`I16` over `E x E` plus 10^6 fixed-seed random pairs; `U32`,
`I32`, `U64`, `I64` over `E x E` plus 10^6 random pairs; the boundary cases
`to_int(U64) * to_int(U64)` and `-min_value<I64>()`; the conversion matrix over
the boundary values of every ordered type pair; composite construction,
projection and match on generated small values. **Mutant evaluators must be
killed:** non-wrapping `U8` (the actual v0 defect), saturating add, arithmetic
`shr`, masked shift count, `popcount` of the sign-extended word, floor-based
`rem_trunc` — each makes the suite fail.

### 10.4 Evidence-generator tests

Domain sizes and first/last elements per 2.6 and 4.7; the demotion rule on
`U16 x U16` (both demoted, leftmost first, at the reference `B`; only the first
at `B = 2^26`), `U16 x U8 x U8` (the one `U16` demoted, then W0706 at the
reference `B`: `281 * 256 * 256` is over it) and `U8 x U8 x U8` unchanged (W0706 at the
reference `B`, passing with the raised option as gf256 does); every fault of
2.7/3.7/4.8/6.4 yields `not ok` under `WARRANT_EVIDENCE_FAULT`; manifest
determinism; regeneration drift check; v0 evidence byte-identical (1.3). New
diagnostics W0708–W0710 each have a falsification case.

### 10.5 Conversion acceptance (one per slice)

WX-B+C: `gf256.warrant` rewritten (drop `xor1..xor8` and the `U64` exponent) and
`Field::new(poly: u16)` bound, with all previous verdicts kept; WX-E: a `sha512`
table plus claims; WX-A: `wasm-leb128`; WX-H: `lzw`, then `lz77`.

## 11. Recommended implementation order

1. **WX-B then WX-C** (about 6–9 engineer-days together, including the evaluator
   fix, generators, differential suite and the gf256 re-conversion). They are
   syntactically tiny, remove all three recorded workarounds, and unlock gf256
   (full model), x25519 (clamp), leb128 (encode-only claims) and des
   (complement/parity).
2. **WX-E** (about 6–8 days): independent of WX-A, the cheapest route to the four
   hashes (14–17 languages each), des KATs and x25519 RFC vectors; the
   fixture-pin machinery is reused by every later conversion.
3. **WX-A** (about 10–15 days; the largest slice): checker exhaustiveness, binding
   schema, generators, TFV shapes. Unlocks leb128 round trips, der-tlv (the
   CWE-436 parser differential is the security prize) and protobuf.
4. **WX-H** (about 5–7 days plus one observer per lane): the language change is
   small; the capped allocation observer is the cost. Unlocks lz*.
5. **WX-F** (about 4–6 days, binding schema only), then **WX-D** (about 8–12 days
   and a VER10 design pass).

Deviation from the survey's cheapest order (B, C, A, E, F, D): E precedes A
because it is smaller, does not depend on composite-type machinery and unlocks
more packages sooner; H precedes F because it alone turns an already identified
defect class (CWE-409/789 in lz*) into checked evidence.

## 12. Open questions for the owner

1. **Header spelling.** `warrant name v1` (as specified) versus per-feature `use`
   lines; the header is simplest but all-or-nothing.
2. **Wrapping default.** `+ - *` wrap on fixed widths (matches v0 and Rust
   `wrapping_*`). Checked-by-default with explicit `wrapping_add` would change v0
   meaning and was rejected; confirm.
3. **Signed division.** `div`/`rem` rejected on signed types, `div_trunc`/`rem_trunc`
   required, and `MIN / -1` is `None`. Confirm versus returning `MIN`.
4. **Shift count** is `Nat` with total `n >= w` semantics (deliberately not
   hardware masking). Confirm this is the intended finding-producing semantics.
5. **Payload errors.** der-tlv's `DerError { kind, offset }` needs the offset
   observable. (a) a second observer op (specified); (b) allow `Result<T, E>` with
   `E` a payload-carrying non-recursive variant. (b) is more natural but touches
   VER04 §3's "finite error type" and `rejects ... with Error.CASE`.
6. **lz\* API change.** WX-H makes the correct specification unsatisfiable by the
   current byte APIs (no `max_output`, `Vec<u8>` with no error channel). Who
   decides the breaking change across 17 ports, and is a Rust-only API plus shim
   acceptable for the pilot?
7. **VER04i exists** (backlog WD-002, Draft on this branch). The encodings in 2.8,
   3.8, 4.9 and 5.6 are requirements on VER04i. Section 12a lists where VER04i
   does not yet satisfy them; the owner decides whether VER04i absorbs the gaps
   or a type-encoding annex is added to this document.
8. **Exhaustive budget.** Raise the reference `B` (400,000) or keep per-package
   `EvidenceOptions`? The demotion rule for `U16` claims depends on it.
9. **Table fixtures.** `code/specs/fixtures/<name>/` with README provenance; who
   vets NIST/RFC data before a pin is accepted?
10. **Nested patterns and positional constructors** (specified) versus flat
    patterns and named arguments; flat patterns halve the exhaustiveness work.

## 12a. Reconciliation with VER04i

VER04j was written without sight of VER04i. This section records where the two
documents disagree or where VER04i does not yet cover a VER04j requirement. It
does not change VER04i. Neither document has been rewritten to match the other;
each item needs an owner decision.

1. **`Result` encoding (conflict).** VER04j 4.9 proposes `L2 S"Ok"|"Err" v`.
   VER04i 2.7 fixes `Result<T,E>` as an `M` with one entry, key `ok` (value) or
   `err` (`S<VARIANT>`), and VER04i 2.6 uses tags `50`/`51` in the canonical
   stream. VER04j 4.9 said to match VER04i's existing ct-compare form, which is
   the `M` form, so VER04j's proposal is the one out of step. Proposed: VER04i's
   form stands and VER04j 4.9 is corrected once the owner confirms.
2. **Boundary rows for 64-bit types (gap in VER04i).** VER04j 2.8 requires
   `2^53 - 1, 2^53, 2^53 + 1` and `2^63 - 1, 2^63, 2^63 + 1`. VER04i 2.6's
   `u64_edge` contains `2^53` (as `1<<53`), `2^63` (as `1<<63`), `2^63 - 1`
   (as `MAX ^ (1<<63)`) and `MAX`, but not `2^53 - 1`, `2^53 + 1` or `2^63 + 1`.
   VER04i must add these rows.
3. **Signed and 32-bit types (gap in VER04i).** VER04j 2.8 requires signed
   boundary rows (`MIN, MIN+1, -1, 0, 1, MAX-1, MAX`), and WX-B adds `U32`,
   `I32` and `I64`. VER04i 2.7's type table covers only `Bool`, `U8`, `U64`,
   `Nat`, `Bytes`, `List`, `Option` and `Result`. VER04i must add rows for these
   before any WX-B claim can be corpus-checked.
4. **Tuples, records and user variants (gap in VER04i).** VER04j 4.9 defines the
   TFV shapes `L<n>` (tuple, record) and `L2 S<ctor> L<k>` (variant). VER04i 2.7
   and its canonical stream (2.6) have no rows for these shapes; a tuple is only
   a concatenation of its elements there, and a record or user variant has no
   tag. VER04i must define them, with constructor name as the key, before WX-A
   corpus cases exist.
5. **`U8` arithmetic (silent in VER04i).** VER04j 1.4 says `U8` and `U64`
   arithmetic wraps, and that the Rust evaluator's non-wrapping `U8` is a defect.
   VER04i 2.7 says `U64` wraps and `Nat` is checked, and says nothing about `U8`.
   VER04i's in-lane model evaluator is checked against Rust per case, so it must
   state the `U8` rule explicitly.
6. **Agreement (no action).** Both documents agree on: the record schema carries
   the type and the decoder range-checks (VER04j 2.8 item 1 and VER04i 2.7 "out of
   range is a corpus error"); `Option` is `L0`/`L1 x`; `U64` is `BigInt` on
   TypeScript and Dart, and `number` is forbidden above 2^53; the lane hazards for
   Lua, OCaml and Java.

VER13 has no reference to VER04i or VER04j, so it needed no edit here.

## 13. Specification change record

- Initial VER04j: five extensions plus two sketches, ordered by the leaf-library
  survey and the `gf256` conversion's forced workarounds. No v0 text is changed;
  section 1.4 records the one evaluator conformance fix against existing v0 text
  (non-wrapping `U8` in the generated evaluator).
- WX-H and WX-E implemented (`rust/warrant-grammar` 0.4.0): see sections 14.2 and
  14.3 for the deviations and clarifications this forced; sections 5 and 6 are
  unchanged except where 14.2/14.3 say a sentence has no antecedent in the checker
  or names a vocabulary the harness does not have.
- WX-A implemented (`rust/warrant-grammar` 0.3.0): see section 14.1 for the
  deviations and clarifications this forced; section 4 itself is unchanged except
  that nothing in it was found wrong.
- WX-B and WX-C implemented (`rust/warrant-grammar` 0.2.0): see section 14 for the
  deviations and clarifications this forced. Sections 1.4, 2.6, 2.7 and 3.7 were
  corrected in place: the `U16 x U16` demotion example, the list of values the edge
  domains guarantee, and the operator-qualified fault names.

## 14. Implementation status and deviations (WX-B, WX-C, WX-A, WX-H, WX-E)

Where the code departs from, or must decide something the text leaves open:

1. **WD-013 and v0 evidence.** See the note under section 1.4: the typed,
   wrapping `U8` exists for v1 documents; v0 generated evidence is frozen
   byte-identical (`ct-compare`, `gf256` regenerate with no diff).
2. **Diagnostics.** A v1 construct in a v0 document, a header word other than
   `v1`, and an out-of-range suffixed literal are `ParserErrorKind::{V1ConstructInV0,
   InvalidVersionHeader, LiteralOutOfRange}`, all with code `W0101`. A trailing
   word after `v1` (`warrant x v1 extra`) is the ordinary `W0001` unexpected token.
   In a **v0** document an out-of-range `u8`/`u64` literal keeps its v0 `W0001`.
   An `Int` overflow in the ground evaluator is `W0703` in a v1 document (section
   2.5) and `W0702` in a v0 document, as before.
3. **Hex literals** are lexed as one token (`0x` followed by every alphanumeric) and
   validated by the parser, so `0xFF` is `W0101`, not the token pair `0x` `FF`. An
   unsuffixed hex literal must fit `u64` (it is an `Int` literal).
4. **`-128i8`** is folded by the parser into a single negative literal when the
   magnitude token is the direct operand of unary minus; `-(128i8)` is rejected.
5. **Shift counts are `Nat` only.** The checker types `Nat + Nat` as `Int`
   (VER04d), and v1 adds no `Int -> Nat` conversion, so `shl(a, n + 1)` is a type
   error. This is a gap in the text (a count computed from another count cannot be
   written); closing it needs either `Nat + Nat : Nat` (a v0 typing change) or a
   `to_nat` on `Int`, and is left to the owner.
6. **`div` on `Int`/`Nat`** is floor division in a v1 document everywhere (as
   VER04 section 3 says); the v0 ground evaluator uses Euclidean division, which
   differs for a negative divisor, and is left unchanged for v0 documents.
   `rem` on `Int`/`Nat` is floor modulo.
7. **Numeric equality** in a v1 document compares `Int` and `Nat` by value
   (`popcount(x) == 8` compares a `Nat` with an `Int` literal), recursively
   through `Option`, `Result` and `List`.
8. **Fault names** carry the operator (`saturate-add`, `trap-as-zero-mul`,
   `rotate-as-shift-left`); see sections 2.7 and 3.7. `endian-swap` is **not
   implemented**: the adapter shapes that need it (a `T -> Bytes<N>` export and a
   `Bytes<N> -> T` export) do not exist yet.
9. **Generated v1 harness.** `V` gains `Int`, `Fixed(F, i128)` and `Opt`; a v1
   document binds every fixed-width binder, `U8` and `U64` included, as `Ty::Fx`.
   The prelude carries `//@if v0` / `//@if v1` regions (nested regions are
   supported), and the v0 regions are the v0 text unchanged. The manifest gains
   `warrant_version` (top level) and `demoted` (exhaustive entries), both v1 only;
   `ca-assurance-report` does not read them yet.
10. **Scalar adapter shape (v1).** Parameters are fixed-width integers or `Nat`;
    the result is a fixed-width integer, `Bool`, `Nat`, `Option<T>` or
    `Result<T, E>` (panic normalisation). `Bytes`-returning exports are not
    adapted yet.
11. **Syntax corpus.** The canonical `warrant.tokens`/`warrant.grammar` gain the new
    literal tokens and the optional second header name additively (the compiled
    artifacts regenerate with the v0 lexemes unchanged, and `warrant-syntax-v1` is
    untouched and still passes). The version *gating* (W0101) lives in the
    hand-written parser, which the grammar-driven corpus runner does not exercise,
    so a `warrant-syntax-v2` fixture was **not** created; its cases are Rust tests
    over the real parser (`tests/wx_syntax.rs`).
12. **Differential tests (section 10.3).** The independent Python reference
    evaluator was **not** written. Three-way equality is established as native
    Rust <-> `fixed.rs` model <-> ground evaluator, and native Rust <-> `fixed.rs`
    <-> the generated model code; the mutant evaluators of section 10.3 are
    reproduced in the test suite and shown to be detected.

### 14.1 WX-A (tuples, records, non-recursive sum types)

The numbering continues from the WX-B/WX-C items above.

13. **Gating and diagnostics.** In a v0 document every WX-A construct is W0101
    (`ParserErrorKind::V1ConstructInV0`): a `record` or `variant` declaration, a
    tuple type, tuple literal or tuple pattern, a projection (`t.0`, `f(x).0`,
    `f(x).field`), `_`, a constructor or nested pattern, and a `true`/`false`
    pattern.  The v0 pattern forms (`None`, `Some(x)`, `Ok(x)`, `Err(x)` with a bare
    NAME) are unchanged and `_` there is still an ordinary binder.  A duplicate
    field name inside one `record`, a duplicate field inside one constructor and a
    duplicate constructor inside one `variant` are W0201 from the parser (they
    are listed under the syntax corpus in section 4.11); duplicates *across*
    declarations are W0201 from the checker.  More than 32 fields, more than 256
    constructors, a tuple of arity above 8 and a pattern nested deeper than 8 are
    W0001 syntax errors.
14. **`PROJ` lexing.** A projection is a `.` with no whitespace on either side,
    followed by a decimal INT with no leading zero and no suffix (`t.00` and `t.0u8`
    are syntax errors) or a NAME.  Projection is also accepted after a literal; the
    checker rejects it (W0301).  A `.` followed by whitespace is never a projection
    (`t. 0` is a syntax error), which keeps the quantifier separator unaffected.
15. **Patterns.** `Pattern` is recursive (`Some(Box<Pattern>)`).  `true` and
    `false` are patterns of `Bool`; `Err(...)` accepts `E.CASE`, where `E` is the
    scrutinee's error type, so error cases are matched like constructors.  A bare
    NAME that is a constructor must be nullary; a constructor with fields is
    matched as `C(p1, ..., pn)` with exactly one pattern per field, and `_` ignores
    one.  A binder repeated inside one pattern, and a binder equal to a constructor
    name, a model, an operation, an error type or a built-in, is W0203.  Integer,
    byte and list scrutinees accept only `_` or a binder.
16. **Exhaustiveness** is the matrix usefulness algorithm in `patterns.rs` over
    `Bool`, `Option`, `Result`, error cases, tuples, records and variants.  A
    non-exhaustive match and a redundant arm are both W0301
    (`NonExhaustiveMatch`, `RedundantArm`).  Work is charged per
    (arm x constructor) visited against the checker's work budget, a match has at
    most 256 arms and a pattern nests at most 8 deep, and exhaustion of either is
    W0602.  In a v1 document a match that repeats an arm of `Option`/`Result` is now
    a redundancy error (v0 accepted it); the legacy check is kept for v0 documents
    so their diagnostics do not change.
17. **Typing.** As section 4.4.  A tuple, record and variant is invariant in its
    components; `Option<Nat>` still widens to `Option<Int>` as in v0, but
    `Option<(Nat, Nat)>` does not widen to `Option<(Int, Int)>`.  A constructor
    name is a module symbol: it clashes (W0201) with every model, operation, error
    type, type name and other constructor, and a record's name is both its type and
    its single constructor.  Calling a constructor with parentheses but no
    arguments (`Empty()`) and naming one that has fields without arguments are
    arity errors.  `Name.field` where `Name` is a binder is projection, taking
    priority over an error constructor of the same spelling.  The type graph must be
    acyclic through tuples, records, variants, `List`, `Option`, `Result` and type
    aliases (W0203).  `labels op: p.field public` accepts a field path of a record
    parameter.
18. **Evaluator.** `EvalValue` gains `Tuple(Vec<_>)` and
    `Con { ty, index, ctor, fields }` (the constructor *name* is kept beside its
    index because the corpus encoding keys on the name).  Construction,
    projection and a match arm charge 1 unit plus one per field touched; a composite
    larger than `max_value_bytes` leaves is W0703.
19. **Corpus encoding.** `EvalValue::encode_portable` implements section 4.9:
    tuples and records `L<n> ...`, variants `L2 S<ctor> L<k> ...`, `Option`
    `L0`/`L1 x`.  For `Result` it uses the VER04i form (`M1 S<ok|err> v`, the
    resolution proposed in section 12a item 1), not the `L2 S<Ok|Err> v` of
    section 4.9.  Container nesting above 64 is W0709 (`encode_checked`).  The
    legacy `encode_tfv` (an `Option` is its payload) is unchanged for the v0 value
    shapes; composites always encode their children with `encode_portable`.
20. **Domains** are as the table in 4.7.  Domain *bytes* are bounded by
    `size x (maximum per-element cost)`, where a node costs `VALUE_OVERHEAD` and a
    `Bytes` payload its maximum length: a safe over-estimate, not an exact sum.  A
    `List` domain is `1 + d + d^2 + e^3` with `d = min(16, |D|)` and
    `e = min(4, |D|)`.  The budget demotion rule of section 2.6 now applies to every
    full `U16`/`I16` *leaf* of a composite binder (depth-first, left to right); the
    manifest's `demoted` entry is the binder name followed by the component path:
    the tuple component or field index, and for a variant the constructor name
    before the field index (`p.0`, `r.1`, `a.Q.0`).  A binder whose domain is over
    budget after demotion is W0706.  A composite binder's domain is one stored
    vector, unlike the lazily iterated product of separate binders, so the 64 MiB
    byte budget bounds the product *inside* a composite even when the case budget
    is raised: `(U16, U16)` demotes both components at 2^26 too, and
    `(U16, U8, U8)` is W0706 at every budget.  `List<T>` is the same size for
    every `T` with at least 16 elements (337), so `List<List<U8>>` is 337.
21. **Generators and shrinkers** are as 4.7.  `Result` draws `Ok` or `Err` with equal
    probability and then an error case uniformly; a `List` draws a length in
    `0..=3`.  Relation bias descends into one leaf: it perturbs one randomly chosen
    component, keeping the constructor.
22. **Bindings (section 4.6).** `evidence.bindings` gains
    `struct T path=P fields=name:ht,...` and `enum T path=P ctors=C(name:ht,...),...`
    lines and, on `op`, `ret=ht` and `errors=P/Variant=CASE,...` (the path and the
    variants are separated by `/`, because a Rust path contains `::`).  A host type `ht` is
    `u8..u64`, `i8..i64`, `usize`, `bool`, `bytes`, `(ht, ht, ...)`, `opt<ht>`,
    `list<ht>` or a declared `struct`/`enum` name; `args=` is split on commas outside
    brackets.  A host field name may be a decimal index, which reaches a tuple
    struct or tuple variant (`P { 0: x }`).  An export returns its value directly
    (a panic is normalised by `error=CASE`) or a host `Result` whose error variants
    are listed in `errors=`.  Payload-carrying host errors are not supported (the
    variant is matched with `{ .. }`).  A value a host integer cannot hold is
    *unreachable by type*: the adapter raises a private panic that the claim wrapper
    turns into a vacuous pass, counted as `unreachable_by_type` in the manifest;
    nothing is narrowed.  Because the variant is matched with `{ .. }`, the path
    may also name a host error *struct* (`errors=wasm_leb128/Leb128Error=MALFORMED`
    matches `wasm_leb128::Leb128Error { .. }`); every value of that struct is then
    the one mapped case.  *Correction (wasm-leb128 adoption):* the
    `gen_evidence` example that every package `BUILD` runs used to build its
    options with empty `hosts`, so these lines reached the generator only through
    in-test drivers; it now passes them through, and
    `tests/gen_evidence_driver.rs` drives the example itself on a bindings file
    with `struct`, `enum`, composite `args=`, `ret=` and `errors=` lines.
23. **Faults** (4.8) are rewrites of the adapter's result.  `off-by-one-consumed`
    adds 1 to an even second component and subtracts 1 from an odd one (so both
    directions of "`+ 1` / `- 1`" occur and a claim that pins `consumed` on every
    input catches it); `wrong-ctor` of the first constructor is the last
    (cyclic), and the predecessor's fields are the first element of each field's
    domain; `field-zero` needs a numeric field and `swap-components` a homogeneous
    pair or longer tuple, each checked at generation (W0701); `ok-as-err` needs a
    `Result` whose payload is composite.  A fault rewrites the composite *inside* an
    `Ok` or a `Some` (an `Option<(U8, U8)>` result can be attacked by
    `swap-components`), and `drop-consumed` and `off-by-one-consumed` need a numeric
    second component.  `swap-components` reverses a tuple, which exchanges a pair.
    `missing-error` is also accepted on a composite `Result` operation (a panic is
    reported as `Ok` of the first element of the payload's domain).  The fault
    applies to the operation it is bound to only (the `scoped` mechanism of the
    scalar adapters).
24. **Evidence bodies.** `match`, tuples, constructors and projections are now
    translated in model and claim bodies (a `match` was W0701 for v1 before WX-A, and
    `tests/wx_generator.rs` was updated accordingly); list literals and
    quantifiers inside a body remain W0701, as for v0.  `len` also accepts a list
    value.  A claim binder may be an `Option`, `Result` or `List` (the domains of
    section 4.7), but an operation that mentions no tuple, record or variant is still
    adapted by the WX-B/WX-C shapes only: an `Option`/`Result`/`List` *parameter* of
    such an operation has no adapter.
25. **Prelude gating.** The WX-A part of the generated prelude (`//@if comp`) is
    emitted only for a v1 document that declares a record or variant, writes a tuple,
    a projection or a `match`, or quantifies over an `Option`, `Result`, `List` or
    composite; every other document's generated output, v0 or v1, is byte-identical
    to the WX-B/WX-C generator's (`tests/wx_regen.rs`, `tests/v0_compat.rs`).
26. **Checker output.** `CheckedWarrantModule` is unchanged (its `Debug` rendering for
    the two checked-in specifications is pinned by `tests/v0_compat.rs`); the
    records and variants stay inside the checker, and `CheckedAdt`/`CheckedCtor` are
    exported for a later consumer.  The canonical `warrant.tokens` and
    `warrant.grammar` are *not* extended with WX-A productions: the hand-written
    parser is the reference for tuples, records, variants, projections and
    patterns, and `warrant-syntax-v1` is untouched (as in item 11, a
    `warrant-syntax-v2` corpus remains unwritten; its cases are
    `tests/wxa_syntax.rs`).
27. **Not implemented in this slice:** list literals, list patterns and quantifiers
    inside evidence bodies (item 24), payload-carrying host errors, a host `Result`
    *parameter*, fixed-width `Bytes<N>` parameters and static widths on a composite
    operation, and the `error_offset` second observer of section 4.2 (it is an
    ordinary `op`).  The checked-in pilot for section 4.11 is a LEB128-shaped
    fixture (`tests/fixtures/wxa.warrant`, decoding the first one or two bytes of a
    stream) with an in-test subject, not the `wasm-leb128` package, which is
    deliberately not converted here.  The exhaustiveness differential covers every
    arm sequence up to four arms on `Bool x Option<Bool>`, every sequence up to
    three arms plus a fixed-seed sample of four-arm sequences on the
    three-component type of section 4.11, and every sequence up to four arms on
    single-column types.

### 14.2 WX-H (output-size bounds and parameterised bounds)

The numbering continues from 14.1.

28. **Gating and typing.** `output` as the metric of a `bound` row is W0101
    (`ParserErrorKind::V1ConstructInV0`) from the parser in a v0 document, where
    it used to be the checker's W0401 "unknown metric" (a valid v0 document is
    unaffected).  In a v1 document the checker accepts `output` only when the
    operation's result is `Bytes`, `Bytes<N>`, `List<T>` or a `Result` whose `Ok`
    payload is one of those; an `Option`, a tuple, a record, a variant and a scalar
    are W0301.  An unknown metric stays W0401.
29. **`imin`/`imax`** are v1 built-ins typed `(Int | Nat, Int | Nat) -> Int` and are
    usable in any expression, not only in a bound; `V1_FUNCTIONS` grows from 31 to
    33 names (a v1 model called `imin` is now W0201, exactly as for the other
    built-ins).  The ground evaluator and the generated model code implement them
    and the differential tests of section 10.3 cover them.
30. **What is evaluated.** The generator evaluates `output` and `heap` rows (over the
    operation's parameters, with the ordinary model translation).  `steps` and
    `depth` stay prover obligations.  The text "a bound mentioning a secret-labelled
    value is unchanged (W0402)" has no antecedent: the checker never rejected a
    secret value in a bound (VER04d does not say it should, and `ct-compare` bounds
    `len` of secret parameters), so nothing changed and no W0402 is raised.
31. **Negative bounds.** "Nonnegative on the admitted domain" is checked per
    instance, not statically: an instance at which an `output` or `heap` bound
    evaluates below zero fails with the note `bound-negative` (a defect of the
    document, never of the subject).  An unevaluable bound (a model that panics,
    arithmetic overflow) fails the instance the same way.
32. **Generated property.** For every operation with an `output` row the generator
    registers `exhaustive`, `property` and `fuzz` suites with the ids
    `warrant/<module>/<kind>/<op>/output-bound` and `claim` field `<op>/output-bound`.
    The property is `match op(args) with Ok(r) -> size(r) <= e | Err(_) -> true`;
    a non-`Result` operation has size `len(result)`.  The suites take the operation's
    parameters (with their refinements) as binders, so every binder domain, budget,
    demotion and cap of VER04h applies (an operation whose parameters are too many
    or too wide for the exhaustive budget is therefore W0706 for this suite, as for a
    written claim; an operation with static widths is W0701); relation bias draws an unsigned limit
    parameter from the boundary list of section 6.3 whenever the claim also has a
    `Bytes` binder (only for documents with an `output` or `heap` row, so every other
    generated harness is byte-identical).
33. **Observer.** A document with an `output` or `heap` row on some operation gets the
    `obs` part of the prelude: a counting `#[global_allocator]` plus a per-thread
    allocation window.  `GlobalAlloc` may neither panic nor, on stable Rust, return
    null without the process aborting (`handle_alloc_error`), so the cap is enforced
    by *containment*: an observed adapter call runs on a worker thread; when it asks
    for memory beyond the cap (default 64 MiB, `OBSERVER_CAP_BYTES`) the allocator
    records the attempted peak, wakes the caller and parks that worker forever, and
    the caller fails the instance with `bound-heap-exceeded` and the measured peak.
    A test that has tripped once fails its later observed calls without running them
    (a shrinker would otherwise abandon one worker per candidate), and at most 64
    workers holding at most 1 GiB between them are ever abandoned in one process
    (`OBS_MAX_ABANDONED`, `OBS_MAX_HELD`); after that an observed call is refused with
    the same failure instead of being started.  The
    window is the whole adapter call (argument conversion, the export, result
    conversion), so the adapter's copy of the result counts: a `heap` bound must
    leave room for one extra copy of the output, as the spec's own example
    (`2 * to_int(max_output)`) does.  The measured peak is compared with the `heap`
    row evaluated at the call's arguments; a call over the row fails.
34. **Failure vocabulary.** `bound-heap-exceeded` is not in the closed TF01 section 5.3
    failure-id vocabulary and `test-bootstrap`'s vocabulary is not changed here.  The
    failing verdict therefore carries a registered id (`not-deep-equal` for an
    example, `property-counterexample`, `fuzz-crash` or `corpus-mismatch` as the
    suite runner chooses) and the observer's text is the note
    (`bound-heap-exceeded`, `bound-negative`, `output-bound-exceeded`).  The manifest
    entry records `max_observed_heap`, `bound_at_max` and `instances_observed` on every
    observed example, exhaustive, property, fuzz and table entry.
35. **Adapter shape.** A v1 operation whose signature has no scalar or byte-pair
    adapter (any `Bytes` result, a `Bytes` parameter beside a numeric one) is adapted
    by the composite adapter of WX-A, including its host `Result`/`errors=` binding
    forms; the prelude's `comp` part is emitted for it.  Before this, such a
    signature was W0701.  A `fresh` result is satisfied by construction there (a host
    `Vec<u8>` cannot alias its input) and is no longer refused.  The model translation
    gained `concat` of two byte strings (it was W0701 before, although VER04 reserves
    it), so a claim can build a well-formed input for the operation; its prelude part
    is emitted only when a body uses it, like `imin`/`imax`.
36. **Faults.** `ignore-limit`, `off-by-one-limit`, `limit-after-alloc` and
    `truncate-silently` are rewrites of the composite adapter.  The *limit
    parameter* is the one unsigned numeric parameter the operation's `output` row
    mentions (none or several is W0701 at generation).  `ignore-limit` replaces it
    with the host type's maximum, `off-by-one-limit` adds one (saturating),
    `limit-after-alloc` holds a zeroed `Vec<u8>` of that many bytes across the
    call, and `truncate-silently` turns an `Err` into `Ok` of the first `limit`
    bytes of the first `Bytes` parameter (the adapter cannot recover what an
    erroring subject would have produced; it needs a `Bytes` parameter and a
    `Result<Bytes, E>` result, W0701 otherwise).
37. **Pilot.** `tests/fixtures/wxh.warrant` (a four-byte length header plus run-length
    body, `decompress(data, max_output)`) with an in-test vulnerable subject (sizes
    `Vec::with_capacity` from the header) and its fix.  It is synthetic: no `lz*`
    API is touched (open question 6 stays with the owner).

53. **Real adopter (`lzw`).** The first document with `output` rows on a real package
    (`lzw.warrant`: `compress` `output <= 3 * len + 16`, `decompress` `output <=
    imin(16777216, 32768 * len)`, each with a `heap` row).  Two findings about the
    section 6 prediction.  (a) "No sound ratio exists" was too strong: a 16-bit code
    stands for at most 65,279 bytes, so 32,768 is a sound ratio, but it allows 2 GB
    from 64 KB and constrains nothing, which is the finding; the document is
    satisfiable only because the crate also caps delivered output at 16 MiB, an
    absolute constant, so the `max_output` parameter was not needed to state it
    (it is still the better API, LP-003).  (b) The `heap` row, not the `output` row,
    exposed the quadratic dictionary: observed peak was 3.4 times the output.  Two
    generator limits met on the way, both recorded as lessons: a `model` may not
    call an operation (W0301), and `take`, `drop`, `slice`, `replicate` and
    `to_bytes` are checked and evaluated but not yet emitted by the Rust generator
    (W0701), so streams are built from hex literals and `concat`, and a bomb of
    285 KB is a ten-fold `concat` doubling of a 279-byte unit.  The observer's
    64 MiB cap is what makes the pre-fix bomb examples fail instead of exhausting
    memory.

### 14.3 WX-E (external known-answer tables)

38. **Gating and syntax.** In a v0 document `vectors` is W0101.  `PATH_LITERAL` is
    the lexer's quoted text token; an unterminated quote is W0001.  The five row
    words (`source sha256 rows column holds`) are contextual and recognised only in
    that position and order.  The path rules of section 5.2 (alphabet
    `[A-Za-z0-9_./-]`, at most 256 bytes, no leading `/`, no `.`/`..`/empty segment)
    are W0708 from the parser.  The test plan's "uppercase" negative is read as an
    uppercase *hex digit in the `sha256` literal* (the path alphabet explicitly
    allows `A-Z`); a `sha256` literal that is not exactly 32 bytes is W0708, an
    unterminated one (`b"01`) and a non-decimal or suffixed `rows` are W0001.
39. **Namespaces and checking.** A table name shares the test-name namespace
    (W0201 on a clash).  `holds` is typed as a Bool in the claim environment (columns
    bound to their declared types, operations allowed); a `holds` that calls no
    operation is W0601 (as is `rows 0`: nothing to check) and a column it never
    mentions is W0710.  `rows` above 100,000,
    and above 250,000 summed over a document, are W0602 from the checker; a duplicate
    column is W0201.  A column type with a symbolic width (`Bytes<n>`) is W0710.
40. **Where fixtures live.** `code/specs/fixtures/` is the fixtures root (a tool
    profile constant, `FIXTURES_ROOT`); a package's `evidence.bindings` does not name
    it.  The synthetic fixtures of this slice are
    `code/specs/fixtures/warrant-vectors-synthetic-v1/` (its README is the provenance
    record).  NIST/RFC vector sets are an owner decision (open question 9); the
    loader is the hook, and the README says how a vetted set is added (the file, its
    pin in the warrant, a regenerated copy).
41. **Loading (fail closed).** `vectors::load_tables` reads each table once: every
    path component is checked with `symlink_metadata` (a symlink anywhere is W0708),
    the file is opened, its metadata re-checked against the open handle (a regular
    file; on Unix the device and inode must match the earlier `lstat`), at most
    16 MiB + 1 byte are read, and the SHA-256 is compared with the pin *before* any
    parsing.  Only then is the TFV parsed (by `test-bootstrap`'s parser, whose
    duplicate-id check is now linear) and every row checked: exactly `rows` records,
    exactly the declared columns (no missing, no extra), each value decoded by its
    column type (integers range-checked, `Bytes<N>` length-checked, constructor
    names for variants, `Result` in the VER04i `M` form).  A row-level mismatch is
    W0710; a map with more than one entry has no Warrant type and is W0709 (it is
    refused from a token scan *before* parsing, because the TFV parser's pairwise
    key check is quadratic in the entries of one map); nesting beyond 64 is W0709;
    the row budget (100,000 per table, 250,000 per document, summed with
    saturating arithmetic) is checked before any file is read and is W0602.
    Nothing is ever fetched from the network.
42. **Materialisation.** `evidence_tables` returns, for each table, the verified bytes
    and digest; the `gen_evidence` driver writes `<dir of output>/tables/<name>.tfv`
    byte-identically and `--check` compares it.  The generated harness embeds the
    copy with `include_bytes!` from its own crate tree, hashes it at the start of the
    table's test and fails with `corpus-malformed` on a mismatch.
43. **Execution and manifest.** One suite test per table,
    `warrant/<module>/table/<name>`; inside it every row is one case
    `vectors/<name>/<row id>` and the entry records `kind: "table"`, `cases`,
    `table_sha256`, `rows_pinned`, `verdict` and `failed_ids` (first 16 row ids;
    when the `CI` variable is set only the count is kept).  The entry carries the
    observer fields of item 34 when the operation is observed.
44. **Discrimination.** Negative controls run per (operation, fault) with kind
    `table`; the control passes iff at least one row fails.  At generation time a table
    whose `holds` is `<expr> == <column>` (or the mirror) and whose `<column>` is the
    same value in every row of a table with at least two rows is W0601 "table is not
    discriminating": the subject cannot be told apart from a constant function, and no
    fault could fail a table that a constant function passes.  Other tables rely on the
    runtime control.
45. **Not implemented.** Seeding the fuzz corpus from table rows (5.3 item 5, optional),
    the lane registry of VER04i, and every lane but Rust (the loader is language
    independent data; the runner is the Rust prelude).
46. **Faults for byte results.** VER04h section 6 has no fault for a digest-like
    operation, so a pinned digest table would have nothing to fail.  Two composite
    faults are added: `flip-result-bit` (the lowest bit of the first byte of a
    `Bytes`/`Bytes<N>` result, inside an `Ok` or `Some`; an empty result gains a
    byte) and `drop-last-result-byte`.  They apply to the operation they are bound
    to only and are checked against the result type at generation (W0701).  A
    `Bytes<N>` result (never a parameter) is now accepted by the composite adapter,
    with the same host mapping as `Bytes`.
47. **Rows are cases, a table is one test.**  The suite registers one test per table,
    `warrant/<module>/table/<name>`; each row is a case named `vectors/<name>/<row id>`
    in the failure message and in `failed_ids`.  A row whose `holds` panics, or
    whose value the host integer cannot hold, fails (it is not skipped as
    unreachable-by-type: a pinned datum the subject cannot take is a finding).  A
    copy that no longer matches its pin, or whose rows no longer decode, fails the
    whole table with `corpus-malformed`.
48. **Build and capability plumbing.**  `coding_adventures_test_bootstrap` and
    `coding_adventures_sha256` become regular dependencies of the crate;
    `required_capabilities.json` declares `fs:read` (the crate already read a file in
    `codegen::verify_regeneration` without declaring it, so the capability analyzer
    failed on the previous slice too; the declaration needs the usual signed
    approval); the Go build tool maps `code/specs/fixtures/warrant-vectors-synthetic-v1/`
    to `rust/warrant-grammar`.
49. **Pilot.** `tests/fixtures/wxv.warrant` pins two synthetic tables (SHA-256 of
    made-up messages, `Result<U8, E>` answers) against an in-test subject; the
    differential test recomputes every digest with Python `hashlib` over the same
    rows and compares it with the pinned data and with the subject.

50. **Observer scope (WX-H).** The window accounts only allocations made through the
    global allocator *on the calling (worker) thread*: memory a subject allocates on
    threads it spawns, or maps directly (`mmap`, a foreign allocator), is not charged
    and not contained.  The cap is a guard against the common bomb, a `Vec` sized
    from a header, not a sandbox.
51. **What kills `truncate-silently`.** The synthesized `output-bound` claim cannot:
    a truncated `Ok` is within the bound.  It is caught by a *written* claim or
    example that expects `Err(E.OUTPUT_LIMIT)` (the pilot's
    `one_byte_over_the_limit` and `empty_input_is_corrupt_under_any_limit`).
52. **Memory caps of the loader (WX-E).** A document may declare at most 16 tables
    (`MAX_DOCUMENT_TABLES`), its tables may hold at most 64 MiB together
    (`MAX_DOCUMENT_BYTES`) and a record line at most 65,536 tokens
    (`MAX_ROW_TOKENS`), each W0602; the first token of a record line (its id) is not
    scanned as a map header; the fixture is opened non-blocking and must be a regular
    file by `fstat` (a FIFO swapped in after the `lstat` cannot hang the generator);
    all-dot path segments (`...`) are refused like `..`; the generator refuses to
    write a table copy or harness through a symlink at any level; the capability
    manifest narrows `fs:read` to `code/specs/fixtures/**` and
    `code/packages/rust/*/tests/generated/**`.
