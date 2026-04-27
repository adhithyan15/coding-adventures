# PR10: Prolog Builtin Goal Adapter

## Summary

Add a shared builtin-goal adapter to `prolog-loader` so parsed Prolog goals can
run against the existing Python runtime without per-call-site glue code.

## Goals

- keep parsing and loading side-effect free
- preserve the existing explicit `run_initialization_goals(...)` hook
- add a shared `adapt_prolog_goal(...)` entry point for parsed Prolog goals
- add a convenience initialization runner that uses the shared adapter by
  default
- map common Prolog builtin names onto `logic-builtins` runtime goals

## Scope

This batch covers:

- term-inspection builtins like `var/1`, `nonvar/1`, `ground/1`, `atom/1`,
  `atomic/1`, `number/1`, `string/1`, `compound/1`, and `callable/1`
- meta/runtime builtins like `call/1`, `once/1`, `not/1`, and `\\+/1`
- term-structure builtins like `functor/3`, `arg/3`, `=../2`, `==/2`,
  `compare/3`, and the standard term-order predicates
- dynamic database builtins like `dynamic/1`, `asserta/1`, `assertz/1`,
  `retract/1`, `retractall/1`, and `abolish/1`
- predicate-inspection builtins like `current_predicate/1`,
  `predicate_property/2`, and `clause/2`

## Design

- `prolog-loader.adapters` owns the shared Prolog-to-runtime builtin lowering
- unmapped goals remain unchanged so ordinary user predicates still execute
  through the loaded `Program`
- declaration-style builtins that consume predicate indicators should support
  both `name/arity` and proper lists of indicators where that form is valid
- `run_prolog_initialization_goals(...)` should simply delegate to
  `run_initialization_goals(...)` with `adapt_prolog_goal(...)`

## Validation

- loader package lint passes
- loader tests cover direct initialization execution with real Prolog builtin
  names instead of handwritten adapter shims
- end-to-end tests prove dynamic declaration, assertion, meta-call, and
  predicate-property checks work through the loader boundary
