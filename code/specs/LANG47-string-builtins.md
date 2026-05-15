# LANG47 — String Heap Objects and Builtins

## Motivation

The Twig self-hosted compiler (TW05) manipulates source text: it reads characters,
slices substrings, converts tokens to strings, and formats diagnostics.  Before
LANG47, the runtime has no proper string type — string literals flow through the
"string-as-symbol" convention (they are interned and stored as `LispyValue`
symbols), which makes structural string operations impossible.

LANG47 adds:

1. A `LangString` heap object in `lispy-runtime` (alongside `ConsCell` and `Closure`).
2. A set of `call_builtin` string operations in `twig-vm`.
3. `Operand::Str` in a `const` instruction now produces a heap string instead of an
   interned symbol (the old behaviour was a temporary stand-in per the
   `string-as-symbol` comment in PR 5; LANG34 removed the last user of
   `Operand::Str` in `const` instructions for non-string purposes).

## Non-goals

- A fully Unicode-aware string type.  Strings are UTF-8 byte sequences; character
  operations work on Unicode scalar values (code points) encoded as `u32`s stored
  in `i64` Lispy integers.
- Mutable strings.  All string values produced by LANG47 are immutable heap objects.
- String interning.  `LangString` is a separate type from `Symbol`; `string->symbol`
  and `symbol->string` bridge between them.

## LangString heap object

Layout:

```text
LangString (32 bytes):
  ┌──────────────────────────────────┐
  │  ObjectHeader (16 bytes)         │  class_or_kind = CLASS_STRING = 3
  ├──────────────────────────────────┤
  │  data: Box<[u8]> (16 bytes)      │  fat pointer: ptr (8) + len (8)
  └──────────────────────────────────┘
```

`Box<[u8]>` is the string's UTF-8 content.  The choice of `Box<[u8]>` rather than
`Vec<u8>` is deliberate: strings are immutable after construction, so the unused
capacity field of `Vec` would waste space and invite bugs.

### Class id

```rust
pub const CLASS_STRING: u32 = 3;
```

Added alongside the existing `CLASS_CONS = 1` and `CLASS_CLOSURE = 2`.

### Public API (in `lispy-runtime::heap`)

```rust
/// Allocate a string from UTF-8 bytes.
pub fn alloc_string(bytes: &[u8]) -> LispyValue;

/// `true` iff `v` is a heap-tagged `LangString`.
pub fn is_string(v: LispyValue) -> bool;

/// Borrow the UTF-8 bytes of a heap string.
///
/// # Safety
/// `v` must satisfy `is_string(v)`.  The returned slice is valid for the
/// lifetime of the heap (i.e. `'static` in the current Box::leak model).
pub unsafe fn string_bytes(v: LispyValue) -> &'static [u8];
```

## `const` instruction change

The `const` instruction handler in `twig-vm` previously converted `Operand::Str(text)`
to an interned symbol (the string-as-symbol stand-in from PR 5).  LANG34 removed
the last non-string use of `Operand::Str` in `const` instructions (closure fn names
now live inline in `alloc_closure`'s operand list, not through a `const` register).

After LANG47, `const dest = Str("hello")` produces a `LangString` heap value:

```
before: Operand::Str("hello") → LispyValue::symbol(intern("hello"))
after:  Operand::Str("hello") → alloc_string(b"hello")
```

The `Operand::Var` arm in `const` is unchanged (it still interns as a symbol;
it is used only by the `string_arg` helper for `global_set`, `global_get`, and
`make_symbol` — all of which need symbol identity, not string values).

## Builtins

All string builtins are dispatched via `call_builtin`.  The name column shows the
IIR builtin name (the first `Var` operand to `call_builtin`).

### Type predicates

| Builtin | Signature | Notes |
|---------|-----------|-------|
| `string?` | `(string? v) → Bool` | Returns `#t` iff `v` is a `LangString`. |
| `symbol?` | (existing) | No change. |

### Length and access

| Builtin | Signature | Notes |
|---------|-----------|-------|
| `string-length` | `(string-length s) → Int` | Number of Unicode scalar values (code points). O(n) via UTF-8 decoding but acceptable at this stage. |
| `string-ref` | `(string-ref s i) → Int` | The i-th code point (0-indexed). Error if `i ≥ string-length`. The result is a code point integer, not a Twig `Char` object. |

### Construction

| Builtin | Signature | Notes |
|---------|-----------|-------|
| `string-append` | `(string-append s1 s2) → String` | Concatenate two strings. |
| `substring` | `(substring s start end) → String` | Slice by code-point index `[start, end)`. Error if indices out of range. |
| `make-string` | `(make-string n ch) → String` | Build an n-char string filled with code point `ch`. |

### Comparison

| Builtin | Signature | Notes |
|---------|-----------|-------|
| `string=?` | `(string=? s1 s2) → Bool` | Byte-for-byte equality. |
| `string<?` | `(string<? s1 s2) → Bool` | Lexicographic order. |
| `string>?` | `(string>? s1 s2) → Bool` | Lexicographic order. |

### Conversion

| Builtin | Signature | Notes |
|---------|-----------|-------|
| `number->string` | `(number->string n) → String` | Decimal representation. |
| `string->number` | `(string->number s) → Int \| #f` | Parse decimal integer; `#f` on failure. |
| `string->symbol` | `(string->symbol s) → Symbol` | Intern the string's bytes as a symbol. |
| `symbol->string` | `(symbol->string sym) → String` | Reverse: look up the symbol's name in the intern table and return it as a new `LangString`. |

### Character predicates

Characters in Twig are represented as code-point integers (the result of `string-ref`).

| Builtin | Signature | Notes |
|---------|-----------|-------|
| `char-alphabetic?` | `(char-alphabetic? code) → Bool` | ASCII alphabetic (a-z, A-Z). Caller is responsible for passing valid code-point integers. |
| `char-numeric?` | `(char-numeric? code) → Bool` | ASCII digits (0-9). |
| `char-whitespace?` | `(char-whitespace? code) → Bool` | Space, tab, newline, carriage return, form feed. |
| `char-upper-case?` | `(char-upper-case? code) → Bool` | ASCII A-Z. |
| `char-lower-case?` | `(char-lower-case? code) → Bool` | ASCII a-z. |
| `char->integer` | `(char->integer code) → Int` | Identity function in this encoding (chars *are* integers). Provided for R7RS source compatibility. |
| `integer->char` | `(integer->char n) → Int` | Identity. |
| `char-upcase` | `(char-upcase code) → Int` | ASCII upper-case conversion. |
| `char-downcase` | `(char-downcase code) → Int` | ASCII lower-case conversion. |

## Error handling

String operations that receive a non-string argument return a `RunError::TypeError`.
Out-of-bounds accesses return `RunError::MalformedInstruction`.

## Relationship to refinement types (TW05)

After LANG47, the compiler can express:

```scheme
(define (string-ref (s : String) (i : (Index (string-length s))) -> Int) ...)
```

The `(Index (string-length s))` refinement is a dependent predicate linking the
index to the string's length.  The `iir-refinement-pass` (LANG42) already discharges
static proof obligations for integer ranges.  Dependent predicates over string length
are `Unknown` at the LANG47 stage (the solver does not yet have a model for string
lengths), so lenient mode remains the default for string-indexed operations until
LANG48 (flow-sensitive narrowing) adds guard-based evidence for length checks.

## Tests required

**`lispy-runtime`:**

- `alloc_string` round-trips via `string_bytes`;
- `alloc_string(b"hello").is_heap()` is true;
- `is_string(alloc_string(b"x"))` is true;
- `is_string(LispyValue::int(0))` is false;
- `is_string(alloc_cons(NIL, NIL))` is false;
- `is_string(alloc_closure(...))` is false;
- empty string round-trips;
- multi-byte UTF-8 string round-trips.

**`twig-vm`:**

Unit tests (hand-built IIR modules) for every builtin listed above:
- `string?` returns `#t` for a heap string and `#f` otherwise;
- `string-length` counts code points correctly (ASCII and multi-byte);
- `string-ref` returns the correct code point;
- `string-ref` out-of-bounds returns a `RunError`;
- `substring` returns the correct slice;
- `substring` with bad indices returns a `RunError`;
- `string-append` concatenates two strings;
- `string=?` compares correctly (equal and not-equal cases);
- `string<?` orders correctly;
- `number->string` round-trips for small positive and negative integers;
- `string->number` parses valid and invalid inputs;
- `string->symbol` / `symbol->string` round-trip;
- all char predicates on ASCII corners;
- `char-upcase` / `char-downcase` round-trip.

**`const Operand::Str` change:**

- A `const` instruction with `Operand::Str("hello")` stores a heap string (not a symbol).

## Definition of done

- `cargo test -p lispy-runtime` passes (string tests added).
- `cargo test -p twig-vm` passes (string builtin tests added).
- `cargo build --workspace` clean.
- String operations available to Twig programs via `call_builtin`.
