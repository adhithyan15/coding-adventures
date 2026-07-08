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
`(?-u)` selects the ASCII sets. Unicode case folding and match extents are
planned additions.

## Usage

```rust
use regex_engine::{Regex, RegexBuilder};

assert!(Regex::new(r"\bcat\b").unwrap().is_match("the cat sat"));
let ci = RegexBuilder::new("hello").case_insensitive(true).build().unwrap();
assert!(ci.is_match("HELLO"));
```

## Fidelity

Cross-checked against the live `regex` crate across **100k+ random (pattern,
input) pairs in ASCII mode** and **80k+ in default Unicode mode** (with
non-ASCII inputs and `\p{…}` classes) — zero `is_match` divergences.

## Testing

```
cargo test -p regex-engine
```
