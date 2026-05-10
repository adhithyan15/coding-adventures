# PR82 - Prolog Current Streams

## Goal

Extend the bounded UTF-8 stream facade with selected current input/output
streams so source-level programs can use common implicit stream predicates
after explicitly choosing an opened file-backed stream.

This batch adds:

- `set_input/1`
- `set_output/1`
- `current_input/1`
- `current_output/1`
- current-input forms for `get_char/1`, `read_string/2`,
  `read_line_to_string/1`, and `at_end_of_stream/0`
- current-output forms for `write/1`, `nl/0`, and `flush_output/0`

## Semantics

`set_input(Stream)` accepts an open bounded read stream handle or alias and
selects it as the current input stream. `set_output(Stream)` accepts an open
bounded write or append stream handle or alias and selects it as the current
output stream.

`current_input(Stream)` and `current_output(Stream)` unify with the selected
opaque stream handles. They fail deterministically when no bounded stream has
been selected or the selected stream has been closed. Closing a selected stream
clears that selection.

Current-input predicates delegate to the same cursor semantics as their
explicit-stream counterparts:

- `get_char(Char)` reads one character atom or `end_of_file`
- `read_string(Length, String)` reads up to `Length` code points
- `read_line_to_string(String)` reads one line or `end_of_file`
- `at_end_of_stream` succeeds when the selected input stream is exhausted

Current-output predicates delegate to the same file-backed write semantics as
their explicit-stream counterparts:

- `write(Term)` writes the bounded textual representation of `Term`
- `nl` writes a newline
- `flush_output` validates that a current output stream exists; writes are
  already flushed by the bounded host facade

`stream_property(Stream, current_input)` and
`stream_property(Stream, current_output)` expose the current selections.

## Validation

Coverage should prove:

- Python builtin helpers can select current input/output streams, use implicit
  read/write forms, and expose current stream properties.
- source-level Prolog calls adapt to the builtin layer.
- structured VM and bytecode VM produce matching answers for current-stream
  queries.
- the capability manifest records PR82 as complete while leaving
  console-backed standard streams, binary streams, rich ISO/SWI options,
  foreign predicates, and async host services deferred.

## Non-goals

- no console-backed `user_input`, `user_output`, or `user_error` streams
- no binary streams or encodings beyond UTF-8
- no output-stream random-access writes
- parser-backed `read/1`, `read_term/2`, and stream term parsing are deferred
  to PR83
- no foreign predicate or async host callback boundary
