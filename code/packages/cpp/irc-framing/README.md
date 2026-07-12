# irc-framing (C++)

A stateful byte-stream-to-line-frame converter in pure ISO C++17, header-only,
in namespace `ca::irc`. A faithful port of the Rust `irc-framing` crate.

TCP delivers a byte stream, not messages: one read may hand you half a message,
one, or several. IRC frames messages with a trailing CRLF (or a lone LF); this
`Framer` absorbs raw byte chunks and emits complete, CRLF-stripped lines to the
layer above (e.g. [`irc-proto`](../irc-proto)).

Per **RFC 1459 §2.3**, a message is at most 512 bytes including CRLF — at most
510 bytes of content. Lines whose content exceeds 510 bytes are silently
discarded.

## API

```cpp
#include "irc_framing.hpp"
using ca::irc::Framer;

Framer f;
f.feed(std::string("NICK alice\r\nUSER a 0 * :Al"));
f.feed(std::string("ice\r\n"));

for (const std::vector<unsigned char>& frame : f.frames()) {
    // frame is raw bytes: "NICK alice", then "USER a 0 * :Alice"
}
```

`frames()` returns `std::vector<std::vector<unsigned char>>` — each frame is raw
bytes (any value). `feed` takes `(const unsigned char*, size_t)` or a
`std::string`. `reset()` clears the buffer; `buffer_size()` reports its length.
Frame extraction scans with a cursor and drains the consumed prefix once.

## Portability

Pure ISO C++17 — standard library only. Compiles clean under GCC, Clang, and
MSVC with `-pedantic-errors` / `/permissive-` and warnings-as-errors, via the
shared [`iso-harness`](../../c/iso-harness).

## Development

```bash
sh BUILD
```
