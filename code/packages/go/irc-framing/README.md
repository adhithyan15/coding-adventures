# irc-framing (Go)

Stateful byte-stream-to-line-frame converter for IRC.

## Overview

TCP delivers a continuous byte stream, not discrete messages. A single `Read()`
call may return half a message, one full message, or several messages concatenated.

`irc-framing` solves this by accumulating bytes in an internal buffer and emitting
complete, CRLF-stripped lines whenever a `\n` terminator appears. Overlong lines
(> 510 bytes of content, per RFC 1459) are silently discarded.

The pending (not yet LF-terminated) line is capped at 511 bytes: 510 content
bytes plus a CR (RFC 1459 and RFC 2812 section 2.3, 512 bytes including CRLF).
A peer that sends more without an LF has the partial line dropped, and input is
skipped up to the next LF, so memory stays constant however much is fed
(SEC-011). No API change.

## API

```go
f := irc_framing.NewFramer()

// Feed raw bytes from a TCP read.
f.Feed(data)

// Get all complete lines (CRLF stripped). May return nil if no complete lines yet.
for _, frame := range f.Frames() {
    msg, err := irc_proto.Parse(string(frame))
    // ...
}

// Reset if the connection is recycled or corrupted.
f.Reset()

// Check how many bytes are buffered (useful for connection-level limits).
size := f.BufferSize()
```

## Coverage

100% statement coverage.
