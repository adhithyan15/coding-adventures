# Changelog

## 0.28.0 — Hash catalog catch-up: `empty?` / `to_a` / `merge` / `dig` / `invert` / `store` / `delete` / `clear` / `reject` / `each_key` / `each_value`

Brings the Rust backend's `map_method` Hash catalog to parity with the
Go/JS/TS/Python `sir-runtime-oop` reference, which already shipped these.  All
dispatch through the same EXPLICIT method-name `match` (never reflection) and
route key comparison through `value_eq`:

- **`empty?`** — `true` iff the hash has no pairs.
- **`to_a`** — an Array of two-element `[key, value]` Arrays, in insertion order.
- **`merge(other)`** — a NEW hash with `other` overlaid on a copy of the receiver;
  on a collision `other` wins while the key holds its first-seen position. A
  non-`Map` argument is ignored.
- **`dig(k, …)`** — a NESTED lookup walking one key per argument, returning `nil`
  the moment a level is missing (never raising). Recurses into a nested `Map`
  (by key) or `Seq` (by integer index, negative-from-end), matching the Go/JS
  backends' nested `dig` (a superset of the single-level Python/TS `dig`).
- **`invert`** — a NEW hash mapping each value back to its key; equal values
  collapse onto one key with the last pair's key at the first-seen position.
- **`store(k, v)` / `[]=`** — MUTATES the receiver (overwrite-in-place or append)
  and returns the value.
- **`delete(k)`** — MUTATES: removes the first entry with a matching key and
  returns its value; a missing key yields `nil`.
- **`clear`** — MUTATES, emptying the receiver, and returns it.
- **`reject { |k, v| … }`** — a NEW hash of the pairs for which the block is
  falsy (the complement of `select`).
- **`each_key { |k| … }` / `each_value { |v| … }`** — yield ONE argument per
  entry and return the receiver.

`responds_to?` now advertises all of the above.

Exec-proof: new `tests/compile_and_run_hash_catalog.rs` compiles and runs (under
real `rustc`) a module exercising every new arm — including a nested `dig`
hit/miss, a `merge` collision, an `invert`, a mutating `delete`/`store`/`clear`,
and `reject`/`each_key`/`each_value` — diffing stdout against the Python
reference semantics.

## 0.27.0 — Hash transforming block methods: `transform_values` / `transform_keys`

Mirrors the Python `sir-runtime-oop` v0.1.18 reference (PR #7909) into the
Rust backend's inline `__sir` runtime (`map_method` + the `Value::Map`
`responds_to?` arm), adding two non-mutating Ruby `Hash` block methods:

- `transform_values { |v| … }` — builds a **new** hash whose keys are copied
  verbatim (so they stay unique and no collision is possible) and whose values
  are the block results.  Yields ONE block argument (the value); insertion order
  is preserved by rebuilding in place.
- `transform_keys { |k| … }` — builds a **new** hash whose values are untouched
  and whose keys are the block results (yields ONE argument, the key).  Two
  source keys can collapse onto one new key; Ruby keeps the **last** colliding
  entry's value while holding the new key at its **first-seen** position, so we
  overwrite an existing slot in place (via `value_eq`) and otherwise append.

Both leave the receiver unmodified.

Exec-proof: new `tests/compile_and_run_hash_transform.rs` compiles and runs
(under real `rustc`) a module exercising `transform_values` ({a:1,b:2} → {a: 99,
b: 99}), an identity `transform_keys` ({a:1,b:2} → {a: 1, b: 2}), and a
**collision** `transform_keys` (constant `:z` key ⇒ {z: 2}), diffing stdout
against the Python/TS reference semantics.

## 0.26.0 — Numeric breadth: `divmod` / `fdiv` / `round(ndigits)` / `clamp` / `between?`

Mirrors the Python `sir-runtime-oop` v0.1.17 reference (and the Go backend
v0.25.0) into the Rust backend's emitted runtime (`numeric_method` +
`responds_to`), adding five Ruby numeric methods:

- `round(ndigits)` — `round` gains an optional digits argument: a positive
  `ndigits` rounds a Float to that many decimals (half **away from zero**, via
  `ruby_round`); `ndigits <= 0` rounds to a power of ten. Rust's `i64`/`f64` are
  FIXED width, so the Python bignum→float `OverflowError` pitfall does not apply
  — the only guards are a place count past i64's ~18 decimal digits (dwarfs the
  value ⇒ `0`, Ruby parity), a positive `ndigits` past Float precision / an
  overflowing scale-up (returns the value unchanged), and an `i64::MAX`/`MIN`
  overflow-degrade in `round_int_to_multiple` (returns the un-rounded value
  rather than a sign-flipped wrap).
- `divmod(n)` — `[quotient, remainder]` with a floored quotient (`floor_div_i64`)
  and the divisor-signed remainder (a `Seq`, so it prints `[3, 1]`); a zero
  divisor raises a typed `ZeroDivisionError`.
- `fdiv(n)` — floating-point division that never panics: a zero divisor yields
  `±Inf`/`NaN` (f64 division already produces these).
- `clamp(min, max)` / `between?(min, max)` — compared numerically.

Dispatch stays an explicit `match` on the interned method name (never
reflection). Exec-proven end-to-end via `rustc` (the numeric exec-proof test now
covers `round(2)`/`round(-2)`, `divmod` incl. the divisor-signed remainder,
`fdiv` incl. the divide-by-zero `Infinity`, `clamp`/`between?`, and the
`i64::MAX.round(-1)` overflow-degrade). Completes the numeric breadth on the Rust
backend.

## 0.25.0 — String justify methods: `ljust` / `rjust` / `center` / `swapcase`

Closes the last String parity gap with the Python/Go/JS/TS runtimes (which
already carry these) by adding four more non-block Ruby String methods to the
emitted runtime's `string_method` `match` and the `responds_to` catalog. All are
**char-based** (`chars().count()` / a rune-cyclic `str_pad`), so a multibyte
receiver and a multibyte pad are never split mid-codepoint:

- `ljust(width, pad = " ")` / `rjust(width, pad = " ")` / `center(width, pad = " ")`
  — pad to `width` **characters** using `pad` cyclically. `width <= the current
  char length` returns the string unchanged; `center` puts any odd extra pad
  char on the **RIGHT** (Ruby's rule). An empty `pad` degrades to a single space
  rather than raising, and the fill count is clamped to a DoS bound
  (`100_000_000`) so a hostile `width` cannot drive an unbounded allocation —
  holding the never-raise floor.
- `swapcase` — flip the case of each ASCII letter, leaving non-letters and
  non-ASCII characters untouched (byte-for-byte identical to the other four
  runtimes).

Dispatch stays an **explicit** `match` on the interned method name (never
reflection over a host method table). Exec-proven end-to-end via `rustc`
(emitted Rust compiled and run; stdout diffed against the Ruby/Python/Go
reference, including the odd-extra-pad-on-the-right `center` case). Completes the
String justify group across all five backends.

## 0.24.0 — String char-set methods: `tr` / `count` / `delete` / `squeeze`

Adds four non-block Ruby String methods to the emitted runtime's `string_method`
`match` and the `responds_to` catalog, mirroring the Python/Go reference
semantics (char-based, so a multibyte receiver is never sliced mid-codepoint):

- `tr(from, to)` — position-wise char translation; a shorter `to` repeats its
  last char, an empty `to` deletes matching chars, and a repeated char in `from`
  keeps the last mapping.
- `count(*sets)` / `delete(*sets)` / `squeeze(*sets)` — char-set methods:
  `count` tallies chars of the receiver in the set, `delete` removes them, and
  `squeeze` collapses consecutive runs (of set chars, or of *all* chars when no
  set is given). Multiple set arguments intersect (Ruby's rule).

Each `set`/`from`/`to` argument is treated **literally** — the range (`"a-z"`)
and negation (`"^abc"`) forms are a follow-up, matching the literal-only
`sub`/`gsub` precedent. Exec-proven end-to-end via `rustc`. Third backend of the
String char-set sweep (Python `sir-runtime-oop` v0.1.16, Go v0.24.0).

## 0.23.0 — slice-selection Array methods: `take` / `drop` / `values_at`

Extends the emitted Rust runtime's `array_method` catalog (and the `Value::Seq`
`respond_to?` table), mirroring the Go backend:

- `take(n)` / `drop(n)` — a fresh Array of the first / all-but-first `n`
  elements; `n` is clamped to `[0, len]` (`n <= 0` and `n > len` both saturate),
  so the slice bounds are always valid. A negative `n` raises `ArgumentError` in
  Ruby; the never-raise floor treats it as `0`.
- `values_at(*idxs)` — a fresh Array of the element at each index, folding a
  negative index from the end once; an out-of-range index yields `nil` (never
  panics).

Verified end-to-end: emitted Rust compiled with `rustc` and executed, output
diffed against the reference.

## 0.22.0 — more Array methods: `zip` / `rotate` / `to_h` / `tally`

Extends the emitted Rust runtime's `array_method` catalog (and the `Value::Seq`
`respond_to?` table) with four more common non-block Array methods:

- `zip(*others)` — Array of tuples `[a[i], b[i], …]`, length = the receiver's;
  a shorter (or non-Array) operand pads with `nil`.
- `rotate(n = 1)` — a fresh Array rotated left by `n` (a negative `n` rotates
  right); the modulo wraps so any `n` terminates and the empty-array early
  return keeps the divisor positive (no divide-by-zero, no negative slice index).
- `to_h` — `[[k, v], …]` → Hash (only 2-element-array elements; others skipped,
  matching the never-raise floor).
- `tally` — Hash of element → occurrence count, first-seen order, keyed by the
  Map's structural `value_eq`.

Also **fills a pre-existing `respond_to?` under-report** for the `Value::Seq`
aggregate methods that already dispatch but were not listed (`min`, `max`,
`sum`, `uniq`, `flatten`, `compact`, `to_a`, `each_with_index`), so the table is
now faithful to `array_method`.

Dispatch stays an explicit `(type, name)` match — no reflection. Verified
end-to-end: emitted Rust compiled with `rustc` and executed, output diffed
against the Python/TS reference.

## 0.21.0 — string/symbol ordering: no-panic `num_lt` comparator

Fixes a reachable panic on the OO surface: the runtime's ordering primitive
`num_lt` (used by the `<`/`>` operators and by `sort`/`min`/`max`/`sort_by`/
`min_by`/`max_by`) fell through to `as_f64` for any non-`Int` pair, which
**panics** on a string, symbol, nil, etc. So a valid Ruby `["b", "a"].sort` or
`"a" < "b"` crashed the emitted program.

`num_lt` now compares strings and symbols **lexicographically**, numbers
numerically, and returns `false` (a stable, defined order — never a panic) for
a genuinely mixed/uncomparable pair. `sort`/`min`/`max`/`sort_by`/… therefore
work on string/symbol arrays, and the `<`/`>` operators no longer crash on a
non-numeric operand. (Ruby raises `ArgumentError` on a genuinely uncomparable
`<`; that typed-error refinement is a separate follow-up — this change removes
the uncontrolled panic.)

Verified end-to-end under `rustc`: `["banana","apple","cherry"].sort` →
`[apple, banana, cherry]`, string `min`/`max`, `"apple" < "banana"` → true, and
a mixed `"apple" < 1` → false (no panic).

## 0.20.0 — Array block-method breadth (sort_by / group_by / partition / …)

Extends the emitted runtime's `array_method` catalog with the common
block-taking Ruby `Enumerable`/`Array` methods that were missing, and grows the
`respond_to?` table to match:

- `sort_by { |x| key }` — key-sorted (Schwartzian: block runs O(n), stable).
- `min_by` / `max_by { |x| key }` — element with the extremal block key
  (first-on-tie; `nil` on empty).
- `group_by { |x| key }` — a Hash of key → Array of elements.
- `partition { |x| pred }` — `[matching, non_matching]`.
- `flat_map` / `collect_concat { |x| … }` — map then splice one level.
- `take_while` / `drop_while { |x| pred }` — the leading truthy run / remainder.
- `count` — block (truthy count), argument (`==` count), or bare (length).
- `each_with_object(memo) { |x, memo| … }` — folds into and returns the memo.

Ordering reuses the runtime's numeric `<` (`num_lt`); a block-less call floors
to the existing `NoMethodError` (Ruby returns an Enumerator, a v0 cut-line).
Verified end-to-end under `rustc`.

## 0.19.0 — source-language display convention: Ruby booleans (`true`/`false`)

First increment of the SIR display-convention spec (`code/specs/sir-display-convention.md`).
A **Ruby**-sourced module now renders booleans as `true`/`false` instead of the
Twig/Lisp `#t`/`#f`, so a translated `puts true` prints `true`.

Mechanism: the runtime carries a compile-time `const SIR_DISPLAY_RUBY` (a
`__SIR_DISPLAY_RUBY__` placeholder); the emitter substitutes `true`/`false`
from `Module.metadata.source_language` (`== "ruby"` → `true`, else `false`).
`format` branches the boolean arm on it. The default is the Lisp form, so all
existing non-Ruby (Twig) output is **byte-for-byte unchanged** (every prior
golden still passes). The branch is a `const`, so it folds at compile time —
zero per-call cost.

Scope: booleans only (the flagship divergence). `nil`, symbols, string
`inspect` quoting, and the Ruby hash `=>` element form remain follow-ups per
the spec's rollout. Verified end-to-end under `rustc`: Ruby source →
`true\nfalse\n`; Twig source → `#t\n#f\n`.

## 0.18.0 — Numeric + String method-catalog parity

Expands the emitted Rust runtime's `numeric_method` and `string_method`
catalogs to Ruby parity, and grows the `respond_to?` tables to match.

**Numeric (`Integer` / `Float`):** `to_int`, `positive?`, `negative?`,
`succ` / `next`, `pred`, `floor`, `ceil`, `round` (banker-free Ruby
round-half-up via `ruby_round`), `gcd` (overflow-saturating `gcd_i64`),
`pow` / `**`, `digits` (`digits_of`), and the block-taking range walkers
`upto` / `downto` / `step`. Counter arithmetic in the range walkers is
`checked_add`/`checked_sub`-guarded so an `i64` boundary can never spin
or panic.

**String:** `capitalize`, `lstrip`, `rstrip`, `chomp`, `chars`, `bytes`,
`start_with?`, `end_with?`, `index`, `replace`, `sub`, `gsub`,
`to_i` / `to_f` (lenient leading-numeric parse via `str_to_i` / `str_to_f`),
`to_sym`, and `empty?`. All arity-guard their optional arguments and
degrade to a typed error rather than panicking.

Dispatch remains receiver-type routed through explicit `match` arms — no
reflection on source-derived method names.

(Consolidates the previously-separate Numeric and String catalog PRs into
one crate change to avoid intra-crate version churn. Note: the `0.16`
Symbol and `0.17` Hash catalog code is already present on `main`; their
CHANGELOG entries were dropped by an earlier bad merge and are tracked
separately.)

## 0.15.0 — Array aggregate / reshape parity (min / max / sum / uniq / flatten / compact / to_a / each_with_index)

Ports the remaining `Array`/`Enumerable` aggregate and reshape methods —
already present in the Python and TypeScript backends (`sir-runtime-oop`) — to
the Rust backend's inlined `__sir` runtime, so a collection program produces
identical output on every backend. Runtime-only change (no core semantic-IR, no
frontend, no emitter change): the frontend already lowers `arr.min`, `arr.uniq`,
`arr.each_with_index { … }`, etc. to the `__method__` dispatch envelope; this
teaches `array_method` to resolve them via new EXPLICIT `match` arms (never
reflection — [[dynamic-dispatch-rce]]).

All eight were ABSENT before this change; each is newly added:

- **`min` / `max`** — element-wise via the runtime's numeric ordering
  (`num_lt`, the same source of truth `sort`/`<` use); an empty array yields
  `nil`. A stable left-fold keeps the first element on a tie. The vector is
  snapshotted before folding so no `RefCell` borrow is held across the scan.
- **`sum`** — numeric fold seeded at `0` (or the explicit `sum(init)` seed arg,
  matching the Python/TS reference). Each step reuses `plus`, so integer-only
  inputs stay `Int` while any float promotes to `Float`; `[].sum == 0`.
- **`uniq`** — first-occurrence-order de-duplication using the runtime's
  structural `value_eq` (so `[1, 1.0]` collapses, as do equal nested arrays),
  into a fresh `Vec`.
- **`flatten`** — recursively splices nested `Seq`s into one freshly-allocated
  flat `Seq`. **CYCLE GUARD:** a `visited` set of seq handle-addresses (the
  same discipline `puts`/`format`/`value_eq` use) bounds the walk so a
  self-referential array terminates; every level snapshots its items — dropping
  the `RefCell` borrow — BEFORE recursing, so no borrow is ever held across a
  re-entrant call and no input handle is aliased into the result.
- **`compact`** — a fresh `Seq` with every `nil` removed.
- **`to_a`** — returns the receiver itself (Ruby `Array#to_a` identity).
- **`each_with_index`** — yields `(element, index)` to the block via
  `apply_closure` and returns the receiver; each element is cloned out of the
  snapshot BEFORE the block runs, so no `RefCell` borrow is held across the
  (re-entrant) closure call.
- New `compile_and_run_array_aggregates` exec proof: hand-builds SIR for
  `[3,1,2].max`/`.min`, `[1,2,3].sum`, `[1,2,2,3].uniq`, `[[1,[2]],3].flatten`,
  `[1,nil,2].compact`, `[1,2,3].to_a`, and `[10,20].each_with_index { |x,i| … }`,
  emits Rust, compiles it with `rustc`, and diffs stdout against the values the
  Python/TS reference produces for the same module.

## 0.14.0 — M6 universal metaprogramming surface (send / tap / then / respond_to?)

Ports the **M6** universal `Object`/`Kernel` metaprogramming surface — already
merged in the Python and TypeScript backends (`sir-runtime-oop`) — to the Rust
backend's inlined `__sir` runtime, so a metaprogramming program produces
identical output on every backend. Runtime-only change (no core semantic-IR, no
frontend, no emitter change): the frontend already lowers `recv.send(:m, …)`,
`recv.tap { … }`, etc. to the `__method__` dispatch envelope; this milestone
teaches `call_method` to resolve them.

- **`send` / `__send__` / `public_send`** — the first argument NAMES a method;
  dispatch RE-ENTERS `call_method` with that name plus the remaining args (a
  trailing block survives as a trailing arg). Placed first so it applies to
  EVERY receiver kind (primitive, collection, user instance) uniformly. A
  user-defined `send` override on an instance wins (resolution order).
  - **SECURITY ([[dynamic-dispatch-rce]]):** the dynamic name feeds back
    through the SAME explicit, closed `call_method` a direct `recv.meth` call
    takes — it indexes the identical hand-written catalogs / `METHOD_TABLE`, so
    an unknown name bottoms out at the identical typed `NoMethodError`. There is
    NO reflective lookup on the source-derived string; the name is inert data
    that can only ever select an arm we spelled out.
- **`tap { |x| … }`** — yields the receiver for a side effect, returns the
  RECEIVER. **`then` / `yield_self { |x| … }`** — yields the receiver, returns
  the BLOCK RESULT (block-less → the receiver, the v0 Enumerator-less floor).
- **`respond_to?(:m)`** — true iff dispatch on the receiver resolves `m`,
  consulting the SAME catalogs / user method table a real call walks (honest: a
  `true` name is exactly one a real call would run; a `false` name is exactly
  one that would raise `NoMethodError`). An explicit membership test, never
  reflection. On a user instance it uses the same `resolve_instance_method`
  ancestry/MRO walk `dispatch_user_method` uses.
- **Boolean `&` / `|` / `^`** on a `true`/`false` receiver — Ruby's EAGER
  (non-short-circuiting) logical operators, distinct from the lazy `&&`/`||`
  keywords; the operand is coerced by Ruby truthiness (`true & nil == false`).
- `dispatch_user_method` now falls through to the M6 universal methods on a
  user-method miss (instead of raising immediately), so `instance.respond_to?` /
  `.tap` / `.then` / `.to_s` resolve on instances too; only a name none of these
  claim is a genuine `NoMethodError`.
- New `compile_and_run_metaprogramming` exec proof (6 tests): `send` dispatches
  through the catalog (primitive + collection + user instance), unknown `send`
  raises a catchable `NoMethodError`, `tap` returns the receiver while `then`/
  `yield_self` return the block result, `respond_to?` reports catalog membership
  honestly, and boolean `&`/`|`/`^` match Ruby. Plus a `runtime.rs` unit-test
  witness for the emitted M6 surface. All 123 lib + exec tests green, no new
  warnings.


## 0.13.2

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


## 0.13.1

### Fixed — `case_eq` builtin (Ruby case-equality `===`) was unimplemented

Ruby's `case`/`when` (and `case`/`in`) lowers, in the frontend, to a chain of
`if`s whose conditions are `BuiltinCall("case_eq", [pattern, scrutinee])`. This
backend's runtime never implemented `case_eq`, so **every** `case` program hit
`call_builtin_by_name`'s `unknown builtin` floor and **panicked at runtime** —
`case` was unusable on the Rust backend (no compile-time gate catches a missing
builtin; only execution does).

- Added `pub fn case_eq(pattern, value) -> Value` to the inlined `__sir` runtime
  and wired it into both the emitter's helper table and `call_builtin_by_name`.
  Ruby keys `===` to the *pattern*'s type (Range → membership, Regexp → match,
  else `==`); `when SomeClass` is lowered to `value.is_a?` at the frontend and
  never reaches here. This backend's `Value` has no `Range`/`Regexp` variant yet,
  so `case_eq` is exactly structural equality (`value_eq`), matching the Python
  reference in `sir-runtime-oop`; extend with membership/match arms later.
- New `compile_and_run_case_eq` exec proof: a `when`-style `if case_eq(…)` chain
  emits Rust, compiles with `rustc`, runs, and matches the expected output.


## 0.13.0 — mixins: include / extend module method resolution (MX6)

Implements the Rust milestone (**MX6**) of the **sir-mixins** cascade — the
LAST of the five backends (Python/TS/JS/Go already merged). A translated Ruby
`module M … end` mixed into a class with `include M` / `extend M` must now
resolve `M`'s methods the way Ruby does. Previously a module method mixed into a
class was **not found** — the call fell through to `NoMethodError`. This is a
**runtime-only** change (no core semantic-IR, no frontend change): the MX1
frontend already lowers `module`/`include`/`extend` to the builtins
`__def_method__("M", …)` (module owner), `__include__("Owner","M")`,
`__extend__("Owner","M")`, and `__class_method__("Owner","m",args…)`; this
milestone teaches the Rust backend's inlined `__sir` OOP runtime to consult
included modules during method resolution and to expose extended module methods
as class methods.

### Ruby MRO (Method Resolution Order) — the exact linearisation

For a receiver of class `C`, instance-method resolution now walks:

```text
  C  →  C's included modules (REVERSE / most-recent-first, depth-first
        through each module's own includes)  →  C's superclass  →
        its included modules  →  …  →  Object
```

- A class's **own** method SHADOWS an included module's (class searched first).
- A module method SHADOWS the **superclass**'s (a module precedes the
  superclass in the ancestor list).
- A **diamond** include (a module reached via two paths / included twice)
  resolves **once**, at its earliest position — a shared `seen` set skips an
  owner already visited.
- The walk is **total**: a self-including module (`module M; include M; end`) or
  a cyclic class hierarchy (`A < B < A`) TERMINATES (the `seen` guard), raising
  a catchable `NoMethodError` rather than hanging.

### Changed (all in the inlined `RUNTIME` string, `runtime.rs`)

- **New `INCLUDED_MODULES` table** — a per-owner `HashMap<String, Vec<String>>`
  appended in source (include) order. An owner is a class OR a module name, so a
  module that itself `include`s another module contributes its includes when the
  walk recurses into it (transitive mixin inclusion). Keyed by source-derived
  NAMES with no reflection (the C3 RCE discipline).
- **`include_module` / `extend_module`.** `__include__` appends to the owner's
  include list. `__extend__` snapshots the module's instance-method names (via
  `module_method_names`, the same include-MRO walk) and copies each into the
  owner's **class-method** table, so they become callable as `Owner.method`; an
  entry the owner already defines is NOT overwritten (own class method shadows).
- **`resolve_method` → `resolve_instance_method` (MRO-aware).** The old
  ancestry-only instance resolver is replaced by the full MRO walk above,
  `seen`-guarded. Its callers — `dispatch_user_method`, `call_new` (the
  `initialize` lookup), and `call_super` (now resolves from the superclass
  through ITS full MRO) — were updated. A dedicated `resolve_class_method`
  retains the plain class-method-table ancestry walk.
- **`call_class_method` (closes #61 for Rust).** New dispatcher for
  `__class_method__("Owner","m",args…)` (`Owner.method`): resolves `m` in the
  owner's class-method table walking ancestry, INCLUDING methods `extend` copied
  in. An unresolved name raises a typed `NoMethodError` (catchable by
  `rescue NoMethodError`), never a reflective fallthrough. This dispatch arm was
  required to prove `extend` and mirrors the four merged backends.

### Emit (`emit.rs`) + acceptance (`lib.rs`)

- Three new `BuiltinCall` emit arms mirroring the existing `__new__`/`__super__`
  /`__def_method__` routing: `__include__` → `include_module`, `__extend__` →
  `extend_module`, `__class_method__` → `call_class_method`. Owner/module/method
  NAMEs emit through the existing `emit_oop_name_arg` (a `StrLit` or a `Const`
  VarRef becomes a Rust `&str` literal — the same lifting that keeps
  `Feature::Constants` sound); call ARGS use the ordinary `emit_expr` path.
- `reject_const_ref` gains a `__class_method__` arm that skips the owner (arg 0)
  and method-name (arg 1) NAME slots (a `Const` owner like `Registry.total` is
  lifted, not read) while still scanning the call args. `Feature::Modules` was
  already accepted.

### Tests

- New exec-proof integration test `compile_and_run_mixins.rs` (builds SIR, emits
  Rust, compiles with `rustc`, runs, asserts stdout): (a) an included module's
  method is reachable on an instance; (b) a class method shadows the module's;
  (c) a module method shadows the superclass's AND a diamond include resolves
  once (terminates); (d) `extend` makes a module method a class method; (e) a
  self-including module TERMINATES (catchable `NoMethodError`, no hang); plus a
  bonus proving a mixed-in method runs on the same self and reads its `@ivars`.
- New emit unit tests for the three routing shapes (incl. `Const`-owner lifting
  and call-arg passthrough) and a `runtime_declares_mixin_helpers` runtime test
  asserting the table + helpers + the reverse most-recent-first walk are shipped.

## 0.12.0 — typed runtime errors: ZeroDivision / Index / Key / NoMethod (T5)

Implements the Rust milestone (**T5**) of the **sir-typed-runtime-errors**
cascade. A translated `begin/rescue <Class>` that rescues a *runtime* fault —
`ZeroDivisionError`, `IndexError`, `KeyError`, `NoMethodError` — must now catch
it, matching Ruby (and the other four backends). Previously these faults were
either an uncatchable host `panic!` (division by zero) or a silent `nil` floor
(unknown method), so `rescue ZeroDivisionError` / `rescue NoMethodError` missed
them entirely. This is a **runtime-only** change (no core semantic-IR, no
frontend change): the faulting operations now raise the correct typed
`SirError` via the existing `raise` entry point (`panic_any(SirError{…})`), so
the existing `catch_unwind` + `rescue_matches` ancestry machinery dispatches
them to the right clause.

The typed-raise is explicit-string (a fixed class name + a data-only message);
there is no reflection over runtime type names — the same C3-allowlist
discipline the dispatch catalog already upholds. Genuine **non-`SirError` host
faults** (a real codegen/translator bug — an `unwrap` on `None`, an internal
`panic!`) are still **re-raised uncaught** by `exc_from_payload`
(`resume_unwind`), never swallowed by a bare `rescue`: only a value a `raise`
produced can ever be caught.

### Precise Ruby semantics (load-bearing — no over-raise)

| operation (Ruby)        | before                     | after (T5)                       |
|-------------------------|----------------------------|----------------------------------|
| `1 / 0`, `1.0 / 0`      | `panic!` (uncatchable)     | `ZeroDivisionError` ("divided by 0") |
| `arr.fetch(oob)`        | (no `fetch`)               | `IndexError`                     |
| `hash.fetch(miss)`      | (no `fetch`)               | `KeyError`                       |
| `arr.fetch(i, default)` | —                          | returns `default` (no raise)     |
| `obj.undefined`         | silent `nil` floor         | `NoMethodError` (`undefined method 'x' for <Class>`) |
| `arr[oob]`, `hash[miss]`| `nil` (arr[] via floor)    | `nil` (unchanged — explicit `[]` arm) |

### Changed (all in the inlined `RUNTIME` string, `runtime.rs`)

- **`divide`.** A zero divisor on the int path OR the float path now raises a
  typed `ZeroDivisionError` (was an int-only uncatchable `panic!`; the float
  path previously returned IEEE `inf`). Ruby raises for both `1/0` and `1.0/0`.
- **`array_method`.** Added `Array#fetch` (strict indexed read — `IndexError`
  out of bounds, honours a negative index and a supplied default) and
  `Array#[]` (lenient read — `nil` out of bounds, so `arr[i]` never reaches the
  new `NoMethodError` floor).
- **`map_method`.** Added `Hash#fetch` (strict keyed read — `KeyError` on a
  missing key, or a supplied default) and `Hash#[]` (lenient — `nil` on miss).
- **Unknown-method floor split into two helpers.** `unknown_method` remains the
  `nil` floor for a KNOWN method used block-less (`map`/`select`/`reduce`
  without a block — Ruby returns an `Enumerator`, we floor to `nil`) and for the
  defensive receiver-type guards. A new `no_method_error` raises a typed
  `NoMethodError` for a genuinely unknown method name; a new `ruby_class_name`
  renders the receiver's Ruby class for the message (parity port of the Go
  backend's `_sir_ruby_class_name`). The true catalog fall-throughs
  (Array/Map/String/Numeric/Symbol `_` arms, `call_method`'s Bool/default arms,
  and `dispatch_user_method`'s unresolved-instance-method arm) now raise via
  `no_method_error`.

### Tests

- New exec-proof integration test `compile_and_run_typed_runtime_errors.rs`:
  builds SIR, emits Rust, compiles with `rustc`, runs, and asserts a
  `begin/rescue` catches `1/0`→`ZeroDivisionError`, `1.0/0`→`ZeroDivisionError`,
  `arr.fetch(oob)`→`IndexError`, `hash.fetch(miss)`→`KeyError`,
  `obj.undefined`→`NoMethodError` (and its superclass `NameError`), that
  `fetch` with a default does not raise, and that `arr[oob]` / `hash[miss]`
  still return `nil` (no over-raise).
- Updated `compile_and_run_collection_methods.rs` (the `[1].bogus_xyz` case
  moved to the new test — it now raises rather than flooring to `nil`) and
  `compile_and_run_oop.rs` (the security "`drop` is inert data" and cyclic-
  ancestry-terminates cases now `rescue NoMethodError` and prove the surfaced
  typed error is CONTROLLED — never a host `Drop`/reflection, never a hang).

## 0.11.0 — polymorphic `+` / `*` for strings and arrays (PO5)

Implements the Rust milestone of the **sir-polymorphic-operators** cascade.
Ruby's `+` and `*` are overloaded by receiver type, but the Rust backend's
runtime `plus`/`times` were **numeric-only**: they called `as_i64`/`as_f64`
on every operand, so `"a" + "b"` produced integer garbage (a correctness
gap on the core translation path, not a missing stdlib method). Both
helpers are now **type-polymorphic**, matching Ruby exactly. This is a
**runtime-only** change — no core semantic-IR change and no frontend change;
`+`/`*` already lower to `__sir::plus`/`__sir::times`.

**Overflow guard (security):** the `*` repeat arms compute `len * count` where
`count` is cast from a program-controlled `i64`. Both arms now guard the product
with `checked_mul` and panic `"argument too big"` (matching Ruby's
`ArgumentError`) on overflow, and the Seq-repeat arm short-circuits an empty
receiver (also avoiding a huge `0..count` loop) — closing a reachable
overflow/OOM vector before any `str::repeat`/`Vec::with_capacity`.

### Semantics (dispatched on the FIRST operand's tag)

| expression      | result      | arm                        |
|-----------------|-------------|----------------------------|
| `"a" + "b"`     | `"ab"`      | `Str` → concat all args    |
| `[1] + [2]`     | `[1, 2]`    | `Seq` → new concatenated   |
| `"ab" * 3`      | `"ababab"`  | `Str * Int` → repeat       |
| `[0] * 3`       | `[0, 0, 0]` | `Seq * Int` → repeat       |
| `[1, 2] * ", "` | `"1, 2"`    | `Seq * Str` → join         |
| `1 + 2`         | `3`         | numeric fold (**unchanged**) |
| `2 * 3`         | `6`         | numeric fold (**unchanged**) |

### Changed

- **`plus` (`runtime.rs`).** Dispatches on `args.first()` via an explicit
  `match`:
  - `Value::Str` first operand → concatenate every operand's string
    contents into a new `Value::Str`. Each operand must be a `Str` (a
    non-`Str` operand panics with `string + expects strings, …` rather than
    silently coercing; typed `TypeError` on `"a" + 1` is deferred to the
    sir-typed-runtime-errors cascade).
  - `Value::Seq` first operand → concatenate the element vectors into a
    **fresh** `Value::Seq` (via `seq_lit`), never aliasing or mutating an
    input handle (Ruby `Array#+` returns a new array).
  - Otherwise → the **unchanged** numeric int/float promotion fold.
- **`times` (`runtime.rs`).** Dispatches on `args.first()`; a `Str`/`Seq`
  first operand folds left-associatively pairwise through a new
  `times_binary(lhs, rhs)` atom (Ruby `*` is binary; the SIR builtin is
  variadic, so this preserves the variadic contract), with three arms:
  - `Str * Int` → repeat the string N times (N ≤ 0 → empty string).
  - `Seq * Int` → a fresh `Seq` with the element vector repeated N times
    (N ≤ 0 → empty), never aliasing the input.
  - `Seq * Str` → join the elements with the separator, returning a
    `Value::Str` (elements rendered via the same `format` display `Array#join`
    uses).
  - Any other first operand → the **unchanged** numeric fold.

### Security

- Dispatch is an **explicit `match` on the runtime tag** of the first
  operand — never a reflective / name-indexed lookup (the
  dynamic-dispatch-RCE lesson). The string/array arms are hand-written; a
  first operand that is neither `Str` nor `Seq` takes exactly the pre-existing
  numeric path. No new `unsafe`.

### Tests

- **Execution proof** (`tests/compile_and_run_polymorphic_ops.rs`, gated on
  `SIR_TEST_RUSTC_LINKER`): a single suite that hand-builds a
  `puts`/`print (<lhs> <op> <rhs>)` SIR module per case, emits Rust, compiles
  with `rustc`, runs the binary, and asserts stdout against the Ruby
  reference — `"a"+"b"→ab`, `"ab"*3→ababab`, `[1]+[2]→[1, 2]` (bracketed via
  `print`/`format`), `[0]*3→[0, 0, 0]`, `[1,2]*", "→1, 2`, plus regressions
  `1+2→3` and `2*3→6` proving the numeric path is unchanged.
- **Runtime-shape unit test** (`runtime.rs`) pinning the new polymorphic arms
  (`string + expects strings`, `array + expects arrays`, `fn times_binary`
  with the three `(lhs, rhs)` arms) and the `match args.first()` tag dispatch
  (no reflection).

### Notes

- No `Feature` variant added and no accepted-feature change: `+`/`*` were
  already accepted and lowering; this only makes their **string/array** cases
  correct instead of producing numeric garbage. Every existing test passes
  unchanged.

## 0.10.0 — `puts` builtin (Ruby semantics)

### Added

- The Rust backend now emits and executes Ruby's `puts`, the most common
  output method. `puts` maps to a new **variadic** runtime helper
  `__sir::puts(vec![…])` (routed both by the emit helper table and
  `call_builtin_by_name`), reusing `__sir::format` for element rendering.
- Ruby semantics implemented exactly: no-arg → one newline; `puts x` →
  `x.to_s` + newline (no double newline when the text already ends in `"\n"`);
  `puts a, b` → one line per arg; `puts []` → a single newline; a
  `Value::Seq` is flattened recursively, one **element** per line; `puts nil`
  → a blank line.
- Execution proof `compile_and_run_puts.rs` compiles `puts "hello"; puts;
  puts [1,2,3]` with `rustc`, runs it, and asserts stdout is exactly
  `hello\n\n1\n2\n3\n` (the Ruby reference output).

### Security — cycle-guard the `puts` array flatten (CWE-674)

- `__sir::puts_one` flattened arrays by recursing per element with **no
  bound**. A `Value::Seq` is a shared, mutable `Rc<RefCell<..>>` handle, so a
  translated program can build a self-referential array
  (`a = []; a << a; puts a`) or a pathologically deep one; the unguarded
  recursion overflowed the native stack and aborted — a denial of service
  (uncontrolled recursion). The flatten now threads a `visited` set of the `Rc`
  handle addresses on the active path (the same `seq_handle_id` key
  `__sir::format` uses): a handle re-encountered within its own subtree is a
  cycle and renders as Ruby's `[...]` placeholder + newline instead of
  recursing, so `puts a` on a self-referential array now **terminates** exactly
  as real Ruby does. Non-cyclic output is byte-for-byte unchanged
  (`puts [1,[2,3]]` → `1\n2\n3\n`); a new regression test
  (`puts_cyclic_array_terminates`) proves the self-referential case exits
  cleanly with `[...]\n`.

## 0.9.0 — user-defined class OOP runtime + emit (O5)

Makes the Rust backend **accept and execute** real user-defined-class OOP
(`Foo.new`, `initialize`, instance/class methods, `super`, `self`, `@ivar`,
`@@cvar`, inheritance) — the Rust analogue of the O1/O3/O4 backends. Before
this change `Feature::Classes` was accepted ONLY for empty-body
exception-subclass declarations, and `@ivar`/`@@cvar`/`self`/`new`/`super`
had no runtime; a real OO program was rejected or hit a `panic!` guard.

### Value-model decision (variant vs. side-table)

A **narrow, dedicated `Value::Instance(u64)` variant backed by a
`thread_local` side-table**. The `u64` is an opaque instance id; the object
state (`SirInstance { class, ivars }`) lives in the `INSTANCES` side-table
keyed by that id. This is a *hybrid* of the two options the milestone
weighed:

- A side-table alone (reusing a magic `Pair`/`Sym` as a disguised handle)
  would **leak**: `pair?`/`car`/`cdr` would operate on an "instance" and
  `format`/`value_eq` would mis-render it.
- Storing `SirInstance` **inline** (`Instance(Rc<SirInstance>)`) would put a
  `RefCell<HashMap>` on the hot, frequently-cloned `Value`.

The id-handle-plus-side-table keeps `Value: Clone` a trivial `u64` copy,
gives instances a *distinct* discriminator (no built-in-type leak, correct
`format`/`value_eq`), and confines mutable object state to one
`thread_local`. Adding the arm touches ONLY this backend's emitted-runtime
`Value` — never the core semantic-IR — and only two existing exhaustive
sites (`format_d`; an identity arm in `value_eq_d`); every other `match`
already has a `_`/`matches!` fallback.

### Added

- **Runtime (`runtime.rs`).** A user-defined-class OOP model in the inline
  `__sir` module, reusing the exception runtime's `seen`-guarded ancestry
  walk (`super_of`/`is_ancestor_or_self`):
  - `Value::Instance(u64)` + `SirInstance { class, ivars: RefCell<HashMap> }`
    in the `INSTANCES` side-table; `new_instance(cls)` allocates a fresh
    handle.
  - `METHOD_TABLE` / `CLASS_METHOD_TABLE` — `HashMap<(String, String),
    Value>` keyed by the `(class, method)` pair (the `Value` is the
    method-body `Closure`). `def_method`/`def_class_method` populate them.
  - `call_new(cls, args)` — allocate → run the inherited `initialize`
    (ancestry-resolved, `seen`-guarded) with `self` bound → return the
    instance (Ruby discards `initialize`'s result). `call_super(method,
    cls, args)` — resolve from the superclass of `cls`, reuse the live
    `self`. `call_method` gains a **user-instance branch, taken FIRST**,
    that resolves the user table walking ancestry; **every other receiver
    keeps the unchanged collection/built-in path.**
  - `current_self()` (`__self__`); `ivar_get`/`ivar_set` and
    `cvar_get`/`cvar_set` acting on the current self (per-class cvar bags).
  - **RAII self-stack:** a `SelfGuard` whose `Drop` pops the self-stack, so
    a panic mid-method (a SIR `raise` unwinds as a panic) still balances the
    stack — the Rust analogue of the JS runtime's `try { … } finally {
    popSelf(); }`.
  - **SECURITY:** every lookup is an EXPLICIT `HashMap::get` on a `(class,
    method)` key — never reflection/`dyn Any`-by-name. A class/method named
    `constructor`/`new`/`drop` is inert data; a miss floors to the honest
    `Nil`/NoMethodError boundary the collection catalog uses. The ancestry
    walk carries a `seen`-set cycle guard so a cyclic hierarchy terminates.
- **Emit (`emit.rs`).** Emit arms mirroring `__method__`→`call_method`:
  `__new__`→`call_new`, `__super__`→`call_super`,
  `__def_method__`→`def_method`, `__def_class_method__`→`def_class_method`,
  `__self__`→`current_self`. Class/method NAME args (a `StrLit` or a `Const`
  VarRef like `Dog.new`) are LIFTED to `&str` string literals via
  `emit_oop_name_arg` (never a runtime constant read). `@ivar`/`@@cvar`
  reads route to `ivar_get`/`cvar_get`, writes to `ivar_set`/`cvar_set`
  (both statement and inline contexts). The user `subclass → superclass`
  ancestry registration now fires for `Feature::Classes` too (not only
  `Exceptions`), so the OOP resolver's shared ancestry table is populated.
- **Feature acceptance (`lib.rs`).** `ACCEPTED_FEATURES` now includes
  `Modules`, `InstanceVars`, `ClassVars`, and widens the `Classes`/`Constants`
  rationale to real OOP. `reject_const_ref` skips the class-name slots of
  `__new__`/`__super__` (lifted to strings), keeping `Constants` acceptance
  sound. `reject_stateful_class` still rejects a class/module with an
  executable body (methods hoist to top-level functions, so an accepted
  class body is empty) — the soundness gate is unchanged.

### Tests

- **Emit-shape unit tests** (`emit.rs`) for `__new__`/`__super__`/
  `__def_method__`/`__def_class_method__`/`__self__` and `@ivar`/`@@cvar`
  read+write routing. **Runtime-shape unit tests** (`runtime.rs`) pinning
  the `Instance` variant, the tables, the explicit-lookup + cycle-guard, and
  the RAII self-guard.
- **Execution proof through `rustc`** (`tests/compile_and_run_oop.rs`, gated
  on `SIR_TEST_RUSTC_LINKER`): P1 `Dog#initialize`/`speak` (ivar-through-
  method dispatch → `42`); P2 inheritance + `super` (`Cat.new(4).describe` →
  `104`); a security test (`constructor` class + unregistered `drop` → clean
  data / `nil` floor); cyclic ancestry terminates (`A<B<A` → `nil`); and a
  self-stack-balanced check.

### Notes

- No new `unsafe`. No core semantic-IR change (the `Instance` arm is the
  backend's emitted-runtime `Value` only). No new clippy warnings on touched
  files.

## 0.8.0 — exception handling via catch_unwind + ancestry (E4)

Makes the Rust backend **accept and execute** structured exceptions
(`Feature::Exceptions`). Before this change `Stmt::TryCatch` and the
`raise` builtin hit `panic!` guards in `emit.rs` (the feature was not in
`ACCEPTED_FEATURES`), so any `begin/rescue/ensure` module was rejected.
Rust has no native exceptions, so v0 maps Ruby's exception model onto
Rust's **unwinding panic** machinery — a *localized* transform touching
only the `raise`/`TryCatch` arms; every other emit path is unchanged.

### Added

- **Runtime (`runtime.rs`).** An exception model in the inline `__sir`
  module:
  - `SirError { class: String, msg: String }` — the panic payload. `msg`
    is a `String` (not `Value`) because `std::panic::panic_any<M>` requires
    `M: Send`, and our `Rc`-based `Value` is not `Send`; the message is
    rendered at raise time, matching Ruby's string `exception.message`.
  - `raise(class, msg: Value) -> !` → `std::panic::panic_any(SirError{…})`;
    `reraise() -> !` for a bare `raise`.
  - `exc_from_payload(Box<dyn Any + Send>) -> SirError` — downcasts the
    caught payload to a `SirError`, or **`resume_unwind`s** a non-`SirError`
    payload (a genuine Rust panic is never swallowed as a rescuable
    exception).
  - `rescue_matches(&SirError, &[&str]) -> bool` over an **explicit**
    built-in ancestry table (a verbatim parity port of the TS
    `sir-runtime-exceptions` `ANCESTRY`) merged with user edges, with a
    `seen`-set **cycle guard**. `exc_value(&SirError) -> Value` re-wraps the
    message for a `rescue … => e` binding.
  - `register_ancestry(&[(&str, &str)])` — the ONLY channel for user
    ancestry edges (no reflection). `install_panic_hook()` quiets Rust's
    default panic banner for `SirError` payloads; `report_uncaught` renders
    an unrescued exception (`Class: message`) and exits non-zero.
- **Emit (`emit.rs`).**
  - `raise` arm: a `Const` class name (`raise Foo`/`raise Foo, "m"`) is
    **lifted to a string literal** (never emitted as a runtime `Const`
    read); a non-const first arg → `raise("RuntimeError", <arg>)`; bare
    `raise` → `reraise()`.
  - `Stmt::TryCatch` → a `std::panic::catch_unwind(AssertUnwindSafe(||
    {…}))` region whose `match` dispatches rescue clauses in order via
    `rescue_matches`, binds `=> e` with `exc_value`, and **runs `ensure` on
    every path** (Ok, matched, and unmatched-before-`resume_unwind`).
  - `main` wraps the user body in a top-level `catch_unwind` so an uncaught
    SIR exception exits cleanly non-zero; the module's `ClassDef` ancestry
    edges are registered once at init (`register_ancestry`), and the quiet
    panic hook is installed.
  - `Stmt::ClassDef` (empty-body exception subclass) emits no runtime code —
    it is pure ancestry metadata.

### Accepted features

- `Feature::Exceptions`, plus `Feature::Classes` and `Feature::Constants`
  **for the narrow exception use case only**:
  - `Classes` — an empty-body exception-subclass declaration `class MyErr <
    StandardError; end` (methods hoist to top-level `Function`s). A
    **non-empty** class body is rejected cleanly by `reject_stateful_class`.
  - `Constants` — a `raise MyErr` names its class via a `Scope::Const`
    VarRef (lifted to a string). Any **other** `Const` reference is rejected
    cleanly by `reject_const_ref`, keeping the acceptance sound (no
    `emit_var_ref` `Const` panic on validated input).

### Tests

- Emitted-shape unit tests (`emit.rs`): `raise` variants, `TryCatch` →
  `catch_unwind`/`match`, ensure-on-all-paths, empty ClassDef.
- Capability-gate unit tests (`lib.rs`): accept exceptions/subclass, reject
  stateful class, reject non-raise const ref, allow raise-class-name const.
- Runtime-shape tests (`runtime.rs`): exception helpers present, explicit
  table + cycle guard, non-`SirError` passthrough.
- Execution-proof through `rustc` (`tests/compile_and_run_exceptions.rs`,
  gated on `SIR_TEST_RUSTC_LINKER`): (a) typed rescue via built-in ancestry,
  (b) bare rescue, (c) unmatched re-raise exits non-zero, (d) ensure runs on
  caught + uncaught, (e) user ancestry `MyErr < StandardError` caught by
  `rescue StandardError`.

### Security

- Rescue matching is an **explicit ancestry-table lookup** — never
  reflection / type-name introspection. A `seen`-set cycle guard bounds the
  ancestry walk. A non-`SirError` panic (a real Rust bug) is `resume_unwind`
  ed, never mis-dispatched to a rescue. `AssertUnwindSafe` is used for
  generated code only (documented rationale: the `Err` path re-derives what
  it needs and never reads partially-mutated captured state). No new
  `unsafe`.

## 0.7.0 — collection-method dispatch + runtime catalog (C6)

Makes the Rust backend **execute** collection-method dispatch. A
source-level `recv.meth(args…)` / `recv.meth { |x| … }` reaches every
backend as the narrow-waist envelope
`BuiltinCall("__method__", [recv, StrLit("meth"), …args, block?])`. Before
this change the Rust backend had no `__method__` arm, so the call fell into
the `call_builtin_by_name` catch-all and hit its runtime floor
`panic!("unknown builtin: __method__")` — a collection program compiled but
crashed at run time. (No capability gate rejected it: `__method__` observes
no dedicated feature, and a pure collection module's observed features —
`Sequences`/`Closures`/`Strings` — were already accepted.)

### Added

- **Emit (`emit.rs`).** A `"__method__"` case in `emit_builtin_call`
  (`emit_method_dispatch`) lowers the envelope to
  `__sir::call_method(<recv>, "meth", vec![<arg0>, …])`. The receiver is
  passed by value; the method name is lifted out of the `StrLit` at
  `args[1]` to a Rust `&str` **literal** (keeping dispatch a closed,
  compile-time-known set); the remaining args — including any trailing
  `MakeClosure` block, which emits a `Value::Closure` — fill the arg `Vec`.
  A `"block_pass"` case lowers `&:sym` / `&blk` to `__sir::sym_to_proc(…)`.
- **Runtime catalog (`runtime.rs`).** A `call_method(recv: Value, name:
  &str, args: Vec<Value>) -> Value` in the inline `__sir` module,
  implementing the collection catalog by an **explicit** match on the
  receiver's runtime type then the method name, ported from the Python/TS
  `sir-runtime-oop` reference for parity:
  - **Array** (`Value::Seq`): `length`/`size`, `first`, `last`, `push`/
    `append`, `pop`, `include?`, `reverse`, `sort`, `join`, `map`/
    `collect`, `select`/`filter`, `reject`, `find`/`detect`, `reduce`/
    `inject`, `each`, `any?`, `all?`, `none?`.
  - **Hash** (`Value::Map`): `keys`, `values`, `size`/`length`,
    `has_key?`/`key?`/`include?`/`member?`, `each`/`each_pair`, `map`,
    `select`/`filter`.
  - **String** (`Value::Str`): `length`/`size`, `upcase`, `downcase`,
    `reverse`, `strip`, `include?`, `split`.
  - **Numeric** (`Value::Int`/`Value::Float`): `abs`, `to_i`, `to_f`,
    `even?`, `odd?`, `zero?`, `times`.
  - Universal `to_s` on every receiver (via the runtime `format`), so
    `&:to_s` works across types.
  - `sym_to_proc` implements Ruby `Symbol#to_proc` (`&:sym`): the returned
    `Closure` dispatches `recv.sym(rest…)` through `call_method`. An
    already-callable `&blk` passes through unchanged.
- **Execution-proof test** (`tests/compile_and_run_collection_methods.rs`):
  hand-builds SIR modules for `map { x*2 }` → `[2, 4, 6]`,
  `select { even? }` → `[2, 4]`, `length` → `3`, `reduce(0)`/`inject` sum
  → `6`, `map(&:to_s).join(",")` → `"1,2,3"`, `sort` → `[1, 2, 3]`, and
  `bogus_xyz` → `nil`; emits Rust, compiles with `rustc`, runs it, and
  diffs stdout against the Python/TS reference. Skips gracefully if
  `rustc`/linker is absent (`SIR_TEST_RUSTC_LINKER`).
- Emitted-shape unit tests for the `__method__` and `block_pass` arms, and
  runtime-content tests asserting the catalog + the absence of a reflective
  fallback.

### Security

- Dispatch is an **explicit allowlist**: `call_method` matches only the
  hand-written `(type, name)` catalog. An unknown method name falls through
  to `unknown_method`, which returns a controlled Ruby `nil` — never a
  reflective lookup on the raw name and never an out-of-catalog effect.
  This mirrors the C3 RCE lesson (the catalog *is* the security boundary).
  No new `unsafe`.

### Notes

- No `Feature` variant added: `Feature::MethodDispatch` (deferred C1) is
  not required here — the catalog is the gate. A pure collection-method
  module was already capability-accepted; this change only makes it
  *execute* instead of panicking. Genuinely-unsupported features stay
  rejected cleanly.

## Unreleased — reject keyword params mixed with rest/kwrest (hardening)

### Fixed

- **Reachable emit panic on validator-accepted input (DoS).** The core
  validator's M3 ordering rule accepts a signature that mixes a keyword
  parameter with a variadic slot (`Required* Rest? Keyword* KwRest?`),
  e.g. Ruby `def f(a, *rest, x: 1)`. Because this backend accepts
  `Feature::KeywordParams`, such a module reached the emitter's static
  keyword→positional resolution path and hit the
  `ParamKind::Rest | ParamKind::KwRest` `panic!` — a reachable panic on
  validated input (and frontend-reachable once the Ruby frontend emits
  keyword+splat methods). Static keyword resolution genuinely cannot
  handle a variadic slot: a `*rest`/`**kwrest` param absorbs a *variable*
  number of arguments, so the name→position map that keyword resolution
  depends on is no longer a function of the signature alone (variable
  arity breaks fixed slot indices). The backend now REJECTS such modules
  cleanly at capability-check time (`reject_keyword_with_variadic`,
  `BackendErrorKind::UnsupportedFeature`, message
  `rust backend cannot emit a function mixing keyword parameters with
  *rest/**kwrest (static keyword resolution requires fixed arity)`)
  instead of panicking. With the check in place, the emit-side variadic
  arm is now a true internal-bug guard, never reachable through the normal
  `compile` path. The happy path (keyword params WITHOUT rest/kwrest) is
  unchanged and still emits.

### Added

- Unit tests: keyword+`*rest` and keyword+`**kwrest` callees with a
  keyword call are rejected via `compile()` (return `Err`, do NOT panic);
  a keyword-only module (no variadic) still compiles.

## 0.6.0 — keyword-parameter & argument emission (KW5)

Adds `Feature::KeywordParams` support: name-matched keyword parameters
(`def f(a:)` / `def f(a: 1)`) and keyword arguments (`f(a: 1)`). Rust has
**no native keyword-argument syntax**, so — per `sir-keyword-params.md`
§4 — the backend performs **static keyword→positional resolution at emit
time** (no runtime library). This replaces the KW1 compile-compat stub
(a `KeywordArg` panic arm; `ParamKind::Keyword` folded into a positional
arm) with real emission.

### Added

- **Def-side positional-ization** — a `Keyword` param emits as an
  ORDINARY positional Rust parameter in its declared order (the by-name
  affordance is dropped; the name becomes the Rust parameter name). An
  OPTIONAL keyword (one carrying a `default`) reuses the existing
  `DefaultParams` body-top prologue unchanged — it is a defaulted
  parameter like any other — so no new def-side machinery is required.
- **Call-side static resolution** — for a `DirectCall` whose callee
  signature is known, the emitter builds the FULL positional argument
  list in the callee's DECLARED order: positionals fill positional params
  in order; each `KeywordArg { name, value }` fills the callee param whose
  name matches `name` (a name→position reorder); an omitted OPTIONAL
  keyword is padded with the `__sir::missing()` sentinel so the callee's
  prologue substitutes its default (deferring default evaluation to callee
  scope — correct even when a default references an earlier param). The
  result is a plain positional Rust call `f(a, b_val, c_default)`.
- **`FN_PARAMS` thread-local signature table** — SIR function name → its
  full parameter list (kinds + defaults). Where `FN_ARITY` records only
  the param count, keyword resolution needs the params' ORDER, NAMES, and
  DEFAULTS. Populated alongside `FN_ARITY` in
  `emit_module_with_arity_table` and consulted by the `DirectCall`
  emitter.
- **`Feature::KeywordParams`** added to the backend's `ACCEPTED_FEATURES`
  (mirroring `DefaultParams`).
- **Unit tests** — def-side positional-ization + default prologue;
  call-side supplied-keyword → positional; call-side omitted-optional →
  sentinel; call-side name→declared-position reorder.
- **Execution proof** (`tests/compile_and_run_keyword_params.rs`) — a
  `def greet(greeting, name: "world") -> name` module, compiled with
  `rustc` and run: `greet("hi")` prints `world` (default) and
  `greet("hi", name: "ada")` prints `ada` (supplied), matching the
  Python/TS reference for `name`. Skips gracefully if `rustc`/linker are
  absent (`SIR_TEST_RUSTC_LINKER`).

### Out of scope (documented)

- **Indirect/closure keyword calls** — an `IndirectCall`/closure carrying
  keywords has no statically-known signature, so keyword→position
  resolution cannot run. The frontends do not emit this (spec
  §"Out of scope"); the `emit_expr` `KeywordArg` arm keeps a positioned
  panic documenting that narrow, internal-bug-only reachability.

## 0.5.0 — default-parameter emission (P2e)

Adds `Feature::DefaultParams` support: a `Param` may now carry a
`default` expression that runs when the caller omits that trailing
argument.  Rust functions are fixed-arity over `__sir::Value` with no
native default parameters, so the backend uses a **runtime-mimic**
strategy built around a `Missing` sentinel — preserving the language's
**call-time, param-scope** default semantics (a default expression is
evaluated on each call that omits the argument, in body scope where
*earlier* parameters are already bound, so `b = a + 1` resolves `a`).

### Added

- **`__sir::Value::Missing`** — a new sentinel variant marking an
  *omitted* positional argument, plus **`__sir::missing()`** (constructor)
  and **`__sir::is_missing(&Value)`** (predicate).  `Missing` is internal:
  it is created only at call sites that drop a trailing argument and is
  consumed by the callee's prologue before any value flows on.
- **Defensive runtime arms** — `format` renders a stray `Missing` as
  `<missing>` and `value_eq` treats `Missing` as equal only to another
  `Missing` (never to `Nil` or a real value).  These should be
  unreachable in well-formed programs but degrade gracefully instead of
  panicking.
- **Function-body default-param prologue** — for each defaulted param, in
  declaration order, the emitter now writes
  `let <name> = if __sir::is_missing(&<name>) { <default> } else { <name> };`
  at the top of the function body.  Emitting the default *inside the body*
  is what gives call-time + param-scope semantics.
- **`DirectCall` caller padding** — a call that omits trailing defaulted
  arguments now pads the omitted positions with `__sir::missing()` so the
  emitted Rust call is full-arity.  The callee's full parameter count is
  read from the existing `FN_ARITY` thread-local arity table (the same
  table `MakeClosure` consults), keyed by the callee's SIR name.
- **`Feature::DefaultParams`** added to `ACCEPTED_FEATURES`.

### Tests

- Unit tests for the emitted shape: the sentinel-guarded prologue (with a
  default that references an earlier param), the padded full-arity call,
  and the no-padding case for a fully-supplied call.
- A `rustc` compile-and-run integration test
  (`tests/compile_and_run_default_params.rs`): hand-builds
  `f(a, b = a + 1) -> b`, prints `f(5)` then `f(5, 10)`, compiles the
  emitted Rust with `rustc`, runs it, and asserts stdout `6` then `10`.

Non-default behaviour is byte-for-byte unchanged — every existing test
(floats, loops, seq/maps, cyclic) passes untouched.

## 0.4.1 — harden emitted runtime against cyclic Seq/Map

`Value::Seq`/`Value::Map` are shared, *mutable* handles, so an emitted
program can build a cyclic structure (`xs = []; xs[0] = xs`).  Before this
release the emitted runtime walked such values structurally with no cycle
protection, so a cyclic value could:

- **`format`** — recurse forever and overflow the stack while printing.
- **`value_eq`** — recurse forever when comparing two *distinct* cyclic
  structures (a self-cycle was already short-circuited by the `Rc::ptr_eq`
  fast path, but distinct cyclic operands were not).
- **`map_get`/`map_set`/`map_lit`** — hit a `RefCell` "already mutably
  borrowed" panic when a self-referential key was compared while the map's
  entries were `borrow_mut`'d.

This is a robustness fix only — the public runtime API and the printed
form of every *non-cyclic* value are byte-identical (all existing tests
pass unchanged).

### Fixed

- **`format` / `format_seq` / `format_map`** now thread a visited-pointer
  set (`HashSet<usize>` of each Seq/Map `Rc` handle address).  A handle is
  inserted on entry and removed on exit, so it is only "seen" along the
  *current* path: a true cycle re-entering a handle within its own subtree
  prints a placeholder (`[...]` for a seq, `{...}` for a map) and returns
  instead of recursing, while a value reached twice by sibling
  (non-cyclic) paths still prints in full.  `format_pair` threads the set
  too (a pair can hold a cyclic seq/map).
- **`value_eq`** keeps the `Rc::ptr_eq` identity fast path and adds a
  co-inductive `pending` set of handle-pairs currently being compared:
  re-encountering a pair already in flight (a cycle matched in lock-step)
  is treated as equal, bounding the deep comparison of two distinct cyclic
  operands so it always terminates.
- **`map_get` / `map_set` / `map_lit`** no longer call `value_eq` while
  holding a borrow on the same map's entries: each snapshots/collects the
  comparison inputs and resolves to an *index* before taking the borrow it
  needs for the read/write, so a self-referential key can no longer trigger
  an "already borrowed" panic.

### Tests

- New `tests/compile_and_run_cyclic.rs` integration test: hand-builds a
  module that constructs a cyclic seq (`xs = [0]; xs[0] = xs; print(xs)`),
  emits Rust, compiles it with `rustc`, runs it, and asserts the program
  *terminates* and prints the `[...]` placeholder.  It also checks that
  `value_eq` terminates on both a self-cyclic operand (via `ptr_eq`) and
  two *distinct* cyclic structures (via the co-inductive guard).

## 0.4.0 — SIR16 Sequences + Maps (completes SIR16 / v1 parity)

The final two SIR-v1 (SIR16) features land in the Rust backend.  With
`Sequences` and `Maps` now accepted, the Rust backend supports **all six**
SIR16 / SIR-v1 features — `Floats`, `ShortCircuit`, `MutableBindings`,
`Loops`, `Sequences`, `Maps` — reaching full v1 parity with the
TypeScript backend.  Every SIR16 IR node now has a real emit arm; the
only remaining `panic!`s cover SIR17/18 nodes (classes, modules,
singleton classes, try/catch, string interpolation, instance/class/const
vars, intrinsics) whose features stay unaccepted, so they are unreachable
for any validated module.

### Added

- `Feature::Sequences` and `Feature::Maps` in the accepted-feature set
  (`lib.rs`).
- **Runtime value model** — two new shared, mutable `Value` arms:
  - `Value::Seq(Rc<RefCell<Vec<Value>>>)` — a growable vector.  The
    `Rc<RefCell<…>>` is essential: `SeqSet` (`xs[i] = v`) must mutate the
    sequence the caller holds, and aliasing bindings must observe each
    other's writes — the reference semantics of a Python list / JS array.
  - `Value::Map(Rc<RefCell<Vec<(Value, Value)>>>)` — an insertion-ordered
    association list.  Keys compare with the runtime's own `value_eq`
    (linear scan) rather than a `HashMap`, because `Value` is neither
    `Hash` nor `Eq` (floats, closures, nested seqs/maps).  This gives
    correct lookup semantics for *any* key type and preserves insertion
    order for deterministic iteration and printing.
- **Sequences** — `SeqLit`/`SeqIndex`/`SeqLen` expressions lower to the
  `seq_lit`/`seq_index`/`seq_len` helpers; the `SeqSet` statement mutates
  the backing vector through `seq_set`.  Out-of-range index reads/writes
  panic (strict, like `car`/`cdr` on a non-pair).
- **Maps** — `MapLit`/`MapGet` expressions lower to `map_lit`/`map_get`;
  the `MapSet` statement mutates via `map_set`.  A missing-key `MapGet`
  returns `Nil` (mirroring the TypeScript backend's `?? null`).  Literal
  and `map_set` writes are last-write-wins on an existing key while
  preserving first-seen insertion order.
- **`format`** renders sequences as `[1, 2, 3]` and maps as `{a: 1, b: 2}`
  (insertion order); **`value_eq`** compares seqs/maps structurally
  (element-wise, with an `Rc::ptr_eq` fast path).

### Changed (ForEach reconciliation)

- A2 introduced `Stmt::ForEach` with a `seq_iter` helper that walked a
  cons-list (there was no `Seq` value yet).  Now that a real
  `Value::Seq` exists, `seq_iter` was reconciled to **snapshot a
  `Value::Seq`** as well as walk the legacy cons-list, so a
  `for x in [1, 2, 3]` (a `SeqLit`) iterates end to end while the
  cons-list path keeps working unchanged.
- Fixed a latent `ForEach` emit bug surfaced by the new end-to-end test:
  the loop emitted `for <var>: __sir::Value in …`, but a type annotation
  on a `for` pattern is not valid Rust.  Dropped the annotation
  (`for <var> in …`); the element type is already `Value` from
  `seq_iter`'s return.  This path had no prior compile-and-run coverage
  (the loops test exercised only `while`/`for-range`).
- The mutable-binding pre-pass (`collect_assigned_locals`) now recurses
  into Seq/Map statements and expressions, so an `Assign` nested inside a
  `SeqSet`/`MapSet` value (or a `SeqLit`/`MapLit` sub-expression) is
  still discovered and its binding declared `let mut`.

### Tests

- `tests/compile_and_run_seq_maps.rs`: an end-to-end proof that emits a
  module using a sequence (literal, index, len, set), a map (literal,
  get with a present *and* a missing key, set), and a `for v in <SeqLit>`
  `ForEach` accumulation, compiles it with `rustc`, runs the binary, and
  asserts its stdout (`20`, `3`, `99`, `2`, `nil`, `7`, `10`).
- Unit tests for every new emit arm (seq/map literal, index, len, get,
  and the seq/map set statements in both the block and inline paths),
  for `ForEach`-over-`SeqLit` composition, and for the mutable-name
  pre-pass recursing into a `SeqSet` value; runtime tests for the new
  value arms and helpers.

## 0.3.0 — SIR16 MutableBindings + Loops

The next two SIR-v1 (SIR16) features land in the Rust backend, matching
the TypeScript backend's existing support.  Until now `MutableBindings`
and `Loops` were undeclared and their IR nodes hit the `panic!` reject
group; this PR replaces those arms with real emission.

### Added

- `Feature::MutableBindings` and `Feature::Loops` in the accepted-feature
  set (`lib.rs`).
- **MutableBindings**: a per-function pre-pass (`collect_assigned_locals`)
  finds every name that is later the target of a `Stmt::Assign`.  A
  `LetBinding` for such a name is emitted as `let mut` (immutable bindings
  stay plain `let`), and `Stmt::Assign` then emits a bare
  `<name> = <value>;` for Local/Param/Capture scopes.  A `Global`-scoped
  assign writes through the runtime store
  (`__sir::global_set(&__sir::intern("name"), value)`).  Mirrors the
  TypeScript backend's `const`/`let` mutable-name tracking.
- **Loops** — all three loop statements emit real Rust:
  - `While { cond, body }` → `while __sir::truthy(&(<cond>)) { <body> }`,
    routing the test through SIR truthiness (only `false`/`nil` are
    falsy), never Rust's native `bool`.
  - `ForRange { var, start, stop, step, body }` → a numeric loop that
    caches `stop`/`step` into block-scoped `i64` temporaries (evaluated
    once, like Python's `range`), with a direction-aware condition so a
    negative `step` counts down.  The loop variable is rebound each
    iteration as a fresh `__sir::Value::Int`.  Fresh per-loop temp ids
    keep nested loops collision-free; the counter resets per module for
    deterministic output.
  - `ForEach { var, iter, body }` → `for <var> in __sir::seq_iter(&(<iter>))`.
    This backend has no dedicated `Seq` value yet (Sequences land in a
    later PR), so a "sequence" is the existing cons-list (`Pair`-chain
    terminated by `Nil`); `seq_iter` flattens it into a `Vec<Value>`.
    No `Feature::Sequences` runtime is required — the validator observes
    only `Feature::Loops` for `ForEach`, so accepting `Loops` covers all
    three loop forms with **no reachable `panic!`**.
- Runtime helpers `as_int` (public face of `as_i64`, for the `ForRange`
  bound temporaries) and `seq_iter` (cons-list → `Vec<Value>` for
  `ForEach`).

### Tests

- `tests/compile_and_run_loops.rs`: an end-to-end proof that emits a
  module using a `while` loop, two `for-range` accumulators, and mutable
  reassignment, compiles it with `rustc`, runs the binary, and asserts
  its stdout (`sum 0..5 = 10`, countdown ends at `0`, product `= 6`).
- Unit tests for each new emit arm: bare/global assign, `let mut`
  selection, while/truthy, for-range bound caching + int var binding +
  direction-aware condition + nested fresh ids, and for-each via
  `seq_iter`.

### Notes

- The remaining two SIR16 features (Sequences, Maps) are still
  undeclared; their `SeqSet`/`MapSet` and Seq/Map expression emit arms
  keep the `panic!` (unreachable via the capability check) until a later
  PR extends them.

## 0.2.0 — SIR16 Floats + ShortCircuit

The first two SIR-v1 (SIR16) features land in the Rust backend.  Until
now `Floats` and `ShortCircuit` were undeclared and their IR nodes hit
the `panic!` reject group; the TypeScript and Python backends already
supported them, so this closes part of the cross-backend parity gap.

### Added

- `Feature::Floats` and `Feature::ShortCircuit` in the accepted-feature
  set (`lib.rs`).
- Runtime value model gains `Value::Float(f64)`.  The arithmetic helpers
  (`plus`/`minus`/`times`/`divide`) stay on the exact i64 path while
  every operand is an integer and promote the whole fold to f64 as soon
  as any operand is a float (Python/Ruby/JS "int op float ⇒ float").
  `number?` now covers floats, `=` is cross-representation (`1 == 1.0`
  is true; `NaN == NaN` is false), and `<`/`>` compare numerically.
- `Expr::FloatLit` emits a `Value::Float` literal — `{:?}` keeps the
  trailing `.0` on integral values so the literal is never mistaken for
  an `i64`; non-finite values use `f64::NAN`/`INFINITY`/`NEG_INFINITY`.
- `Expr::LogicalAnd`/`Expr::LogicalOr` emit a truthy-guarded block
  (`{ let __l = lhs; if truthy(&__l) { ... } else { ... } }`) that
  evaluates the rhs only when the lhs decides — same semantics as the
  TypeScript backend's truthy-guarded arrow IIFE.

### Tests

- `tests/compile_and_run_floats.rs`: an end-to-end proof that emits a
  float + short-circuit module, compiles it with `rustc`, runs the
  binary, and asserts its stdout.

### Notes

- The remaining four SIR16 features (MutableBindings, Loops, Sequences,
  Maps) are still undeclared; their emit arms keep the `panic!`
  (unreachable via the capability check) until later PRs extend them.

## 0.1.2 — SIR18 exhaustiveness (no behaviour change)

semantic-ir 0.10.0 adds `Expr::StrConcat` (the SIR18 string-concat
node).  This backend gains a `StrConcat` arm in its expression emitter
so it stays exhaustive.  The arm joins the existing SIR16+ reject group
and `panic!`s with a "capability check should have rejected it"
message: `Feature::StringInterpolation` is not in this backend's
accepted-feature set, so a concat-using module is rejected at the
capability check before emit, making the arm unreachable.  No output or
accepted-feature changes.

## 0.1.1 — SIR17 exhaustiveness (no behaviour change)

semantic-ir 0.2.0 adds `Stmt::ClassDef` (the SIR17 class node).  This
backend gains a `ClassDef` match arm in its statement emitter so it
stays exhaustive.  The arm `panic!`s with a "capability check should
have rejected it" message: `Feature::Classes` is not in this
backend's accepted-feature set, so a class-using module is rejected
at the capability check before emit, making the arm unreachable.  No
output or accepted-feature changes.

## 0.1.0 — initial release (SIR13 v0)

Second backend for the narrow-waist Semantic IR.  Emits self-contained
Rust source from a `semantic_ir::Module`.

### Added

- `RustBackend` implementing `semantic_ir::Backend` with:
  - `target_tag() = "rust"`
  - `accepts_features()` covering the full v0 surface minus
    `TailCalls` and `Intrinsics`.
- `compile(module)` convenience function returning an
  `Artifact { filename, source, metadata }`.
- Per-node lowering rules per SIR13:
  - Literals → typed `__sir::Value::*` constructors.
  - Symbols → `__sir::intern("...")`.
  - VarRef Local/Param/Capture → `<name>.clone()`.
  - VarRef Global → `__sir::global_get_static("...")`.
  - VarRef Builtin → `__sir::builtin_closure("...")`.
  - If → Rust `if/else` with `__sir::truthy(&cond)`.
  - Block → Rust block expression `{ stmts...; value }`.
  - LetBinding / LetStarBinding → `let name: __sir::Value = ...;`.
  - DirectCall → `<fn>(<args>)`; SIR `main` is renamed to
    `__sir_user_main` to avoid collision with Rust's process entry.
  - IndirectCall → `__sir::apply_closure(&target, vec![args])`.
  - BuiltinCall → typed helper or `call_builtin_by_name` fallback.
  - MakeClosure → `__sir::Value::Closure(Rc::new(__sir::Closure {
    fun: Box::new(move |args| <fn>(<captures>, <pos-args>)) }))`.
- Inlined `__sir` runtime (~280 lines) covering:
  - `Value` enum, `Pair` struct, `Closure` wrapping a `Box<dyn Fn>`.
  - `intern` / `apply_closure` / `truthy` / `format`.
  - All v0 builtins (`plus`, `minus`, `times`, `divide`, `eq`,
    `lt`, `gt`, `cons`, `car`, `cdr`, `is_null`, `is_pair`,
    `is_number`, `is_symbol`, `print`).
  - `thread_local!` storage for globals + symbol interning.
  - `call_builtin_by_name` dispatch for VarRef Builtin and
    forward-compat new builtins.
- Identifier sanitisation:
  - Valid Rust identifiers pass through.
  - Rust keywords (`fn`, `type`, `match`, etc.) get the `r#`
    raw-identifier prefix so the original spelling stays visible.
  - Other invalid characters (`?`, `!`, `-`, `+`, `*`) are encoded
    as `_<hex>` underscore-escaped forms.
  - Empty input becomes `"_$empty"`.
  - SIR's `main` is specially renamed to `__sir_user_main`.
- Function arity table threaded via TLS so `MakeClosure` knows
  how many positional arguments to drain from the runtime args
  iterator when calling the synthesised lambda function.
- `sanitize_comment` strips line terminators (`\n`, `\r`, U+0085,
  U+2028, U+2029) from any external string written into `//`
  comments, mirroring the TypeScript backend's defense.
- Pre-lowering validation via `semantic_ir::validate`; capability
  check via the `Backend::check_module` default impl.

### Deferred

- Static type narrowing.  Optional SIR types widen to `Value`.
- `no_std` / `alloc`-only target.
- Source-map generation (function-level comments only).
- Raw-Rust intrinsic embedding.
- Async / `await` support (no SIR async surface yet).
