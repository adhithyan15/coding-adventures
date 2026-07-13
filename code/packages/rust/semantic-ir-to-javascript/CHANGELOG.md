# Changelog

## 0.35.0 — SIR23 symbolic-expression + pattern/rewrite codegen (HML01 Stream B, item 7 JS half)

Real codegen for the SIR23 symbolic/pattern domain, replacing the deferred
`panic!` placeholder these seven `Expr` nodes had (`SymSymbol`, `SymRational`,
`SymApply`, `SymPatternBlank`, `SymPatternNamed`, `SymRule`,
`SymReplaceAll`). Mirrors the TypeScript backend's SIR23 codegen (already
shipped) exactly at the `emit.rs` call-site shape, but targets an *inlined*
runtime rather than an imported npm package — the same "port it inline so the
JavaScript artifact stays self-contained" treatment the exception runtime
(`sir-runtime-exceptions`) already got.

- `runtime.rs` gains `__Sir.Symbolic`, a plain-JS port of the published
  `@coding-adventures/symbolic-ir` (term-tree type + constructors),
  `@coding-adventures/cas-pattern-matching` (the five-case structural
  matcher/substitution algorithm), and `@coding-adventures/sir-runtime-symbolic`
  (`replaceAll`/`replaceRepeated`/`unwrap`) TypeScript packages. Deliberate
  divergence from the TS packages: terms use plain JS `number` for
  `integer`/`rational` values rather than `bigint`, matching how every other
  numeric value in this backend already works (`IntLit` emits a bare JS number
  literal) — there is no `bigint` anywhere else in this runtime.
  `replaceRepeated` carries forward the TS package's own
  `/security-review`-found fix: a rule firing loops at the *same* call frame
  (not a recursive call), so a caller-supplied `maxIterations` bounds CPU time
  only, never native stack depth. `MAX_TERM_DEPTH = 512` caps the tree walk
  itself against unbounded runtime data (CWE-674).
- `emit.rs`: all seven `Expr::Sym*` arms now emit real `__Sir.Symbolic.*` calls
  instead of panicking; `emit_sym_operand` wraps a bare `IntLit`/`FloatLit`/
  `StrLit` operand through the matching leaf-term constructor before it can
  sit inside a term tree.
- `lib.rs`: `Feature::SymbolicExpr`, `Feature::PatternMatching`, and
  `Feature::Rationals` (shared with the still-deferred SIR22 array/matrix
  domain) join `ACCEPTED_FEATURES`.
- `formatSeen` (the `print`/`puts` display path) now recognizes a Symbolic
  term (a plain object carrying a `.kind` tag) and renders it via a new
  `Symbolic.toDisplayString` — so `print`ing a `SymReplaceAll` result reads as
  `f(x, 1/3)` rather than `[object Object]`.
- New `tests/sir23_symbolic.rs`: three real `node`-execution tests (not just
  string-shape assertions) proving the ported algorithm is actually correct —
  `replaceRepeated` reduces `Add(Add(z, 0), 0)` to the bare symbol `z` via
  `x_ + 0 -> x_`, `replaceAll`'s single-pass contract, and a head-typed blank
  (`x_Integer`) matching selectively. `lib.rs` gains the TypeScript backend's
  own shape-assertion test suite (leaf constructors, literal wrapping,
  blank/blankTyped, the non-`SymSymbol`-head panic guard, `rule` vs.
  `ruleDelayed`, and `replaceAll`/`replaceRepeated` both routing through
  `unwrap`), plus an end-to-end `wolfram-to-semantic-ir` compile test (new dev
  dependency).

## 0.34.0 — Array `cycle(n)`

Mirrors the Python reference (PR #8117), Go (PR #8123), and Rust (PR #8131) into
the JS backend's inline `arrayMethod` (beside the existing `chunk_while`/
`slice_when` arms + the `ARRAY_METHODS` `respond_to?` set), continuing the
`cycle` cross-backend cascade.

- `cycle(n) { |x| … }` (block) → iterate the array `n` full passes in order,
  yielding each element on every pass; always returns `null` (Ruby nil).
  `[1,2,3].cycle(2)` yields `1,2,3,1,2,3`. `n <= 0`, a negative count, an empty
  receiver, or a nil / non-integer count (Ruby's block-less Enumerator and
  infinite no-`n` forms) yields nothing rather than hanging — the count is taken
  only when `Number.isInteger(args[0])`, so a `null` / non-number falls through
  to the no-yield path. `respond_to?("cycle")` reports `true`.
- The `run_with_node` suite gains `array_cycle`: the block `print`s each yielded
  element, proving the two passes (`1,2,3,1,2,3`) and the `nil` returns for
  `cycle(2)`, `cycle(0)`, and `[].cycle(5)` under a real `node` run.

## 0.33.0 — Array `minmax`

Mirrors the Python reference (PR #8092), Go (PR #8098), and Rust (PR #8103) into
the JS backend's inline `arrayMethod` (beside the existing `min`/`max` arms + the
`ARRAY_METHODS` `respond_to?` set), continuing the `minmax` cross-backend
cascade.

- `minmax` (non-block) → the two-element array `[min, max]` in one pass, via
  `<`/`>` (the same comparison the `min`/`max` arms use). `[3,1,2].minmax` →
  `[1, 3]`. An empty array yields `[null, null]` (Ruby `[nil, nil]` — no
  smallest/largest element), matching the Go/Rust/Python references' 2-element
  nil array.
- The `array_catalog_methods` exec-proof test gains `minmax` (non-empty and
  empty), run through `node`, asserting `[1, 3]` / `[nil, nil]`.

## 0.32.0 — Array `slice_when`

Mirrors the Python reference (PR #8070), Go (PR #8073), and Rust (PR #8077) into
the JS backend's inline `arrayMethod` (+ the `ARRAY_METHODS` `respond_to?` set),
continuing the `slice_when` cross-backend cascade.

- `slice_when { |prev, cur| pred }` is the INVERSE of `chunk_while`: it splits
  into runs of consecutive elements, starting a NEW run BETWEEN an adjacent pair
  exactly WHERE the block is truthy (whereas `chunk_while` starts a new run where
  the block is FALSY).
  `[1,2,4,9,10,11,12].slice_when { |a,b| b-a>1 }` → `[[1,2],[4],[9,10,11,12]]`;
  an empty array yields `[]`, a single element `[[x]]`.
- `tests/run_with_node.rs::array_slice_when` emits a program with a `b - a > 1`
  predicate, runs it through `node`, and asserts the printed runs.

## 0.31.0 — Array `tally`

Mirrors the Python reference (PR #8054) into the JS backend's inline
`arrayMethod` (+ the `ARRAY_METHODS` `respond_to?` set), completing the JS side
of the `tally` cross-backend catch-up (Go and Rust already shipped it).

- `tally` → a Hash counting how many times each element occurs, keyed in
  first-seen order (`["a","b","a","c","a"].tally` → `{"a"=>3, "b"=>1, "c"=>1}`;
  `[].tally` → `{}`).
- Realised as an insertion-ordered `Map` — the same shape `group_by` returns,
  printed `{k: v}` by the shared display path (`formatSeen`).  Keys compare by JS
  SameValueZero, which agrees with Ruby `eql?`/hash on the scalar elements this
  covers, matching the Go/Rust/Python references.
- `tests/run_with_node.rs::array_tally` emits three programs (string counts with
  first-seen ordering, integer counts, empty → `{}`), runs them through `node`,
  and asserts the printed Hash.

## 0.30.0 — Array `each_slice` / `each_cons` / `chunk_while`

Mirrors the Python reference (PR #8031), Go (PR #8036), and Rust (PR #8042) into
the JS backend's inline `arrayMethod` (+ the `ARRAY_METHODS` `respond_to?` set),
adding the Array consecutive-grouping family.

- `each_slice(n)` → consecutive sub-arrays of at most `n` elements, the last
  possibly shorter (`[1,2,3,4,5].each_slice(2)` → `[[1,2],[3,4],[5]]`).
- `each_cons(n)` → every consecutive `n`-element sliding window
  (`[1,2,3,4].each_cons(2)` → `[[1,2],[2,3],[3,4]]`); a window larger than the
  array yields `[]`.
- Both read `n` via `Number.isInteger` and treat `n <= 0` as `[]` (Ruby raises
  `ArgumentError`; the never-throw floor yields empty).
- `chunk_while { |prev, cur| pred }` → runs of consecutive elements; the block is
  called on each ADJACENT pair, a truthy result extends the run and a falsy one
  starts a new run (`[1,2,4,5,7].chunk_while { |a,b| b-a==1 }` →
  `[[1,2],[4,5],[7]]`).  Empty → `[]`; single element → `[[x]]`.

Exec-proof: `tests/run_with_node.rs` gains `array_each_slice_each_cons_chunk_while`,
running each_slice/each_cons (incl. `n<=0` and oversized-window → `[]`) and
chunk_while (adjacent `b-a==1` predicate; empty → `[]`) under real `node`, diffed
against the Python/Go/Rust reference.

## 0.29.0 — Hash `to_h` (block + no-block) / `each_with_index` / `each_with_object`

Mirrors the Python reference (PR #8009), Go (PR #8015), and Rust (PR #8020)
into the JS backend's inline `hashMethod` (+ the `HASH_METHODS` `respond_to?`
set), rounding out Hash's Enumerable iteration surface.

- `to_h` **without** a block → a shallow copy of the hash (`new Map(recv)`, so
  mutating it does not alias the receiver).
- `to_h { |k, v| [new_k, new_v] }` → a NEW `Map` from the block-returned
  `[k, v]` pairs; the block is yielded the two args `(k, v)`; a non-pair result
  (checked `Array.isArray` + length 2) is skipped, and a later pair with a
  duplicate key wins (Ruby's rule / `Map.set`).
- `each_with_index { |(k, v), i| … }` → yields each `[k, v]` pair with its
  0-based index, returns the receiver.
- `each_with_object(memo) { |(k, v), memo| … }` → yields each `[k, v]` pair with
  the memo, returns the memo; no-memo arg returns the receiver.

Unlike `each`'s two-arg `(k, v)` yield, `each_with_index`/`each_with_object` pass
the element as a single `[k, v]` JS Array (the second block param is the
index/memo), matching Ruby's Enumerable convention.  (A printed hash already
renders `{k: v}` after the display fix in 0.28.0.)

Exec-proof: `tests/run_with_node.rs` gains `hash_to_h_and_indexed_iteration`,
running to_h (copy + re-map), each_with_index (observed pair+index yield, returns
self), and each_with_object (observed pair+memo yield, returns memo, and no-memo
passthrough) under real `node`, diffed against the Python/Go/Rust reference.

## 0.28.0 — Hash Enumerable breadth: `group_by` / `partition` / `flat_map` / `collect_concat` / `reduce` / `inject` / `sum` (+ Hash display)

Mirrors the Python reference (PR #7978), the Go backend (PR #7983), and the Rust
backend (PR #7989) into the JS backend's inline `hashMethod` (+ the `HASH_METHODS`
`respond_to?` set), completing the Hash Enumerable reshape/fold surface.  Ruby's
`Hash` mixes in `Enumerable`, so every method iterates the hash as `[key, value]`
pairs and yields the two-arg `(key, value)` EXCEPT `reduce`/`inject`, which follow
Ruby's memo convention and yield `(memo, [k, v])` — the pair as ONE argument.

- `group_by { |k, v| key }` — a `Map` from each block key to the Array of the
  `[k, v]` pairs that produced it, in first-seen key order (mirrors Array#group_by,
  which also returns a `Map`).
- `partition { |k, v| pred }` — `[[matching pairs], [rest pairs]]`.
- `flat_map`/`collect_concat { |k, v| … }` — map then concatenate one level (an
  Array result splices, a scalar appends).
- `reduce`/`inject` — Ruby's memo fold over the `[k, v]` pairs; explicit seed or
  first pair; empty seedless → `nil`.
- `sum(init = 0) { |k, v| … }` — numeric fold seeded at `0` (or the seed arg) over
  the block results (native `+`, same as Array#sum).

**Hash display fix:** `formatSeen` previously had no `Map` branch, so a printed
Hash rendered as `[object Map]` (every prior hash test called `.to_a` first, so
this was never exercised).  A Hash now renders `{k: v, …}` — the same surface the
Go/Rust backends emit — so a printed `group_by` result round-trips identically
across backends.  Cycle-guarded via `seen` like Arrays (`{...}` on a self-cycle).

Exec-proof: `tests/run_with_node.rs` gains `hash_enumerable_breadth`, running
`group_by`/`partition` (even-value predicate), `flat_map` (pair projection), `sum`
(value projection), and `reduce(100)` (memo `acc + pair[1]` via `SeqIndex`) under
real `node`, diffed against the Python/Go/Rust reference semantics.

## 0.27.0 — Hash Enumerable aggregates: `find` / `any?` / `all?` / `none?` / `count` / `sort_by` / `min_by` / `max_by`

Mirrors the Python `sir-runtime-oop` v0.1.19 reference (PR #7957) into the
JavaScript backend's emitted runtime (`hashMethod` + the `HASH_METHODS`
`respond_to?` set).  Ruby's `Hash` mixes in `Enumerable`, so these iterate the
hash as a sequence of `[key, value]` pairs: the block is yielded `(key, value)`
(two arguments, matching `each`), and the "element" an aggregate returns is the
two-element `[key, value]` Array.

- `find`/`detect` — first `[k, v]` pair with a truthy block result; `nil` if none.
- `any?`/`all?`/`none?` — booleans over `block(k, v)` (the block-less forms
  degrade to the emptiness checks Ruby uses).
- `count { |k, v| … }` — number of pairs with a truthy block result (block-less
  `count` returns the size).
- `sort_by` — a NEW Array of `[k, v]` pairs sorted by the block key (`arrCmp`,
  the never-throw comparator used by `Array#sort_by`).
- `min_by`/`max_by` — the extremal `[k, v]` pair (first-on-tie; `nil` on empty).

Because these return plain JS Arrays (not a `Map`), they format directly.

Exec-proof: `tests/run_with_node.rs` gains `hash_enumerable_aggregates`, running
`sort_by`/`min_by`/`max_by` (by value) and `find`/`count`/`any?`/`all?`/`none?`
(even-value predicate) under real `node`, diffed against the Python reference.

## 0.26.0 — Hash transforming block methods: `transform_values` / `transform_keys`

Mirrors the Python `sir-runtime-oop` v0.1.18 reference (PR #7909) into the
JavaScript backend's emitted runtime (`hashMethod` + the `HASH_METHODS`
`respond_to?` set), adding two non-mutating Ruby `Hash` block methods:

- `transform_values { |v| … }` — builds a **new** `Map` whose keys are copied
  verbatim (unique ⇒ no collision) and whose values are the block results.
  Yields ONE block argument (the value); insertion order is preserved.
- `transform_keys { |k| … }` — builds a **new** `Map` whose values are untouched
  and whose keys are the block results (yields ONE argument, the key).  Two
  source keys can collapse onto one new key; Ruby keeps the **last** colliding
  entry's value at the **first-seen** position — which is exactly how native
  `Map.set` behaves on an existing key (updates the value, keeps the slot).

Both leave the receiver unmodified (a non-function block returns a shallow copy
of the receiver, matching the sibling `select`/`reject` arms).

Exec-proof: `tests/run_with_node.rs` gains `hash_transform_values_and_keys`,
running under real `node` a `transform_values` case
(`{a:1,b:2}.transform_values { 99 }.to_a` → `[[a, 99], [b, 99]]`), an identity
`transform_keys` (→ `[[a, 1], [b, 2]]`), and a **collision** `transform_keys`
(constant `"z"` key ⇒ `[[z, 2]]`), diffed against the Python/TS reference.

## 0.25.0 — Numeric breadth: `divmod` / `fdiv` / `round(ndigits)` / `clamp` / `between?`

Mirrors the Python `sir-runtime-oop` v0.1.17 reference (and the Go v0.25.0 /
Rust v0.26.0 backends) into the JavaScript backend's inlined runtime
(`numericMethod` + the `NUMERIC_METHODS` `respond_to?` set), adding five Ruby
numeric methods:

- `round(ndigits)` — `round` gains an optional digits argument: a positive
  `ndigits` rounds to that many decimals (half **away from zero**, via
  `rubyRound`, not `Math.round`); `ndigits <= 0` rounds to a power of ten. JS
  numbers are f64, so a hostile-magnitude `ndigits` degrades naturally (the
  `factor` saturates to `Infinity` and `recv / Infinity` is `0`) — no bignum,
  no allocation, no i64-overflow pitfall. A non-finite receiver returns
  unchanged.
- `divmod(n)` — `[quotient, remainder]` with a floored quotient and the
  divisor-signed remainder (a JS array, so it prints `[3, 1]`); a zero divisor
  raises a typed `ZeroDivisionError`.
- `fdiv(n)` — floating-point division that never throws: a zero divisor yields
  `Infinity`/`-Infinity`/`NaN` (JS `/` already produces these).
- `clamp(min, max)` / `between?(min, max)` — compared numerically.

Dispatch stays an explicit `switch` on the literal method name (never
reflection). Exec-proven end-to-end under Node (the `numeric_catalog_nonblock_methods`
test now covers `round(2)`/`round(-2)`, `divmod` incl. the divisor-signed
remainder, `fdiv` incl. the divide-by-zero `Infinity`, and `clamp`/`between?`).

## 0.24.0 — Hash breadth: `fetch` / `clear` / `[]=`

Closes the JavaScript-backend Hash parity gap (the Python/TS reference and the
Go/Rust runtimes already carry these) by adding three Ruby `Hash` methods to the
inlined runtime's `hashMethod` switch and the `HASH_METHODS` `respond_to?` set:

- `fetch(k[, default])` — returns the value for `k` if present; a **missing** key
  with no default raises a typed `KeyError` (unlike `hash[k]`, which returns
  `nil`), so a translated `rescue KeyError` catches it; a second argument
  supplies a default returned instead of raising. (The block form is deferred.)
- `clear` — mutates, removing every pair, and returns the now-empty receiver.
- `[]=` — wired as an explicit alias of `store` (`recv.set(k, v)`; returns `v`).

Dispatch stays an explicit `switch` on the literal method name (never
`recv[name]`). Exec-proven end-to-end under Node (the `hash_catalog_methods`
test now also covers `[]=`, `fetch` present/default, and `clear`; the
missing-key `KeyError` path remains covered by
`t3_hash_fetch_missing_raises_key_error`).

## 0.23.0 — String char-set methods: `tr` / `count` / `delete` / `squeeze`

Adds four non-block Ruby String methods to the inlined runtime's `stringMethod`
switch and the `STRING_METHODS` `respond_to?` set, iterating by code point
(`for..of` / `[...str]`) so a multibyte receiver is never split, mirroring the
Python/Go/Rust reference:

- `tr(from, to)` — position-wise code-point translation; a shorter `to` repeats
  its last code point, an empty `to` deletes matching code points, and a
  repeated code point in `from` keeps the last mapping.
- `count(*sets)` / `delete(*sets)` / `squeeze(*sets)` — char-set methods:
  `count` tallies receiver code points in the set, `delete` removes them, and
  `squeeze` collapses consecutive runs (of set code points, or of *all* when no
  set is given). Multiple set arguments intersect (Ruby's rule).

Each `set`/`from`/`to` argument is treated **literally** — the range (`"a-z"`)
and negation (`"^abc"`) forms are a follow-up, matching the literal-only
`sub`/`gsub` precedent. Exec-proven end-to-end under Node. Fourth backend of the
String char-set sweep (Python, Go, Rust already landed).

## 0.22.0 — Ruby value-equality for `Array#include?` / `Array#index`

Fixes two native-alias semantic divergences on Array receivers, routed through
the explicit `arrayMethod` switch (never `recv[name]`) with a new `valEq` helper
that mirrors the Go/Python reference `_sir_value_eq` (scalars by `===`, Symbols
by name, Arrays element-wise, Maps entry-wise):

- **`index`** was previously **absent** for arrays (`index` ≠ native `indexOf`,
  not aliased, not on the allowlist) → `[1, 2, 3].index(2)` raised NoMethodError.
  It now returns the first index whose element `== x` by **value**, or **`nil`**
  when absent (native `indexOf` returns `-1` and uses identity).
- **`include?`** previously used native `Array#includes` (SameValueZero /
  identity), so a nested Array or Symbol wrongly missed. It now compares by
  **value**, so `[[1, 2]].include?([1, 2])` is `true`, matching Ruby and the
  sibling backends. (String `include?` is unaffected — strings resolve via
  `stringMethod`/the native alias before the Array path.)

Exec-proven end-to-end under Node.

> Deferred (display-frontier entanglement): `Array#join`'s default separator
> (Ruby `""` vs native JS `","`) and element `to_s` rendering intersect the
> in-progress source-language display-convention work, so they are left to that
> effort rather than fixed here.

## 0.21.0 — non-block Array catch-up: `flatten` / `compact` / `rotate` / `zip`

Closes the JS backend's remaining gap on the reference (Go/Rust/Python/TS)
non-block Array surface. Adds four methods to the inlined runtime's `arrayMethod`
switch and the `ARRAY_METHODS` `respond_to?` set — all Ruby-correct and routed
through the explicit switch (never `recv[name]`):

- `flatten` — fully flattens nested Arrays (`flatten(n)` to depth `n`, a negative
  `n` meaning no limit). Handled explicitly rather than via the native `flat`
  alias, so the no-arg form is full-depth (not JS `flat`'s default depth 1). Only
  Array elements flatten; strings and other values stay intact, matching Ruby.
- `compact` — a copy with every `nil` (`null`) removed.
- `rotate(n=1)` — rotate left by `n` (a negative `n` rotates right); the modulo
  wraps so any magnitude terminates, and an empty array stays `[]`. A non-numeric
  arg degrades to `0`.
- `zip(*others)` — an Array of tuples `[self[i], others..[i]]` of length
  `recv.length`; a shorter operand pads with `nil` (`null`), a longer one is
  truncated, and a non-array operand is treated as empty (pad-only).

Exec-proven end-to-end under Node.

## 0.20.0 — slice-selection Array methods: `take` / `drop` / `values_at`

Extends the inlined JS runtime's `arrayMethod` switch (and the `ARRAY_METHODS`
`respond_to?` set), mirroring the Go/Rust backends:

- `take(n)` / `drop(n)` — the first / all-but-first `n` elements; `n` is clamped
  to `[0, len]` (`n <= 0` → `[]`/full copy, `n > len` → full copy/`[]`), so
  `recv.slice` never throws. A negative `n` raises `ArgumentError` in Ruby; the
  never-raise floor treats it as `0`.
- `values_at(*idxs)` — the element at each index, folding a negative index from
  the end once; an out-of-range index yields `null` (never throws).

Dispatch stays an explicit `switch (name)` — never `recv[name]`. Verified
end-to-end under Node.

## 0.19.0 — more String methods: `ljust` / `rjust` / `center` / `swapcase`

Extends the inlined JS runtime's `stringMethod` switch (and the `STRING_METHODS`
`respond_to?` set):

- `ljust(width, pad = " ")` / `rjust(...)` / `center(...)` — pad to `width`
  **runes** using `pad` cyclically; `width <= length` returns the string
  unchanged; `center` puts an odd extra pad rune on the RIGHT (Ruby's rule).
  An empty pad degrades to a single space (never-raise floor).
- `swapcase` — flips the case of each ASCII letter (rune-aware; non-letters and
  non-ASCII code points pass through).

Dispatch stays an explicit `switch (name)` — never `recv[name]`. Verified
end-to-end under Node.

## 0.18.0 — Ruby Array / Enumerable method catalog

Adds a hand-implemented Ruby Array/Enumerable catalog (`arrayMethod`) to the
emitted JS runtime, dispatched by an **explicit `switch` on the source-derived
name** (never `recv[name]`) ahead of the native-method allowlist. JS arrays
previously had **no** Ruby Array catalog — only native JS methods via the
allowlist — so Ruby-named methods (`select`/`reject`/`inject`/`detect`/`any?`/
…) were unsupported, and `sort` used JS's lexicographic default (wrong for
numbers: `[10, 2].sort == [10, 2]`).

Methods: `each`, `each_with_index`, `map`/`collect`, `select`/`filter`,
`reject`, `find`/`detect`, `reduce`/`inject`, `any?`/`all?`/`none?`, `count`
(block/arg/bare), `sort` (numeric via `<`/`>`), `sort_by`, `min`/`max`,
`min_by`/`max_by`, `group_by` (→ a `Map` Hash), `partition`, `flat_map`/
`collect_concat`, `take_while`/`drop_while`, `each_with_object`, `sum`
(with optional init/block), `uniq`, `first`/`last` (with optional count),
`empty?`, `to_a`. Predicates route through SIR `truthy`; a block-less block
method falls through (`ARR_MISS`) so native mutators/accessors
(`push`/`pop`/`slice`/…) still resolve. `respond_to?` kept honest via
`ARRAY_METHODS`.

Verified end-to-end under Node (`run_with_node`): numeric `sort`, the
previously-missing `select`/`reject`/`inject`, and the full breadth set.

(Stacked on the v0.17.0 Symbol-catalog change.)

## 0.17.0 — Ruby Symbol method-catalog parity

Adds a hand-implemented Ruby Symbol catalog (`symbolMethod`) to the emitted JS
runtime for `Sym` receivers, dispatched by an **explicit `switch` on the
source-derived name** (never `recv[name]`) ahead of the native allowlist. This
completes JS's core method-dispatch surface (Numeric + String + Hash + Symbol).

Methods: `to_s` (name string), `to_sym` (self), `inspect` (`:`-prefixed form),
`length`/`size` (rune count), `empty?`, `upcase`/`downcase`/`capitalize`
(Ruby-faithfully returning a **new Symbol**, e.g. `:foo.upcase == :FOO`), and
`to_proc` (a Closure that dispatches `.name(rest…)` on its first argument —
routed back through `callMethod`'s allowlist/method-table gate, never
`recv[name]`, per the C3 RCE discipline). `respond_to?` is kept honest via
`SYMBOL_METHODS`.

Verified end-to-end under Node (`run_with_node`): the emitted JS executes and
matches Ruby-faithful output for the catalog.

(Stacked on the v0.16.0 display-convention change.)

## 0.16.0 — source-language display convention: Ruby booleans (`true`/`false`)

Mirrors the Rust/Go backends' display-convention increment (SIR
display-convention spec) to JavaScript. A **Ruby**-sourced module now renders
booleans as `true`/`false` instead of the Twig/Lisp `#t`/`#f`, so a translated
`puts true` prints `true`.

Mechanism: the runtime carries a `const SIR_DISPLAY_RUBY` (a
`__SIR_DISPLAY_RUBY__` placeholder); the emitter substitutes `true`/`false`
from `Module.metadata.source_language` (`== "ruby"` → `true`, else `false`).
`formatSeen` branches the boolean arm on it. The default is the Lisp form, so
all existing non-Ruby (Twig) output is **byte-for-byte unchanged**.

Scope: booleans only; `nil`, symbols, string `inspect` quoting, and the Ruby
hash `=>` element form remain follow-ups per the spec's rollout. Verified
end-to-end under Node: Ruby source → `true\nfalse`; Twig source → `#t\n#f`.

## 0.15.0 — Ruby Hash method-catalog parity

Adds a hand-implemented Ruby Hash catalog (`hashMethod`) to the emitted JS
runtime for `Map` receivers, dispatched by an **explicit `switch` on the
source-derived name** (never `recv[name]`) ahead of the native allowlist. This
also fixes a latent bug: `keys`/`values` previously mis-routed to the native
`Map.prototype.keys()`/`values()`, which return lazy iterators rather than the
Ruby Arrays a translated program expects.

Methods: `keys`, `values`, `size`/`length`, `empty?`, `has_key?`/`key?`/
`include?`/`member?`, `has_value?`/`value?`, `to_a` (Array of `[k, v]` pairs),
`merge` (non-mutating), `dig` (nested, nil on miss), `invert`, `delete`
(mutating, returns removed value), `store`, and block-taking `each`/`each_pair`,
`map`, `select`/`filter`/`reject`. Value comparison uses `===` (exact for
primitives / strings / interned symbols — deep-equal is a follow-up).
`respond_to?` kept honest via `HASH_METHODS`. `fetch` (raising) is unchanged.

Verified end-to-end under Node (`run_with_node`): keys/values/to_a as real
Arrays, `dig`, a `merge`-chain, and `delete` mutation all Ruby-faithful.

(Stacked on the v0.14.0 String-catalog change.)

## 0.14.0 — Ruby String method-catalog parity

Adds a hand-implemented Ruby String catalog (`stringMethod`) to the emitted JS
runtime, dispatched by an **explicit `switch` on the source-derived name**
(never `recv[name]`) ahead of the native-method allowlist — so the methods with
no JS-native spelling or with diverging semantics resolve, while the existing
aliased natives (`upcase`→`toUpperCase`, `strip`→`trim`, …) still fall through.

Methods: `capitalize`, `chomp`, `chars`, `bytes`, `to_i`, `to_f`, `to_sym`,
`to_s`, `empty?`, `size`, `reverse` (rune-aware; JS strings have no native
`reverse`), `index` (rune index), and literal `sub`/`gsub` (first/all
occurrence, no regex or back-reference expansion — Ruby's string-argument
semantics). Non-string arguments are guarded and degrade to the receiver/`nil`
rather than throwing. `respond_to?` is kept honest via `STRING_METHODS`.

Verified end-to-end under Node (`run_with_node`): the emitted JS executes and
matches Ruby-faithful output for the catalog.

(Stacked on the v0.13.0 Numeric-catalog change.)

## 0.13.0 — Ruby Numeric method-catalog parity

Adds a hand-implemented Ruby Numeric catalog (`numericMethod`) to the emitted
JS runtime, dispatched by an **explicit `switch` on the source-derived name**
(never `recv[name]`) ahead of the native-method allowlist — so `gcd`/`digits`/
`upto`/… resolve on a `number` receiver while `toString`/`toFixed` still fall
through to the RCE-hardened allowlist. Brings JS toward the Go/Rust/Python
Numeric surface.

Methods: `abs`, `to_i`/`to_int`, `to_f`, `even?`, `odd?`, `zero?`,
`positive?`, `negative?`, `succ`/`next`, `pred`, `floor`, `ceil`, `round`
(Ruby round-half-away-from-zero via `rubyRound`), `gcd`, `pow`/`**`, `digits`,
and the block-taking walkers `times`, `upto`, `downto`, `step`. A non-numeric
argument degrades to `0` (`numArg`, the lenient never-raise floor); a zero/
non-numeric `step` stride yields nothing rather than spinning. `respond_to?`
is kept honest via `NUMERIC_METHODS` (mirrors the case labels exactly).

Verified end-to-end under Node (`run_with_node`): the emitted JS executes and
matches Ruby-faithful output for the catalog and a block-driven `upto`.

## 0.12.0

### Added — M6 universal Object metaprogramming surface (send/tap/then/respond_to?)

Parity fill: the M6 Kernel/Object surface already shipped in the Python and
TypeScript backends is now ported to the JS OOP runtime (`callMethod` in
`src/runtime.rs`), matching those references' return-value rules exactly. These
methods are mixed into EVERY receiver — primitives, arrays, hashes, and
user-defined `SirInstance`s alike.

- **`send` / `__send__` / `public_send`** — the first argument (a Symbol or
  string) names a method; dispatch re-enters `callMethod` with that name and the
  remaining args, so `x.send(:upcase)` is exactly `x.upcase` and a trailing
  block survives. **Security-critical (the C3 dynamic-dispatch RCE lesson):** the
  dynamic name routes through the SAME gate a direct call uses — the explicit
  `(class, method)` `Map` for a `SirInstance`, the fixed `METHOD_ALLOWLIST` for a
  primitive. There is NO `recv[name]`, `eval`, `new Function`, or host reflection
  on the source-derived name; an unknown/gadget name (`constructor`, `__proto__`,
  …) raises `NoMethodError` exactly as a direct call would, and no payload runs.
- **`tap`** — yields the receiver to the block (side effect), returns the
  RECEIVER.
- **`then` / `yield_self`** — yields the receiver, returns the BLOCK'S RESULT;
  a block-less `then` returns the receiver (matching the Python v0 floor).
- **`respond_to?`** — true iff dispatch would resolve the name, checked against
  the same method table / allowlist dispatch uses (a new `respondsTo` helper), so
  it never lies — a name not resolvable is both a `NoMethodError` on call and
  `respond_to? == false`.
- **Boolean `&` / `|` / `^`** on a `true`/`false` receiver — Ruby's *eager*
  (non-short-circuiting) logical operators, distinct from the lazy `&&`/`||`
  keywords, coercing the operand by SIR truthiness (`true & nil == false`,
  `false | 0 == true`).

Dispatch integrates with the existing JS `callMethod` model: M6 names are
recognised BEFORE the native-method allowlist (so `tap`/`send`/… are not wrongly
rejected as unknown natives), a `SirInstance` still resolves a user override of
`send`/`tap` first, and everything remains an explicit table/Set lookup —
cycle-safe and reflection-free. Verified end-to-end under Node (8 new
`run_with_node` tests covering send-to-instance, send-of-string-name-on-primitive,
send-of-gadget-name → NoMethodError, tap/then/yield_self return rules,
respond_to? true/false on primitive and instance, and the boolean operators) plus
a runtime-shape unit test asserting the surface is present and gadget-free.


## 0.11.3

### Fixed — Ruby String methods whose names differ from JS natives (`upcase`/…)

The JS backend dispatches a method call by checking the name against a fixed
allowlist of NATIVE JS method names and then invoking `recv[name]`.  Ruby method
names that happen to match JS (`push`, `map`, `split`, …) worked, but ones that
differ (`upcase` vs `toUpperCase`) missed the allowlist and raised a spurious
`NoMethodError` on JS — while Python/Go/Rust, which dispatch Ruby names in a
runtime catalog, handled them.

- Added a `RUBY_METHOD_ALIASES` table (Ruby spelling → native name) resolved in
  `callMethod` BEFORE the allowlist check, so `upcase` → `toUpperCase` etc.
  dispatch while the allowlist stays a fixed set of native names — the
  reflective-gadget security gate is UNCHANGED (every alias target is itself on
  the allowlist; lookup is a fixed table, never a reflective transform of a
  source name; the `NoMethodError` message still reports the original Ruby name).
- Aliases (unambiguous 1:1 only): `upcase`→`toUpperCase`, `downcase`→
  `toLowerCase`, `strip`→`trim`, `lstrip`→`trimStart`, `rstrip`→`trimEnd`,
  `start_with?`→`startsWith`, `end_with?`→`endsWith`, `include?`→`includes`.
  Semantics-diverging pairs (e.g. `gsub`/`replaceAll`) are deliberately omitted.
- Runtime shape test; verified end-to-end via the sir-conformance `string_case`
  program (14 corpus x 4 backends, all agree).


## 0.11.2

### Fixed — `or`/`and` builtins (Ruby `||`/`&&`) were unimplemented

Ruby `&&`/`and` and `||`/`or` lower (in the frontend) to
`BuiltinCall("and"/"or", [lhs, rhs])` — the fold covers BOTH the 2-operand
`a || b` form and a multi-value `when 1, 2, 3` chain. Only the Python backend's
emitter handled them; this backend fell through to the eager runtime dispatcher,
which has no `or`/`and` entry, so ANY `||`/`&&` (and every multi-value `when`)
crashed at runtime with `unknown builtin: or` / `and`. A case_eq-style gap: no
compile-time gate catches a frontend-emitted builtin the backend never handled.

- The emitter now special-cases `BuiltinCall("or"/"and", [a, b])`, emitting the
  SAME truthy-guarded short-circuit form as `Expr::LogicalOr`/`LogicalAnd`: rhs
  is not evaluated once lhs decides, SIR truthiness is used, and the deciding
  OPERAND is returned (Ruby semantics — `nil || "b"` is `"b"`, `"a" || "b"` is
  `"a"`), never a bare bool.
- Emit-shape regression test; verified end-to-end via the sir-conformance
  `logical_ops` + `multi_when` programs (13 corpus x 4 backends, all agree).


## 0.11.1

### Fixed — `case_eq` builtin (Ruby case-equality `===`) was unimplemented

Ruby's `case`/`when` (and `case`/`in`) lowers, in the frontend, to a chain of
`if`s whose conditions are `BuiltinCall("case_eq", [pattern, scrutinee])`. The
JS runtime's builtin table had no `case_eq`, so **every** `case` program threw
`TypeError: unknown builtin: case_eq` **at runtime** — `case` was unusable on
the JavaScript backend (no compile-time gate catches a missing builtin).

- Added `"case_eq"` to the inlined `builtins` table. The emitter already routes
  unknown builtins through `__Sir.callBuiltin`, so no emitter change was needed.
  Ruby keys `===` to the *pattern*'s type (Range → membership, Regexp → match,
  else `==`); `when SomeClass` is lowered to `.is_a?` at the frontend and never
  reaches here. This backend has no Range/Regexp value, so `case_eq` is native
  `===` — the same equality its `=` builtin uses.
- New `compile_and_run_case_eq` exec proof: a `when`-style `if case_eq(…)` chain
  emits self-contained JS, runs under `node`, and matches the expected output.


All notable changes to `semantic-ir-to-javascript` are documented here.

## 0.11.0 — mixins: `include` / `extend` module method resolution (MX4)

### Added

- The inlined `__Sir` OOP runtime now executes **Ruby mixins** — a module's
  methods are found via `include` / `extend` — so a translated
  `module M; def foo; …; end; end` + `include M` resolves `foo` on including
  classes' instances, identically to the reference backends
  (sir-mixins, MX4). Runtime-only: no core-IR / frontend change (the merged
  MX1 frontend already lowers module bodies + `include` / `extend`).
  - `Feature::Modules` is now **accepted** by the JS backend. A module body's
    `def`s register into the SAME `methodTable` a class uses, keyed by the
    module name (via the existing `__def_method__` builtin — an "owner" is now
    a class *or* a module).
  - **`include M`** → `__include__("Owner", "M")` →
    `__Sir.includeModule("Owner", "M")`, appending `M` to a per-owner
    `includedModules` list in include order.
  - **`extend M`** → `__extend__("Owner", "M")` →
    `__Sir.extendModule("Owner", "M")`, appending to a per-owner
    `extendedModules` list; `M`'s (instance) methods become **class methods**
    of the owner, callable as `Owner.method`.
  - **`Klass.method(args…)`** on a constant receiver →
    `__class_method__("Klass", "method", args…)` →
    `__Sir.callClassMethod(…)` (the class-method dispatch arm — previously
    unhandled by this backend), resolving through the class-method MRO.
  - **`resolveMethod` now follows Ruby's MRO**: for a receiver of class `C`
    the walk searches `C` → `C`'s included modules **most-recent-first**
    (reverse of include order, each expanded depth-first through its own
    `include`s) → `C`'s superclass → its modules → … A class-defined method
    **shadows** a mixed-in module method (class-first MRO), and a **diamond**
    include (a module reached via two paths) is resolved **once** at its
    earliest position. `super` and `initialize` resolution are MRO-aware too.

### Security

- Dispatch stays **explicit-table, cycle-guarded** (the C3 RCE bar). The new
  `includedModules` / `extendedModules` are real `Map`s keyed by owner *name
  strings* holding module *name strings* — never `Object` properties — so a
  module or owner literally named `constructor` / `__proto__` is inert data,
  never a prototype write or a reflective host callable. A single shared
  `seen` set spans the whole MRO walk, so a self-including module
  (`module M; include M; end`) or a cyclic hierarchy **terminates** with a
  `NoMethodError` instead of looping.

### Tests

- Unit (emit shape): `__include__` / `__extend__` / `__class_method__` route
  to the runtime helpers.
- Execution-proofs under Node (hand-built SIR mirroring the MX1 frontend):
  included-module method callable; class method shadows module; diamond
  include resolves once; `extend` makes a class method; self-including module
  terminates.

## 0.10.0 — typed runtime errors (ZeroDivision/Index/Key/NoMethod, T3)

### Added

- Faulting emitted-runtime operations now raise the **correct typed
  `SirError`** (matching Ruby), so a translated
  `begin; …; rescue ZeroDivisionError => e; …; end` catches them — and
  identically across backends (sir-typed-runtime-errors, T3). Runtime-only:
  no core-IR / frontend change.
  - **Division by zero** (`1 / 0`, `1.0 / 0`) → `ZeroDivisionError`
    (`"divided by 0"`). Native JS `/` yields `Infinity`, so the emitter now
    routes the 2-arg `/` builtin through a new inlined `__Sir.divide(a, b)`
    helper that adds an explicit `b === 0` check (covering integer-zero,
    float-zero, and `-0` divisors uniformly) and `raiseError`s the typed
    error. Non-zero divisors divide natively as before — no numeric program
    changes. (`-`/`%` and comparisons keep native infix.)
  - **`arr.fetch(oob)`** → `IndexError`; **`hash.fetch(missing)`** with no
    default → `KeyError`. A supplied default (`fetch(k, d)`) is returned
    instead of raising, matching Ruby. Handled in `callMethod` ahead of the
    method allowlist (negative array indices count from the end).
    - **Security (CWE-470):** `arr.fetch` first validates its index is a real
      integer (`typeof === "number" && Number.isInteger`) — a non-integer,
      source-controlled index (`arr.fetch("constructor")`, `"__proto__"`, …)
      raises `TypeError` (Ruby: *no implicit conversion of String into
      Integer*) instead of sailing past the `NaN`-poisoned bounds checks to a
      reflective `recv[idx]` read that would leak prototype/host gadgets and
      bypass the allowlist. Regression: `t3_array_fetch_non_integer_index_raises_type_error_not_gadget`.
  - **Unknown method** (an allowlist miss, or a `SirInstance` method miss) →
    `NoMethodError` (`undefined method \`x\` for <class>`) via a new
    `classDescription` receiver-describer, replacing the previous JS-native
    `TypeError` floor (which a `rescue` would miss or catch over-broadly).
- The plain index operators `arr[i]` / `hash[k]` are **unchanged**: they
  still return `nil` (Ruby does NOT raise for `[]`) — no over-raise.
- Dispatch remains an explicit runtime **tag** test / typed-string raise,
  never reflection / `eval` on a source-derived name
  ([[dynamic-dispatch-rce]]); the method allowlist still blocks reflective
  gadgets — now surfacing the rejection as a typed `NoMethodError`.
- Execution proofs in `run_with_node.rs` (`t3_*`) run each case under `node`
  and assert the typed clause catches (`1/0`→ZeroDivisionError,
  `arr.fetch(oob)`→IndexError, `h.fetch(miss)`→KeyError,
  `obj.frobnicate`→NoMethodError), that `ZeroDivisionError` also chains up to
  `StandardError`, that `arr[oob]`/`h[miss]` still return `nil` (no
  over-raise), and that `fetch` with a default returns it.

## 0.9.0 — polymorphic `+` / `*` for strings and arrays (PO3)

### Added

- `+` and `*` are now **type-polymorphic**, matching Ruby's operator
  overloading (sir-polymorphic-operators, PO3). All these lower to the same
  SIR `+`/`*` builtins, so dispatch happens at runtime on the **first
  operand's type** via two new inlined helpers `__Sir.plus` / `__Sir.times`
  (also exported and used by `builtins["+"]` / `builtins["*"]` for the
  variadic / value-reference paths):
  - `"a" + "b"` → `"ab"` (String concat), `[1] + [2]` → `[1, 2]` (Array concat).
  - `"ab" * 3` → `"ababab"` (String repeat), `[0] * 3` → `[0, 0, 0]`
    (Array repeat), `[1, 2] * ", "` → `"1, 2"` (Array join via the same
    `format` display helper `puts`/`print` use).
- The emitter now routes the **2-arg** `+`/`*` through `__Sir.plus`/`__Sir.times`
  instead of native infix; numeric `+`/`*` semantics (int/float promotion,
  variadic fold) are byte-for-byte unchanged — the String/Array arms sit
  strictly ahead of the numeric path. `-`/`/`/`%` and the comparisons keep
  native infix.
- Dispatch is a runtime **tag** test (`typeof x === "string"` /
  `Array.isArray(x)`), never reflection / `eval` / property access on a
  source-derived name ([[dynamic-dispatch-rce]]).
- Fixes the `[] + []` bug: native JS `[1] + [2]` coerces to the string
  `"1,2"`; the Array-concat arm returns a **fresh** array with no aliasing or
  mutation of the inputs.
- Execution proofs in `run_with_node.rs` (`poly_*`) run each arm under `node`
  and assert stdout, plus a regression that `1 + 2` → 3 and `2 * 3` → 6 are
  unchanged.

### Security — bound the repeat count (CWE-1284 / CWE-400)

- The String- and Array-repeat arms multiply a length by a
  **program-controlled** `count`. Unguarded, `String.prototype.repeat` throws a
  raw `RangeError` on a negative/huge count and an array-repeat loop can
  allocate until the process OOMs — a denial of service. A shared `repeatCount`
  guard clamps a non-finite / non-integer / `count <= 0` to an **empty** result
  and rejects an oversized product (`unitLen * count > Number.MAX_SAFE_INTEGER`)
  with a Ruby-shaped `ArgumentError: argument too big` **before** any
  allocation; an empty receiver short-circuits so a huge count on `"" * n` /
  `[] * n` does no work. Regression: `poly_string_repeat_overflow_is_rejected`
  asserts node exits non-zero with the `argument too big` message.

## 0.8.0 — `puts` builtin (Ruby semantics)

### Added

- The JavaScript backend now emits and executes Ruby's `puts`, the most common
  output method. `puts` maps to a new variadic runtime helper `__Sir.puts(...)`
  (routed by the emit helper table, with a matching `builtins["puts"]` entry),
  reusing `format` for element rendering.
- Ruby semantics implemented exactly: no-arg → one newline; `puts x` →
  `x` + newline (no double newline when the text already ends in `"\n"`);
  `puts a, b` → one line per arg; `puts []` → a single newline; a native array
  is flattened recursively, one **element** per line; `puts null` → a blank
  line. Writes via `process.stdout.write` (not `console.log`) so the
  trailing-newline suppression is honoured.
- Execution proof `run_with_node.rs::puts_matches_ruby_output` runs
  `puts "hello"; puts; puts [1,2,3]` under `node` and asserts stdout is exactly
  `hello\n\n1\n2\n3\n` (the Ruby reference output).

### Security — cycle-guard the `puts` array flatten (CWE-674)

- `putsOne` flattened arrays by recursing per element with **no bound**. A JS
  array is a shared, mutable reference, so a translated program can build a
  self-referential array (`a = []; a << a; puts a`) or a pathologically deep
  one; the unguarded recursion threw `RangeError: Maximum call stack size
  exceeded` — a denial of service (uncontrolled recursion). The flatten now
  threads a `Set` of the array references on the active path: an array
  re-encountered within its own subtree is a cycle and is written as Ruby's
  `[...]` placeholder + newline instead of recursing, so `puts a` on a
  self-referential array now **terminates** exactly as real Ruby does.
  Non-cyclic output is byte-for-byte unchanged (`puts [1,[2,3]]` →
  `1\n2\n3\n`); a new regression test (`puts_cyclic_array_terminates`) proves
  the self-referential case exits cleanly with `[...]\n`.

## Unreleased

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

### Security

- **Allowlist method-dispatch names in `callMethod` to block a
  Function-constructor RCE (C3).**  `callMethod(recv, name, …args)` performed
  an unrestricted dynamic `recv[name]` lookup with an attacker-controlled
  `name`.  A translated untrusted program could therefore reach reflective
  gadgets — chiefly `constructor`, which on any function yields the global
  `Function` constructor, letting `id.constructor("return …evil…")` synthesise
  and run arbitrary code (a native higher-order method like
  `Array.prototype.map` then invokes it → remote code execution).  `apply`,
  `call`, `bind`, `__proto__`, `prototype`, and the `__define/lookup*etter__`
  pair were equally reachable.  `callMethod` now dispatches **only** through a
  fixed allowlist of known-safe Array / String / Number methods; any name
  outside it (every gadget included) throws a `TypeError` *before* the lookup.
  This is the primary, load-bearing gate — the emitted JS is what executes.
  A node execution-proof asserts `callMethod(id, "constructor", …)` throws
  instead of building a function.  `length` remains special-cased ahead of the
  allowlist as a property read.

## 0.7.0 — user-defined-class OOP: instantiation, dispatch, super, ivars (O3)

The JavaScript analogue of O1's Python/TypeScript OOP runtime.  The
backend now **executes** user-defined-class object-orientation end-to-end
through Node, using an inlined `__Sir` OOP runtime (no import, no
`npm install`) — the JS half of the SIR18 `Classes` dispatch surface.

### Added

- **Inlined OOP runtime (`runtime.rs`).**  Added to the self-contained
  `__Sir` IIFE:
  - `SirInstance` — a user object tagged with its class name, carrying a
    prototype-less (`Object.create(null)`) instance-variable bag; plus
    `newInstance(cls)`.
  - `methodTable` / `classMethodTable` — instance and class ("static")
    method tables, each a real `Map` keyed on a **flat `"Class\x00method"`
    string** (NUL-joined so distinct `(class, method)` pairs never
    collide).  `defMethod(cls, name, fn)` / `defClassMethod(cls, name, fn)`
    register a method body closure.
  - `callNew(cls, …args)` — allocate, resolve the inherited `initialize`
    by walking `class → superclass` (the SAME `seen`-guarded ancestry map
    the exception runtime uses), apply it with `self` bound, and return the
    instance (Ruby discards `initialize`'s result).
  - `callMethod` **extended**: a `SirInstance` receiver resolves the user
    method table (walking ancestry) and applies with `self` bound; every
    other receiver falls through to the **unchanged** built-in / collection
    path (arrays' `push`/`map`/…, strings, the RCE-hardened allowlist).
  - `callSuper(method, cls, …args)` — resolve `method` from the
    *superclass* of `cls` and apply with the current `self` still bound.
  - `currentSelf()` + a `pushSelf`/`popSelf` self-stack (balanced with
    try/finally, so an exception thrown mid-method still unwinds `self`).
  - `ivarGet`/`ivarSet` and `cvarGet`/`cvarSet` acting on the current
    `self` (unset reads yield `null`, matching Ruby nil).

- **OOP emit arms (`emit.rs`).**  `emit_builtin_call` now routes the O2
  frontend's OOP builtins to the runtime: `__new__`→`__Sir.callNew`,
  `__super__`→`callSuper`, `__def_method__`→`defMethod`,
  `__def_class_method__`→`defClassMethod`, `__self__`→`currentSelf()`.
  Class/method-name operands (a `StrLit`, or a `Const` VarRef like
  `Dog.new`) emit as string literals via `quote_js_string`.  `@x`/`@@x`
  reads and writes (`Scope::Instance`/`ClassVar`) lower to
  `ivarGet`/`ivarSet` / `cvarGet`/`cvarSet` — these scopes previously hit
  the deferred-scope panic.

- **Feature acceptance (`lib.rs`).**  `ACCEPTED_FEATURES` now includes
  `InstanceVars` and `ClassVars` (alongside the already-accepted
  `Classes`/`Constants`).  Genuinely-unsupported constructs (e.g.
  `StrConcat` string interpolation, `TailCalls`, `Intrinsics`) are still
  rejected cleanly rather than mis-emitted.

### Security

- **All OOP dispatch is explicit `Map` lookup on a `(class, method)`
  string key — never `recv[name]`, reflection, `eval`, or `new Function`
  on a source-derived name** (the same C3 RCE lesson that bit this crate's
  `callMethod`).  A user class or method literally named `constructor` /
  `__proto__` / `prototype` is only ever a Map *key*: a miss floors to a
  clean `NoMethodError`, never reaching a host callable.  The method tables
  are real `Map`s (not `{}`) and the instance/class-var bags are
  prototype-less, so a `"__proto__"` name cannot poison any prototype
  chain.  Every ancestry walk is `seen`-guarded, so a cyclic hierarchy
  terminates instead of looping.

### Tests

- Emitted-shape unit tests for every new builtin (`__new__`→`callNew`,
  `__super__`→`callSuper`, `def`/`def self`→`defMethod`/`defClassMethod`,
  `__self__`→`currentSelf`) and for `@ivar`/`@@cvar` reads/writes.
- Node execution-proofs (hand-built SIR modules): **P1** Dog
  `initialize`/`speak` prints `Rex says woof`; **P2** `Cat < Animal` with
  `super(4)` and a parent-set ivar prints `Tom with 4`; a security proof
  that `__new__("constructor")` + a `__proto__` method dispatch does NOT
  execute host code (clean method-miss); and a cyclic-ancestry (`A<B<A`)
  proof that resolution terminates.

## 0.6.0 — exception handling (try/catch/raise) + user-class ancestry (E1)

### Added

- **`Stmt::TryCatch` lowers to native `try`/`catch`/`finally` (E1).**  The
  backend previously *panicked* on any `TryCatch`.  It now emits a native
  `try { <body> } catch (__exc) { … } finally { <ensure> }`.  Because a native
  `catch` binds one variable and catches everything while Ruby has an ordered
  list of *typed* `rescue` clauses, the catch body is an if/else-if chain that
  asks `__Sir.rescueMatches(__exc, ["Foo", "Bar"])` for each clause in source
  order, binds `=> e` when present, and re-`throw`s the original exception if
  no clause matches (Ruby's "propagate when unrescued").  An empty
  `exception_types` is a bare `rescue` (catch-all).  Mirrors the TypeScript
  backend's `TryCatch` arm exactly, minus the type annotation on the binding.
- **`raise` builtin lowers to `__Sir.raiseError` (E1).**  `raise Foo, "msg"`
  (a `Const` class name + message) → `__Sir.raiseError("Foo", <msg>)`;
  `raise Foo` → `__Sir.raiseError("Foo")`; a non-`Const` first arg
  (`raise "msg"`) → `__Sir.raiseError("RuntimeError", <arg>)`; bare `raise` →
  `__Sir.raiseError()` (a generic `RuntimeError` re-raise).  Matches the TS
  backend's shape.
- **Inlined exception runtime.**  Ported the plain-JS-compatible pieces of the
  published `@coding-adventures/sir-runtime-exceptions` package into the
  backend's self-contained `__Sir` IIFE: a class-name-tagged `SirError` (a real
  `Error` subclass), `raiseError(cls, msg)`, `rescueMatches(exc, classNames)`,
  and the built-in Ruby `ANCESTRY` table (so `rescue StandardError` catches a
  `RuntimeError`/`ArgumentError`/…).  No `import`/`require`; the emitted `.js`
  still runs directly under `node`.
- **User-defined class ancestry (E2, the JS half).**  Added
  `__Sir.registerAncestry(map)`, which merges a user
  `{ childClass: superclassName }` map into the runtime's ancestry lookup.  The
  emitter collects every `Stmt::ClassDef { name, superclass: Some(_) }` pair in
  the module (recursing into nested bodies) and emits one
  `__Sir.registerAncestry({ … })` at program init — so
  `class MyErr < StandardError; raise MyErr; rescue StandardError` matches
  through the merged chain.  A `ClassDef` body's (non-`def`) statements are now
  emitted inline instead of panicking.
- **Accepts `Feature::Exceptions`, `Feature::Classes`, and `Feature::Constants`.**
  Exceptions and classes are lowered as above; `Constants` is accepted because
  `raise Foo` names its class as a `Const` `VarRef` (consumed by the `raise` arm
  as a string) — any other constant read emits its bare identifier.

### Security

- **Ancestry dispatch is by explicit table lookup, never reflection.**
  `rescueMatches` / `isAncestorOrSelf` resolve a class's superclass chain via
  `ancestry[cur]` string-map reads only — no `eval`, no dynamic code
  synthesis; class and method names are treated as pure data.  The mutable
  ancestry map is `Object.create(null)` (prototype-less), so a user class
  literally named `constructor`/`__proto__` cannot poison the lookup, and a
  malformed (cyclic) user map terminates via a `seen` guard.

### Tests

- Emitted-shape unit tests for the `TryCatch` else-chain, the four `raise`
  shapes, and one-shot `registerAncestry` emission (present iff a class
  inherits).
- Four `node` execution-proofs: built-in ancestry (`ArgumentError` caught by
  `rescue StandardError`), bare `rescue` catch-all, an unmatched type
  re-raising to a non-zero exit, and USER ancestry
  (`class MyErr < StandardError` caught by `rescue StandardError`).

## 0.5.0 — method dispatch (`__method__`) execution

Adds the minimal runtime support the JavaScript frontend's C3 member-method
lowering needs to **run**.  A method call `recv.meth(args…)` reaches the
backend as `BuiltinCall("__method__", [recv, StrLit("meth"), args…])`; the
emitter now routes it to a new runtime helper, `__Sir.callMethod`, which
invokes the JS-native method on the receiver (arrays' `push`/`pop`/`map`/
`filter`/`forEach`/`includes`/`reduce`/…, strings' `toUpperCase`/…) and
unwraps any `Closure` callback argument into a plain JS function.  This lets
JavaScript→SIR→JS collection programs execute end-to-end under `node`.

### Added

- `emit_builtin_call` special-cases `BuiltinCall("__method__", [recv,
  StrLit(name), args…])` → `__Sir.callMethod(recv, "name", args…)` (receiver
  first, method name second, call args after).
- Runtime `callMethod(recv, name, ...args)`: unwraps `Closure` args via
  `applyClosure`, accepts `length` as a nullary method, and dispatches to the
  native `recv[name]` method (throwing a clear `TypeError` when absent).

## 0.4.0 — KW4 (keyword-parameter & argument emission)

Replaces the KW1 compile-compat stubs with **real** keyword-parameter and
keyword-argument emission.  JavaScript has no native keyword-argument call
form, so — exactly as the TypeScript backend does (spec §4) — keyword
constructs lower to a zero-dependency **options object**.  No runtime
library is required; the lowering is direct.

### Added

- `accepts_features()` now declares `KeywordParams` (mirrors `DefaultParams`).
- **Def side.** A function's `Keyword` params (`def f(a:)` / `def f(a: 1)`)
  are folded into a single trailing options-object parameter `__kw`; the
  body prologue destructures it: `const { b, c = <default> } = __kw ?? {};`.
  A **required** keyword (`Keyword`, `default: None`) destructures bare; an
  **optional** keyword (`Keyword`, `default: Some(e)`) carries a JS
  destructuring default `name = <e>`, which fires on `undefined` exactly
  like SIR optional-keyword semantics.  The `?? {}` guard lets an
  all-optional callee be called with no options object.  When a keyword
  name is not a valid JS identifier, the prologue emits the explicit
  `{ "raw key": sanitized_local }` rename form so the object key still
  matches the call site.  `__kw` is collision-safe: `sanitize_ident` never
  produces a leading `__`, so no user parameter can sanitize to it.
- **Call side.** In a call's `args`, positionals emit as before and every
  `Expr::KeywordArg` collapses into one trailing object literal:
  `f(1, b: 2, c: 3)` → `f(1, { b: 2, c: 3 })`; a call with only keyword
  args → `f({ b: 2 })`; none → no trailing object.  `IndirectCall` routes
  the same object as the last element of its argument array.  The object
  key is the raw keyword `name`, matching the callee's destructuring
  prologue.  A new `emit_call_args` helper drives both call sites.

### Changed

- The `emit_expr` `KeywordArg` arm is now a pure defensive panic: keyword
  args are peeled off by `emit_call_args` before recursion, so reaching
  that arm signals a backend bug rather than a deferred feature.

### Tests

- Emitted-shape unit tests: trailing `__kw` object + destructuring
  prologue (required & optional keywords), keyword-only function, call-side
  object collapse (positional+keyword, keyword-only, none), and the
  `IndirectCall` object placement.
- Execution-proof through `node` (skips gracefully if absent):
  `add(5)` defaults the omitted keyword to 10 (→15) and
  `add(5, delta: 100)` supplies it (→105); a required-keyword call
  `pick(chosen: 7)` returns 7.

## 0.3.0 — P2d (default-parameter emission)

Adds **default parameters** to the JavaScript backend.  JavaScript's
native default-parameter feature has *exactly* SIR's semantics — the
default expression is evaluated **at call time**, only when the argument
is omitted, in **param scope** (so a later default may reference an
earlier parameter by name).  The lowering is therefore a direct native
inline: no runtime helper, no call-site padding.

### Added

- `accepts_features()` now declares `DefaultParams`.
- Emit: a `Param { default: Some(expr) }` lowers to a native JS default
  parameter `name = <emitted default>`.  The default expression is
  emitted with the ordinary `emit_expr`, so a default that references an
  earlier parameter (`VarRef { scope: Param }`) becomes a bare name —
  valid JavaScript, since earlier params are in scope left-to-right.
  `Rest`/`KwRest` params are unchanged; `IndirectCall` and closure
  defaults are unchanged / deferred.
- `DirectCall` documented and confirmed to emit **only the args present**
  — the SIR validator allows omitting trailing defaulted args (arity ≥
  `required_param_count`), and native JS defaults fill the omitted
  trailing params at call time.  No padding is inserted.
- Unit tests: `f(a, b = a + 1)` emits `function f(a, b = (a + 1)) {`; a
  short `DirectCall` (`f(5)`) is not padded.
- Integration test (`tests/run_with_node.rs`,
  `default_param_is_call_time_and_param_scoped`): hand-builds a module
  with `f(a, b = a + 1)` returning `b` and a `main` that calls
  `print(f(5))` then `print(f(5, 10))`, emits JavaScript, **runs it under
  `node`**, and asserts stdout `6` then `10` — proving the default is
  evaluated at call time (depends on the actual `a = 5`) and in param
  scope (references the earlier param `a`).

## 0.2.0 — D4 (completes SIR16 / v1 parity for the JS backend)

Brings the JavaScript backend to **full SIR16 / v1 parity**: the six
SIR16 features it previously deferred are now emitted and accepted.
JavaScript supports all of them natively, so each lowering is direct.

### Added

- `accepts_features()` now declares the v0 surface **plus all of SIR16**:
  `Floats`, `ShortCircuit`, `Sequences`, `Maps`, `MutableBindings`,
  `Loops`. (`accepts_intrinsics()` stays empty.)
- Emit arms for every SIR16 node:
  - `Floats` — `FloatLit` emits a native `number` literal (already wired
    in D1; the `Floats` capability is now accepted). `NaN`/`Infinity`/
    `-Infinity` spelled out; integer-valued floats keep an explicit `.0`.
  - `ShortCircuit` — `LogicalAnd`/`LogicalOr` emit a truthy-guarded arrow
    IIFE (`((__l) => __Sir.truthy(__l) ? (rhs) : __l)(lhs)` for And, the
    mirror for Or) so the rhs runs only when the lhs decides, routing the
    test through `__Sir.truthy` (only `false`/`nil` are falsy).
  - `Sequences` — `SeqLit` → `[…]`, `SeqIndex` → `(arr)[i]`, `SeqLen` →
    `(arr).length`, `SeqSet` → `(arr)[i] = v;` (native arrays).
  - `Maps` — `MapLit` → `new Map([[k, v], …])`, `MapGet` →
    `((m).get(k) ?? null)` (missing key reads as nil), `MapSet` →
    `(m).set(k, v);` (native `Map`, matching the TypeScript backend's
    representation).
  - `MutableBindings` — `Assign` (Local/Param/Capture/Global) → a plain
    `name = value;` reassignment. `let` (never `const`) is already the
    keyword for every binding, so no const→let pre-pass is needed (unlike
    the Rust/TypeScript backends).
  - `Loops` — `While` → `while (__Sir.truthy(cond)) { … }`; `ForRange` →
    a direction-aware C-style `for` with `stop`/`step` evaluated once into
    block-scoped `__sir_stop_N`/`__sir_step_N` temporaries (a per-module
    monotonic counter keeps them deterministic); `ForEach` → `for (let x
    of iter) { … }`.
- `emit_block_as_stmts` helper for loop bodies (trailing value discarded;
  a bare `nil` value is dropped).
- Unit tests for every new emit arm (floats incl. specials, short-circuit
  And/Or, seq build/index/len, map lit/get, assign, seq-set, map-set,
  while, for-range incl. distinct nested temporaries, for-each).
- Integration tests (`tests/run_with_node.rs`) that hand-build SIR16
  modules, emit JavaScript, **run it under `node`**, and assert stdout:
  float arithmetic promotion (`3.5`), short-circuit (rhs not evaluated),
  `or` first-truthy (`7`), sequence build/index/len/set, map
  build/get/set (incl. missing-key → nil), a `while` counter, a
  for-range accumulator (and a descending step), for-each over a
  sequence, and mutable reassignment (`42`).

### Still deferred (rejected at the capability check)

- String interpolation — `StrConcat` (`StringInterpolation`).
- OOP & exceptions — `ClassDef`/`ModuleDef`/`SingletonClassDef`,
  `TryCatch`, and the `Instance`/`ClassVar`/`Const` scopes (`Classes`,
  `Modules`, `InstanceVars`, `ClassVars`, `Constants`, `Exceptions`).
- `TailCalls` (V8 has no reliable TCO) and `Intrinsics` (empty
  whitelist).

The remaining `panic!` arms in `emit` cover only these unaccepted nodes,
so they are defence-in-depth (unreachable for a capability-checked
module), never reachable for an accepted feature.

## 0.1.0 — D1 (initial runnable core)

The first slice of the SIR18 JavaScript backend: the v0 expression /
statement core, emitting self-contained JavaScript that runs under
Node.js with no dependencies.

### Added

- `JavaScriptBackend` implementing `semantic_ir::Backend`:
  - `target_tag()` → `"javascript"`.
  - `accepts_features()` → the **v0 feature set** (`Closures`, `Pairs`,
    `Symbols`, `Strings`, `DynamicTyping`, `OptionalTypeAnnotations`,
    `MutualRecursion`, `Globals`).
  - `accepts_intrinsics()` → empty.
  - `compile()` → validate → capability check → reject `TailCalls` →
    lower to JavaScript.
- `compile(&module)` convenience free function.
- Inlined `__Sir` runtime (`src/runtime.rs`): an IIFE with `Sym`/`Pair`/
  `Closure` classes, symbol interning, `applyClosure`, SIR `truthy`,
  `format`/`print`, and a builtins dispatch table (arithmetic,
  comparison, pair ops, predicates, `len`, `range`). Pasted verbatim
  into every artifact, so output is fully self-contained.
- Emitter (`src/emit.rs`) for the v0 nodes:
  - Literals: `IntLit`, `FloatLit`, `BoolLit`, `NilLit`, `StrLit`,
    `SymLit`.
  - `VarRef` by scope: `Local`/`Param`/`Capture`/`Global` → bare
    identifier; `Builtin` → `__Sir.builtinClosure("name")`.
  - `If` → SIR-truthy ternary.
  - `Block`: function-body form (flat `{ …; return v; }`) and
    expression form (IIFE).
  - `DirectCall`, `IndirectCall` (`__Sir.applyClosure`), `BuiltinCall`
    (native-infix specialisation for `+ - * / % = != < > <= >=`,
    `not`/`neg`/`len`, `__Sir.print`; everything else via
    `__Sir.callBuiltin`).
  - `Function` declarations (captures prepended before params; native
    `...rest` for rest params).
  - `LetBinding`/`LetStarBinding`/`ExprStmt`; the `_init` `global_set`
    pattern renders as a direct assignment.
  - `MakeClosure` → `new __Sir.Closure((..._a) => fn(caps…, ..._a))`.
  - Module wrapping: banner comment, `"use strict";`, inlined runtime,
    module globals, function declarations, then `_init()` and `main()`.
- `sanitize_ident` (reserved words → `_$` prefix; invalid chars →
  `_$<hex>`; empty → `_$empty`), JS string escaping, and float
  formatting (explicit decimal point; `NaN`/`Infinity` handled).
- Tests: unit coverage for `sanitize_ident` and each emit arm, a
  determinism test, and an end-to-end integration test
  (`tests/run_with_node.rs`) that lowers Twig → SIR → JS and **executes
  the result under `node`** (add → `3`, factorial → `120`,
  closure-adder → `8`), skipping execution when `node` is absent.
- Package scaffolding: `Cargo.toml`, `README.md`, this changelog, and
  `BUILD` / `BUILD_windows`. Registered in the Rust workspace
  (`code/packages/rust/Cargo.toml`).

### Deferred

The following are intentionally **not** implemented in this milestone and
are **rejected at the capability check** (their `Feature`s are absent
from `accepts_features()`), so a module that uses them is turned away
rather than mis-compiled:

- Collections — `SeqLit`/`SeqIndex`/`SeqLen`, `MapLit`/`MapGet`
  (`Sequences`, `Maps`).
- Loops — `While`/`ForRange`/`ForEach` (`Loops`).
- Mutation — mutable `Assign`, `SeqSet`/`MapSet` (`MutableBindings`).
- Short-circuit — `LogicalAnd`/`LogicalOr` (`ShortCircuit`).
- Floats as a declared feature (`FloatLit` *emission* is implemented,
  but the `Floats` capability is not yet accepted).
- String interpolation — `StrConcat` (`StringInterpolation`).
- OOP & exceptions — `ClassDef`/`ModuleDef`/`SingletonClassDef`,
  `TryCatch`, and the `Instance`/`ClassVar`/`Const` scopes
  (`Classes`, `Modules`, `InstanceVars`, `ClassVars`, `Constants`,
  `Exceptions`).
- `TailCalls` (V8 has no reliable TCO) and `Intrinsics` (empty
  whitelist) — fundamentally unsupported / out of scope for v0.
