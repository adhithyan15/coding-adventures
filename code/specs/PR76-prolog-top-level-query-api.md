# PR76: Prolog Top-Level Query API

## Goal

Make the Prolog-on-Logic-VM stack usable as a Python library without requiring
callers to embed `?-` source queries or manually create a stateful runtime for
one-off questions. A caller should be able to provide source, a file, or a
linked project plus a top-level query string and get back named Prolog answers
or raw answer values.

## API Shape

- `query_prolog_source(...)` and `query_prolog_source_values(...)` load source
  text and answer one ad-hoc query.
- `query_swi_prolog_source(...)`, `query_swi_prolog_source_values(...)`,
  `query_iso_prolog_source(...)`, and `query_iso_prolog_source_values(...)`
  provide dialect-specific source wrappers.
- `query_prolog_file(...)` and `query_prolog_file_values(...)` load one file
  and answer one ad-hoc query.
- `query_swi_prolog_file(...)` and `query_swi_prolog_file_values(...)` provide
  SWI-compatible file wrappers.
- `query_prolog_project(...)` and `query_prolog_project_values(...)` load linked
  source strings before answering an ad-hoc query.
- `query_swi_prolog_project(...)` and
  `query_swi_prolog_project_values(...)` provide SWI-compatible linked-source
  wrappers.
- `query_prolog_project_file(...)` and
  `query_prolog_project_file_values(...)` load linked file graphs before
  answering an ad-hoc query.
- `query_swi_prolog_project_file(...)` and
  `query_swi_prolog_project_file_values(...)` provide SWI-compatible file-graph
  wrappers.

All helpers accept `backend="structured"` or `backend="bytecode"` and default
to `initialize=True` so initialization directives behave like consulted Prolog
program startup. Project helpers accept `query_module=...` so top-level queries
can resolve through module imports.

## Why This Matters

The previous one-shot runner APIs were excellent for source files that already
contain `?-` directives. Python callers still needed boilerplate when the query
was supplied interactively, through a UI, or by another program. This API closes
that library ergonomics gap while preserving the same runtime path used by
stateful query runtimes.

## Validation

Coverage should prove:

- source top-level queries return named answers and raw values
- initialization directives run before ad-hoc queries by default
- ISO/Core source wrappers route through the ISO dialect
- file top-level queries work through the bytecode backend
- linked source and linked file project helpers respect module query context
