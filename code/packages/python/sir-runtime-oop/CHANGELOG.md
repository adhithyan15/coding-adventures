# Changelog

All notable changes to `coding-adventures-sir-runtime-oop` are documented here.

## [0.1.17] - 2026-07-10

### Added — Numeric breadth: `divmod` / `fdiv` / `round(ndigits)` / `clamp` / `between?`

Extends the `Integer`/`Float` catalog (`_numeric_method` + `_NUMERIC_METHODS`)
with five more Ruby numeric methods — this establishes the **reference**
semantics for the N1 breadth sweep before the embedded backends mirror them:

- `round(ndigits)` — `round` gains an optional digits argument: a positive
  `ndigits` rounds a `Float` to that many decimals (half **away from zero**, not
  Python's banker's rounding); `ndigits <= 0` rounds to an `Integer` power of ten.
  A non-finite `Float` returns unchanged (never-raise floor).
- `divmod(n)` — `[quotient, remainder]` with a floored quotient and the
  divisor-signed remainder; division by zero raises a typed `ZeroDivisionError`.
- `fdiv(n)` — floating-point division that **never raises**: dividing by zero
  yields `Infinity`/`-Infinity`/`NaN` (matching Ruby) rather than raising.
- `clamp(min, max)` — `min` if `recv < min`, `max` if `recv > max`, else `recv`.
- `between?(min, max)` — `min <= recv <= max`.

The `clamp`/`between?` `Range` form is deferred, matching the literal-only
precedent elsewhere in the catalog. All arithmetic is hardened to the
never-raise floor: `round` bounds a hostile `ndigits` (no bignum allocation),
guards non-finite arguments, and uses all-integer rounding for large-integer
receivers; `divmod`/`fdiv` saturate bignum operands to `±Infinity` (via
`_sat_float`) and route a non-numeric argument to the typed `ZeroDivisionError`
rather than an untyped `OverflowError`/`ValueError`/`TypeError`.

## [0.1.16] - 2026-07-07

### Added — String char-set methods: `tr` / `count` / `delete` / `squeeze`

Extends `_string_method` (and the `_STRING_METHODS` `respond_to?` catalog) with
four more non-block Ruby String methods:

- `tr(from, to)` — position-wise character translation; a shorter `to` repeats
  its last char, an empty `to` deletes matching chars, and the last mapping wins
  when `from` repeats a char.
- `count(*sets)` / `delete(*sets)` / `squeeze(*sets)` — char-set methods:
  `count` tallies chars of the receiver in the set, `delete` removes them, and
  `squeeze` collapses consecutive runs (of set chars, or of *all* chars when no
  set is given). Multiple set arguments intersect (Ruby's rule).

Each `set`/`from`/`to` argument is treated **literally** — the character-range
(`"a-z"`) and negation (`"^abc"`) forms are a follow-up, matching the existing
literal-only `sub`/`gsub` precedent. First backend of the String char-set sweep
(reference; the Go/Rust/JS/TS backends follow).

## [0.1.15] - 2026-07-07

### Added — Array reorder/combine methods: `rotate` / `zip`

Closes the parity gap with the Go/Rust runtimes (which already carry these) by
adding two more non-block Ruby Array methods to `_array_method` and the
`_ARRAY_METHODS` `respond_to?` catalog:

- `rotate(n=1)` — rotate left by `n` (a negative `n` rotates right); the modulo
  wraps so any magnitude terminates, and an empty array stays `[]`. No argument
  defaults to `1`; a non-numeric argument degrades to `0` (never raises).
- `zip(*others)` — an Array of tuples `[self[i], others..[i]]` of length
  `len(self)`; a shorter operand pads with `nil` (`None`), a longer one is
  truncated, and a non-array operand is treated as empty (pad-only).

## [0.1.14] - 2026-07-07

### Added — Array slice-selection methods: `take` / `drop` / `values_at`

Extends `_array_method` (and the `_ARRAY_METHODS` `respond_to?` catalog) with
three more non-block Ruby Array methods, mirroring the Go/Rust/JS runtimes:

- `take(n)` / `drop(n)` — the first `n` elements, or all elements *after* the
  first `n`. `n` is clamped to `[0, len]`: Ruby raises `ArgumentError` on a
  negative `n`, but the never-raise floor folds it to 0, and Python slicing
  already saturates `n > len`. A non-numeric argument degrades to 0.
- `values_at(*idxs)` — one element per index, with a negative index folded from
  the end **once**; an out-of-range index yields `nil` (`None`) rather than
  raising, matching the sibling backends.

## [0.1.13] - 2026-07-07

### Added — more String methods: `ljust` / `rjust` / `center` / `swapcase`

Extends `_string_method` (and the `_STRING_METHODS` `respond_to?` catalog) with
four more common non-block Ruby String methods, mirroring the Go/JS runtimes:

- `ljust(width, pad=" ")` / `rjust(...)` / `center(...)` — pad to `width`
  characters using `pad` cyclically; `width <= len` returns the string
  unchanged; `center` puts an odd extra pad char on the **right** (Ruby's rule,
  the opposite of Python's built-in `str.center`, which also rejects a
  multi-char fill). An empty pad degrades to a single space (never-raise floor).
  New helper `_str_pad` builds the exact-length cyclic padding.
- `swapcase` — flips each ASCII letter (non-letters / non-ASCII untouched),
  matching the Go/JS runtimes byte-for-byte.

## [0.1.12] - 2026-07-07

### Added — Array block-method breadth (sort_by / group_by / partition / …)

Extends `_array_block_method` with the common block-taking Ruby
`Enumerable`/`Array` methods that were missing (map/select/reduce/find/flat_map
were already present), and adds them to the `_ARRAY_BLOCK_METHODS` catalog so
`respond_to?` stays honest. Mirrors the Rust/Go backends' array-block batch.

- `sort_by { |x| key }` — key-sorted (stable, like Ruby).
- `min_by` / `max_by { |x| key }` — extremal block key (`nil` on empty).
- `group_by { |x| key }` — a Hash (dict) of key → list of elements.
- `partition { |x| pred }` — `[matching, non_matching]`.
- `collect_concat` — alias of `flat_map`.
- `take_while` / `drop_while { |x| pred }` — leading truthy run / remainder.
- `count { |x| pred }` — truthy count (arg/bare forms unchanged in
  `_array_method`).
- `each_with_object(memo) { |x, memo| … }` — folds into and returns the memo.

Predicate results route through SIR `truthy`; a block-less call keeps the nil
Enumerator floor. Ordering uses Python's native comparison (a non-mutually-
comparable key raises `TypeError`, identical to the existing `sort` arm).

## [0.1.11] - 2026-07-02

### Added — mixins: `include` / `extend` + Ruby MRO (MX2 of sir-mixins)

Ruby modules mixed into a class now resolve correctly. A module registers its
`def`s in the same method table as a class (owner key = module name, via the
existing `__def_method__`); `include`/`extend` wire them in. See
[`code/specs/sir-mixins.md`](../../../specs/sir-mixins.md).

- **`include_module(owner, module)`** (emitted `__include__`) — appends `module`
  to the owner's included-modules list (`_included_modules[owner]`) in **include
  order**.
- **`extend_module(owner, module)`** (emitted `__extend__`) — copies the
  module's instance methods into the owner's **class-method** table, so they
  answer as `Owner.method` (singleton methods).
- **Ruby MRO** — instance-method resolution now walks the linearised order
  **class → its included modules (reverse / most-recent-first, depth-first) →
  superclass → its modules → … → Object**, via the new `_owner_mro` helper. A
  diamond include resolves the shared module **once** (first occurrence fixes its
  position); the walk is cycle-guarded by a `seen` set, so a self-including
  module terminates. A class's own method **shadows** an included module's; a
  module method **shadows** the superclass's; the most-recently-included module
  wins among modules. Dispatch stays explicit-table only — never reflection on a
  source-derived name (the C3 RCE lesson).
- `reset_oop` clears the new `_included_modules` table.

## [0.1.10] - 2026-07-01

### Changed — typed runtime errors (T1 of sir-typed-runtime-errors)

Faulting Ruby method calls now raise the **typed** `SirError` the rescue matcher
names, replacing the old blanket nil floor for `.fetch` misses and unknown
methods — so `rescue IndexError` / `rescue KeyError` / `rescue NoMethodError`
catch them, identically to Ruby. See
[`code/specs/sir-typed-runtime-errors.md`](../../../specs/sir-typed-runtime-errors.md).

- **`Array#fetch`** — added to the array catalog. Returns the element for an
  in-range index (negatives count from the end), like `arr[i]`; an
  **out-of-bounds** index with no default now raises `IndexError`
  (`"index N outside of array bounds: -M...M"`). A second argument supplies a
  default returned instead of raising.
- **`Hash#fetch`** — a **missing** key with no default now raises `KeyError`
  (`"key not found: <inspect>"`) instead of returning nil. A second argument
  still supplies a default.
- **Unknown method** — `call_method`'s floor now raises `NoMethodError`
  (`"undefined method 'x' for <class>"`) for a **genuinely unknown** method
  (`obj.undefined`, `nil.foo`, `"s".scan`). A *known* block-taking method invoked
  **without** a block (e.g. `[1,2,3].map`, `5.times`) still returns nil — Ruby
  returns an Enumerator there, the documented v0 floor — discriminated by
  `_responds_to`.
- **Unchanged (no over-raise):** the plain index operators `arr[i]` / `hash[k]`
  still return nil (Ruby does not raise for `[]`); they are emitted as native
  Python subscripts and never route through `.fetch` / `call_method`. Catalogued
  accessors that legitimately return nil (`arr.first` on empty, `hash.dig` miss,
  `str.index` miss) are unaffected.
- No reflection or `eval`: every typed raise is an explicit class-name string via
  the shared `raise_error` entry point; the unknown method name is interpolated
  as an opaque message string, never used to reflect a Python attribute (the C3
  dynamic-dispatch RCE lesson).

### Dependency

- Added `coding-adventures-sir-runtime-exceptions` for the typed-raise entry
  point.

## [0.1.9] - 2026-07-01

### Added (O1 — user class/instance method tables + new/super/self)

The runtime now executes user-defined Ruby OOP (dispatch, construction,
inheritance) — the substrate the frontend's O2 pass will target. Additive: no
existing behaviour changes; these helpers only run once the new builtins appear.

- **Method tables.** Two explicit `(class, method) → Closure` tables —
  `_instance_methods` (populated by `def_method`, for `def m`) and
  `_class_methods` (populated by `def_class_method`, for `def self.m`). Keyed on
  a `(class, method)` tuple; dispatch is **always** an explicit dict lookup
  walking the registered ancestry chain — **never** `getattr`/`eval`/reflection
  on a source-derived name (the C3 RCE lesson). Every ancestry walk is
  cycle-guarded with the same `seen`-set pattern as `is_a`.
- **`call_new(class, *args)`** — allocates an instance, pushes it as the current
  self, runs an inherited `initialize` (walking ancestry) with the args, pops
  self, and returns the object (a plain allocation when no `initialize` exists).
- **`call_method` user-object path** — when the receiver is a `SirInstance`, a
  registered instance method (resolved through ancestry) is dispatched first
  under a pushed self; only if none resolves does dispatch fall through to the
  existing reflective built-ins and primitive catalog (no regression of
  `obj.class` / collection methods).
- **`call_super(method, class, *args)`** — walks from the class's parent upward
  and re-runs the first ancestor implementation with the **current** self still
  bound (super shares the receiver — no push/pop). Returns `nil` when no
  ancestor defines the method.
- **`call_class_method(class, method, *args)`** — dispatches `def self.m` class
  methods via the class-method table (ancestry-walked); `nil` when unresolved.
- **`current_self()`** — the top of the self-stack (or `nil` at top level);
  backs a bare `self`. Named `current_self` to avoid the Python keyword clash.

**Single-threaded caveat.** Self is a process-global stack, so this is faithful
for the single-threaded transpiled scripts we target; true per-object/per-thread
binding remains out of v0 scope (as documented at the module head).

## [0.1.8] - 2026-06-30

### Added (M6 — Kernel flow-control + boolean operators)

Completes the spec's v0 universal-`Object` surface
(`code/specs/sir-method-dispatch.md`), which listed `tap`, `then`/`yield_self`,
and `send`/`__send__` but had not yet implemented them, plus the `nil`/`true`/
`false` boolean operators:

- **`send`/`__send__`/`public_send`** — dynamic dispatch. The first argument
  names the method (a `Symbol` — the emitted form — or a bare string); the rest
  forward unchanged and a **trailing block survives**, so
  `[1, 2].send(:each) { … }` reaches the block-taking catalog method. Routed
  ahead of the catalog because it recurses through `call_method`; an empty arg
  list (`send` with no method name) bottoms out at the `nil` floor rather than
  raising. This is the substrate the spec flagged for later metaprogramming.
- **`tap`** — yields the receiver to the block, returns the **receiver**
  (pipeline-friendly side effect).
- **`then`/`yield_self`** — yields the receiver, returns the **block's result**
  (functional "pipe into a block"). Block-less `tap`/`then`/`yield_self` return
  the receiver (the documented v0 Enumerator-less floor).
- **`TrueClass`/`FalseClass` `&` / `|` / `^`** — Ruby's *eager*
  (non-short-circuit) logical operators, distinct from the lazy `&&`/`||`
  keywords. The operand is coerced by SIR `truthy`, so `true & nil == false`,
  `false | 0 == true`, `true ^ true == false`. They resolve on a `bool`
  receiver before the universal `Object` table.

`respond_to?` reports each new name honestly — `tap`/`then`/`send` on every
receiver, the boolean operators on `bool` receivers only — and an out-of-catalog
name stays both `nil` *and* `respond_to? == False`. `mypy --strict` + ruff clean;
79 pytest cases at ~96% coverage.

## [0.1.7] - 2026-06-26

### Added (M5 — case-equality `===`)

- `case_eq(pattern, value)` — Ruby case-equality, the test a `when` clause
  runs. Dispatches on the *pattern*'s type: a `re.Pattern` (Regexp) → the regex
  matches `str(value)`; a `Range` (detected structurally by type name, so no
  dependency on `sir-runtime-range`) → membership via its `includes`; anything
  else → value equality (`eq`). A range/non-comparable-type mismatch
  (`(1..5) === "x"`) returns `False` rather than raising, mirroring Ruby. The
  class case (`when Integer`) is handled at the frontend (`value.is_a?(Const)`)
  and never reaches here.

## [0.1.6] - 2026-06-22

### Added

**`Symbol#to_proc` (`&:sym`) — `sym_to_proc`** (per `code/specs/sir-method-dispatch.md`,
item M2). New `sym_to_proc(sym) -> Closure`: builds a `sir-runtime-core`
`Closure` equivalent to Ruby's `sym.to_proc`, so a `&:sym` block argument on a
dispatched call works — `[1, 2, 3].map(&:to_s)` now evaluates to
`["1", "2", "3"]`.

- The Ruby→SIR frontend lowers `&:sym` to `block_pass(SymLit("sym"))`; the
  backend emits the surviving envelope as `_sir_oop_sym_to_proc(intern("sym"))`,
  and the resulting `Closure` is driven by the block-taking catalog methods
  (`map`/`select`/`each`/…) through `apply` exactly like a `{ }` block.
- The closure's `arity` is `None` (variadic), so `apply` forwards a block
  method's arguments unadjusted: the first becomes the **receiver**, the rest
  are forwarded as method arguments. This matches `&:sym`'s Ruby arity (one
  required receiver plus a rest) — correct for the one-arg (`map`) and two-arg
  (`include?`-style) shapes alike.
- The proc body dispatches through `call_method`, so an **out-of-catalog method
  bottoms out at `nil`** rather than raising — the never-raise-on-the-OO-surface
  invariant holds for the proc body too. A bare string name is accepted
  defensively in addition to a `Symbol`.

### Known v0 limitation

Arithmetic/comparison operators (`&:+`, `&:<`) are emitted as **native**
operations, not routed through the dispatch catalog, so `inject(&:+)` is not yet
supported (the proc would resolve `+` to `nil`). Operator dispatch is tracked in
the later numeric-fidelity item.

## [0.1.5] - 2026-06-22

### Added

Built-in method dispatch, part 5 — the **`Integer`/`Float`**, **`Symbol`**, and
**`nil`/`true`/`false`** catalogs (per `code/specs/sir-method-dispatch.md`, item
M1c), completing the M1c primitive surface:

- **Numeric** (`int`/`float`): `abs`, `to_i`, `to_f`, `even?`, `odd?`, `zero?`,
  `positive?`, `negative?`, `succ`/`next`, `pred`, `floor`, `ceil`, `round`
  (half **away from zero**, unlike Python's banker's rounding), `gcd`, `pow`/`**`,
  `digits`; block forms `times`, `upto`, `downto`, `step`.
- **Symbol** (`sir-runtime-core` `Symbol`): `to_s`, `to_sym`, `length`/`size`,
  `upcase`/`downcase` (return a new interned symbol), `inspect`, `empty?`.
- **`to_s`/`inspect` are now universal `Object` methods** (Ruby display forms):
  `nil.to_s == ""` / `nil.inspect == "nil"`, `true.to_s == "true"`, numbers and
  symbols print faithfully, and an `Array`/`Hash` renders `"[1, 2]"` / `"{:k=>v}"`.
  This means **`nil`/`true`/`false` need no catalog of their own** (`nil.to_a`
  already returns `[]`). Added **`Array#join`** (elements via `to_s`, default sep
  `""`).
- Dispatch orders the `bool` check **before** `int` (a Python `bool` is an `int`
  subclass) so `True`/`False` resolve only the `Object` methods, never the numeric
  catalog.

`respond_to?` reports each new catalog honestly; out-of-catalog stays `nil`.

### Security / robustness

- **`**`/`pow` and `digits` bound hostile bignums.** A repeat/exponent count can
  come from untrusted input; Python ints are arbitrary precision, so
  `2 ** (10 ** 9)` would allocate ~125 MB. `**` now refuses an integer result
  past a ~1M-bit budget (returns `0`); `digits` refuses an over-budget bignum;
  float overflow returns `inf` instead of raising.
- **`to_s`/`inspect`/`join` are cycle- and depth-safe.** A self-referential
  `Array`/`Hash` renders `[...]`/`{...}` (Ruby's behaviour) and depth is capped,
  so a cyclic or deeply-nested structure can no longer raise `RecursionError`.
- **Numeric methods never raise on `inf`/`nan`.** `to_i`/`even?`/`odd?`/`gcd`/
  `floor`/`ceil`/`round`/`digits` degrade gracefully rather than raising
  `OverflowError`, upholding the never-raise-on-the-OO-surface invariant.

## [0.1.4] - 2026-06-22

### Added

Built-in method dispatch, part 4 — the **`String`** catalog (per
`code/specs/sir-method-dispatch.md`, item M1c; a Ruby `String` is a Python `str`,
which is **immutable**, so every method is non-mutating and returns a fresh
value):

- Non-block: `length`/`size`, `upcase`, `downcase`, `capitalize`, `reverse`,
  `strip`/`lstrip`/`rstrip`, `chomp` (default line-ending or explicit suffix),
  `chars`, `bytes` (UTF-8), `split` (whitespace default or literal separator),
  `include?`, `start_with?`, `end_with?`, `index` (nil when absent), `replace`,
  `sub`/`gsub` (**literal** — pattern matched as a plain substring, replacement
  inserted verbatim with no `\1`/`&` back-reference expansion), `to_i`/`to_f`
  (leading-numeric, `0`/`0.0` when none — never raising), `to_sym` (interns a
  `sir-runtime-core` `Symbol`), `empty?`, `*` (repeat), `+` (concat).
- Block: `each_char` — applied via `sir-runtime-core` `apply`.

`respond_to?` reports the String methods; out-of-catalog stays `nil`. Universal
`Object` methods still resolve on a String receiver.

## [0.1.3] - 2026-06-22

### Added

Built-in method dispatch, part 3 — the **`Hash`** catalog (per
`code/specs/sir-method-dispatch.md`, item M1c; Hash is a Python `dict`):

- Non-block: `keys`, `values`, `has_key?`/`key?`/`include?`/`member?`,
  `has_value?`/`value?`, `fetch` (with optional default), `size`/`length`,
  `empty?`, `to_a` (`[[k, v], …]`), `dig` (single-level v0), `store`/`[]=`,
  `merge` (new dict), `delete`, `clear`, `invert`.
- Block (block receives `[key, value]`): `each`/`each_pair`, `each_key`,
  `each_value`, `map`, `select`/`filter`, `reject` — applied via
  `sir-runtime-core` `apply`, predicates through SIR `truthy`.

`respond_to?` reports the Hash methods; out-of-catalog stays `nil`. Universal
`Object` methods still resolve on a Hash receiver.

## [0.1.2] - 2026-06-22

### Added

Built-in method dispatch, part 2 — **block-taking `Array`/`Enumerable` methods**
(per `code/specs/sir-method-dispatch.md`, item M1b):
`each`, `each_with_index`, `map`/`collect`, `select`/`filter`, `reject`,
`reduce`/`inject` (with/without initial), `find`/`detect`, `flat_map`,
`any?`/`all?`/`none?`. A trailing `Closure` block is applied via
`sir-runtime-core`'s `apply` (proc-lenient arity); predicate results route
through SIR `truthy` (only `false`/`nil` are falsy, so `0`/`""` are kept).
`respond_to?` now reports these method names.

### Changed

- Adds a dependency on **`coding-adventures-sir-runtime-core`** (for `apply`,
  `Closure`, `truthy`); wired via `[tool.uv.sources]` + leaf-to-root BUILD
  install (pairs → core → self).
- A block method invoked **without** a block still bottoms out at `nil` (Ruby
  returns an `Enumerator`; v0 floor, documented).

## [0.1.1] - 2026-06-22

### Added

Built-in method dispatch, part 1 — non-block `Array` + universal `Object`
methods (per `code/specs/sir-method-dispatch.md`, item M1a). Before this,
`call_method` returned `nil` for every method outside `is_a?`/`kind_of?`/
`instance_of?`/`class` + the `define_method` table, so `[1,2,3].reverse`
evaluated to nil instead of running.

- **Resolution order** in `call_method` is now reflective built-ins → user
  `define_method` table → built-in catalog → `nil` floor.
- **Array (non-block):** `length`/`size`/`count`, `first`/`last` (±count),
  `include?`, `index`, `push`/`append`/`<<`, `pop`, `shift`, `unshift`/`prepend`,
  `reverse`, `sort`, `min`, `max`, `sum`, `uniq`, `flatten`, `compact`, `empty?`,
  `to_a`. Mutating methods mutate in place and return the Ruby-specified value;
  `reverse`/`sort` are non-mutating.
- **Object (universal):** `nil?`, `==`, `!=`, `equal?`, `respond_to?`, `freeze`,
  `frozen?`, `dup`/`clone`, `itself`, `to_a` (nil→`[]`).
- **`respond_to?` is honest** — reports true only for names dispatch actually
  resolves, so an out-of-catalog method is both `nil` *and* `respond_to? == False`.

Deferred to follow-ups: block-taking methods (`each`/`map`/`select`/…, need the
`sir-runtime-core` `apply` dependency) and the Hash/String/Numeric/Symbol catalogs.

## [0.1.0] - 2026-06-13

### Added

Initial release — the OOP runtime imported by Semantic-IR-emitted Python.
Provides the object-model semantics that have no faithful native equivalent once
the Ruby→SIR frontend has hoisted methods to detached, receiver-less top-level
functions:

- **Class registry** — `define_class(name, super_name=None)`, `superclass_of`.
- **Class identity / ancestry** — `class_of(value)` (maps Python values to Ruby
  class names, including `Integer`/`Float`/`String`/`Array`/`Hash`/`NilClass`/
  `TrueClass`/`FalseClass`), and `is_a(value, class_name)` with ancestry-chain
  walking, the `Numeric` umbrella, and `Object`/`BasicObject` universal roots
  (cycle-safe).
- **Instances** — `SirInstance` (class tag + instance-variable bag),
  `new_instance(class_name)`.
- **Instance-variable store** — `push_self`/`pop_self` (a current-self stack) +
  `ivar_get`/`ivar_set`; unset reads yield `None` (nil); `pop_self` on an empty
  stack is a safe no-op.
- **Class-variable store** — `cvar_get`/`cvar_set`.
- **Method dispatch** — `call_method(recv, name, *args)` handling the reflective
  built-ins the frontend emits as `__method__` calls (`is_a?`, `kind_of?`,
  `instance_of?`, `class`) plus a `define_method` fallback table; unknown methods
  return `None` rather than raising.
- `reset_oop()` for test isolation.

Mirrors the TypeScript `@coding-adventures/sir-runtime-oop` package
(snake_case API). `mypy --strict` + ruff clean; 14 pytest cases at 100%
coverage.

**v0 limitation (documented):** the frontend does not thread receivers into
method bodies, so the current-self is a process-global stack and class variables
share one namespace keyed by bare name. This models single-instance /
single-class programs faithfully without raising; full multi-object semantics
await frontend receiver threading. See `code/specs/sir-runtime.md`.
