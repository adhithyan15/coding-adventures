# PR86 - Prolog Binary Byte Stream I/O

## Goal

Close the bounded binary file stream gap on top of the existing stream
facade. This batch keeps host I/O deterministic and file-backed while adding
the byte-oriented predicates needed by binary Prolog programs.

This batch adds:

- `open/4` with `type(binary)`
- `stream_property/2` support for `type(text)` and `type(binary)`
- `get_byte/1`
- `get_byte/2`
- `peek_byte/1`
- `peek_byte/2`
- `put_byte/1`
- `put_byte/2`

## Semantics

`open(Path, Mode, Stream, Options)` accepts `type(binary)` for file-backed
read, write, and append streams. `open/3` and omitted `type/1` options keep the
existing UTF-8 text behavior. `encoding/1` remains a text-stream option and
fails deterministically when combined with `type(binary)`.

The arity-two byte read predicates accept an open bounded binary read stream
handle or alias. The arity-one forms use the selected current input stream from
PR82. `get_byte` reads one byte and advances the cursor. `peek_byte` inspects
the next byte without advancing. Both predicates unify with `-1` at EOF.

The arity-two write predicates accept an open bounded binary write/append stream
handle or alias. The arity-one forms use the selected current output stream from
PR82. `put_byte` writes an integer in the inclusive range `0..255`.

Positioning and EOF predicates operate over byte offsets for binary streams and
keep code-point offsets for text streams.

Invalid stream handles, reads from output streams, writes to input streams,
byte operations on text streams, text operations on binary streams, non-integer
bytes, and out-of-range bytes fail deterministically.

## Validation

Coverage should prove:

- direct logic-builtin goals preserve binary cursor movement, peeking,
  selected current streams, and output bytes.
- source-level Prolog calls adapt explicit and current byte stream predicates
  through the loader layer.
- structured VM and bytecode VM run matching byte stream programs.
- the capability manifest records PR86 as complete while leaving
  console-backed streams, rich stream options, foreign predicates, and async
  host services deferred.

## Non-goals

- no console-backed `user_input`, `user_output`, or `user_error` streams
- no binary stream encodings, repositioning policies, or buffering options
  beyond bounded file-backed byte offsets
- no block byte-array predicates such as `read_pending_codes/3`
- no rich stream exception taxonomy; invalid bounded operations fail
  deterministically
- no foreign predicate or async host callback boundary
