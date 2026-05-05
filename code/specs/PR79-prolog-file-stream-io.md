# PR79 - Prolog File Stream I/O

## Goal

Close the practical stream-handle portion of the Prolog-on-Logic-VM host I/O
gap without pretending to implement every ISO/SWI stream facility.

This batch adds a bounded UTF-8 file stream facade that runs through the same
parsed Prolog source, builtin adapter, structured VM, and bytecode VM path as
the completed core:

- `open/3`
- `close/1`
- `at_end_of_stream/1`
- `read_string/3`
- `read_line_to_string/2`
- `get_char/2`
- `write/2`
- `nl/1`

## Semantics

`open(Path, Mode, Stream)` accepts a bound atom/string path and a mode atom of
`read`, `write`, or `append`. It returns an opaque stream handle atom that can
be passed to the other bounded stream predicates.

Read streams load UTF-8 text from an existing regular file and maintain a
cursor across consecutive stream calls. Invalid paths, unreadable files,
directories, unsupported modes, and invalid UTF-8 fail deterministically.

Write streams truncate the target file on open and append textual writes until
closed. Append streams create the file if needed and append textual writes to
the existing contents.

`read_string(Stream, Length, String)` reads up to `Length` Unicode code points
from the current stream cursor. `read_line_to_string(Stream, String)` reads one
line without the trailing newline, or returns `end_of_file` at EOF.
`get_char(Stream, Char)` reads one character atom or `end_of_file`.
`at_end_of_stream(Stream)` succeeds once when a read stream cursor has consumed
all available text. `write(Stream, Term)` writes a bounded textual rendering of
atoms, strings, numbers, and compounds. `nl(Stream)` writes a newline.

## Validation

Coverage should prove:

- Python builtin helpers preserve stream cursors across consecutive reads.
- write-mode streams flush text to UTF-8 files.
- source-level Prolog calls adapt to the builtin layer.
- structured VM and bytecode VM produce matching answers for stream queries.
- the capability manifest records PR79 as complete while leaving standard
  streams, binary streams, repositioning, aliases, rich options, foreign
  predicates, and async host services deferred.

## Non-goals

- no `current_input/1`, `current_output/1`, standard stream aliases, or
  implicit one-argument `write/1`/`nl/0`
- no stream options, aliases, repositioning, binary I/O, or encodings beyond
  UTF-8
- no `read/1`, `read_term/2`, or term parsing from stream handles
- no directory enumeration
- no foreign predicate or async host callback boundary
