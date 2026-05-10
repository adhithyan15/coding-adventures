# PR80 - Prolog Stream Options and Metadata

## Goal

Extend the bounded UTF-8 file stream facade from PR79 with the practical
metadata and option subset needed by source-level Prolog programs that name,
inspect, and flush file streams.

This batch keeps stream support intentionally bounded while adding:

- `open/4`
- `current_stream/3`
- `stream_property/2`
- `flush_output/1`
- alias-aware stream operations

## Semantics

`open(Path, Mode, Stream, Options)` accepts the PR79 path and mode subset plus
a finite option list. Supported options are:

- `alias(Name)`, where `Name` is a non-handle atom
- `encoding(utf8)` or `encoding('utf-8')`
- `type(text)`

Unsupported options fail deterministically instead of being silently ignored.
Aliases are unique among open streams. Once opened, the alias can be used in
place of the opaque stream handle for `close/1`, `read_string/3`,
`read_line_to_string/2`, `get_char/2`, `at_end_of_stream/1`, `write/2`,
`nl/1`, `flush_output/1`, and `stream_property/2`.

`current_stream(Path, Mode, Stream)` enumerates currently open bounded streams.
`stream_property(Stream, Property)` exposes finite metadata including
`file_name(Path)`, `mode(Mode)`, `position(Position)`, `input`, `output`,
`end_of_stream(at|not)`, `alias(Alias)`, and `handle(Handle)`.
`flush_output(Stream)` validates a write/append stream and succeeds; writes are
already flushed because the bounded facade writes each operation through the
host file system.

## Validation

Coverage should prove:

- Python builtin helpers support alias options, metadata, enumeration, and
  flush validation.
- source-level Prolog calls adapt to the builtin layer.
- structured VM and bytecode VM produce matching answers for option/metadata
  stream queries.
- the capability manifest records PR80 as complete while leaving standard
  streams, binary streams, rich ISO/SWI options, foreign
  predicates, and async host services deferred.

## Non-goals

- no standard stream aliases
- no binary streams or encodings beyond UTF-8
- parser-backed `read/1`, `read_term/2`, and stream term parsing are deferred
  to PR83
- no foreign predicate or async host callback boundary
