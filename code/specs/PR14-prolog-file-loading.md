# PR14: Prolog File Loading

## Summary

This batch moves the SWI loader from in-memory source strings to real file
graphs.

It adds explicit source-path metadata to loaded sources, records file
dependencies referenced by directives, and introduces a recursive
`load_swi_prolog_project_from_files(...)` entry point that reads `.pl` files and
links the resulting sources into one runnable project.

## Goals

- load one SWI source file with its absolute source path preserved
- collect file dependencies from:
  - `consult/1`
  - `ensure_loaded/1`
  - file-backed `use_module/1`
  - file-backed `use_module/2`
- resolve relative file references from the current source file directory
- support bare atom sibling references such as `use_module(family, [...])` by
  resolving them to `family.pl`
- normalize file-backed `use_module` edges into ordinary module imports before
  project linking
- keep the existing string-based loader APIs working unchanged

## Design

`LoadedPrologSource` now carries:

- `source_path`
- `file_dependencies`

Each dependency records:

- the directive kind
- the original requested reference text
- the resolved absolute file path
- optional import metadata for `use_module/2`

The new `load_swi_prolog_project_from_files(...)` helper walks the dependency
graph, loads each file once, rewrites file-backed `use_module` edges into
module-name imports, and then reuses `link_loaded_prolog_sources(...)`.

## Non-goals

- `library(...)` resolution
- search paths and Prolog file aliases
- `include/1`
- module-qualified operator import semantics
- consulting a non-module source directly into another module's local namespace
