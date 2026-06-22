# Changelog

All notable changes to `coding-adventures-sir-runtime-oop` are documented here.

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
