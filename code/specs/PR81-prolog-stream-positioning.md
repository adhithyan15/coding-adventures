# PR81 - Prolog Stream Positioning

## Goal

Extend the bounded UTF-8 read-stream facade from PR79/PR80 with deterministic
cursor repositioning so source-level programs can save, restore, and seek
within open text streams.

This batch adds:

- `set_stream_position/2`
- `seek/4`

## Semantics

`stream_property(Stream, position(Position))` continues to expose a bounded
integer cursor position for an open read stream. `set_stream_position(Stream,
Position)` accepts that integer position, or any non-negative integer within
the stream contents, and restores the cursor to that offset. Positions are
measured in decoded UTF-8 code points, matching `read_string/3` and
`get_char/2` cursor behavior.

`seek(Stream, Offset, Method, NewLocation)` moves a bounded read stream relative
to one of the supported methods:

- `bof`: `Offset` code points from the beginning of file
- `current`: `Offset` code points from the current cursor
- `eof`: `Offset` code points from end of file

The resulting cursor must be between `0` and the stream length, inclusive.
Successful seeks unify `NewLocation` with the new integer cursor. Invalid
handles, output streams, non-integer offsets, unsupported methods, and
out-of-bounds targets fail deterministically.

Aliases created by `open/4` can be used anywhere a stream handle is accepted.

## Validation

Coverage should prove:

- Python builtin helpers can restore a captured `position/1` value and seek
  relative to EOF.
- out-of-bounds positioning fails without silently clamping.
- source-level Prolog calls adapt to the builtin layer.
- structured VM and bytecode VM produce matching answers for positioning
  stream queries.
- the capability manifest records PR81 as complete while leaving current or
  console streams, binary streams, rich ISO/SWI options, foreign predicates,
  and async host services deferred.

## Non-goals

- no standard stream aliases
- no output-stream truncation or random-access write positioning
- no binary streams or encodings beyond UTF-8
- parser-backed `read/1`, `read_term/2`, and stream term parsing are deferred
  to PR83
- no foreign predicate or async host callback boundary
