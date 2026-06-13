# Changelog

All notable changes to `@coding-adventures/sir-runtime-oop` are documented here.

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
