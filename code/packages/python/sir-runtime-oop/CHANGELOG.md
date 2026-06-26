# Changelog

All notable changes to `coding-adventures-sir-runtime-oop` are documented here.

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
