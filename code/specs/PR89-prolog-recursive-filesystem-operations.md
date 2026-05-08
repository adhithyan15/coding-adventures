# PR89 - Prolog Recursive Filesystem Operations

## Overview

PR88 added explicit one-path filesystem operations. PR89 closes the remaining
practical host-filesystem gap by adding bounded recursive and wildcard helpers
that still require instantiated atom/string paths.

This batch adds:

- `expand_file_name/2`
- `make_directory_path/1`
- `delete_directory_and_contents/1`
- `copy_file/2`

The predicates are exposed through `logic-builtins`, adapted from source-level
Prolog calls by `prolog-loader`, and covered through both structured and
bytecode Prolog VM execution.

## Semantics

`expand_file_name(Pattern, Matches)` requires `Pattern` to be a bound atom or
string. It expands `~`, evaluates ordinary glob wildcards including recursive
`**`, and unifies `Matches` with a deterministic sorted list of path atoms.

`make_directory_path(Path)` requires `Path` to be bound and creates missing
parent directories. It succeeds when the resulting path is a directory.

`delete_directory_and_contents(Path)` requires `Path` to be bound and removes
that explicit directory tree recursively. It fails when the path is unbound or
does not name a directory.

`copy_file(Source, Target)` requires both paths to be bound. It copies one
regular file to the target path and succeeds when the target becomes a regular
file.

## Boundaries

These helpers are intentionally direct host operations, not a sandbox or virtual
filesystem abstraction. They are suitable for tests, tools, and trusted local
automation that already controls the paths it passes in.

Deferred work remains outside this batch:

- console-backed standard streams
- rich stream options beyond the bounded text/binary subset
- foreign predicates and host callbacks
- engines, concurrency, and async host integration
