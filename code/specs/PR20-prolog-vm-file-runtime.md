# PR20: Prolog VM File Runtime

## Summary

This batch makes the Prolog VM compiler usable with real source files, not just
in-memory source strings.

The VM path already supports stateful ad-hoc queries once a program is loaded.
This layer lets callers point that same runtime at `.pl` files and file graphs
loaded by `prolog-loader`, including include expansion and linked project source
queries.

## Goals

- compile one SWI-compatible Prolog file into a Logic VM program
- compile a linked SWI-compatible project file graph into a Logic VM program
- create a reusable stateful runtime from one source file
- create a reusable stateful runtime from linked in-memory sources or project
  entry files
- preserve operator tables from loaded sources for later ad-hoc query parsing
- keep initialization behavior identical to the existing source-string runtime

## Public API

`prolog-vm-compiler` now exposes:

```python
compile_swi_prolog_file(...)
compile_swi_prolog_project_from_files(...)
create_swi_prolog_file_runtime(...)
create_swi_prolog_project_runtime(...)
create_swi_prolog_project_file_runtime(...)
```

Example:

```python
from prolog_vm_compiler import create_swi_prolog_file_runtime

runtime = create_swi_prolog_file_runtime("app.pl")

answers = runtime.query("ancestor(homer, Who)")
```

## Runtime Behavior

`create_swi_prolog_file_runtime(...)`:

1. reads and loads one SWI-compatible source file
2. expands supported file-local directives such as `include/1`
3. compiles the loaded source into `logic-instructions`
4. loads those instructions into `logic-vm`
5. optionally runs initialization query slots
6. returns a reusable `PrologVMRuntime`

`compile_swi_prolog_project_from_files(...)` uses the loader's file graph
resolver for `consult/1`, `use_module/1`, and related supported directives, then
links source-level queries through the existing module-aware project linker.

## Non-goals

- Prolog REPL commands such as `consult/1` issued as interactive commands
- full module/import-aware resolution for later ad-hoc top-level query strings
- package manager integration for locating third-party Prolog libraries
