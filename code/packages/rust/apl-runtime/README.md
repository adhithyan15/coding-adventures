# APL Runtime

A tree-walking evaluator for [APL](https://en.wikipedia.org/wiki/APL_(programming_language)),
over [`array-runtime`](../array-runtime). Item **MA-4e** of the APL frontend
(spec [`MA05`](../../../specs/MA05-apl-language.md)): the piece that makes
the APL lexer/parser (`apl-lexer`/`apl-parser`) executable.

## Why APL needed its own runtime, not a reused one

Two properties have no precedent among this repo's other array/symbolic
frontends (MA05 §1):

1. **Functions are values, and operators act on them.** `/` (reduce), `\`
   (scan), and `∘.` (outer product) take a *function* — one of the 12
   primitive glyphs that map onto `array_runtime::ops::BinOp` — and produce
   a **derived function**, applied to array operands afterward. `+/A` reduces
   `A` with `+`; `A∘.×B` is the outer product of `A` and `B` with `×`.
2. **No operator precedence — everything evaluates right-to-left.** `2×3+4`
   is `2×(3+4) = 14`, not `(2×3)+4`. The grammar's right-recursive
   `value_expr` production already encodes this; the evaluator just walks it
   top-down.

```rust
use coding_adventures_apl_runtime::eval;

// Right-to-left evaluation: 2×3+4 is 2×(3+4) = 14, not (2×3)+4 = 10.
assert_eq!(eval("2×3+4\n").unwrap().trim(), "14");

// Assignment is silent; a bare expression auto-prints (real APL session
// behavior — not MATLAB's `;`-suppression).
assert_eq!(eval("A←5\n").unwrap(), "");
assert_eq!(eval("A←5\nA\n").unwrap().trim(), "5");
```

## What it evaluates

- **Arrays only** (MA05 §4): dense, rectangular, numeric. A scalar is a
  rank-0 array. No control flow, no user-defined functions, no strings.
- **Primitive functions** (monadic / dyadic meaning): `+` (conjugate / add),
  `-` (negate / subtract), `×` (sign / multiply), `÷` (reciprocal / divide),
  `⌈`/`⌊` (ceiling·floor / max·min), `⍴` (shape / reshape), `⍳` (index
  generator / index-of), `,` (ravel / catenate), `=` `≠` `<` `≤` `≥` `>`
  (dyadic-only comparison, `1`/`0` result).
- **Operators**: `/` (reduce), `\` (scan), `∘.` (outer product) — lowered
  onto `array_runtime::ops::{reduce, scan, outer}` (item **AR-2**).
- **Assignment** `←` (right-associative chaining: `A←B←3` sets both) and
  parenthesised grouping `( )`.
- **Comments** `⍝` and blank lines (both no-ops; stripped by the lexer).

### Deferred (documented, MA05 §4)

Nested/ragged arrays, mixed numeric+character arrays, user-defined
functions/dfns (`∇`, `{…}`), axis-specific reduce/scan (`⌿`/`⍀`), the `¨`
(each) operator, `⍉` with axis permutation, complex numbers. Dyadic `⍴`
(reshape) and `,` (catenate) are additionally scoped to rank ≤ 2, matching
`array_runtime::ops`'s own ceiling.

## DoS guards

- An independent recursion-depth guard in the evaluator (`eval.rs::MAX_DEPTH`)
  — defense in depth on top of `apl-parser`'s own `MAX_RULE_DEPTH`, which
  already bounds how deep an untrusted-input CST can be before it ever
  reaches this crate.
- `builtins::MAX_ARRAY_LENGTH` (1,000,000) bounds every primitive whose
  output size or work scales with runtime-computed values — monadic `⍳n`,
  dyadic `⍴`'s target element count, dyadic `,`'s combined output length,
  `∘.`'s `len(a)×len(b)` output size, and dyadic `⍳`'s `len(a)×len(b)` work
  — each checked *before* allocating or scanning. `,` and `∘.` in particular
  can each grow a result *larger* than either input, so a naive per-operand
  cap alone isn't enough — see `CHANGELOG.md` for the security-review
  findings that added the `,`/`∘.`/dyadic-`⍳` checks.

## Testing

```sh
cargo test -p coding-adventures-apl-runtime
```
