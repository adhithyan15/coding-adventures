# Q Runtime

A tree-walking evaluator for [Q](https://en.wikipedia.org/wiki/Q_(programming_language_from_Kx_Systems)),
kdb+'s scripting language, over [`array-runtime`](../array-runtime). Item
**MA-11d** of the Q frontend (spec
[`MA11`](../../../specs/MA11-q-language.md)): the piece that makes the Q
lexer/parser (`q-lexer`/`q-parser`) executable.

## Why Q needed its own runtime, not a reused one

Q is APL/J's second-generation descendant (MA11 §1) and reuses their exact
two-nonterminal, right-to-left, no-precedence grammar shape (MA11 §3) — but
one property has no APL/J precedent at all: **a genuine user-defined
function literal**. `{[x;y] stmt; stmt; ...}` is a real callable value with
named parameters, a multi-statement body, and (implicit) access to
whatever's visible at the top level when it's called. Neither
`apl-runtime::eval::AplFn` nor `j-runtime::eval::JFn` has ever needed to
represent this, because APL's/J's in-scope grammars are expression-only —
trains *recombine* existing primitives, they never introduce a brand-new
parameter name a body can reference. This is why `QValue` (this crate's
own value type) is `Arr(Array)` **or** `Fn(Rc<Lambda>)`, not just a bare
`Array` the way `j-runtime`'s evaluator gets away with.

```rust
use coding_adventures_q_runtime::eval;

// `!` (til) is 0-based, matching J's `i.` -- never APL's 1-based `⍳`.
assert_eq!(eval("!5\n").unwrap().trim(), "0 1 2 3 4");

// `%` is always true/float division -- no integer-division special case.
assert_eq!(eval("6%4\n").unwrap().trim(), "1.5");

// A function literal: bracket-omitted implicit x/y, applied by
// juxtaposition exactly like a primitive verb.
assert_eq!(eval("f:{x+y}\n2 f 3\n").unwrap().trim(), "5");

// Assignment is silent; a bare expression auto-prints.
assert_eq!(eval("a:5\n").unwrap(), "");
assert_eq!(eval("a:5\na\n").unwrap().trim(), "5");
```

## What it evaluates

- **Arrays only** (MA11 §4): dense, numeric, rank <= 2. A scalar is a
  rank-0 array. No booleans, no symbols, no strings, no temporal literals,
  no tables.
- **16 primitive verbs** (monadic / dyadic meaning): `+` (flip / add), `-`
  (negate / subtract), `*` (first / multiply — a completely different
  monadic pairing from J's "sign / multiply"), `%` (reciprocal / divide,
  always true/float), `!` (til, **0-based** monadic-only — dyadic is a
  clean, explicit "not yet implemented" error), `,` (enlist / join), `#`
  (count / take — Q's `#` is *take*, genuinely different from J's dyadic
  `#`, which is *replicate*), `_` (floor / drop), `&` (where — indices of
  nonzero elements — / min), `|` (reverse / max), `~` (not / match — deep
  equality, producing a scalar, the one primitive here whose dyadic
  meaning is *not* elementwise), `=` `<` `>` `<=` `>=` `<>` (dyadic-only
  comparison; Q's own not-equal spelling is `<>`, never `~=`/`#`).
- **Adverbs**: `'` (each), `/` (reduce), `\` (scan) — lowered onto
  `array_runtime::ops::{reduce, scan}` (item AR-2), same kernels APL/J
  already reuse. Reduce/scan are restricted to the 12 primitives that map
  onto a `BinOp`; each has a well-defined, non-redundant meaning only for
  the primitives whose direct application isn't already elementwise (see
  "Each degenerates to direct application" below).
- **Function literals**: `{[x;y] ...}` and the bracket-omitted implicit
  `x`/`y`/`z` form. Assignable without being called, passable as an
  argument (including to another function, which can then apply it), and
  applied via the *same* juxtaposition mechanism as a primitive verb — no
  separate "call a lambda" code path. No recursion, no nested function
  literals (both explicitly out of scope, MA11 §4).
- **Dual list-literal syntax**: adjacent stranding (`1 2 3`) and explicit
  `(a; b; c)` both lower to the identical vector value, for the case this
  cut's value model can actually represent (every element a plain scalar).
- **Assignment** `name:expr`, right-associative chaining (`a:b:3` sets
  both), local to the enclosing function call when used inside one.
- **Comments** `/`-to-end-of-line (a lexer-level concern, confirmed here by
  an end-to-end evaluation test) and parenthesised grouping.

### Deferred (documented, MA11 §4)

Booleans, symbols, strings, temporal literals, dictionaries and tables
(the flagship Q/kdb+ feature — q-SQL is by far the largest deferred
surface), `?`/`.`/`@` (all heavily overloaded, deferred whole), each-prior/
each-right/each-left adverbs, recursion, and nested function-literal
definitions. Each of these that the grammar happens to still parse (a few
do, since `q-lexer`/`q-parser` were built slightly ahead of this crate's
own semantic scope) is a clean, specific "not yet implemented" error here —
never silently misinterpreted as something else.

## `QFn::Lambda` — the one genuinely new evaluator concept

`QFn` (this crate's own representation of "which verb, and with which
adverb applied", generalizing `j-runtime::eval::JFn` per MA11 §2's own
instruction) has **no** train-shaped variants at all — Q has no trains and
no `@` compose in this cut (MA11 §3/§4), so every `QFn` variant here is a
leaf dispatch with no self-referential recursion, unlike `JFn` (which
needed a hand-rolled iterative `Drop` specifically because `Compose`/
`Hook`/`Fork` box their own operands).

`Lambda { params: Vec<String>, body: Vec<GrammarASTNode> }` stores no
captured environment at all: since nested function *definitions* are
explicitly out of scope, every `Lambda` is always defined at the top
level, so its body only ever needs its own parameters plus whatever's in
the *global* frame at call time — resolved via this evaluator's ordinary
two-tier lookup (call-local frame first, global frame beneath it), not a
snapshot taken at definition time.

### Each degenerates to direct application (a disclosed design choice)

This cut's value model is flat, dense arrays only — no nested/boxed list
type at all. Real Q's `'` (each) earns its keep by applying a function to
each *item* of a possibly-nested list; without nesting, "apply per
element" and "apply directly" are indistinguishable for every primitive
whose direct meaning is already elementwise (`-` `%` `_` `~` monadically;
all 12 `BinOp`-mappable primitives dyadically). `each` is therefore
well-defined but redundant for those, and a clean, disclosed error for
everything else (`+` `*` `!` `,` `#` `&` `|` monadically; `!` `,` `#` `_`
`~` dyadically) — see `builtins.rs`'s own `each_monadic_supported`/
`each_dyadic_supported` for the exact rationale.

## DoS guards

- An independent recursion-depth guard in the evaluator
  (`eval.rs::MAX_DEPTH`, 512) — **genuinely reachable**, unlike
  `j-runtime`'s identical guard, through a legitimate (if unusual) long
  chain of already-defined-function calls spread across many separate
  top-level lines, since Q (unlike J) has real function calls to chain at
  all.
- `builtins::MAX_ARRAY_LENGTH` (1,000,000) bounds every primitive whose
  output size or work scales with a runtime-computed value (`!n`, take,
  join) and the flat stranded-literal/list-literal element counts, each
  checked *before* allocating or scanning.

## Testing

```sh
cargo test -p coding-adventures-q-runtime
```
