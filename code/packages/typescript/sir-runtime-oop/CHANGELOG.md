# Changelog

All notable changes to `@coding-adventures/sir-runtime-oop` are documented here.

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
