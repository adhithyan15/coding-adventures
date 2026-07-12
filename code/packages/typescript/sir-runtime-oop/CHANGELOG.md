# Changelog

All notable changes to `@coding-adventures/sir-runtime-oop` are documented here.

## [0.1.24] - 2026-07-11

### Added

- **`Array#slice_when`** — the FINAL backend of the `slice_when` cross-backend
  cascade (Python reference #8070, Go #8073, Rust #8077, JS #8082), completing it
  across all five backends.
  - `slice_when { |prev, cur| pred }` (`arrayBlockMethod` + `ARRAY_BLOCK_METHODS`)
    is the INVERSE of `chunk_while`: it splits into runs of consecutive elements,
    starting a NEW run BETWEEN an adjacent pair exactly WHERE the block is truthy
    (whereas `chunk_while` starts a new run where the block is FALSY).
    `[1,2,4,9,10,11,12].slice_when { |a,b| b-a>1 }` → `[[1,2],[4],[9,10,11,12]]`;
    an empty array yields `[]`, a single element `[[x]]`.
  - `respond_to?("slice_when")` now reports `true`.

## [0.1.23] - 2026-07-11

### Added

- **`Array#tally`** — the FINAL backend of the `tally` cross-backend catch-up
  (Python reference #8054, JS #8058; Go and Rust already shipped it), completing
  it across all five backends.
  - `tally` (`arrayMethod` + `ARRAY_METHODS`) → a Hash counting how many times
    each element occurs, keyed in first-seen order
    (`["a","b","a","c","a"].tally` → `{"a"=>3, "b"=>1, "c"=>1}`; `[].tally` →
    `{}`).
  - Realised as an insertion-ordered `Map` — the same shape `group_by` returns,
    printed `{k=>v}` by `rubyInspect` (no display change needed). Keys compare by
    JS SameValueZero, which agrees with Ruby `eql?`/hash on the scalar elements
    this covers, matching the Go/Rust/Python/JS references.
  - `respond_to?("tally")` now reports `true`.

## [0.1.22] - 2026-07-11

### Added

- **`Array#each_slice`, `Array#each_cons`, `Array#chunk_while`** — the FINAL
  backend of this cross-backend batch (Python reference #8031, then Go #8036 /
  Rust #8042 / JS #8048), the consecutive-grouping family.
  - `each_slice(n)` (`arrayMethod` + `ARRAY_METHODS`) → consecutive sub-arrays of
    at most `n` elements, the last possibly shorter.
  - `each_cons(n)` (non-block) → every consecutive `n`-element sliding window; a
    window larger than the array → `[]`.
  - Both read `n` via `Number.isInteger` and treat `n <= 0` as `[]` (Ruby raises
    `ArgumentError`; the never-raise floor yields empty).
  - `chunk_while { |prev, cur| pred }` (`arrayBlockMethod` + `ARRAY_BLOCK_METHODS`)
    → runs of consecutive elements; the block is called on each ADJACENT pair, a
    truthy result extends the run and a falsy one starts a new run.  Empty →
    `[]`; single element → `[[x]]`.

## [0.1.21] - 2026-07-11

### Added

- **`Hash#to_h`, `Hash#each_with_index`, `Hash#each_with_object`** — the FINAL
  backend of this cross-backend batch (Python reference #8009, then Go #8015 /
  Rust #8020 / JS #8024), rounding out Hash's Enumerable iteration surface.
  - `to_h` **without** a block (`hashMethod` + `HASH_METHODS`) → a shallow copy
    of the hash (`new Map(recv)`, so mutating it does not alias the receiver).
  - `to_h { |k, v| [new_k, new_v] }` (`hashBlockMethod` + `HASH_BLOCK_METHODS`)
    → a NEW hash from the block-returned `[k, v]` pairs; the block is yielded the
    two args `(k, v)`; a non-pair result (checked `Array.isArray` + length 2) is
    skipped, and colliding new keys keep the LAST pair (Ruby's rule).
  - `each_with_index { |(k, v), i| … }` → yields each `[k, v]` pair with its
    0-based index, returns the receiver.
  - `each_with_object(memo) { |(k, v), memo| … }` → yields each `[k, v]` pair
    with the memo, returns the memo; with no memo argument the receiver is
    returned unchanged.
  - Unlike `each`'s two-arg `(k, v)` yield, `each_with_index`/`each_with_object`
    pass the element as a single `[k, v]` Array (the second block param is the
    index/memo), matching Ruby's Enumerable convention.

## [0.1.20] - 2026-07-11

### Added

- **`Hash` Enumerable breadth** (`hashBlockMethod` + `HASH_BLOCK_METHODS`):
  `group_by`, `partition`, `flat_map`/`collect_concat`, `reduce`/`inject`, `sum`
  — the FINAL backend of this cross-backend batch (Python reference #7978, then
  Go #7983 / Rust #7989 / JS #7993). Ruby's `Hash` mixes in `Enumerable`, so
  these iterate the hash as `[key, value]` pairs and yield the two-arg
  `(key, value)` **except** `reduce`/`inject`, which follow Ruby's memo
  convention and yield `(memo, [k, v])` — the pair as ONE argument.
  - `group_by { |k, v| key }` → a Hash (`Map`) of block key → Array of the
    `[k, v]` pairs that produced it, in first-seen key order.
  - `partition { |k, v| pred }` → `[[matching pairs], [rest pairs]]`.
  - `flat_map`/`collect_concat { |k, v| … }` → map then concatenate one level.
  - `reduce`/`inject` → memo fold; explicit seed or first pair; empty seedless
    → `nil`.
  - `sum(init = 0) { |k, v| … }` → numeric fold seeded at `0` (or the seed arg).

### Changed

- **`hashBlockMethod` signature** now takes `(recv, name, args, block)` (was
  `(recv, name, block)`), and the dispatch call site passes the positional args
  before the block — so `reduce`/`sum` can read a seed argument. Mirrors the
  same change the Python reference made in #7978. `arrayBlockMethod` already had
  this shape.

## [0.1.19] - 2026-07-11

### Added

- **`Hash` Enumerable aggregates** (`hashBlockMethod` + `HASH_BLOCK_METHODS`):
  `find`/`detect`, `any?`, `all?`, `none?`, `count` (block form), `sort_by`,
  `min_by`, `max_by` — completing the cross-backend mirror of this batch
  (Python reference, then Go/Rust/JS backends).  Ruby's `Hash` mixes in
  `Enumerable`, so these iterate the hash as a sequence of `[key, value]` pairs:
  the block is yielded `[key, value]` (two arguments, matching `each`), and the
  "element" an aggregate returns is the two-element `[key, value]` list — e.g.
  `{a: 1, b: 2}.min_by { |k, v| v }` is `[:a, 1]`, and
  `{a: 1, b: 2, c: 3}.sort_by { |k, v| -v }` is `[[:c, 3], [:b, 2], [:a, 1]]`.
  `min_by`/`max_by` on an empty hash return `nil`; all are non-mutating.
- Vitest coverage for the full batch, including the empty-hash `min_by` → nil,
  a receiver-unchanged assertion, and `respond_to?`.

## [0.1.18] - 2026-07-11

### Added

- **`Hash#transform_values` / `Hash#transform_keys`** block methods
  (`hashBlockMethod` + `HASH_BLOCK_METHODS`), completing the cross-backend
  mirror of this pair (Python reference, then Go/Rust/JS backends):
  - `transform_values { |v| … }` — a NEW hash whose keys are copied verbatim
    (unique ⇒ no collision) and whose values are the block results. Yields ONE
    argument (the value); insertion order preserved; receiver unmodified.
  - `transform_keys { |k| … }` — a NEW hash whose values are untouched and whose
    keys are the block results (yields ONE argument, the key). Two source keys
    can collapse onto one new key; Ruby keeps the **last** value at the
    **first-seen** position — matching how the `Map` constructor folds a list of
    pairs.
- Vitest coverage for both, including a `transform_keys` **collision** case
  (`{a:1,b:2}` with a constant `"z"` key ⇒ `{z: 2}`) and a receiver-unchanged
  assertion.

## [0.1.17] - 2026-07-11

### Added — Numeric breadth: `divmod` / `fdiv` / `round(ndigits)` / `clamp` / `between?`

Completes the numeric-breadth sweep across all five backends (Python
`sir-runtime-oop` v0.1.17 is the reference; Go/Rust/JS backends already landed).
Extends the `Integer`/`Float` catalog (`numericMethod` + the `NUMERIC_METHODS`
`respond_to?` set) with five more Ruby numeric methods:

- `round(ndigits)` — `round` gains an optional digits argument: a positive
  `ndigits` rounds to that many decimals (half **away from zero**, via
  `rubyRound`, not `Math.round`); `ndigits <= 0` rounds to a power of ten. TS
  numbers are f64, so a hostile-magnitude `ndigits` degrades naturally (the
  `factor` saturates to `Infinity` and `recv / Infinity` is `0`) — no bignum, no
  allocation. A non-finite receiver returns unchanged.
- `divmod(n)` — `[quotient, remainder]` with a floored quotient and the
  divisor-signed remainder (a JS array, so it prints `[3, 1]`); a zero divisor
  raises a typed `ZeroDivisionError`.
- `fdiv(n)` — floating-point division that never throws: a zero divisor yields
  `Infinity`/`-Infinity`/`NaN`.
- `clamp(min, max)` / `between?(min, max)` — compared numerically.

Dispatch stays an explicit `switch` on the literal method name (never
reflection). Numeric breadth is now complete across all five backends.

## [0.1.16] - 2026-07-10

### Added — String char-set methods: `tr` / `count` / `delete` / `squeeze`

Completes the char-set string sweep across all five backends (Python
`sir-runtime-oop` is the reference; Go/Rust/JS backends already landed) by adding
four more non-block Ruby `String` methods to `stringMethod` and the
`STRING_METHODS` `respond_to?` catalog:

- `tr(from, to)` — positional translate: each char of `from` maps to the char at
  the same index in `to`. A shorter `to` repeats its **last** char; an empty `to`
  **deletes** matching chars; a repeated `from` char takes its **last** mapping.
  A missing/non-string argument is a no-op (never raises).
- `count(*sets)` — how many chars of the receiver lie in the set(s).
- `delete(*sets)` — the receiver with all set chars removed.
- `squeeze(*sets)` — collapse consecutive runs: of set chars when a set is given,
  or of **all** identical runs when no set is given.

Multiple set arguments **intersect** (Ruby's rule — a char must appear in every
set). All iteration is over whole code points (`for…of` / `[...str]`) so astral
runes are never split mid-surrogate. The char-**range** (`"a-z"`) and
**negation** (`"^abc"`) forms are a documented follow-up, matching the
literal-only `sub`/`gsub` precedent.

## [0.1.15] - 2026-07-07

### Added — Array reorder/combine methods: `rotate` / `zip`

Closes the parity gap with the Go/Rust runtimes (which already carry these) by
adding two more non-block Ruby Array methods to `arrayMethod` and the
`ARRAY_METHODS` `respond_to?` catalog:

- `rotate(n=1)` — rotate left by `n` (a negative `n` rotates right); the shift is
  re-folded into `[0, len)` (JS `%` keeps the dividend's sign) so any magnitude
  terminates, and an empty array stays `[]`. No argument defaults to `1`; a
  non-numeric argument degrades to `0` (never raises).
- `zip(*others)` — an Array of tuples `[self[i], others..[i]]` of length
  `recv.length`; a shorter operand pads with `nil` (`null`), a longer one is
  truncated, and a non-array operand is treated as empty (pad-only).

## [0.1.14] - 2026-07-07

### Added — Array slice-selection methods: `take` / `drop` / `values_at`

Extends `arrayMethod` (and the `ARRAY_METHODS` `respond_to?` catalog) with three
more non-block Ruby Array methods, mirroring the Go/Rust/Python/JS runtimes:

- `take(n)` / `drop(n)` — the first `n` elements, or all elements *after* the
  first `n`. `n` is clamped to `[0, len]`: Ruby raises `ArgumentError` on a
  negative `n`, but the never-raise floor folds it to `0`, and `slice` saturates
  `n > len`. A non-numeric argument degrades to `0`.
- `values_at(*idxs)` — one element per index, with a negative index folded from
  the end **once**; an out-of-range index yields `nil` (`null`) rather than
  raising, matching the sibling backends.

## [0.1.13] - 2026-07-07

### Added — more String methods: `ljust` / `rjust` / `center` / `swapcase`

Extends `stringMethod` (and the `STRING_METHODS` `respond_to?` set) with four
more common non-block Ruby String methods, completing the cross-backend parity
sweep (Go / JS / Rust / Python already have these):

- `ljust(width, pad = " ")` / `rjust(...)` / `center(...)` — pad to `width`
  code points using `pad` cyclically; `width <= length` returns the string
  unchanged; `center` puts an odd extra pad rune on the **right** (Ruby's rule).
  An empty pad degrades to a single space, and the padding length is clamped to
  `MAX_REPEAT_LEN` (like `strRepeat`) to bound a DoS.
- `swapcase` — flips each ASCII letter (non-letters / non-ASCII untouched),
  iterating whole code points so astral runes are never split.

## [0.1.12] - 2026-07-07

### Added — Array block-method breadth (sort_by / group_by / partition / …)

Extends `arrayBlockMethod` with the common block-taking Ruby
`Enumerable`/`Array` methods that were missing (map/select/reduce/find/flat_map
were already present), and adds them to the `ARRAY_BLOCK_METHODS` catalog so
`respond_to?` stays honest. Mirrors the Rust/Go/Python backends' array-block
batch — TS is the fourth backend to reach this parity.

- `sort_by { |x| key }` — key-sorted (stable, `<`/`>` keeps numbers numeric).
- `min_by` / `max_by { |x| key }` — extremal block key (`null` on empty).
- `group_by { |x| key }` — a Hash (`Map`) of key → array of elements.
- `partition { |x| pred }` — `[matching, non_matching]`.
- `collect_concat` — alias of `flat_map`.
- `take_while` / `drop_while { |x| pred }` — leading truthy run / remainder.
- `count { |x| pred }` — truthy count (arg/bare forms unchanged in
  `arrayMethod`).
- `each_with_object(memo) { |x, memo| … }` — folds into and returns the memo.

Predicate results route through SIR `truthy`; a block-less call keeps the nil
Enumerator floor. Ordering uses `<`/`>` (never throws on mixed types),
consistent with the existing `sort`/`min`/`max` arms.

## [0.1.11] - 2026-07-02

### Added — Ruby mixins: `include` / `extend` + module-aware MRO (MX3)

Part of the `sir-mixins` cascade (spec `code/specs/sir-mixins.md`). The OOP
runtime now executes Ruby mixins: a module registers its `def`s via the
existing `defMethod` keyed on the *module* name (as the frontend's MX1 lowering
emits), and two new helpers weave modules into an owner's method resolution.
All dispatch stays explicit-table and cycle-guarded (never reflection — the C3
RCE lesson).

- **`includeModule(owner, moduleName)`** (from `__include__("Owner", "M")`) —
  appends `M` to the owner's per-owner **include-order list**
  (`includedModules`). A repeated include is a no-op.
- **`extendModule(owner, moduleName)`** (from `__extend__("Owner", "M")`) —
  copies the module's instance methods into the owner's **class-method** table,
  so they become callable as `Owner.method` (Ruby `extend`).
- **Module-aware MRO in `resolveInstanceMethod`.** The method-resolution walk
  now implements Ruby's method resolution order: class → the class's included
  modules **most-recent-first** (depth-first, recursing into a module's own
  includes) → superclass → its modules → … A single `seen` set spans the whole
  walk, so a **diamond** include (a module reachable by two paths) resolves
  **once** at its earliest position, and a module that (transitively) includes
  itself terminates rather than looping. A class's own method **shadows** a
  module's (class-first); a module's method shadows the superclass's.
- `resetOop` now also clears `includedModules`.
- Extensive vitest coverage: included-method callable, class-shadows-module,
  most-recent-included-wins, superclass-module reachability, diamond-resolves-
  once, self-include terminates, re-include de-duplicated, `extend`→class
  method (and NOT an instance method), multi-method extend, reset clears the
  include table.

## [0.1.10] - 2026-07-01

### Changed — typed runtime errors from `.fetch` and unknown methods (T2)

Part of the `sir-typed-runtime-errors` cascade (spec
`code/specs/sir-typed-runtime-errors.md`). Three faulting dispatch paths now
raise the correct typed `SirError` (matching Ruby) so `rescue
IndexError`/`KeyError`/`NoMethodError` catches them, replacing the previous
silent `nil` floor. Raises go through the exceptions runtime's `raiseError`
(explicit-string, no reflection).

- **`Array#fetch(index)`** — added to the catalog. Returns the element for an
  in-range index (negatives count from the end); an out-of-bounds index with no
  default raises `IndexError` (`index N outside of array bounds: -M...M`). A
  second argument is returned as the default (no raise). Unlike `arr[i]` (the
  index operator), which still returns nil.
- **`Hash#fetch(key)`** — a missing key with no default now raises `KeyError`
  (`key not found: <inspect>`) instead of returning nil. A second argument is
  still returned as the default. Plain `hash[k]` still returns nil.
- **Unknown method** — the `callMethod` floor now raises `NoMethodError`
  (`undefined method 'x' for <Class>`) for a method the receiver genuinely does
  not have. **Guarded by `respondsTo`**: a *known* method invoked in a shape v0
  does not model — most notably a block-taking method called *without* a block
  (`[1,2,3].map`, `5.times`, which Ruby answers with an Enumerator) — still
  bottoms out at `nil` and is NOT mis-raised.

### Dependencies

- Adds a `file:` dependency on `@coding-adventures/sir-runtime-exceptions` (for
  the shared `raiseError`/`SirError` typed-raise entry point). BUILD deps updated
  to list the full transitive `file:` set (core + exceptions + pairs).

### Note

This is a behaviour change: programs previously relying on the silent nil floor
for unknown methods, `Array#fetch` OOB, or `Hash#fetch` miss now see a typed
raise — which is the correct Ruby semantics and the point of T2.

## [0.1.9] - 2026-07-01

### Added (O1 — user class/instance method tables + new/super/self)

Mirrors the Python runtime's O1 additions so Ruby→TS executes user-defined OOP.
Additive: no existing behaviour changes; these helpers only run once the new
builtins appear (frontend O2).

- **Method tables.** Two explicit `(class, method) → Closure` `Map`s —
  `instanceMethods` (populated by `defMethod`, for `def m`) and `classMethods`
  (populated by `defClassMethod`, for `def self.m`). Because a JS `Map` keys
  arrays by *identity*, the `(class, method)` pair is joined with a NUL
  separator (`"class\x00method"`); dispatch is **always** an explicit `Map`
  lookup walking the ancestry chain — **never** reflection on a source-derived
  name (the C3 RCE lesson). Ancestry walks are cycle-guarded like `isA`.
- **`callNew(class, ...args)`** — allocate → push self → run inherited
  `initialize` → pop self → return the object.
- **`callMethod` user-object path** — a `SirInstance` receiver dispatches a
  registered instance method (ancestry-walked) first under a pushed self, then
  falls through to the existing reflective built-ins / primitive catalog (no
  regression).
- **`callSuper(method, class, ...args)`** — walks from the parent up, re-runs the
  first ancestor implementation with the current self still bound; `null` when
  unresolved.
- **`callClassMethod(class, method, ...args)`** — `def self.m` dispatch via the
  class-method table; `null` when unresolved.
- **`currentSelfVal()`** — the top of the self-stack (or `null` at top level).
  Named `currentSelfVal` to avoid clashing with the module-private
  `currentSelf` (the ivar-store default-self helper).

**Single-threaded caveat.** Self is a process-global stack — faithful for the
single-threaded transpiled scripts we target; true per-object/per-thread binding
is out of v0 scope.

## [0.1.8] - 2026-06-30

### Added (M6 — Kernel flow-control + boolean operators)

Completes the spec's v0 universal-`Object` surface
(`code/specs/sir-method-dispatch.md`), which listed `tap`, `then`/`yield_self`,
and `send`/`__send__` but had not yet implemented them, plus the `null`/`true`/
`false` boolean operators:

- **`send`/`__send__`/`public_send`** — dynamic dispatch. The first argument
  names the method (a `Sym` — the emitted form — or a bare string); the rest
  forward unchanged and a **trailing block survives**, so
  `[1, 2].send("each", blk)` reaches the block-taking catalog method. Routed
  ahead of the catalog because it recurses through `callMethod`; an empty arg
  list (`send` with no method name) bottoms out at the `null` floor rather than
  throwing. This is the substrate the spec flagged for later metaprogramming.
- **`tap`** — yields the receiver to the block, returns the **receiver**
  (pipeline-friendly side effect).
- **`then`/`yield_self`** — yields the receiver, returns the **block's result**
  (functional "pipe into a block"). Block-less `tap`/`then`/`yield_self` return
  the receiver (the documented v0 Enumerator-less floor).
- **`TrueClass`/`FalseClass` `&` / `|` / `^`** — Ruby's *eager*
  (non-short-circuit) logical operators, distinct from the lazy `&&`/`||`
  keywords. The operand is coerced by SIR `truthy`, so `true & null == false`,
  `false | 0 == true`, `true ^ true == false`. They resolve on a `boolean`
  receiver before the universal `Object` table.

`respond_to?` reports each new name honestly — `tap`/`then`/`send` on every
receiver, the boolean operators on `boolean` receivers only — and an
out-of-catalog name stays both `null` *and* `respond_to? == false`. Strict `tsc`
clean; 77 vitest cases.

## [0.1.7] - 2026-06-26

### Added (M5 — case-equality `===`)

- `caseEq(pattern, value)` — Ruby case-equality, the test a `when` clause runs.
  Dispatches on the *pattern*'s type: a `RegExp` → the regex matches
  `String(value)`; a `Range` (detected structurally by constructor name + an
  `includes` method, so no dependency on `sir-runtime-range`) → membership;
  anything else → value equality (`eq`). The class case (`when Integer`) is
  handled at the frontend (`value.is_a?(Const)`) and never reaches here.

## [0.1.6] - 2026-06-22

### Added

**`Symbol#to_proc` (`&:sym`) — `symToProc`** (per `code/specs/sir-method-dispatch.md`,
item M2). New `symToProc(sym): Closure`: builds a `@coding-adventures/sir-runtime-core`
`Closure` equivalent to Ruby's `sym.to_proc`, so a `&:sym` block argument on a
dispatched call works — `[1, 2, 3].map(&:to_s)` now evaluates to
`["1", "2", "3"]`.

- The Ruby→SIR frontend lowers `&:sym` to `block_pass(SymLit("sym"))`; the
  backend emits the surviving envelope as `__SirOop.symToProc(intern("sym"))`,
  and the resulting `Closure` is driven by the block-taking catalog methods
  (`map`/`select`/`each`/…) through `apply` exactly like a `{ }` block.
- `apply` forwards a block method's arguments unadjusted: the first becomes the
  **receiver**, the rest are forwarded as method arguments. This matches
  `&:sym`'s Ruby arity (one required receiver plus a rest) — correct for the
  one-arg (`map`) and two-arg (`include?`-style) shapes alike.
- The proc body dispatches through `callMethod`, so an **out-of-catalog method
  bottoms out at `null`** rather than throwing — the never-throw-on-the-OO-surface
  invariant holds for the proc body too. A bare string name is accepted
  defensively in addition to a `Sym`.

### Known v0 limitation

Arithmetic/comparison operators (`&:+`, `&:<`) are emitted as **native**
operations, not routed through the dispatch catalog, so `inject(&:+)` is not yet
supported (the proc would resolve `+` to `null`). Operator dispatch is tracked in
the later numeric-fidelity item.

## [0.1.5] - 2026-06-22

### Added

Built-in method dispatch, part 5 — the **`Integer`/`Float`**, **`Symbol`**, and
**`nil`/`true`/`false`** catalogs (per `code/specs/sir-method-dispatch.md`, item
M1c), completing the M1c primitive surface:

- **Numeric** (`number`): `abs`, `to_i`, `to_f`, `even?`, `odd?`, `zero?`,
  `positive?`, `negative?`, `succ`/`next`, `pred`, `floor`, `ceil`, `round`
  (half **away from zero**, unlike JS `Math.round`), `gcd`, `pow`/`**`, `digits`;
  block forms `times`, `upto`, `downto`, `step`.
- **Symbol** (`sir-runtime-core` `Sym`): `to_s`, `to_sym`, `length`/`size`,
  `upcase`/`downcase` (return a new interned symbol), `inspect`, `empty?`.
- **`to_s`/`inspect` are now universal `Object` methods** (Ruby display forms):
  `nil.to_s == ""` / `nil.inspect == "nil"`, `true.to_s == "true"`, numbers and
  symbols print faithfully, and an `Array`/`Map` renders `"[1, 2]"` / `"{:k=>v}"`.
  This means **`null`/`true`/`false` need no catalog of their own** (`nil.to_a`
  already returns `[]`). Added **`Array#join`** (elements via `to_s`, default sep
  `""`).
- `boolean` is a distinct `typeof` from `number`, so `true`/`false` resolve only
  the `Object` methods, never the numeric catalog.

`respond_to?` reports each new catalog honestly; out-of-catalog stays `null`.

### Security / robustness

- **`digits` guards a non-finite receiver.** `2 ** 1e9` saturates to `Infinity`
  in IEEE-754; `digits(Infinity)` would otherwise spin forever, so it now returns
  `[0]`. (Integer `**`/`pow` saturate to `Infinity` in O(1) — no bignum DoS as in
  the Python backend.)
- **`to_s`/`inspect`/`join` are cycle- and depth-safe.** A self-referential
  `Array`/`Map` renders `[...]`/`{...}` (Ruby's behaviour) and depth is capped, so
  a cyclic or deeply-nested structure can no longer overflow the stack.

### Known v0 limitation

JavaScript cannot distinguish `3.0` from `3` (both `number`), so a whole-valued
Ruby `Float` prints as an integer via `to_s`/`inspect` — documented, matching the
existing `classOf` `Integer`/`Float` split.

## [0.1.4] - 2026-06-22

### Added

Built-in method dispatch, part 4 — the **`String`** catalog (per
`code/specs/sir-method-dispatch.md`, item M1c; a Ruby `String` is a JS `string`,
which is **immutable**, so every method is non-mutating and returns a fresh
value):

- Non-block: `length`/`size`, `upcase`, `downcase`, `capitalize`, `reverse`,
  `strip`/`lstrip`/`rstrip`, `chomp` (default line-ending or explicit suffix),
  `chars`, `bytes` (UTF-8), `split` (whitespace default or literal separator),
  `include?`, `start_with?`, `end_with?`, `index` (null when absent), `replace`,
  `sub`/`gsub` (**literal** — pattern matched as a plain substring, replacement
  inserted verbatim; deliberately side-steps `String.prototype.replace`'s
  special-replacement parsing of `$&`/`$1`/`$$`), `to_i`/`to_f` (leading-numeric,
  `0` when none — never `NaN`), `to_sym` (interns a `@coding-adventures/sir-runtime-core`
  `Sym`), `empty?`, `*` (repeat), `+` (concat).
- Block: `each_char` — applied via `@coding-adventures/sir-runtime-core` `apply`.

`respond_to?` reports the String methods; out-of-catalog stays `null`. Universal
`Object` methods still resolve on a String receiver.

## [0.1.3] - 2026-06-22

### Added

Built-in method dispatch, part 3 — the **`Hash`** catalog (per
`code/specs/sir-method-dispatch.md`, item M1c; Hash is a JS `Map`):

- Non-block: `keys`, `values`, `has_key?`/`key?`/`include?`/`member?`,
  `has_value?`/`value?` (deep value equality), `fetch` (with optional default),
  `size`/`length`, `empty?`, `to_a` (`[[k, v], …]`), `dig` (single-level v0),
  `store`/`[]=`, `merge` (new `Map`), `delete`, `clear`, `invert`.
- Block (block receives `[key, value]`): `each`/`each_pair`, `each_key`,
  `each_value`, `map`, `select`/`filter`, `reject` — applied via
  `@coding-adventures/sir-runtime-core` `apply`, predicates through SIR `truthy`.

`respond_to?` reports the Hash methods; out-of-catalog stays `null`. Universal
`Object` methods still resolve on a Hash receiver.

## [0.1.2] - 2026-06-22

### Added

Built-in method dispatch, part 2 — **block-taking `Array`/`Enumerable` methods**
(per `code/specs/sir-method-dispatch.md`, item M1b):
`each`, `each_with_index`, `map`/`collect`, `select`/`filter`, `reject`,
`reduce`/`inject` (with/without initial), `find`/`detect`, `flat_map`,
`any?`/`all?`/`none?`. A trailing `Closure` block is applied via
`@coding-adventures/sir-runtime-core`'s `apply` (proc-lenient arity); predicate
results route through SIR `truthy` (only `false`/`nil` are falsy, so `0`/`""` are
kept). `respond_to?` now reports these method names.

### Changed

- Adds a dependency on **`@coding-adventures/sir-runtime-core`** (for `apply`,
  `Closure`, `truthy`). Its transitive `sir-runtime-pairs` dep is also listed as a
  direct dep (npm does not recursively install file deps' own file deps).
- A block method invoked **without** a block still bottoms out at `null` (Ruby
  returns an `Enumerator`; v0 floor, documented).

## [0.1.1] - 2026-06-22

### Added

Built-in method dispatch, part 1 — non-block `Array` + universal `Object`
methods (per `code/specs/sir-method-dispatch.md`, item M1a). Before this,
`callMethod` returned `null` for every method outside `is_a?`/`kind_of?`/
`instance_of?`/`class` + the `defineMethod` table, so `[1,2,3].reverse`
evaluated to nil instead of running.

- **Resolution order** in `callMethod` is now reflective built-ins → user
  `defineMethod` table → built-in catalog → `null` floor.
- **Array (non-block):** `length`/`size`/`count`, `first`/`last` (±count),
  `include?`, `index`, `push`/`append`/`<<`, `pop`, `shift`, `unshift`/`prepend`,
  `reverse`, `sort`, `min`, `max`, `sum`, `uniq`, `flatten`, `compact`, `empty?`,
  `to_a`. Mutating methods mutate in place and return the Ruby-specified value;
  `reverse`/`sort` are non-mutating. `include?`/`index`/`==` use deep value
  equality (Ruby `==`).
- **Object (universal):** `nil?`, `==`, `!=`, `equal?`, `respond_to?`, `freeze`,
  `frozen?`, `dup`/`clone`, `itself`, `to_a` (nil→`[]`).
- **`respond_to?` is honest** — reports true only for names dispatch actually
  resolves, so an out-of-catalog method is both `null` *and* `respond_to? == false`.

Deferred to follow-ups: block-taking methods (`each`/`map`/`select`/…, need the
`@coding-adventures/sir-runtime-core` `apply` dependency) and the
Hash/String/Numeric/Symbol catalogs.

## [0.1.0] - 2026-06-13

### Added

Initial release — the OOP runtime imported by Semantic-IR-emitted
TypeScript/JavaScript. Provides the object-model semantics that have no faithful
native equivalent once the Ruby→SIR frontend has hoisted methods to detached,
receiver-less top-level functions:

- **Class registry** — `defineClass(name, superName?)`, `superclassOf(name)`.
- **Class identity / ancestry** — `classOf(value)` (maps JS values to Ruby class
  names, including `Integer`/`Float`/`String`/`Array`/`Hash`/`NilClass`/
  `TrueClass`/`FalseClass`), and `isA(value, className)` with ancestry-chain
  walking, the `Numeric` umbrella, and `Object`/`BasicObject` universal roots
  (cycle-safe).
- **Instances** — `SirInstance` (class tag + instance-variable bag),
  `newInstance(className)`.
- **Instance-variable store** — `pushSelf`/`popSelf` (a current-self stack) +
  `ivarGet`/`ivarSet`; unset reads yield `null` (nil).
- **Class-variable store** — `cvarGet`/`cvarSet`.
- **Method dispatch** — `callMethod(recv, name, ...args)` handling the reflective
  built-ins the frontend emits as `__method__` calls (`is_a?`, `kind_of?`,
  `instance_of?`, `class`) plus a `defineMethod` fallback table; unknown methods
  return `null` rather than throwing.
- `resetOop()` for test isolation.

**v0 limitation (documented):** the frontend does not thread receivers into
method bodies, so the current-self is a process-global stack and class variables
share one namespace keyed by bare name. This models single-instance /
single-class programs faithfully without crashing; full multi-object semantics
await frontend receiver threading. See `code/specs/sir-runtime.md`.
