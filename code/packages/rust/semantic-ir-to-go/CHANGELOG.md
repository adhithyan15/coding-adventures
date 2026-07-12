# Changelog

## 0.30.0 — Array `each_slice` / `each_cons` / `chunk_while`

Mirrors the Python reference (PR #8031) into the Go backend's inline `__sir`
runtime, adding the Array consecutive-grouping family (`_sir_array_method` for the
non-block `each_slice`/`each_cons`, `_sir_array_block_method` for `chunk_while`,
plus the `_sir_array_responds` `respond_to?` arm).

- `each_slice(n)` → consecutive sub-arrays of at most `n` elements, the last
  possibly shorter (`[1,2,3,4,5].each_slice(2)` → `[[1,2],[3,4],[5]]`).
- `each_cons(n)` → every consecutive `n`-element sliding window
  (`[1,2,3,4].each_cons(2)` → `[[1,2],[2,3],[3,4]]`); a window larger than the
  array yields `[]`.
- Both treat `n <= 0` as `[]` (Ruby raises `ArgumentError`; the never-panic floor
  yields empty instead).
- `chunk_while { |prev, cur| pred }` → runs of consecutive elements; the block is
  called on each ADJACENT pair, a truthy result extends the run and a falsy one
  starts a new run (`[1,2,4,5,7].chunk_while { |a,b| b-a==1 }` →
  `[[1,2],[4,5],[7]]`).  Empty → `[]`; single element → `[[x]]`.

Exec-proof: `tests/compile_and_run_array_methods.rs` gains
`array_each_slice_each_cons_chunk_while_compile_and_run`, running each_slice/
each_cons (incl. `n<=0` and oversized-window → `[]`) and chunk_while (adjacent
`b-a==1` predicate; empty → `[]`) under real `go run`, diffed against the Python
reference semantics.

## 0.29.0 — Hash `to_h` (block + no-block) / `each_with_index` / `each_with_object`

Mirrors the Python reference (PR #8009) into the Go backend's inline `__sir`
runtime, rounding out Hash's Enumerable iteration surface (`_sir_hash_method` for
the no-block `to_h`, `_sir_hash_block_method` for the block forms, plus the
`_sir_hash_responds` `respond_to?` arm).

- `to_h` **without** a block → a shallow copy of the hash (a fresh `*Map`, so
  mutating it does not alias the receiver's entries).
- `to_h { |k, v| [new_k, new_v] }` → a NEW hash from the block-returned `[k, v]`
  pairs; the block is yielded the two args `(k, v)`; a non-pair result is skipped
  (never-raise floor — Ruby's TypeError is deferred to the typed-error cascade),
  and a later pair with a duplicate key wins (Ruby's rule, `_sir_map_set`).
- `each_with_index { |(k, v), i| … }` → yields each `[k, v]` pair with its
  0-based index, returns the receiver.
- `each_with_object(memo) { |(k, v), memo| … }` → yields each `[k, v]` pair with
  the memo, returns the (mutated) memo; no-memo arg returns the receiver.

Unlike `each`'s two-arg `(k, v)` yield, `each_with_index`/`each_with_object` pass
the element as a single `[k, v]` `*Seq` (the second block param is the
index/memo), matching Ruby's Enumerable convention.

Exec-proof: `tests/compile_and_run_hash_methods.rs` gains
`hash_to_h_and_indexed_iteration_compile_and_run`, running to_h (copy + re-map),
each_with_index (observed pair+index yield, returns self), and each_with_object
(observed pair+memo yield, returns memo, and no-memo passthrough) under real
`go run`, diffed against the Python reference semantics.

## 0.28.0 — Hash Enumerable breadth: `group_by` / `partition` / `flat_map` / `reduce` / `inject` / `sum`

Mirrors the Python `sir-runtime-oop` v0.1.20 reference (PR #7978) into the Go
backend's emitted runtime (`_sir_hash_block_method` + `_sir_hash_responds`).
The block is yielded `(key, value)` (two arguments) — except `reduce`/`inject`,
which follow Ruby's memo convention and yield `(memo, [key, value])` (the pair
as one second argument).  Every "element" a result carries is the two-element
`[key, value]` Array (`&Seq{key, value}`).

- `group_by { |k, v| … }` — a Hash of block key → Array of `[k, v]` pairs, in
  first-seen key order.
- `partition { |k, v| … }` — `[[matching pairs], [non-matching pairs]]`.
- `flat_map`/`collect_concat { |k, v| … }` — one-level splice of block results.
- `reduce`/`inject(init) { |memo, (k, v)| … }` — fold; a seedless `reduce`
  starts from the first pair, and an empty seedless `reduce` returns `nil`.
- `sum(init = 0) { |k, v| … }` — `init` plus the polymorphic-`+` (`_sir_plus`)
  sum of the block results.

`_sir_hash_responds` now advertises all of the above (the hash block dispatch
already forwards the positional args before the block, so `reduce`/`sum` read
their seed).

Exec-proof: `tests/compile_and_run_hash_methods.rs` gains
`hash_enumerable_breadth_compile_and_run`, running `group_by` (even-value
predicate ⇒ bool-keyed Hash of pairs), `partition`, `flat_map`, `reduce(0)`, and
`sum(100)` under real `go run`, diffed against the Python reference semantics.

## 0.27.0 — Hash Enumerable aggregates: `find` / `any?` / `all?` / `none?` / `count` / `sort_by` / `min_by` / `max_by`

Mirrors the Python `sir-runtime-oop` v0.1.19 reference (PR #7957) into the Go
backend's emitted runtime (`_sir_hash_block_method` + `_sir_hash_responds`).
Ruby's `Hash` mixes in `Enumerable`, so these iterate the hash as a sequence of
`[key, value]` pairs: the block is yielded `(key, value)` (two arguments,
matching `each`), and the "element" an aggregate returns is the two-element
`[key, value]` Array (`&Seq{key, value}`).

- `find`/`detect` — first `[k, v]` pair with a truthy block result; `nil` if none.
- `any?`/`all?`/`none?` — booleans over `block(k, v)`.
- `count { |k, v| … }` — number of pairs with a truthy block result.
- `sort_by` — a NEW Array of `[k, v]` pairs sorted by the block key (stable on
  ties, Schwartzian; the never-panic `_sir_value_lt` comparator).
- `min_by`/`max_by` — the extremal `[k, v]` pair (first-on-tie; `nil` on empty).

`_sir_hash_responds` now advertises all of the above.

Exec-proof: `tests/compile_and_run_hash_methods.rs` gains
`hash_enumerable_aggregates_compile_and_run`, running `sort_by`/`min_by`/
`max_by` (by value), `find`/`count`/`any?`/`all?`/`none?` (even-value
predicate) under real `go run`, diffed against the Python reference semantics.

## 0.26.0 — Hash transforming block methods: `transform_values` / `transform_keys`

Mirrors the Python `sir-runtime-oop` v0.1.18 reference into the Go backend's
emitted runtime (`_sir_hash_block_method` + `_sir_hash_responds`), adding two
non-mutating Ruby `Hash` block methods:

- `transform_values { |v| … }` — builds a **new** hash whose keys are copied
  verbatim (so no collision is possible) and whose values are the block results.
  Original insertion order is preserved via a straight append.
- `transform_keys { |k| … }` — builds a **new** hash whose values are untouched
  and whose keys are the block results.  Two source keys can map to the SAME new
  key; Ruby keeps the **last** colliding entry's value, so every write is routed
  through `_sir_map_put`, which overwrites an existing key in place.

Both yield exactly ONE block argument (the value / the key) and leave the
receiver unmodified.  `_sir_hash_responds` now also advertises the pre-existing
`each_key` / `each_value` block methods (previously reachable but not reported by
`respond_to?`).

Exec-proof: `tests/compile_and_run_hash_methods.rs` gains a `transform_values`
case ({a:1,b:2} → {a: 99, b: 99}) and a `transform_keys` **collision** case
({a:1,b:2} with a constant `:z` key → {z: 2}), compiled and run under real
`go run` with stdout diffed against the Python/TS reference semantics.

## 0.25.0 — Numeric breadth: `divmod` / `fdiv` / `round(ndigits)` / `clamp` / `between?`

Mirrors the Python `sir-runtime-oop` v0.1.17 reference into the Go backend's
emitted runtime (`_sir_numeric_method` + `_sir_numeric_responds`), adding five
Ruby numeric methods:

- `round(ndigits)` — `round` gains an optional digits argument: a positive
  `ndigits` rounds a Float to that many decimals (half **away from zero**, via
  `_sir_ruby_round`); `ndigits <= 0` rounds to a power of ten.  Go's `int64`/
  `float64` are FIXED width, so the Python bignum→float `OverflowError` pitfall
  does not apply — the only guards are a place count past int64's ~18 decimal
  digits (dwarfs the value ⇒ `0`, Ruby parity) and a positive `ndigits` past
  Float precision / an overflowing scale-up (returns the value unchanged).
- `divmod(n)` — `[quotient, remainder]` with a floored quotient (`_sir_floor_div`)
  and the divisor-signed remainder; a zero divisor raises a typed
  `ZeroDivisionError`.
- `fdiv(n)` — floating-point division that never panics: a zero divisor yields
  `±Inf`/`NaN` (Go float division already produces these).
- `clamp(min, max)` / `between?(min, max)` — compared numerically.

Dispatch stays an explicit `switch` on the interned method name (never
reflection).  Exec-proven end-to-end via `go run` (the numeric exec-proof test
now covers `round(2)`/`round(-2)`, `divmod` incl. the divisor-signed remainder,
`fdiv` incl. the divide-by-zero `Infinity`, and `clamp`/`between?`).

## 0.24.0 — String char-set methods: `tr` / `count` / `delete` / `squeeze`

Adds four non-block Ruby String methods to the emitted runtime's
`_sir_string_method` switch and the `_sir_string_responds` catalog, mirroring
the Python `sir-runtime-oop` reference semantics (rune-based, so multibyte
strings are never split mid-codepoint):

- `tr(from, to)` — position-wise rune translation; a shorter `to` repeats its
  last rune, an empty `to` deletes matching runes, and a repeated rune in `from`
  keeps the last mapping.
- `count(*sets)` / `delete(*sets)` / `squeeze(*sets)` — char-set methods:
  `count` tallies runes of the receiver in the set, `delete` removes them, and
  `squeeze` collapses consecutive runs (of set runes, or of *all* runes when no
  set is given). Multiple set arguments intersect (Ruby's rule).

Each `set`/`from`/`to` argument is treated **literally** — the range (`"a-z"`)
and negation (`"^abc"`) forms are a follow-up, matching the literal-only
`sub`/`gsub` precedent. Exec-proven end-to-end via `go run`. Second backend of
the String char-set sweep (Python landed in `sir-runtime-oop` v0.1.16).

## 0.23.0 — slice-selection Array methods: `take` / `drop` / `values_at`

Extends the emitted Go runtime's non-block `Array` catalog (and the
`respond_to?` table):

- `take(n)` — a fresh Array of the first `n` elements; `n` is clamped to
  `[0, len]` (`n <= 0` → `[]`, `n > len` → a full copy). A negative `n` raises
  `ArgumentError` in Ruby; the never-raise floor treats it as `0`.
- `drop(n)` — a fresh Array with the first `n` elements removed (same clamp;
  `n >= len` → `[]`).
- `values_at(*idxs)` — a fresh Array of the element at each index, folding a
  negative index from the end; an out-of-range index yields `nil` (never
  panics).

Verified end-to-end under `go run`.

## 0.22.0 — more String methods: `ljust` / `rjust` / `center` / `swapcase`

Extends the emitted Go runtime's `_sir_string_method` catalog (and its
`respond_to?` table):

- `ljust(width, pad = " ")` / `rjust(...)` / `center(...)` — pad to `width`
  **runes** using `pad` cyclically; `width <= length` returns the string
  unchanged; `center` puts an odd extra pad rune on the RIGHT (Ruby's rule).
  An empty pad degrades to a single space rather than raising (never-raise
  floor). New helper `_sir_str_pad` builds the exact-length cyclic padding.
- `swapcase` — flips the case of each ASCII letter (rune-safe; non-letters and
  non-ASCII runes pass through).

Also **fills a pre-existing `respond_to?` under-report**: `capitalize`,
`chomp`, `bytes`, `index`, `replace`, `sub`, `gsub` already dispatch in
`_sir_string_method` but were unlisted; the table is now faithful.

Verified end-to-end under `go run`.

## 0.21.0 — more Array methods: `zip` / `rotate` / `to_h` / `tally`

Extends the emitted Go runtime's non-block `Array` catalog and the
`respond_to?` table:

- `zip(*others)` — Array of tuples `[a[i], b[i], …]`, length = the receiver's;
  a shorter operand pads with nil; a non-array operand is treated as empty.
- `rotate(n = 1)` — elements rotated left by `n` (a negative `n` rotates
  right); the modulo wraps so any `n` terminates without panicking.
- `to_h` — `[[k, v], …]` → a Hash (2-element-array elements only; others
  skipped, matching the never-raise floor).
- `tally` — a Hash of element → occurrence count, first-seen order, keyed by
  structural value-equality.

Verified end-to-end under `go run`.

## 0.20.0 — Array block-method breadth (sort_by / group_by / partition / …)

Mirrors the Rust backend's array block-method batch to Go: extends
`_sir_array_block_method` with the common block-taking Ruby
`Enumerable`/`Array` methods that were missing, and grows the `respond_to?`
table to match.

- `sort_by { |x| key }` — key-sorted (Schwartzian, stable).
- `min_by` / `max_by { |x| key }` — extremal block key (first-on-tie; `nil` on
  empty).
- `group_by { |x| key }` — a Hash of key → Array of elements.
- `partition { |x| pred }` — `[matching, non_matching]`.
- `flat_map` / `collect_concat { |x| … }` — map then splice one level.
- `take_while` / `drop_while { |x| pred }` — leading truthy run / remainder.
- `count { |x| pred }` — number of truthy results (bare/arg forms unchanged).
- `each_with_object(memo) { |x, memo| … }` — folds into and returns the memo.

Ordering reuses `_sir_value_lt` — the never-panic comparator, so a non-numeric
block key degrades to a stable order rather than raising (unlike a naive
numeric coerce). A block-less call floors to `NoMethodError` (Ruby returns an
Enumerator — a v0 cut-line). Verified end-to-end under `go run`.

## 0.19.0 — source-language display convention: Ruby booleans (`true`/`false`)

Mirrors the Rust backend's first increment of the SIR display-convention spec
(`code/specs/sir-display-convention.md`) to Go. A **Ruby**-sourced module now
renders booleans as `true`/`false` instead of the Twig/Lisp `#t`/`#f`, so a
translated `puts true` prints `true`.

Mechanism: the runtime carries a compile-time `const _sir_display_ruby` (a
`__SIR_DISPLAY_RUBY__` placeholder); the emitter substitutes `true`/`false`
from `Module.metadata.source_language` (`== "ruby"` → `true`, else `false`).
`_sir_format` branches the boolean arm on it. The default is the Lisp form, so
all existing non-Ruby (Twig) output is **byte-for-byte unchanged**. The Go
compiler folds the `const` branch — zero per-call cost.

Scope: booleans only (the flagship divergence); `nil`, symbols, string
`inspect` quoting, and the Ruby hash `=>` element form remain follow-ups per
the spec's rollout. Verified end-to-end under `go run`: Ruby source →
`true\nfalse\n`; Twig source → `#t\n#f\n`.

## 0.18.0 — Numeric + String method-catalog parity

Expands the emitted Go runtime's `_sir_numeric_method` and
`_sir_string_method` catalogs to Ruby parity, and grows the matching
`_sir_numeric_responds` / `_sir_string_responds` predicates so
`respond_to?` stays honest.

**Numeric (`int64` / `float64`):** `to_int`, `positive?`, `negative?`,
`succ` / `next`, `pred`, `floor`, `ceil`, `round` (`_sir_ruby_round`,
round-half-up), `gcd` (`_sir_gcd`, overflow-safe), `pow` / `**`
(`_sir_int_pow`, with a closed-form short-circuit for base ∈ {0, 1, −1}
and a bit-width guard so a large exponent can't spin), `digits`
(`_sir_digits`), and the block-taking walkers `upto` / `downto` / `step`
(counter arithmetic guarded against `int64` boundary overflow).

**String:** `capitalize`, `chomp`, `bytes`, `index`, `replace`, `sub`,
`gsub` (literal, first/all-occurrence; no regex or back-reference
expansion). All arity-guard their optional arguments (`len(args)` checks
before any `args[0]`), returning `nil`/receiver rather than panicking.

Dispatch stays receiver-type routed through explicit `switch` labels — no
reflection on source-derived method names.

(Consolidates the previously-separate Numeric and String catalog PRs into
one crate change to avoid intra-crate version churn.)

## 0.16.0

### Added â€” Ruby Symbol method catalog completion (`capitalize` / `inspect` / `to_proc`)

Parity-fill: the Python + TypeScript `sir-runtime-oop` Symbol catalogs already
expose `inspect`; this ports it into the Go runtime's `_sir_symbol_method`
switch and adds the two task-mandated Ruby Symbol methods `capitalize` and
`to_proc`, so a translated Ruby program's Symbol calls execute on the Go
backend instead of hitting the `NoMethodError` floor.

- **`inspect`** â€” returns the source form `":name"` (a String). Matches the
  Python/TS reference semantics.
- **`capitalize`** â€” returns a NEW interned `*Symbol` whose name has an
  uppercase first char and a lowercase remainder (rune-aware, mirroring the
  existing `upcase`/`downcase` arms).
- **`to_proc`** â€” an explicit `sym.to_proc` call returns a `*Closure` built by
  the SAME `_sir_sym_to_proc` helper the `&:sym` block-pass form uses. The
  resulting proc routes each application through the explicit
  `_sir_call_method` switch â€” NEVER Go `reflect` ([[dynamic-dispatch-rce]]); an
  out-of-catalog method surfaces the ordinary `NoMethodError`. Note: the
  `&:sym` block-pass form is FRONTEND-lowered straight to `_sir_sym_to_proc`
  (see `try_emit_block_pass` in `emit.rs`) and never reaches this catalog arm;
  `to_proc` is added for the explicit-call path and full correctness.
- `_sir_symbol_responds` (`respond_to?`) updated to include `capitalize`,
  `inspect`, and `to_proc`.

Exec-proof: `tests/compile_and_run_symbol_methods.rs` runs the emitted Go under
a real `go run` toolchain and asserts `:hello.to_s`â†’"hello", `:hi.length`â†’"2",
`:abc.upcase`â†’"ABC", `:ABC.downcase`â†’"abc", `:hELLO.capitalize`â†’"Hello",
`:x.inspect`â†’":x", `[1,2,3].map(&:to_s).join`â†’"123" (block-pass form), and
`[4,5,6].map(:to_s.to_proc).join`â†’"456" (explicit catalog `to_proc`).

## 0.16.0

### Added â€” Ruby Symbol method catalog completion (`capitalize` / `inspect` / `to_proc`)

Parity-fill: the Python + TypeScript `sir-runtime-oop` Symbol catalogs already
expose `inspect`; this ports it into the Go runtime's `_sir_symbol_method`
switch and adds the two task-mandated Ruby Symbol methods `capitalize` and
`to_proc`, so a translated Ruby program's Symbol calls execute on the Go
backend instead of hitting the `NoMethodError` floor.

- **`inspect`** â€” returns the source form `":name"` (a String). Matches the
  Python/TS reference semantics.
- **`capitalize`** â€” returns a NEW interned `*Symbol` whose name has an
  uppercase first char and a lowercase remainder (rune-aware, mirroring the
  existing `upcase`/`downcase` arms).
- **`to_proc`** â€” an explicit `sym.to_proc` call returns a `*Closure` built by
  the SAME `_sir_sym_to_proc` helper the `&:sym` block-pass form uses. The
  resulting proc routes each application through the explicit
  `_sir_call_method` switch â€” NEVER Go `reflect` ([[dynamic-dispatch-rce]]); an
  out-of-catalog method surfaces the ordinary `NoMethodError`. Note: the
  `&:sym` block-pass form is FRONTEND-lowered straight to `_sir_sym_to_proc`
  (see `try_emit_block_pass` in `emit.rs`) and never reaches this catalog arm;
  `to_proc` is added for the explicit-call path and full correctness.
- `_sir_symbol_responds` (`respond_to?`) updated to include `capitalize`,
  `inspect`, and `to_proc`.

Exec-proof: `tests/compile_and_run_symbol_methods.rs` runs the emitted Go under
a real `go run` toolchain and asserts `:hello.to_s`â†’"hello", `:hi.length`â†’"2",
`:abc.upcase`â†’"ABC", `:ABC.downcase`â†’"abc", `:hELLO.capitalize`â†’"Hello",
`:x.inspect`â†’":x", `[1,2,3].map(&:to_s).join`â†’"123" (block-pass form), and
`[4,5,6].map(:to_s.to_proc).join`â†’"456" (explicit catalog `to_proc`).

## 0.15.0

### Added â€” Array collection-method parity (min / max / sum / uniq / flatten / compact / each_with_index)

Parity-fill: these Ruby `Array` methods already shipped in the Python + TypeScript
`sir-runtime-oop` backends; this ports the SAME surface into the Go runtime's
array dispatch, so a translated Ruby program now executes them on the Go backend
instead of hitting the `NoMethodError` floor. `to_a` was already present and is
unchanged. Semantics match the Python/TS reference impls exactly.

- **`min` / `max`** (non-block, v0) â€” element-wise extremum via `_sir_value_lt`
  (Ruby's `<`/`>`); empty array â‡’ nil. Dispatched in `_sir_array_method`.
- **`sum`** â€” folds with the polymorphic `_sir_plus` over an initial value
  (default `0`, or the supplied `sum(init)` argument), preserving int/float;
  empty array â‡’ the initial value. Dispatched in `_sir_array_method`.
- **`uniq`** â€” order-preserving de-duplication via structural value-equality
  (`_sir_value_eq`); returns a fresh `*Seq`. Dispatched in `_sir_array_method`.
- **`flatten`** â€” recursively flattens nested `*Seq` into a fresh flat `*Seq`.
  **Cycle-guarded** (CWE-674, uncontrolled recursion): the new
  `_sir_flatten_into` helper threads a `visited` set of `*Seq` handle pointers
  on the active recursion path â€” mirroring `_sir_puts_one` â€” so a self-referential
  array (`a = []; a << a`) terminates instead of overflowing the Go stack.
  Sibling (non-cyclic) occurrences still flatten in full.
- **`compact`** â€” fresh `*Seq` with nil elements removed. Dispatched in
  `_sir_array_method`.
- **`each_with_index`** â€” block-taking; yields `(element, index)` pairs and
  returns the receiver. Dispatched in `_sir_array_block_method`.
- `_sir_array_responds` now advertises all of the above for `respond_to?` parity.

Execution proof: `tests/compile_and_run_array_methods.rs`
(`array_methods_compile_and_run`) hand-builds SIR exercising each method, emits
Go, runs it under `go run`, and diffs stdout against the Python/TS reference
values (`[3,1,2].max` â†’ 3, `[1,2,2,3,1].uniq` â†’ `[1,2,3]`, `[[1,[2]],3].flatten`
â†’ `[1,2,3]`, `[1,nil,2,nil].compact` â†’ `[1,2]`, `[1,2,3].sum` â†’ 6,
`[10,20].each_with_index` â†’ `0:10`/`1:20` then the returned receiver).

## 0.14.0

### Security (review-driven)

- Arity guards on `equal?` and boolean `&`/`|`/`^`: these became reachable with
  ZERO args via the new `send` surface (`obj.send(:equal?)`, `true.send(:&)`),
  where indexing `args[0]` was a raw Go index-out-of-range panic (catchable only
  as `StandardError`, or a native crash if uncaught). They now raise a typed
  `ArgumentError` ("wrong number of arguments (given 0, expected 1)") â€” matching
  Ruby. Regression: `send_zero_arg_method_raises_argument_error_not_native_panic`.

### Added â€” M6 universal Object metaprogramming (send / tap / then / respond_to? / boolean &|^)

Parity-fill: M6 shipped in the Python + TypeScript `sir-runtime-oop` backends;
this ports the SAME surface into the Go runtime's method-dispatch path
(`_sir_call_method`), so a translated Ruby program's `send`/`tap`/`then`/
`respond_to?` and boolean `&`/`|`/`^` now execute on the Go backend instead of
hitting the `NoMethodError` floor.

- **`send`/`__send__`/`public_send`** â€” the first argument names a method; the
  dispatcher re-enters `_sir_call_method` with that name and the remaining args
  (a trailing block survives as a trailing arg). **Security ([[dynamic-dispatch-rce]]):**
  the dynamic name is coerced to a string and used ONLY as the key into the
  SAME explicit catalog/switch a normal call walks â€” an unknown name surfaces
  the ordinary `NoMethodError`. NEVER Go `reflect`/`MethodByName` on the
  source-derived name.
- **`tap`** â€” yields the receiver to the block and returns the RECEIVER; a
  block-less `tap` returns the receiver (v0 Enumerator-less floor).
- **`then`/`yield_self`** â€” yields the receiver and returns the BLOCK RESULT;
  block-less returns the receiver.
- **`respond_to?`** â€” true iff dispatch resolves the name, consulting the same
  reflective / `define_method` / type-specific + universal catalog tiers a real
  call uses (`_sir_responds_to` + per-catalog `_sir_*_responds` predicates kept
  in lockstep with the dispatch switches). Out-of-catalog â†’ honest `false`.
- **Boolean `&`/`|`/`^`** on `true`/`false` â€” Ruby's EAGER (non-short-circuit)
  logical operators, coercing the argument by SIR truthiness (`true & nil` is
  `false`, `false | 0` is `true`, `^` is XOR).
- Also filled the universal `Object` table: `inspect`, `equal?` (identity â€”
  value-equal for interned primitives, pointer-equal for `*Seq`/`*Map`/
  `*SirInstance`), `freeze`/`frozen?`, `dup`/`clone` (shallow copy of the
  mutable handles), and `nil.to_a == []` / `Array#to_a == self`.
- Exec-proof via `go run` (`tests/compile_and_run_m6_meta.rs`):
  `"hello".send(:upcase)` â†’ `HELLO`, `[1,2,3].send(:map,&blk)` â†’ `[2,4,6]`,
  `5.tap{â€¦}` â†’ `5`, `5.then{|x|x*2}` â†’ `10`, `respond_to?` true/false honesty,
  the boolean operators, and an unknown `send(:bogus)` failing cleanly through
  the NoMethodError floor (no reflection).


## 0.13.2

### Fixed â€” `or`/`and` builtins (Ruby `||`/`&&`) were unimplemented

Ruby `&&`/`and` and `||`/`or` lower (in the frontend) to
`BuiltinCall("and"/"or", [lhs, rhs])` â€” the fold covers BOTH the 2-operand
`a || b` form and a multi-value `when 1, 2, 3` chain. Only the Python backend's
emitter handled them; this backend fell through to the eager runtime dispatcher,
which has no `or`/`and` entry, so ANY `||`/`&&` (and every multi-value `when`)
crashed at runtime with `unknown builtin: or` / `and`. A case_eq-style gap: no
compile-time gate catches a frontend-emitted builtin the backend never handled.

- The emitter now special-cases `BuiltinCall("or"/"and", [a, b])`, emitting the
  SAME truthy-guarded short-circuit form as `Expr::LogicalOr`/`LogicalAnd`: rhs
  is not evaluated once lhs decides, SIR truthiness is used, and the deciding
  OPERAND is returned (Ruby semantics â€” `nil || "b"` is `"b"`, `"a" || "b"` is
  `"a"`), never a bare bool.
- Emit-shape regression test; verified end-to-end via the sir-conformance
  `logical_ops` + `multi_when` programs (13 corpus x 4 backends, all agree).


## 0.13.1

### Fixed â€” `case_eq` builtin (Ruby case-equality `===`) was unimplemented

Ruby's `case`/`when` (and `case`/`in`) lowers, in the frontend, to a chain of
`if`s whose conditions are `BuiltinCall("case_eq", [pattern, scrutinee])`. This
backend's runtime never implemented `case_eq`, so **every** `case` program hit
`_sir_call_builtin_by_name`'s `unknown builtin` floor and **panicked at
runtime** â€” `case` was unusable on the Go backend (no compile-time gate catches
a missing builtin; only execution does).

- Added `_sir_case_eq(args) Value` to the inlined runtime and wired it into both
  the emitter's helper table (direct-call path) and `_sir_call_builtin_by_name`
  (reified-closure path). Ruby keys `===` to the *pattern*'s type (Range â†’
  membership, Regexp â†’ match, else `==`); the `when SomeClass` case is lowered to
  `value.is_a?(SomeClass)` at the frontend and never reaches here. This backend's
  Value model has no Range/Regexp variant yet, so `case_eq` is exactly structural
  equality (`_sir_value_eq`), matching the Python reference in `sir-runtime-oop`;
  extend with membership/match arms when those value types land.
- New `compile_and_run_case_eq` exec proof: a `when`-style `if case_eq(â€¦)` chain
  emits Go, runs under `go run`, and matches the expected dispatch output.


## 0.13.0

### Added â€” Ruby mixins: `module` + `include` / `extend` MRO (sir-mixins MX5)

- The Go backend's emitted OOP runtime now EXECUTES Ruby mixins. A method
  defined in a `module` and mixed into a class via `include` is found through
  the class's Method Resolution Order; `extend` exposes a module's methods as
  class methods. Runtime-only change; no core-IR or frontend edit. Dispatch
  stays explicit NAME-keyed map lookup â€” NEVER reflection (the
  [[dynamic-dispatch-rce]] discipline).
- **`Feature::Modules` is now ACCEPTED.** A `Stmt::ModuleDef` (`module M; â€¦;
  end`) is hosted as a method *owner* alongside classes: its body's `def`s
  register via the SAME `__def_method__("M", â€¦)` builtin classes use (keyed by
  the module name), and its body is emitted in order like a `ClassDef` body.
  Previously `ModuleDef` was rejected at the soundness gate; the gate now
  recurses into a module body for the residual `Const` checks instead.
- **`__include__("Owner", "M")` â†’ `_sir_include`** â€” appends `M` to a per-owner
  included-module list (`_sir_included_modules map[string][]string`) in include
  order. Ruby searches the most-recently-included module first, so the
  resolution walk iterates this slice in REVERSE.
- **MRO-extended method resolution** (`_sir_resolve_instance_method`): the walk
  now follows class â†’ its included modules (reverse, recursing so a module that
  itself includes another is honoured) â†’ superclass â†’ its modules â†’ â€¦ â†’ Object.
  A class's own method SHADOWS an included module's; a module method shadows the
  superclass's. A module reached via two paths (a diamond) resolves ONCE, at its
  earliest position, because the `seen` set skips an already-visited owner. The
  walk is cycle-guarded (a self-including module or cyclic class hierarchy
  TERMINATES).
- **`__extend__("Owner", "M")` â†’ `_sir_extend`** â€” copies `M`'s instance
  methods (including those `M` itself includes) into `Owner`'s class-method
  table, so they become callable as `Owner.method`. An entry `Owner` already
  defines is not overwritten (own/class method shadows the extended module's).
- **`__class_method__("C", "m", argsâ€¦)` â†’ `_sir_call_class_method`** â€” a new
  emit arm + runtime helper wiring class-method *calls* (`Foo.bar`) through an
  ancestry-walking lookup in the class-method table (which `extend` populates).
  An unresolved name hits the controlled `NoMethodError` floor.
- Emit arms added for `__include__`, `__extend__`, and `__class_method__`; all
  owner/module/method NAMES ride in as `StrLit`s emitted through
  `quote_go_string` (never interpolated), keeping the runtime side reflection-free.
- Tests: five `go run` execution proofs (`compile_and_run_mixins.rs`) â€” an
  included-module method callable on an instance, a class method shadowing the
  module's, a module method shadowing the superclass's with a diamond include
  resolving once, `extend` making a module method a class method, and a mixed-in
  method reading an including class's `@ivar` through the shared self-stack â€”
  plus emit + runtime unit tests for the new arms and helpers.

## 0.12.0

### Added â€” typed runtime errors: ZeroDivision / Index / Key / NoMethod (sir-typed-runtime-errors T4)

- A faulting emitted runtime operation now raises the CORRECT **typed**
  `SirError` (via the existing `_sir_new_error` + `panic` entry point â€” the same
  one an explicit `raise` uses), so a translated `rescue
  ZeroDivisionError`/`IndexError`/`KeyError`/`NoMethodError` catches it exactly
  as Ruby would, and uniformly with the other backends. Runtime-only change; no
  core-IR or frontend edit. Dispatch stays explicit-string (no reflection â€” the
  [[dynamic-dispatch-rce]] discipline).
- **Division by zero** (`_sir_divide`): both the integer path and the
  float-promoted path now reject a zero divisor with
  `ZeroDivisionError` ("divided by 0"). Previously the int path did a raw
  `panic("division by zero")` (caught only as an over-broad generic
  `StandardError`) and the float path returned IEEE-754 `+Inf` (no error at
  all). This matches the spec's load-bearing rule that `1/0` **and** `1.0/0`
  raise `ZeroDivisionError`.
- **`Array#fetch`** (new entry in `_sir_array_method`): an out-of-bounds index
  raises `IndexError`; a supplied default (`fetch(i, d)`) is returned instead of
  raising; negative indices count from the end. The plain index operator
  `arr[i]` is UNCHANGED â€” `.fetch` is the raising read, `[]` is not.
- **`Hash#fetch`** (new entry in `_sir_hash_method`): a missing key raises
  `KeyError` ("key not found: â€¦"); a supplied default (`fetch(k, d)`) is
  returned instead. Because `KeyError < IndexError` in the ancestry table, a
  `rescue IndexError` also catches it. The plain `hash[k]` (`MapGet`) still
  returns `nil` â€” UNCHANGED (no over-raise).
- **Unknown method** (`_sir_method_unknown`): now raises a typed `NoMethodError`
  with a Ruby-shaped message `undefined method 'x' for <class>`, replacing the
  previous raw `panic(string)` (which was caught only as generic
  `StandardError`). The dispatch catalog remains the allowlist â€” an unknown
  name still surfaces a controlled, typed failure, never arbitrary behaviour.
- `*SirError` now implements Go's `error` interface (`Error() string`), so an
  UNCAUGHT typed panic prints a readable `panic: <Class>: <message>` banner
  instead of Go's default `(*main.SirError) 0xâ€¦` pointer dump. Cosmetic for the
  uncaught path only; `recover`/rescue matching still keys off the `Class` tag.
- Execution proof `compile_and_run_typed_errors.rs` (8 tests) runs each case
  through `go run`: `1/0` caught as `ZeroDivisionError` (and as `StandardError`
  via ancestry); `arr.fetch(oob)` â†’ `IndexError`; `h.fetch(miss)` â†’ `KeyError`
  (and caught as `IndexError` via ancestry); `obj.undefined` â†’ `NoMethodError`;
  regression that `h[miss]` (`MapGet`) still yields `nil`; and that
  `.fetch(k, default)` / an in-bounds `.fetch` do NOT over-raise.

## 0.11.0

### Added â€” polymorphic `+` / `*` for strings and arrays (sir-polymorphic-operators PO4)

- Ruby overloads `+` and `*` by receiver type, and every case lowers to the
  same SIR builtins (`_sir_plus` / `_sir_times`). The Go runtime helpers were
  previously **numeric-only** â€” they ran `_sir_as_int`/`_sir_as_float` on every
  operand â€” so `"a" + "b"` and `[1] + [2]` produced garbage or panicked. Both
  helpers now dispatch on the FIRST operand's runtime tag via a Go **type
  switch** (never reflection â€” the [[dynamic-dispatch-rce]] discipline) and add
  the string/array arms ahead of the unchanged numeric fold:
  - `_sir_plus`: first operand a `string` â†’ concatenate all operands as strings
    (`"a"+"b"` â†’ `"ab"`); first operand a `*Seq` â†’ concatenate element slices
    into a **fresh** backing array with no aliasing of any input (`[1]+[2]` â†’
    `[1, 2]`); otherwise the existing int/float-promoting numeric fold.
  - `_sir_times`: `string Ã— Integer` â†’ repeat via `strings.Repeat` (`"ab"*3` â†’
    `"ababab"`; a non-positive count yields `""`, clamped so `strings.Repeat`
    never panics); `*Seq Ã— Integer` â†’ repeat the element list into a fresh slice
    (`[0]*3` â†’ `[0, 0, 0]`; non-positive â†’ empty array); `*Seq Ã— string` â†’ join
    the elements with the separator using the same value-display helper `puts`
    uses (`_sir_format`), so `[1,2]*", "` â†’ `"1, 2"`; otherwise the numeric fold.
- Numeric `+`/`*` semantics (int64 fast path, intâ†’float promotion, variadic
  fold) are **preserved exactly** â€” the new arms only run when the first operand
  is a string/`*Seq`. Ruby `+`/`*` are binary; the string/array arms fold
  left-associatively over the variadic operand list.
- A controlled-panic helper `_sir_as_string` coerces string-`+` operands (a
  non-string operand â€” e.g. `"a" + 1` â€” panics with a Ruby-shaped "no implicit
  conversion of Integer into String" message rather than emitting garbage; the
  strict `TypeError` is deferred to the typed-runtime-errors cascade).
- Execution proof `compile_and_run_polyops.rs` runs `"a"+"b"`, `"ab"*3`,
  `[1]+[2]`, `[0]*3`, `[1,2]*", "`, and the numeric regressions `1+2` / `2*3`
  under `go run` and asserts stdout is exactly `ab\nababab\n[1, 2]\n[0, 0, 0]\n1, 2\n3\n6\n`.
- **Overflow guard (security):** the `*` repeat arms compute `len Ã— count` in a
  fixed-width host `int`, which on a large count could overflow (wrapping to a
  negative/absurd `make` capacity â†’ opaque panic) or drive a multi-gigabyte
  allocation â†’ OOM. Both arms now short-circuit an empty receiver (also avoiding
  a huge append loop) and guard `count > maxInt/len` with a controlled
  `panic("argument too big")` â€” matching Ruby's `ArgumentError: argument too
  big` â€” before any `strings.Repeat`/`make`. The count is program-controlled, so
  this closes a reachable resource-exhaustion vector.

## 0.10.0

### Added â€” `puts` builtin (Ruby semantics)

- The Go backend now emits and executes Ruby's `puts`, the most common output
  method. `puts` maps to a new variadic runtime helper `_sir_puts([]Value{â€¦})`
  (routed both by the emit helper table and the `_sir_call_builtin_by_name`
  dispatch), reusing `_sir_format` for element rendering.
- Ruby semantics implemented exactly: no-arg â†’ one newline; `puts x` â†’
  `x.to_s` + newline (no double newline when the text already ends in `"\n"`);
  `puts a, b` â†’ one line per arg; `puts []` â†’ a single newline; a `*Seq` is
  flattened recursively, one **element** per line; `puts nil` â†’ a blank line.
- Execution proof `compile_and_run_puts.rs` runs `puts "hello"; puts;
  puts [1,2,3]` under `go run` and asserts stdout is exactly
  `hello\n\n1\n2\n3\n` (the Ruby reference output).

### Security â€” cycle-guard the `puts` array flatten (CWE-674)

- `_sir_puts_one` flattened arrays by recursing per element with **no bound**.
  A `*Seq` is a shared, mutable handle, so a translated program can build a
  self-referential array (`a = []; a << a; puts a`) or a pathologically deep
  one; the unguarded recursion overflowed the Go stack and aborted the process
  â€” a denial of service (uncontrolled recursion). The flatten now threads a
  `visited` set of the `*Seq` pointers on the active path (the same identity
  key `_sir_format` uses): a handle re-encountered within its own subtree is a
  cycle and renders as Ruby's `[...]` placeholder + newline instead of
  recursing, so `puts a` on a self-referential array now **terminates** exactly
  as real Ruby does. Non-cyclic output is byte-for-byte unchanged
  (`puts [1,[2,3]]` â†’ `1\n2\n3\n`); a new regression test
  (`puts_cyclic_array_terminates`) proves the self-referential case exits
  cleanly with `[...]\n`.

## 0.9.0

### Added

- **User-defined class OOP â€” method dispatch, `new`, `self`, `super` (O4).**
  The Go backend now EXECUTES real user-defined classes (the Go analogue of the
  Python/TS `sir-runtime-oop` O1 path), not just exception subclasses.  The
  methodâ†”class association â€” which the Ruby frontend loses when it HOISTS every
  `def` to a detached top-level function â€” is recovered at RUNTIME via explicit
  `(class, method)` map tables.
  - **Inlined Go runtime** (`runtime.rs`, verbatim in every artifact):
    - `SirInstance { Class string; Ivars map[string]Value }` + `_sir_new_instance`.
    - Instance/class method tables `map[string]Value` keyed by a NUL-joined
      `class + "\x00" + method` string (a NUL cannot appear in an identifier, so
      the flattened key is unambiguous) â€” `_sir_def_method` /
      `_sir_def_class_method`.
    - `_sir_call_new(cls, argsâ€¦)` â€” allocate â†’ push self â†’ resolve an inherited
      `initialize` (walking the SHARED `_sir_ancestry` table, seen-guarded) â†’
      apply â†’ pop self via `defer` â†’ return the instance.
    - `_sir_call_method` extended: a `*SirInstance` receiver resolves the user
      method table walking ancestry (push self, apply, pop via `defer`); a miss
      falls through to universal Object methods, else the NoMethodError floor.
      NON-instance receivers reach the existing collection/built-in catalog
      **UNCHANGED**.
    - `_sir_call_super(method, cls, argsâ€¦)` â€” walk from the superclass, apply
      with the CURRENT self still bound (no push/pop â€” `super` re-dispatches on
      the same receiver).
    - `_sir_current_self()` (`__self__`), `_sir_ivar_get`/`_sir_ivar_set` on the
      current self (self-stack top, with a default-self so top-level `@x` never
      panics), and `_sir_cvar_get`/`_sir_cvar_set` for class variables.
  - **Emit arms** (`emit::emit_builtin_call`, mirroring `__method__`):
    `__new__`â†’`_sir_call_new`, `__super__`â†’`_sir_call_super`,
    `__def_method__`/`__def_class_method__`â†’ the table registrations,
    `__self__`â†’`_sir_current_self`.  Class/method names ride in as `StrLit`s and
    are emitted through `quote_go_string` â€” never interpolated.
  - **`@ivar` / `@@cvar`** (`emit::emit_var_ref` + `emit_stmt`):
    `VarRef`/`Assign{scope:Instance}` â†’ `_sir_ivar_get`/`set("@x", â€¦)`;
    `scope:ClassVar` â†’ the `_sir_cvar_*` helpers.
  - **Feature acceptance** (`lib.rs`): `ACCEPTED_FEATURES` now includes
    `InstanceVars` + `ClassVars` (alongside the existing `Classes`/`Constants`),
    so a REAL OO module is accepted and routed through the runtime.  The existing
    soundness gate still cleanly REJECTS genuinely-unsupported constructs â€” a
    general `Const` used as a value, a `Const` assignment, a `ModuleDef`
    (`Feature::Modules` stays unaccepted â€” no mixin/MRO runtime in v0).
  - **SECURITY (the C3 RCE lesson).**  Dispatch is ONLY an explicit map lookup on
    the `(class, method)` key â€” NEVER Go `reflect`/`MethodByName` on a
    source-derived name.  A class/method named `constructor`/`__proto__` is just
    a map key (a miss â†’ the clean NoMethodError floor).  Every ancestry walk
    carries a `seen` set so a cyclic hierarchy TERMINATES; self-stack pops go
    through `defer` so a panic still unwinds correctly.
  - **Tests.**  Emitted-shape unit tests for the five builtins + `@ivar`/`@@cvar`
    refs, plus `tests/compile_and_run_oop.rs` execution proofs through `go run`:
    P1 (`Dog.new("Rex").speak` â†’ `Rex`), P2 (inheritance + `super`, parent-set
    ivar visible â†’ `4`), a security case (class/method named `constructor`
    dispatches the user method; unknown `__proto__` hits the NoMethodError
    floor), and a cyclic-ancestry-terminates case.

## 0.8.0

### Added

- **Exception handling via panic/recover + ancestry (E3).**  The Go backend now
  EXECUTES `begin/rescue/ensure` and `raise` end to end.  Go has NO native
  try/catch, so exceptions are modelled with `panic` + a deferred `recover`:
  - **`Stmt::TryCatch` â†’ an immediately-invoked func** (`emit::emit_try_catch`).
    The func registers up to two deferred closures and then runs the try body:
    ```go
    func() {
      defer func() { <ensure> }()            // only if ensure present
      defer func() {
        if r := recover(); r != nil {
          if _sir_rescue_matches(r, []string{"Foo","Bar"}) { e := _sir_exc_value(r); <body> } else
          if _sir_rescue_matches(r, []string{"Baz"}) { <body> } else { panic(r) }
        }
      }()
      <try body>
    }()
    ```
    Rescue clauses are tried in **source order**; the first whose class list
    matches (per the ancestry table) runs, and if **none** match the recovered
    value is re-`panic`ked so it propagates (Ruby's "propagate when unrescued").
    An empty `exception_types` is a bare `rescue` (catch-all); `=> e` binds the
    caught value via `_sir_exc_value(r)`.
  - **ENSURE ORDERING (LIFO).**  Deferred funcs run last-in-first-out, and Ruby's
    `ensure` must run whether or not a rescue matched â€” i.e. it must run LAST â€” so
    its `defer` is registered **first** (deferred earliest â‡’ runs last).  The
    recover/dispatch `defer` is registered second (runs first): it recovers,
    dispatches, and re-`panic`s unmatched exceptions â€” a re-panic still unwinds
    through the already-registered ensure defer, so `ensure` runs on the
    propagating path too.
  - **`raise` â†’ `panic`** (`emit::emit_builtin_call`).  `raise Foo, "m"` â†’
    `panic(_sir_new_error("Foo", <msg>))` (the `Const` class name is intercepted
    and passed as a string â€” it never reaches `emit_var_ref`); `raise "boom"`
    (non-const first arg) â†’ an implicit `RuntimeError`; bare `raise` â†’ a generic
    `RuntimeError` (SIR v0 does not thread the in-flight exception into a bare
    re-raise â€” Go's `recover()` only works in a deferred func, matching the
    TS/Python backends' documented limitation).
  - **Runtime helpers** (`runtime.rs`, inlined verbatim): a `SirError` struct
    `{ Class string; Msg Value }`; `_sir_new_error(class, msg)`;
    `_sir_exc_value(r)` (the `Value` a `rescue => e` binds â€” a `*SirError`
    verbatim, or a synthesised `StandardError` wrapping a native Go panic);
    `_sir_rescue_matches(r, classNames)` (the ordered, ancestry-aware type test);
    and `_sir_register_ancestry(edges)` for user-defined class edges.  A
    `_sir_format` arm makes a caught exception print as its message (Ruby's
    `exception.message`).
  - **Built-in Ruby ancestry table** (`_sir_ancestry`), **ported from the
    TS/Python `sir-runtime-exceptions` reference for parity**: `StandardError â†’
    Exception`, `ArgumentError`/`TypeError`/`RuntimeError`/`RangeError`/
    `ZeroDivisionError`/`IOError`/`StopIteration`/`NotImplementedError`/
    `NameError`/`IndexError â†’ StandardError`, `NoMethodError â†’ NameError`,
    `KeyError â†’ IndexError`.  User `class MyErr < StandardError` declarations
    contribute one edge each, collected from every `ClassDef{superclass:Some}`
    and registered **once at program init** (`emit::emit_ancestry_init`).
  - **SECURITY â€” no reflection, cycle-guarded.**  Rescue matching is an EXPLICIT
    string-map lookup (`_sir_ancestry`), never reflection on a Go type name; user
    edges enter only via `_sir_register_ancestry` (built-in edges are never
    overwritten).  The ancestry walk carries a `seen` set so a malicious cyclic
    hierarchy (`class A<B; class B<A`) terminates instead of looping.

### Changed

- **`Feature::Exceptions`, `Feature::Classes`, `Feature::Constants` are now
  accepted** â€” but `Classes`/`Constants` ONLY for exception subclasses and the
  `raise Foo`/`rescue Foo` class-name references they carry, NOT general OOP.  A
  new structural gate `check_exception_soundness` (beside `check_no_keyword_rest_mix`)
  keeps the backend's "never mis-emit" promise: a `Const` reference/assignment
  OUTSIDE a `raise ClassName`, or a `module â€¦ end`, is rejected CLEANLY with an
  `UnsupportedFeature` error.  A class carrying instance/class variables observes
  `InstanceVars`/`ClassVars` (still unaccepted) and is rejected at the manifest
  gate; method-bearing classes hoist their `def`s to top-level Functions, so the
  `ClassDef` body reaching emit is ordinary supported statements.

## 0.7.0

### Added

- **Collection-method dispatch + runtime catalog (C5).**  The Go backend now
  EXECUTES `recv.meth(argsâ€¦)` end to end.  A method call reaches the backend as
  `BuiltinCall("__method__", [recv, StrLit("meth"), â€¦args])`; previously it fell
  through to the generic `_sir_call_builtin_by_name` fallback, which has no
  method-dispatch arm â€” so any collection method failed at runtime.  Now:
  - **Emit** (`emit.rs`): a `"__method__"` case in `emit_builtin_call` lowers the
    dispatch to `_sir_call_method(recv, "name", []Value{â€¦args})`.  A trailing
    block (`MakeClosure`) rides in as the last `[]Value` element; a `&:sym` /
    `&proc` block-pass that survives on the dispatch is converted via
    `try_emit_block_pass` (`_sir_sym_to_proc(intern("sym"))` for `&:sym`, the
    proc verbatim otherwise).  A `Const`-scoped class operand on a class
    predicate (`x.is_a?(Integer)`) is passed as its name string.
  - **Runtime** (`runtime.rs`): a new inlined `_sir_call_method(recv, name, args)`
    implements the collection-method catalog by an **explicit type-switch +
    method-name switch** (Array `*Seq` / Hash `*Map` / String / Numeric / Symbol),
    **ported from the Python/TS `sir-runtime-oop` reference for behavioural
    parity** (same method names, same semantics).  Implemented:
    - **Array**: `length`/`size`/`count`, `first`, `last`, `empty?`, `include?`,
      `index`, `push`/`append`, `<<`, `pop`, `shift`, `reverse`, `sort`, `join`,
      `to_a`, plus block methods `each`, `map`/`collect`, `select`/`filter`,
      `reject`, `reduce`/`inject`, `find`/`detect`, `any?`, `all?`, `none?`.
    - **Hash**: `keys`, `values`, `has_key?`/`key?`/`include?`/`member?`,
      `has_value?`/`value?`, `size`/`length`, `empty?`, plus block methods `each`/
      `each_pair`, `map`, `select`/`filter`, `reject`.
    - **String**: `length`/`size`, `upcase`, `downcase`, `reverse`, `strip`/
      `lstrip`/`rstrip`, `empty?`, `include?`, `start_with?`, `end_with?`, `split`,
      `chars`, `to_i`, `to_f`, `to_sym`.
    - **Numeric**: `abs`, `to_i`, `to_f`, `even?`, `odd?`, `zero?`, `positive?`,
      `negative?`, `succ`/`next`, `pred`, plus the block method `times`.
    - **Symbol**: `to_s`, `to_sym`, `length`/`size`, `upcase`, `downcase`,
      `empty?`.
    - **Universal** (every receiver): `nil?`, `==`, `!=`, `class`, `to_s`,
      `itself`.
    - **`Symbol#to_proc`** (`_sir_sym_to_proc`): `&:sym` becomes a `*Closure`
      that re-enters dispatch on its first argument, so `map(&:to_s)` behaves
      exactly like `map { |x| x.to_s }`.
  - **Security (the C3 RCE lesson)**: dispatch is ONLY through the explicit
    catalog switches â€” there is **no reflection** on the raw method name, no
    dynamic Go method/field lookup.  The catalog switch IS the allowlist.  An
    unknown method on a known receiver falls through to `_sir_method_unknown`,
    which panics with a controlled `undefined method '<name>' for <Class>`
    message â€” a surfaced runtime error, never arbitrary behaviour.
  - **Capability gate** (`lib.rs`): a **pure** collection-method module (a
    `__method__` dispatch with NO class features) is now proven accepted.  This
    needs no gate change and no new `Feature` variant (the deferred C1
    `MethodDispatch` is not required): the validator observes no feature for
    `__method__`, so such a module carries only its receiver/argument features
    (`Sequences`/`Strings`/`Closures`/`Symbols`/`Maps`/`DynamicTyping`), all
    already accepted â€” while class-bearing modules stay rejected
    (`Feature::Classes` is not accepted).  The runtime catalog is the real gate.
  - Adds `sort` + `strings` to the emitted import block (the runtime catalog
    always references both).
  - Tests: emitted-shape unit tests (dispatch call shape, block/`&:sym` shapes,
    class-predicate name-string, catalog present in the preamble); acceptance
    tests (pure dispatch accepted, classes still rejected); and an
    **execution-proof** integration test (`compile_and_run_coll_methods.rs`) that
    runs `.map`/`.select`/`.length`/`.reduce`/`.join`/`.sort`/`.reverse`/
    `.upcase`/`.split`/`.even?`/`.abs`/`.keys`/`&:to_s` through real `go run` and
    diffs stdout against the Python/TS reference values, plus a proof that an
    unknown method (`[1].bogus_xyz`) exits non-zero with the controlled
    "undefined method" message.

## Unreleased

### Fixed

- **Reject keyword params mixed with `*rest`/`**kwrest` (unsound static
  resolution).**  KW6 resolves keyword arguments by *static* keywordâ†’positional
  slot mapping, which is only sound for **fixed-arity** callees.  The core
  validator, however, accepts a callee that mixes a `Keyword` param with a
  variadic (its ordering rule is `Required* Rest? Keyword* KwRest?`, so Ruby's
  `def f(a, *rest, x: 1)` is well-formed), and this backend accepts
  `Feature::KeywordParams` â€” so such a module reached `emit_direct_call`, where
  the `*rest` slot has no fixed position for a keyword to resolve against.  The
  result was a **panic** in debug builds (`debug_assert!` in the slot loop) or a
  **silent mis-emit** in release builds (a single `_sir_missing` sentinel landed
  in the variadic slot instead of a collected sequence).  A new capability check
  (`check_no_keyword_rest_mix`, run beside the manifest gate in `compile`) now
  returns a clean `BackendError { kind: UnsupportedFeature }` for any function
  carrying BOTH a `Keyword` param AND a `Rest`/`KwRest` param, naming the
  offending function.  This becomes frontend-reachable once the Ruby frontend
  (KW7) emits keyword+splat methods.  The keyword-params-**without**-rest happy
  path (fixed arity) is unchanged and still passes all existing tests.
  Added unit tests for both the `*rest` and `**kwrest` rejections and for the
  preserved happy path.

## 0.6.0 â€” KW6 keyword parameters & arguments via static positional resolution

Adds `Feature::KeywordParams` to the Go backend's accepted set (see
`code/specs/sir-keyword-params.md`, Â§4 Go row).  Go has **no** native keyword
arguments, so the backend lowers them **directly** â€” no runtime library â€” by
resolving each keyword to a positional slot at *emit time* (a `DirectCall`'s
callee signature is statically known).  This mirrors the Rust backend's
strategy and reuses the SIR19 default-parameter machinery (the `_sir_missing`
sentinel + callee body prologue) unchanged.

### Added

- **Keyword def params are positional-ized.**  A `ParamKind::Keyword` parameter
  emits as an ordinary positional Go parameter in declared order â€” the
  by-name-ness is a source affordance the backend resolves at the call site.
  An *optional* keyword (`Keyword` + `default: Some`) reuses the existing
  default-param prologue: `if _sir_is_missing(name) { name = <default> }`.
- **Static keywordâ†’positional call resolution.**  A `DirectCall` whose `args`
  contain `Expr::KeywordArg{ name, value }` elements is emitted as a plain
  positional Go call, built in the callee's declared param order:
  leading positionals fill leading slots; each `KeywordArg` fills the slot
  whose param **name** matches (source order irrelevant); every omitted
  *optional* slot is padded with `_sir_missing` (the callee prologue supplies
  the default). Worked example â€” `greet(greeting:, name: "world")`:
  `greet(greeting: "hi")` â†’ `greet("hi", _sir_missing)`;
  `greet(name: "ada", greeting: "hi")` â†’ `greet("hi", "ada")`.
- **`FN_PARAMS` signature table.**  A new per-module thread-local mapping each
  function name to its parameter shapes (name, is-keyword, has-default), in
  order, populated by `emit_module` alongside `FN_ARITY`.  The `DirectCall`
  arm consults it to reorder keywords by name â€” `FN_ARITY` alone knows only
  *how many* params, not their names.

### Tests

- Emitted-shape unit tests: positional-ized keyword def with optional-keyword
  default prologue; keyword call reordered to declared order (source order
  scrambled); omitted optional keyword padded with the sentinel; mixed
  positional + keyword call.
- Execution proof (`tests/compile_and_run_keyword_params.rs`): a
  `greet(greeting:, name: "world")` module compiled and run through `go run`,
  asserting `greet(greeting: "hi")` prints `(hi world)` (default filled) and
  `greet(greeting: "hi", name: "ada")` prints `(hi ada)` (supplied). Skips
  gracefully if `go` is absent.

### Deferred (spec Â§Out of scope)

- **Indirect/closure keyword calls.**  An `IndirectCall`/`MakeClosure` cannot
  resolve keywords by name (the callee signature is not statically known); the
  frontends do not emit such calls, so a `KeywordArg` reaching that path
  panics with a documented deferral message rather than mis-emitting.

## 0.5.0 â€” SIR19 default parameters (P2f) via missing-sentinel runtime-mimic

Adds `Feature::DefaultParams` to the Go backend's accepted set.  Go has no
native optional/default parameters and emitted functions are *fixed-arity*
over `Value`, so the backend uses a **runtime-mimic** strategy: a unique
package-level MISSING sentinel flows through the ordinary `Value` channel.

Semantics are **call-time, param-scope**: a default expression is evaluated
each call, in the callee, where the *earlier* parameters are already bound
(so a later default may reference an earlier param â€” `def f(a, b = a + 1)`).

### Added

- **Runtime MISSING sentinel.**  A distinct, otherwise-empty `_missingMarker`
  struct type plus the single shared instance `var _sir_missing Value =
  &_missingMarker{}`.  A program can never construct one itself (no IR node
  lowers to it), so pointer identity makes the new
  `func _sir_is_missing(v Value) bool` predicate exact and total.
- **Caller-side padding.**  A `DirectCall` that omits trailing defaulted
  arguments pads the call up to the callee's full (fixed) param count with
  `_sir_missing`, e.g. `f(5)` for `f(a, b = â€¦)` emits
  `f(Value(int64(5)), _sir_missing)`.  The full param count comes from the
  module's function table (`FN_ARITY`, populated by `emit_module` before any
  body is walked).
- **Callee body prologue.**  Each defaulted parameter gets a guard at the top
  of the function body, in declaration order:
  `if _sir_is_missing(<name>) { <name> = <emitted default expr> }`.  Ordering
  is what makes a later default see an earlier param's already-resolved
  value.  Reassigning a parameter is ordinary Go (parameters are mutable
  locals) and the guard itself "uses" the param, so Go's strict
  unused-variable rule is satisfied even when the body never reads it.

### Changed

- **`_sir_format` / `_sir_value_eq`** defensively handle the sentinel â€” it
  never reaches a print or `=` path in a well-formed program (a defaulted
  param is always replaced before use), but `_sir_format` renders a stray
  sentinel as `<missing>` and `_sir_value_eq` treats two sentinels as equal
  and a sentinel as equal to nothing else, so it can never masquerade as a
  user value.

### Tested

- Unit tests assert the emitted shape: the body prologue (`if
  _sir_is_missing(b) { b = _sir_plus([]Value{a, Value(int64(1))}) }`), that a
  required param emits no guard, and that `DirectCall` padding appends the
  right number of sentinels.  Runtime tests assert the sentinel type, the
  `_sir_is_missing` helper, and the defensive format/eq guards.
- New `go run` integration test (`compile_and_run_default_params.rs`):
  module `f(a, b = a + 1)` returning `b`, `main` prints `f(5)` then
  `f(5, 10)`; the emitted Go is compiled and run under the real Go toolchain
  and stdout is asserted to be `6` then `10` (the default ran and saw
  `a = 5`; a supplied argument suppressed it).  The four existing `go run`
  tests (floats / loops / seq+maps / cyclic) still pass.

### Housekeeping

- Fixed three pre-existing `clippy` lints in `emit.rs` (a `write!`-with-
  trailing-newline, a needless lifetime on `pick_global_set`, and a
  `len() >= 1`) so the crate is clippy-clean under `--all-targets`.

## 0.4.1 â€” harden emitted Go runtime against cyclic Seq/Map

`*Seq`/`*Map` are shared, *mutable* handles, so an emitted Go program can
build a cyclic structure (`xs = [0]; xs[0] = xs`).  Before this release the
emitted runtime walked such values structurally with no cycle protection,
so a cyclic value would make **`_sir_format`** recurse forever and overflow
the stack while printing, and make **`_sir_value_eq`** recurse forever when
comparing two *distinct* cyclic structures (a self-cycle was already short-
circuited by the same-pointer fast path, but distinct cyclic operands were
not).  This mirrors the Rust backend's `0.4.1` cyclic-guard.

This is a robustness fix only â€” the public runtime API and the printed form
of every *non-cyclic* value are byte-identical (all existing tests pass
unchanged).

### Fixed

- **`_sir_format` / `_sir_format_seq` / `_sir_format_map`** now thread a
  visited-pointer set through a new `_sir_format_d(v, visited)` variant.
  The set is a `map[Value]bool` keyed on the Seq/Map **pointer** â€” a
  `*Seq`/`*Map` boxed in the `Value` (`interface{}`) compares by pointer
  identity, the idiomatic Go way to key on handle identity.  A handle is
  inserted on entry and removed on exit, so it is only "seen" along the
  *current* path: a true cycle re-entering a handle within its own subtree
  prints a placeholder (`[...]` for a seq, `{...}` for a map) and returns
  instead of recursing, while a value reached twice by sibling (non-cyclic)
  paths still prints in full.  `_sir_format_pair` threads the set too (a
  pair can hold a cyclic seq/map).  The public `_sir_format(Value) string`
  signature is unchanged â€” it allocates a fresh visited set and delegates.
- **`_sir_value_eq`** keeps the same-pointer (`as == bs`) identity fast
  path and adds a co-inductive `pending` set of handle-pairs currently
  being compared (a `map[[2]Value]bool` keyed on the two boxed pointers)
  via a new `_sir_value_eq_d(a, b, pending)` variant: re-encountering a
  pair already in flight (a cycle matched in lock-step) is treated as
  equal, bounding the deep comparison of two distinct cyclic operands so it
  always terminates.
- **`_sir_map_get` / `_sir_map_set` / `_sir_map_put`** need no
  restructuring: Go has no `RefCell`-style aliasing-borrow check (the Rust
  backend's "already mutably borrowed" panic on a self-referential key has
  no Go analogue), and the remaining hazard â€” a cyclic key making
  `_sir_value_eq` recurse forever â€” is now handled by that function's
  co-inductive guard.  A comment on `_sir_map_put` records this.

### Tests

- New `tests/compile_and_run_cyclic.rs` integration test: hand-builds a
  module that constructs a cyclic seq (`xs = [0]; xs[0] = xs; print(xs)`),
  emits Go, `go run`s it (gated on `go` availability), and asserts the
  program *terminates* and prints the `[[...]]` placeholder.  It also
  checks that `_sir_value_eq` terminates on both a self-cyclic operand (via
  the same-pointer fast path) and two *distinct* cyclic structures (via the
  co-inductive guard), both `#t`.
- Two new runtime unit tests assert the cycle-guard plumbing is present in
  the emitted runtime string (`_sir_format_d` / `_sir_value_eq_d` and the
  placeholder literals).

## 0.4.0 â€” SIR16 Sequences + Maps â€” completes Go v1 parity (A6)

The final two SIR16 (v1) features land in the Go backend.  With them the
Go backend accepts **all six** SIR16 features (Floats, ShortCircuit,
MutableBindings, Loops, Sequences, Maps) â€” reaching **full SIR-v1
parity**.  Go is the **fifth and last backend to reach v1**, completing
the backend fleet (joining TypeScript, Rust, Python, and the others).
Before this release a module using `SeqLit` / `SeqIndex` / `SeqLen` /
`MapLit` / `MapGet` / `SeqSet` / `MapSet` was rejected at the capability
check and those emit arms were unreachable `panic!`s; this release wires
them up end-to-end.

### Added

- `Feature::Sequences` and `Feature::Maps` join the backend's
  `ACCEPTED_FEATURES`, so a module declaring them is no longer rejected
  by the capability check.
- **Sequences** â€” the inlined Go runtime gains a `*Seq` value (a struct
  `Seq{ Items []Value }` held by pointer).  The pointer is the crux: a
  `SeqSet` (`xs[i] = v`) mutates the very sequence the caller holds, and
  two bindings that alias the same literal observe each other's writes â€”
  the reference semantics of a Python list / JS array.  Copying a `Value`
  that holds a `*Seq` copies the handle, not the backing slice.
  - `SeqLit` â†’ `_sir_seq_lit([]Value{...})` builds a fresh shared seq.
  - `SeqIndex` â†’ `_sir_seq_index(seq, i)` (strict bounds; out-of-range
    panics, like `car`/`cdr`).
  - `SeqLen` â†’ `_sir_seq_len(seq)` returns the element count as `int64`.
  - `SeqSet` â†’ `_ = _sir_seq_set(seq, i, v)` mutates in place (no
    auto-grow; out-of-range panics).
- **Maps** â€” the runtime gains a `*Map` value (a struct
  `Map{ Entries []MapEntry }`, an *insertion-ordered* association list).
  Go's native `map` can't key on an arbitrary `Value` (floats, closures,
  nested seqs/maps aren't usable keys), so â€” mirroring the Rust backend â€”
  keys are compared with the runtime's structural value-equality
  (`_sir_value_eq`, a linear scan).  A missing key reads as `nil`.
  - `MapLit` â†’ `_sir_map_lit([]Value{keys...}, []Value{vals...})` (keys
    and values emitted as two parallel slices since Go has no tuple
    literal); last-write-wins on duplicate keys, first-seen order kept.
  - `MapGet` â†’ `_sir_map_get(map, key)` (missing key â‡’ `nil`).
  - `MapSet` â†’ `_ = _sir_map_set(map, key, v)` inserts (appends, order-
    preserving) or overwrites in place.
- **Structural value-equality** â€” `_sir_eq` now routes through a new
  `_sir_value_eq` that handles the whole value tower (numbers cross-type,
  symbols, pairs, and now seqs/maps element-wise / entry-wise, with
  identical-handle short-circuit).  This is the single source of truth
  shared by `=` and map-key lookup.
- **ForEach reconciliation** â€” `_sir_seq_iter` (the A5 cons-list
  flattener used by `ForEach`) now *also* snapshots a real `*Seq`, so
  `for x in [1, 2, 3]` (a `SeqLit`) iterates end to end while
  `ForEach`-over-cons-list keeps working.  A `*Seq` is copied element-wise
  into a fresh `[]Value` so the loop body sees a stable view even if it
  mutates the underlying sequence.
- **Display** â€” `_sir_format` renders a seq as a bracketed list
  (`[1, 2, 3]`) and a map as a brace-wrapped, insertion-ordered entry
  list (`{a: 1, b: 2}`).
- New integration test `tests/compile_and_run_seq_maps.rs` â€” hand-builds
  a module that exercises a sequence (lit/index/len/set + aliasing), a
  map (lit/get/set + missing-key â‡’ nil), and a `for x in [10,20,30]`
  ForEach accumulation; emits Go, `go run`s it (gated on `go`
  availability), and asserts stdout (`99 / 3 / 99 / 2 / 3 / nil / 60`).
  This is the only check that catches Go's `:=`-vs-`=` and
  unused-variable strictness.

### Notes

- `accepts_features` is now in lockstep with emit for **all six** SIR16
  features: every declared feature has a real (non-panicking) emit path.
  The only remaining `panic!` reject arms cover SIR17/18 nodes
  (classes / module-defs / exceptions / `StrConcat`) whose features stay
  unaccepted, so they remain strictly unreachable.

## 0.3.0 â€” SIR16 MutableBindings + Loops (A5)

The next two SIR16 (v1) features land in the Go backend, mirroring the
merged Rust backend equivalent.  Before this release the Go backend
accepted only `Floats` + `ShortCircuit`, so every `Assign` / `While` /
`ForRange` / `ForEach` IR node hit a `panic!` reject arm.  This release
wires up mutation and the three loop forms end-to-end onto Go's native
`for`.

### Added

- `Feature::MutableBindings` and `Feature::Loops` join the backend's
  `ACCEPTED_FEATURES`, so a module declaring them is no longer rejected
  by the capability check.
- **MutableBindings** â€” `Stmt::Assign` to a Local/Param/Capture emits a
  plain `<name> = <value>`.  Go has no const/mut distinction, so unlike
  the Rust backend (which needs a `let mut` pre-pass) reassignment just
  works against the name already declared by the matching `LetBinding`
  (`:=`) or parameter.  A `Global` assignment writes through the runtime
  global store (`_sir_globals[<key>] = <value>`).
- **Loops** â€” `Stmt::While` / `ForRange` / `ForEach` map onto Go's
  native `for`:
  - `While` â†’ `for _sir_truthy(<cond>) { <body> }` (Go's `for` is its
    `while`; the test routes through SIR truthiness, never Go `bool`).
  - `ForRange` â†’ a native three-clause `for` whose `stop`/`step` bounds
    are cached **once** into `int64` temporaries (re-evaluating Python's
    `range` bounds each turn would be wrong).  A direction-aware
    continue test (`_sir_range_cont`) lets a negative `step` count down.
    The loop variable is re-bound each turn as a fresh `Value(int64(â€¦))`
    and guarded with `_ = <var>` so an unused loop var still compiles.
  - `ForEach` â†’ `for _, <var> := range _sir_seq_iter(<iter>)`.  The new
    runtime `_sir_seq_iter` flattens a cons-list (`Pair`-chain ending in
    `nil`) into a `[]Value` (Sequences land in a later PR, so a
    "sequence" is still the classic cons-list).
- Loop bodies emit in statement context: a body's trailing non-`nil`
  value becomes `_ = <value>` (so side effects fire), and introduced
  loop variables get a `_ = <var>` guard â€” satisfying Go's strict
  unused-variable rule even when the body ignores them.
- New runtime helpers `_sir_range_cont` and `_sir_seq_iter`.  (`ForRange`
  reuses the existing `_sir_as_int` from the Floats release for its
  bound extraction.)
- New integration test `tests/compile_and_run_loops.rs` â€” hand-builds a
  module using a mutable accumulator, a `for`-range, and a `while`
  countdown, emits Go, `go run`s it (gated on `go` availability), and
  asserts stdout (`sum 0..5 = 10`, countdown to `0`, reassign to `99`).
  This is the only check that catches Go's `:=`-vs-`=` and
  unused-variable strictness.

### Notes

- Only two SIR16 features remain undeclared (`Sequences`, `Maps`); their
  `SeqLit` / `MapLit` / `SeqSet` / `MapSet` nodes still hit `panic!`
  reject arms, kept strictly unreachable by the capability check until a
  later PR.  `accepts_features` stays in lockstep with emit: every
  declared feature has a real (non-panicking) emit path.

## 0.2.0 â€” SIR16 Floats + ShortCircuit (A4)

First two SIR16 (v1) features land in the Go backend, mirroring the
just-merged Rust backend equivalent.  Before this release the Go backend
declared *none* of the six SIR16 features, so every SIR16 IR node hit a
`panic!` reject arm.  This release wires up two of them end-to-end.

### Added

- `Feature::Floats` and `Feature::ShortCircuit` join the backend's
  `ACCEPTED_FEATURES`, so a module declaring them is no longer rejected
  by the capability check.
- **Floats** â€” the inlined Go runtime's `Value` (`interface{}`) now
  accepts a `float64` arm:
  - New helpers `_sir_as_float`, `_sir_any_float`, `_sir_is_number_val`,
    and `_sir_format_float`.
  - Arithmetic (`+ - * /`) keeps the exact int64 fast-path while every
    operand is an integer, and promotes the whole fold to `float64` the
    moment any operand is a float ("int op float â‡’ float").  Integer
    division keeps its divide-by-zero panic; float division follows
    IEEE-754 (`1.0/0.0 â‡’ +Inf`).
  - `=` is cross-type for numbers (`1 == 1.0` is true) and uses IEEE
    equality for floats (`NaN != NaN`).  `<` / `>` compare numerically,
    staying on the int path when both operands are int64.
  - `number?` is true for both integers and floats.
  - `FloatLit` emits `Value(float64(<lit>))`; integral values spell out
    `3.0` (never `3`) so the runtime type-switch hits the float arm.
    Non-finite values route through `math.NaN()` / `math.Inf(Â±1)` since
    Go has no float literal for them.
  - Display: `_sir_format_float` prints integral floats with a trailing
    `.0` (`3.0`, not Go's default `%v`-style `3`), fractional values via
    `strconv.FormatFloat(x, 'g', -1, 64)`, and non-finite values as
    `NaN` / `inf` / `-inf` â€” matching the Rust backend's intent.
- **ShortCircuit** â€” `LogicalAnd` / `LogicalOr` emit a truthy-guarded
  immediately-invoked func literal:
  `func() Value { __l := <lhs>; if _sir_truthy(__l) { return <rhs> } else { return __l } }()`
  (and the mirror for `or`).  The operand value is returned (not a
  coerced bool), `lhs` is evaluated exactly once, and each IIFE scopes
  its own `__l` so nesting never collides.  Pure emit â€” no runtime
  change.
- The emitter now imports `"math"` (alongside `"fmt"` and `"strconv"`);
  the runtime always references it via the float `NaN`/`Inf` checks, so
  Go's unused-import rule stays satisfied.
- Integration test `tests/compile_and_run_floats.rs`: hand-builds a SIR
  module exercising floats, short-circuit, and cross-type equality;
  emits Go, runs it with `go run`, and asserts stdout
  (`4.0 / 4.0 / 5 / 7 / #f / #t`).  Gated on `go version` â€” skips with a
  log line if the Go toolchain is absent.

### Notes

- The remaining four SIR16 features (MutableBindings, Loops, Sequences,
  Maps) are still **not** declared, so the corresponding emit arms
  (`SeqLit`, `MapLit`, `Assign`, `While`, â€¦) remain reachable only as
  internal-bug `panic!`s â€” the capability check rejects such modules
  before emit.  They land in later Go PRs.

## 0.1.2 â€” SIR18 exhaustiveness (no behaviour change)

semantic-ir 0.10.0 adds `Expr::StrConcat` (the SIR18 string-concat
node).  This backend gains a `StrConcat` arm in its expression emitter
so it stays exhaustive.  The arm joins the existing SIR16+ reject group
and `panic!`s with a "capability check should have rejected it"
message: `Feature::StringInterpolation` is not in this backend's
accepted-feature set, so a concat-using module is rejected at the
capability check before emit, making the arm unreachable.  No output or
accepted-feature changes.

## 0.1.1 â€” SIR17 exhaustiveness (no behaviour change)

semantic-ir 0.2.0 adds `Stmt::ClassDef` (the SIR17 class node).  This
backend gains a `ClassDef` match arm in its statement emitter so it
stays exhaustive.  The arm `panic!`s with a "capability check should
have rejected it" message: `Feature::Classes` is not in this
backend's accepted-feature set, so a class-using module is rejected
at the capability check before emit, making the arm unreachable.  No
output or accepted-feature changes.

## 0.1.0 â€” initial release (SIR15 v0)

Fourth backend for the narrow-waist Semantic IR.  Emits
self-contained Go source from a `semantic_ir::Module`.

### Added

- `GoBackend` implementing `semantic_ir::Backend` with
  `target_tag = "go"`; accepts the v0 feature set minus
  `TailCalls` and `Intrinsics`.
- Per-node lowering per SIR15.  Notable Go-isms:
  - `If` and non-trivial `Block` lower to immediately-invoked
    function expressions (`func() Value { ... }()`) since Go has
    no expression-position blocks.
  - `MakeClosure` emits an adapter `func([]Value) Value` that
    splats the runtime args into the synthesised lambda's
    positional parameters; the per-function arity table is
    threaded through TLS so the splat is sized correctly.
  - `LetBinding` emits `name := value` followed by a defensive
    `_ = name` so unused bindings don't break Go's strict
    unused-variable rule.
  - `ExprStmt` emits `_ = expr` for the same reason.
- Inlined Go runtime (~280 lines) covering `Value` (`interface{}`),
  `Symbol`, `Pair`, `Closure`, all 15 Twig builtins, symbol
  interning, module globals, `_sir_format` and `_sir_truthy` and
  `_sir_apply` and `_sir_make_closure`, plus a `_sir_call_builtin_by_name`
  dispatch table for `VarRef Builtin`.
- Identifier sanitisation handles Go keywords (`for`, `func`,
  `chan`, etc.) and predeclared builtins (`int`, `string`,
  `print`, `len`, etc.) by appending `_`.  Other invalid chars
  encode as `_<hex>`.  Empty â†’ `_sir_empty`.  SIR's `main` is
  renamed to `_sir_user_main` so the emitter's own `main()`
  doesn't collide.
- `sanitize_comment` strips line terminators from external
  strings written into `//` comments â€” same defence as SIR12 /
  SIR13 / SIR14.
- Pre-lowering validation via `semantic_ir::validate`; capability
  check via `Backend::check_module`.

### Notes

- The runtime always imports both `"fmt"` and `"strconv"` â€” both
  are referenced inside the runtime block, so Go's strict
  unused-import rule never fires regardless of what the user
  module uses.
