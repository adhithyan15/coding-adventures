# PR88 - Prolog Filesystem Operations

## Goal

Close the bounded filesystem operation gap that remains after PR87 metadata.
This batch gives Prolog programs deterministic directory enumeration and
explicit path mutation predicates without adding recursive deletion, globbing,
shell execution, or ambient host callbacks.

This batch adds:

- `directory_files/2`
- `make_directory/1`
- `delete_file/1`
- `delete_directory/1`
- `rename_file/2`
- `working_directory/2`

## Semantics

All path arguments must be bound atoms or strings. Invalid path shapes, missing
inputs, existing outputs, non-empty directories, and operating system failures
fail deterministically.

`directory_files(Directory, Entries)` relates a bound directory path to a
sorted list of entry-name atoms. The bounded subset reports actual entries in
that directory and does not synthesize `.` or `..`.

`make_directory/1` creates one directory level when the immediate parent exists.
`delete_directory/1` removes one empty directory. `delete_file/1` removes one
bound file path. `rename_file/2` renames one bound filesystem path to another
bound path.

`working_directory(Old, New)` unifies `Old` with the process working directory
before changing to the bound directory `New`. This operation is intentionally
explicit because the underlying host process working directory is global.

## Validation

Coverage should prove:

- direct logic-builtin goals mutate only explicitly bound temp paths and restore
  process working directory state in tests.
- source-level Prolog calls adapt through `prolog-loader`.
- structured VM and bytecode VM run matching filesystem operation programs.
- the capability manifest records PR88 as complete while leaving recursive or
  wildcard filesystem services, console streams, rich stream options, foreign
  predicates, engines, and async host services deferred.

## Non-goals

- no recursive directory deletion
- no globbing, wildcard expansion, or directory walking
- no shell/process predicates
- no ISO/SWI exception taxonomy; invalid bounded operations fail
  deterministically
- no sandbox or capability policy beyond explicit bound path arguments and the
  host process permissions
