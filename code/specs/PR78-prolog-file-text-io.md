# PR78 - Prolog File Text I/O

## Goal

Close the first host file I/O gap in the Prolog-on-Logic-VM path without
introducing a full stream subsystem yet.

This batch adds deterministic, bounded predicates for reading UTF-8 text files
from bound atom/string paths:

- `exists_file/1`
- `read_file_to_string/2`
- `read_file_to_codes/2`

The predicates run through the same parsed Prolog source, builtin adapter,
Logic VM, and Logic bytecode VM path as the rest of the completed core.

## Semantics

`exists_file(Path)` succeeds once when `Path` is a bound atom or string naming
an existing regular file. It fails for unbound paths, non-text path terms,
missing files, and directories.

`read_file_to_string(Path, String)` reads a bound atom/string path as UTF-8 and
unifies `String` with a logic string term containing the entire file contents.
It fails for missing files, unreadable files, directories, invalid UTF-8, and
non-bound path terms.

`read_file_to_codes(Path, Codes)` reads a bound atom/string path as UTF-8 and
unifies `Codes` with a proper list of Unicode code-point numbers.

These predicates are intentionally non-enumerating: they do not generate paths
from open variables, and they do not scan directories.

## Validation

Coverage should prove:

- Python builtin helpers read UTF-8 text through atom and string paths.
- missing files fail without throwing host exceptions through ordinary query
  execution.
- source-level Prolog calls adapt to the builtin layer.
- structured VM and bytecode VM produce matching answers.
- the capability manifest records PR78 as complete while leaving full stream
  handles as deferred host runtime work.

## Non-goals

- no stream handles
- no `open/3`, `close/1`, `read/1`, or `write/1`
- no stream options, aliases, repositioning, binary I/O, or encodings beyond
  UTF-8
- no directory enumeration
- no foreign predicate or async host callback boundary
