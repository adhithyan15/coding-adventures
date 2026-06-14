# PR75: Prolog File And Project Runner API

## Goal

Close the usability gap between source-string one-shot runners and real Prolog
program entry points. Callers should be able to run source queries embedded in
`.pl` files, linked source projects, and linked file graphs without manually
threading together compile and VM execution helpers.

## API Shape

- `run_prolog_file_query(...)` and `run_prolog_file_query_answers(...)` read one
  source file, compile it, and execute one source query by index.
- `run_swi_prolog_file_query(...)` and
  `run_swi_prolog_file_query_answers(...)` provide SWI-compatible convenience
  wrappers for the file path.
- `run_prolog_project_query(...)` and
  `run_prolog_project_query_answers(...)` link multiple source strings before
  executing one source query.
- `run_swi_prolog_project_query(...)` and
  `run_swi_prolog_project_query_answers(...)` provide SWI-compatible convenience
  wrappers for linked source strings.
- `run_prolog_project_file_query(...)` and
  `run_prolog_project_file_query_answers(...)` load and link entry files,
  including consulted or included dependencies, before executing one source
  query.
- `run_swi_prolog_project_file_query(...)` and
  `run_swi_prolog_project_file_query_answers(...)` provide SWI-compatible
  convenience wrappers for linked file graphs.

All helpers accept `backend="structured"` or `backend="bytecode"`. Helpers that
read files accept `source_resolver=...` so callers can preserve loader-level
library resolution. All helpers accept `initialize=True` when initialization
directives should run before the selected query slot.

## Why This Matters

The Prolog-on-Logic-VM stack already supports parsing, loading, compiling,
initialization, structured execution, bytecode execution, named answers, and
linked projects. This layer makes those capabilities feel like a usable
library-facing Prolog runner instead of a collection of lower-level building
blocks.

## Validation

The package should cover:

- one-shot file execution through the bytecode backend
- one-shot file execution after initialization directives
- one-shot linked source project execution with module imports
- one-shot linked source project raw-value results
- one-shot linked file graph execution with consulted dependencies
