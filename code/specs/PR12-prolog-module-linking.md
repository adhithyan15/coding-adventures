# PR12 - Prolog Module Metadata And Linking

## Overview

The project can now parse and execute a meaningful Prolog subset, but all
loaded sources still behave like one flat predicate namespace. This batch adds
the first real module-aware loading slice, focused on the most common SWI-style
workflow:

- declare a module with `module/2`
- import exported predicates with `use_module/1` or `use_module/2`
- link multiple loaded sources into one runnable project

## Scope

### Shared metadata in `prolog-core`

Add shared frontend/runtime metadata for:

- `PrologModule`
- `PrologModuleImport`
- `module_spec_from_directive(...)`
- `module_import_from_directive(...)`

This first slice supports:

- `module(Name, ExportList)` where `Name` is an atom
- export lists containing predicate indicators `name/arity`
- export lists containing `op(Precedence, Type, Name)` operator declarations
- `use_module(Module)`
- `use_module(Module, ImportList)` where `ImportList` is a list of predicate
  indicators

It intentionally does not yet implement the wider SWI import option space such
as `except(...)` or renaming with `as`.

### Loader integration

`LoadedPrologSource` now retains:

- one parsed module declaration, when present
- parsed `use_module` directives

Add a new linked-project layer:

- `LoadedPrologProject`
- `link_loaded_prolog_sources(...)`
- `load_swi_prolog_project(...)`

The linker:

- namespaces local predicates by module
- resolves imported predicates against exported predicates of known modules
- keeps local definitions stronger than weak imports
- rewrites queries and initialization goals through the same resolution logic

## Non-Goals

This batch does not yet implement:

- parse-time operator importing from other modules
- `reexport/1` or `reexport/2`
- `use_module/2` renaming or exclusion options
- explicit qualified call syntax such as `module:goal`
- module-sensitive meta-predicate semantics
- full file/path resolution for module loading

Those are natural follow-up batches once the shared metadata and project linker
exist.
