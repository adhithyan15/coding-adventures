# regex-engine — zero-dependency, linear-time regex (`is_match` core)

A from-scratch regular-expression engine covering the subset the Engram
flashcard search needs, built so the Engram stack needs no third-party regex
crate (zero-dependency policy). This first release ships the **`is_match`**
(boolean) core.

## Why a Pike VM

Engram runs **user-supplied** `re:` search patterns. A recursive-backtracking
matcher can take *exponential* time on adversarial patterns (e.g. `(a*)*b`) — a
denial-of-service. This engine compiles to bytecode and runs a **Pike VM**
(Thompson NFA simulation): it advances all possible matches in lockstep, so
matching is always **O(pattern × input)**, never exponential.

## Where it sits in the stack

Engram's search uses regex in three boolean ways — user `re:` patterns,
whole-word matching, and `*`/`_` glob matching — all of which are `is_match`.
Those are the target of this crate. (The one place Engram needs a match *extent*,
the media-tag `replace_all`, keeps using the `regex` crate until a later,
separately-verified change adds extents here.)

## Supported syntax

Literals; `.`; `\d \D \w \W \s \S` and escaped metacharacters; the Unicode
property classes `\p{Alphabetic}`, `\p{Mark}`, `\p{Nd}` (and `\P{…}`);
`[...]`/`[^...]` with ranges; `(...)`/`(?:...)`; `|`; `* + ?` and
`{m}`/`{m,}`/`{m,n}` (greedy or lazy); `^ $`; `\b \B`; leading
`(?i)`/`(?s)`/`(?u)`.

Character classes and `\b` are **Unicode-aware by default** (matching `regex`);
`(?-u)` selects the ASCII sets. `(?i)` uses Unicode simple case folding.

## Match extents

`is_match` (boolean) and `find` (the overall match extent) are implemented;
capture groups and `replace_all` build on `find` in later changes. `find`
resolves ambiguity leftmost-first with greedy quantifiers preferring more — the
`regex` crate's default — and reports **byte** offsets.

Getting extents right for **nullable loops** (a quantified body that can match
empty, e.g. `(a?)*`, `(a??)+`, `(a*)*`) is the subtle part: the star of a nullable
body compiles to an "optional-plus" shape whose empty iteration routes to the loop
exit at the correct priority.

`find` is verified against the `regex` crate by its **defining properties**, not a
byte-identical span — because on adversarial patterns the `regex` crate's own
unanchored `find` returns results its *anchored* matcher contradicts (it can skip
the genuine leftmost match), making its `find` the wrong oracle. Using its anchored
matcher as an independent oracle, 40k+ random cases (greedy *and* lazy quantifiers,
alternation, nested groups, nullable loops; multibyte inputs) confirm every
reported span is a **valid** match at the **leftmost** start. The exact greedy
extents — including the nullable-loop fixes — are pinned by hand-verified unit
tests.

The *reported extent* can still differ from the `regex` crate on some adversarial
patterns, chiefly around **lazy** quantifiers and **overlapping greedy
alternation** (e.g. `.+c+|.+.+`, where `regex` reports `0..6` though the first
branch matches `0..3`): the `regex` crate resolves those ambiguities via its NFA's
specific thread priority, which differs from textbook leftmost-first. **`is_match`
stays exact** regardless (separately cross-checked over 35k+ pairs). Real search
patterns — including Engram's only extent consumer, the media-tag regex (disjoint
alternation, greedy throughout) — are unaffected.

## Usage

```rust
use regex_engine::{escape, Regex, RegexBuilder};

assert!(Regex::new(r"\bcat\b").unwrap().is_match("the cat sat"));
let ci = RegexBuilder::new("hello").case_insensitive(true).build().unwrap();
assert!(ci.is_match("HELLO"));

// `find` reports the leftmost match's byte extent and substring.
let m = Regex::new(r"\d+").unwrap().find("abc123def").unwrap();
assert_eq!((m.start(), m.end(), m.as_str()), (3, 6, "123"));

// `escape` neutralizes metacharacters so a string matches literally — used when
// building a pattern that interleaves user text with wildcard fragments.
assert_eq!(escape("a.c"), r"a\.c");
assert!(Regex::new(&escape("a.c")).unwrap().is_match("a.c"));
assert!(!Regex::new(&escape("a.c")).unwrap().is_match("axc"));
```

## Fidelity

Cross-checked against the live `regex` crate across **100k+ pairs in ASCII mode**,
**80k+ in Unicode mode** (`\p{…}`, non-ASCII), and **60k+ case-insensitive**
(Unicode fold orbits) — zero `is_match` divergences. `find` is verified as a
leftmost + valid match over **40k+ random cases** (full construct space, multibyte
input) against `regex`'s anchored oracle, plus **35k+** `is_match` pairs across the
same space — all exact.

## Testing

```
cargo test -p regex-engine
```
