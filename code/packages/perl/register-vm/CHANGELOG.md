# Changelog

All notable changes to `CodingAdventures::RegisterVM` will be documented here.

## [0.1.0] - 2026-04-06

### Added

- Initial implementation of the register-based VM with accumulator model.
- `CodingAdventures::RegisterVM` — main module with dispatch-table executor.
  - `new(%args)` constructor with configurable `max_depth`.
  - `run($code, $globals)` class-method convenience wrapper.
  - `execute($code, $globals)` instance method for reuse across calls.
- `CodingAdventures::RegisterVM::Opcodes` — ~70 opcode constants grouped by
  category (load, move, global, arithmetic, compare, jump, call, property,
  create, iterator, exception, context, meta).
- `CodingAdventures::RegisterVM::Feedback` — four-state feedback-slot state
  machine (uninitialized → monomorphic → polymorphic → megamorphic) with
  type-pair deduplication.
- `CodingAdventures::RegisterVM::Scope` — linked-list lexical scope chain
  supporting depth-indexed slot access for context variables.
- Hidden-class registry: stable integer IDs assigned to objects by their
  property keyset, enabling inline-cache type profiling.
- Call-depth limiting via `STACK_CHECK` opcode and `$vm->{call_depth}`.
- Full test suite in `t/01-basic.t` covering all ten required scenarios.
