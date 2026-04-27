# PR09 - Prolog Loader And Explicit `initialization/1` Execution

## Overview

PR08 added predicate-property directives and started collecting structured
`initialization/1` metadata, but the system still stopped at “parsed source”.
This batch adds the first shared loading layer above the dialect frontends.

The loader keeps parsing side-effect free, then exposes an explicit API that:

- normalizes dialect-specific parsed sources into one loaded shape
- collects startup directives in source order
- runs those initialization goals later and on purpose

## Scope

### New `prolog-loader` package

Add a new Python package that exports:

- `LoadedPrologSource`
- `load_parsed_prolog_source(parsed_source)`
- `load_iso_prolog_source(source, *, operator_table=None)`
- `load_swi_prolog_source(source, *, operator_table=None)`
- `run_initialization_goals(loaded_source, *, state=None, goal_adapter=None)`
- `PrologInitializationError`

`LoadedPrologSource` retains the parsed executable artifacts:

- `program`
- `clauses`
- `queries`
- `directives`
- `operator_table`
- `predicate_registry`

and adds derived loader-facing startup views:

- `initialization_directives`
- `initialization_goals`
- `initialization_terms`

### Explicit startup execution

`run_initialization_goals(...)` executes initialization goals in source order
against the loaded `Program`.

Important behavior:

- parsing still does not auto-run startup code
- startup goals run only when the caller asks
- each startup goal starts from the state produced by the previous one
- failure raises `PrologInitializationError`
- callers may supply `goal_adapter` to rewrite parsed goals into richer runtime
  or builtin goals before execution

The adapter hook is how we can start bridging parser-lowered goals into more
complete Prolog runtime behavior without hardwiring every future builtin into
the parser or loader immediately.

## Non-Goals

This PR does not yet implement:

- automatic builtin translation from plain Prolog callable terms
- module-aware loading
- `consult/1` or file inclusion
- DCG expansion
- auto-running `initialization/1` during parsing

Those follow naturally on top of the loader boundary introduced here.
