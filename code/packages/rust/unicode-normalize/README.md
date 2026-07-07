# unicode-normalize — zero-dependency Unicode NFD/NFC (Unicode 17.0.0)

A from-scratch, **zero-dependency** Rust implementation of Unicode canonical
normalization — **NFD** (canonical decomposition) and **NFC** (canonical
composition) — plus `char::is_combining_mark`. Targets **Unicode 17.0.0** and
reproduces the third-party [`unicode-normalization`](https://crates.io/crates/unicode-normalization)
crate's output **bit-for-bit** across every code point.

## Why it exists

The Engram flashcard stack needs to (a) strip accents for search — decompose
text and drop combining marks — and (b) de-duplicate visually-identical strings
by composing them to a canonical form. That needs NFD, NFC, and combining-mark
detection. Pulling in a third-party crate for it violates the repository's
zero-dependency policy, so this crate provides exactly that surface with **no
dependencies**.

## Where it sits in the stack

```
engram-core (search.rs, template.rs)
        │  use unicode_normalize::{char::is_combining_mark, UnicodeNormalize};
        │  value.nfd() … / text.chars().nfc() …
        ▼
   unicode-normalize (this crate)   ← zero third-party deps
```

## What it does

- `str::nfd()` / `Chars::nfd()` — iterate the input in Normalization Form D.
- `str::nfc()` / `Chars::nfc()` — iterate the input in Normalization Form C.
- `char::is_combining_mark(c)` — is `c` a combining mark (`General_Category` = Mark)?
- `char::canonical_combining_class(c)` — the CCC (0 for non-combining characters).

```rust
use unicode_normalize::{char::is_combining_mark, UnicodeNormalize};

// Strip accents: decompose, then drop combining marks.
let ascii: String = "Crème brûlée"
    .nfd()
    .filter(|c| !is_combining_mark(*c))
    .collect();
assert_eq!(ascii, "Creme brulee");

// Canonicalise for de-duplication.
let composed: String = "e\u{0301}".chars().nfc().collect(); // "e" + combining acute
assert_eq!(composed, "\u{00E9}");                            // → "é"
```

## What it does *not* do

Compatibility forms (NFKD/NFKC), streaming/incremental normalization, and the
quick-check tables. Only the canonical NFD/NFC path Engram uses.

## How it works

Three generated tables (canonical combining class, recursive canonical
decomposition, canonical composition) drive binary-search lookups; Korean
**Hangul** syllables are decomposed/composed by arithmetic (per the Unicode
standard) rather than by table, saving ~11,000 entries. The tables were
generated once from the Unicode Character Database (via `unicode-normalization`
0.1.25) and frozen; the generator and that dev-dependency were then removed.

## Fidelity

Before the third-party crate was dropped, a throwaway cross-check compared this
crate against it across **every Unicode scalar value** (~1.1M code points, for
CCC / `is_combining_mark` / single-char NFD / single-char NFC) **and 200,000
random multi-character strings** (NFD/NFC) — zero mismatches.

## Testing

```
cargo test -p unicode-normalize
```
