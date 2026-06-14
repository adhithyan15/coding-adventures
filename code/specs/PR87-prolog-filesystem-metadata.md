# PR87 - Prolog Filesystem Metadata

## Goal

Close the bounded read-only filesystem metadata gap on top of the existing
host file I/O facade. This batch gives Prolog programs portable path
inspection and metadata predicates while keeping host effects deterministic and
non-mutating.

This batch adds:

- `exists_directory/1`
- `absolute_file_name/2`
- `access_file/2`
- `file_directory_name/2`
- `file_base_name/2`
- `directory_file_path/3`
- `file_name_extension/3`
- `same_file/2`
- `size_file/2`
- `time_file/2`

## Semantics

All predicates accept bound atom/string path values at the host boundary.
Invalid path shapes, missing required files, unsupported modes, and operating
system failures fail deterministically.

`exists_directory/1` succeeds for existing directories. `access_file/2`
supports the bounded access modes `read`, `write`, `execute`, and `exist`.
`absolute_file_name/2` relates a bound path to a deterministic absolute path
atom without requiring the target to exist.

`file_directory_name/2`, `file_base_name/2`, `directory_file_path/3`, and
`file_name_extension/3` provide finite path decomposition and composition
modes. `directory_file_path/3` supports bound directory/file to path and bound
path to directory/file. `file_name_extension/3` supports bound name to
base/extension and bound base/extension to name.

`same_file/2` succeeds when two bound paths refer to the same existing
filesystem object. `size_file/2` relates a bound regular file path to its byte
size. `time_file/2` relates a bound existing path to its modification timestamp.

## Validation

Coverage should prove:

- direct logic-builtin goals expose the metadata and path decomposition facade.
- source-level Prolog calls adapt through `prolog-loader`.
- structured VM and bytecode VM run matching filesystem metadata programs.
- the capability manifest records PR87 as complete while leaving mutable
  filesystem services, console streams, rich stream options, foreign predicates,
  engines, and async host services deferred.

## Non-goals

- no filesystem mutation predicates such as delete, rename, make-directory, or
  working-directory changes
- no directory enumeration predicates
- no ISO/SWI exception taxonomy; invalid bounded operations fail
  deterministically
- no permission model beyond the host process and bounded `access_file/2` mode
  checks
- no foreign predicate or async host callback boundary
