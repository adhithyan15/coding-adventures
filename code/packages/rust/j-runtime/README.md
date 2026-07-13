# J Runtime

A tree-walking evaluator for [J](https://en.wikipedia.org/wiki/J_(programming_language)),
over [`array-runtime`](../array-runtime). Item **MA-6d** of the J frontend
(spec [`MA06`](../../../specs/MA06-j-language.md)): the piece that makes the
J lexer/parser (`j-lexer`/`j-parser`) executable.

## Why J needed its own runtime, not a reused one

J is APL's direct successor (MA06 §1) and reuses APL's own grammar shape
almost verbatim (MA06 §3) — but three properties have no APL precedent at
all:

1. **Trains.** Two or more verbs (or a leading noun) written consecutively,
   with no operands between them, form a brand-new derived verb: a **hook**
   (exactly 2 teeth) or a **fork** (3+ teeth, folding right-to-left). `(+ *)`
   is a hook; `(+ * -)` is a fork; `(+ - * %)` folds as `(+ (- * %))`. This
   is the one genuinely new grammar/runtime problem MA06 fixes — no existing
   frontend in this repo has a "juxtaposed-verbs-form-a-new-verb" production.
2. **0-based indexing.** `i.5` is `0 1 2 3 4`, never APL's 1-based
   `1 2 3 4 5` — the single most safety-critical numeric difference from
   APL's own `⍳`.
3. **`/` is never division.** J needs `/` for the reduce adverb (mirroring
   APL's own `/`), so division is spelled `%` instead — the single most
   common APL→J transliteration mistake.

```rust
use coding_adventures_j_runtime::eval;

// i. is 0-based -- the signature difference from APL's ⍳.
assert_eq!(eval("i.5\n").unwrap().trim(), "0 1 2 3 4");

// / is reduce, never division -- % is divide.
assert_eq!(eval("+/1 2 3 4\n").unwrap().trim(), "10");
assert_eq!(eval("6%2\n").unwrap().trim(), "3");

// Assignment is silent; a bare expression auto-prints.
assert_eq!(eval("A=.5\n").unwrap(), "");
assert_eq!(eval("A=.5\nA\n").unwrap().trim(), "5");
```

## What it evaluates

- **Arrays only** (MA06 §4): dense, rectangular, numeric. A scalar is a
  rank-0 array. No control flow, no user-defined verbs, no strings, no
  boxing.
- **Primitive verbs** (monadic / dyadic meaning): `+` (conjugate / add), `-`
  (negate / subtract), `*` (sign / multiply), `%` (reciprocal / divide), `^`
  (exponential / power), `<.`/`>.` (floor·ceiling / min·max — note the
  digraph spelling is the *opposite* character from APL's `⌊`/`⌈`, since `<`
  already means "less than"; the underlying min/max meaning is identical to
  APL's own mapping), `$` (shape / reshape), `i.` (index generator,
  **0-based** / index-of), `,` (ravel / catenate), `#` (tally / replicate —
  new relative to this repo's APL cut, which never had a tally primitive),
  `=` `~:` `<` `>` `<:` `>:` (dyadic-only comparison, `1`/`0` result).
- **Adverbs**: `/` (reduce), `\` (scan) — lowered onto
  `array_runtime::ops::{reduce, scan}` (item **AR-2**), same as APL.
- **One conjunction**: `@` (compose/"atop" — `(f@g) y = f (g y)`).
- **Trains**: hooks and forks, parenthesised only (`(f g)`, `(f g h)`, ...),
  including the leading-noun fork case (`(5 * -)`) and the 4+-tooth
  peel-from-the-left folding rule.
- **Assignment** `=.` (local) / `=:` (global) — identical behavior in this
  cut (right-associative chaining: `A=.B=.3` sets both) — and parenthesised
  grouping `( )`.
- **Comments** `NB.` and blank lines (both no-ops; stripped by the lexer).

### Deferred (documented, MA06 §4)

Boxing and nested/ragged arrays, the `"` rank conjunction and axis-specific
reduce/scan, user-defined explicit verbs and named tacit definitions
(only inline parenthesised trains are in scope), the `¨` each-equivalent,
complex numbers, unparenthesised top-level trains (this cut requires
parentheses to avoid a genuine grammar ambiguity). Dyadic `$` (reshape) and
`,` (catenate) are additionally scoped to rank ≤ 2; dyadic `#` (replicate) is
scoped to a rank ≤ 1 right operand.

## Trains: the one genuinely new evaluation shape

`JFn` (this crate's runtime function representation, generalizing
`apl-runtime::eval::AplFn` per MA06 §5) grows three variants beyond APL's
`Atom`/`NonScalar`/`Reduce`/`Scan`:

- **`Compose(f, g)`** (`@`, atop): monadic `f (g y)`; dyadic
  `f (x g y)` (this crate's own considered generalization of "atop" to the
  dyadic case — MA06 only spells out the monadic formula).
- **`Hook(f, g)`**: monadic `(f g) y = y f (g y)`; dyadic
  `x (f g) y = x f (g y)` — `g` *always* applies monadically to `y` alone,
  regardless of the surrounding call's arity. This is the hook's defining
  property, not a bug.
- **`Fork(left, g, h)`**: monadic `(f g h) y = (f y) g (h y)`; dyadic
  `x (f g h) y = (x f y) g (x h y)`. When `left` is a captured literal noun
  `n` instead of a verb: monadic `n g (h y)`; dyadic `n g (x h y)` (the noun
  never depends on `x`/`y` since it isn't "applied" — only `h` picks up the
  surrounding call's arity).

A 4+-tooth train folds by peeling the leftmost tooth off and recursing on
the rest — `(a b c d)` is `Hook(a, Fork(b, c, d))` — per MA06 §3's corrected
folding rule.

## DoS guards

- An independent recursion-depth guard in the evaluator (`eval.rs::MAX_DEPTH`)
  — defense in depth on top of `j-parser`'s own `MAX_RULE_DEPTH` (70), which
  already bounds how deep an untrusted-input CST can be before it ever
  reaches this crate. Unlike APL, this guard is also exercised by
  `apply_monadic`/`apply_dyadic` themselves, since `Compose`/`Hook`/`Fork`
  recurse back through those two functions — a genuinely new recursion
  shape APL's own evaluator never had.
- `builtins::MAX_ARRAY_LENGTH` (1,000,000) bounds every primitive whose
  output size or work scales with runtime-computed values — monadic `i.n`,
  dyadic `$`'s target element count, dyadic `,`'s combined output length,
  dyadic `i.`'s `len(a)×len(b)` work, and dyadic `#`'s total replicated
  output length — each checked *before* allocating or scanning.

## Testing

```sh
cargo test -p coding-adventures-j-runtime
```
