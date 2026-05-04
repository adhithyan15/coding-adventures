# PR21: Prolog VM Module Query Runtime

## Summary

This batch lets stateful Prolog VM runtimes answer ad-hoc top-level queries in a
linked module context.

Before this layer, source-level `?-` queries embedded in a project were
module-aware, but later runtime queries were parsed directly and could only
address global predicates reliably. This adds a public loader query rewrite hook
and threads it into the VM runtime so imported predicates resolve the same way
they do during project linking.

## Goals

- expose module-aware ad-hoc query rewriting from `prolog-loader`
- let project runtimes choose a default query module
- resolve imported predicates for later runtime query strings
- preserve explicit `module:goal` support through the existing rewrite path
- keep non-project and single-file runtimes unchanged

## Public API

`prolog-loader` now exposes:

```python
rewrite_loaded_prolog_query(...)
```

`prolog-vm-compiler` project runtime constructors now accept:

```python
query_module="app"
```

Example:

```python
from prolog_vm_compiler import create_swi_prolog_project_file_runtime

runtime = create_swi_prolog_project_file_runtime("app.pl", query_module="app")

answers = runtime.query("ancestor(homer, Who)")
```

If `app.pl` imports `ancestor/2` from another module, the runtime query is
rewritten to the linked module-qualified relation before execution.

## Non-goals

- REPL commands such as `consult/1` typed at the query prompt
- dynamic runtime changes to a module's import table
- package/library search policies beyond the existing `SourceResolver` hook
