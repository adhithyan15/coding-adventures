# PR85 - Prolog Stream Character and Code I/O

## Goal

Close the bounded UTF-8 character/code stream I/O gap on top of the existing
file stream, current stream, positioning, and term I/O layers without widening
the host runtime beyond deterministic text streams.

This batch adds:

- `get_code/1`
- `get_code/2`
- `peek_char/1`
- `peek_char/2`
- `peek_code/1`
- `peek_code/2`
- `put_char/1`
- `put_char/2`
- `put_code/1`
- `put_code/2`

## Semantics

The arity-two read predicates accept an open bounded read stream handle or
alias. The arity-one forms use the selected current input stream from PR82.

`get_code` reads one Unicode code point and advances the stream cursor. At EOF
it unifies with `-1`. `peek_char` and `peek_code` inspect the next character or
code point without advancing. `peek_char` follows `get_char`'s EOF convention
and unifies with `end_of_file`; `peek_code` follows `get_code` and unifies with
`-1`.

The arity-two write predicates accept an open bounded write/append stream handle
or alias. The arity-one forms use the selected current output stream from PR82.
`put_char` writes a one-character atom. `put_code` writes a non-negative Unicode
code point.

Invalid stream handles, reads from output streams, writes to input streams,
multi-character atoms, non-integer codes, negative codes, and invalid Unicode
code points fail deterministically.

## Validation

Coverage should prove:

- direct logic-builtin goals preserve cursor movement and peeking behavior.
- source-level Prolog calls adapt explicit and current character/code stream
  predicates through the loader layer.
- structured VM and bytecode VM run matching character/code stream programs.
- the capability manifest records PR85 as complete while leaving
  console-backed standard streams, binary streams, rich ISO/SWI stream options,
  foreign predicates, and async host services deferred.

## Non-goals

- no console-backed `user_input`, `user_output`, or `user_error` streams
- no binary byte streams or encodings beyond UTF-8
- no `get_byte/1,2`, `peek_byte/1,2`, or `put_byte/1,2`
- no rich stream exception taxonomy; invalid bounded operations fail
  deterministically
- no foreign predicate or async host callback boundary
