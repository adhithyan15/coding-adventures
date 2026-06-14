# PR83 - Prolog Stream Term I/O

## Goal

Close the parser-backed term I/O gap for the bounded UTF-8 stream facade without
pretending to implement every ISO/SWI stream feature.

This batch adds:

- `read/1`
- `read/2`
- `read_term/2`
- `read_term/3`
- `write_term/2`
- `write_term/3`

## Semantics

`read(Stream, Term)` and `read_term(Stream, Term, Options)` accept an open
bounded read stream handle or alias. They skip leading layout/comments, consume
one dot-terminated Prolog term, advance the stream cursor past trailing layout,
and parse the consumed term through the SWI-compatible term parser used by
`read_term_from_atom/3`.

`read(Term)` and `read_term(Term, Options)` delegate to the selected current
input stream from PR82. At end of file, the read predicates unify `Term` with
`end_of_file` and leave the cursor at EOF.

`read_term` supports the same finite option subset as `read_term_from_atom/3`:

- `variable_names(Names)`
- `variables(Vars)`

`write_term(Stream, Term, Options)` writes the finite term renderer used by
`write_term_to_atom/3` to an open bounded write/append stream handle or alias.
`write_term(Term, Options)` writes to the selected current output stream.
Supported write options remain the bounded subset:

- `quoted(true|false)`
- `ignore_ops(true|false)`
- `numbervars(true|false)`

As with earlier stream batches, invalid handles, output-stream reads,
input-stream writes, malformed option lists, unterminated stream terms, and
unparseable term text fail deterministically.

## Validation

Coverage should prove:

- source-level Prolog calls adapt explicit and current stream term reads/writes
  through the parser-backed loader layer.
- stream reads advance one term at a time and return `end_of_file` after the
  final term.
- `read_term` preserves `variable_names/1` and `variables/1` bindings.
- structured VM and bytecode VM run matching stream term I/O programs.
- the capability manifest records PR83 as complete while leaving
  console-backed standard streams, binary streams, rich ISO/SWI options,
  foreign predicates, and async host services deferred.

## Non-goals

- no console-backed `user_input`, `user_output`, or `user_error` streams
- no binary streams or encodings beyond UTF-8
- no full ISO/SWI reader option matrix
- no operator-sensitive stream parser beyond the existing SWI-compatible term
  parser subset
- no foreign predicate or async host callback boundary
