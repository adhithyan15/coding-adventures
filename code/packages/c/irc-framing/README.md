# irc-framing (C)

A stateful byte-stream-to-line-frame converter in pure ISO C17. A faithful port
of the Rust `irc-framing` crate.

TCP delivers a byte stream, not messages: one `read()` may hand you half a
message, one, or several. IRC frames messages with a trailing CRLF (or a lone
LF); this framer absorbs raw byte chunks and emits complete, CRLF-stripped lines
to the layer above (e.g. [`irc-proto`](../irc-proto)).

Per **RFC 1459 §2.3**, a message is at most 512 bytes including CRLF — at most
510 bytes of content. Lines whose content exceeds 510 bytes are silently
discarded.

## API

```c
#include "irc_framing.h"

IrcFramer *f = irc_framer_new();
irc_framer_feed(f, (const unsigned char *)"NICK alice\r\nUSER a 0 * :Al", 26);
irc_framer_feed(f, (const unsigned char *)"ice\r\n", 5);

IrcFrames fr;
if (irc_framer_frames(f, &fr) == 0) {
    /* fr.count == 2; fr.frames[0] = "NICK alice", fr.frames[1] = "USER a 0 * :Alice" */
    irc_frames_free(&fr);
}
irc_framer_free(f);
```

Frames are raw byte slices (`{unsigned char *data; size_t len}`), not
NUL-terminated — a frame may hold any byte. `irc_framer_frames` drains the
buffer and returns a batch (release with `irc_frames_free`); on an allocation
failure it returns `-1` and leaves the buffer intact, so the call is retryable.
The internal buffer grows with an overflow-guarded doubling.

This port scans with a cursor and drains the whole consumed prefix in one move,
where the Rust original drains after every line; the observable result is
identical.

## Portability

Pure ISO C17 — no extensions. Compiles clean under GCC, Clang, and MSVC with
`-pedantic-errors` / `/permissive-` and warnings-as-errors, via the shared
[`iso-harness`](../iso-harness).

## Development

```bash
sh BUILD
```
