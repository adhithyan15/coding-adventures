# PR84 - Prolog Term I/O Conveniences

## Goal

Close the next practical term I/O gap by adding the common reader and writer
conveniences that Prolog programs expect on top of the bounded parser-backed
term I/O layer from PR83.

This batch adds:

- `read_term` `singletons/1`
- `writeq/1`
- `writeq/2`
- `write_canonical/1`
- `write_canonical/2`
- `writeln/1`
- `writeln/2`
- `portray_clause/1`
- `portray_clause/2`

## Semantics

`read_term_from_atom/3`, `read_term/2`, and `read_term/3` support
`singletons(Singletons)` in the same finite parser-backed option subset as
`variable_names/1` and `variables/1`. The value is a list of `Name = Var`
pairs for named variables that occur exactly once in the parsed term.

`writeq/1` and `writeq/2` render through `write_term` with `quoted(true)`.
`write_canonical/1` and `write_canonical/2` render through `write_term` with
`ignore_ops(true)` and `numbervars(true)`. `writeln/1` and `writeln/2` render a
term and then emit a newline.

`portray_clause/1` and `portray_clause/2` render a term in canonical
numbervars-aware form, append a full stop, and terminate the clause with a
newline. The arity-one forms target the selected current output stream from
PR82; the arity-two forms target an explicit bounded output stream handle or
alias.

The predicates intentionally inherit the finite bounded stream model from
PR79-PR83. Invalid stream handles, input-stream writes, malformed option lists,
unparseable term text, and unsupported rich SWI/ISO reader/writer options fail
deterministically.

## Validation

Coverage should prove:

- source-level Prolog calls adapt singleton-aware term reads through the
  parser-backed loader layer.
- source-level writer convenience predicates lower to executable bounded stream
  writes for explicit and current output streams.
- quoted, canonical numbervars, newline, and clause-terminating rendering all
  produce deterministic text.
- structured VM and bytecode VM run matching term I/O convenience programs.
- the capability manifest records PR84 as complete while leaving
  console-backed standard streams, binary streams, rich ISO/SWI options,
  foreign predicates, and async host services deferred.

## Non-goals

- no console-backed `user_input`, `user_output`, or `user_error` streams
- no binary streams or encodings beyond UTF-8
- no full ISO/SWI reader or writer option matrix
- no custom portray hooks or module-sensitive operator printing
- no foreign predicate or async host callback boundary
