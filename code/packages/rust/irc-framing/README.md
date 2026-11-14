# irc-framing

Stateful byte-stream-to-line converter for the IRC protocol. No I/O, no dependencies.

## Usage

```rust
use irc_framing::Framer;

let mut framer = Framer::new();
framer.feed(b"NICK alice\r\n");
let frames = framer.frames();
assert_eq!(frames[0], b"NICK alice");
```

## Running tests

```bash
cargo test -p irc-framing
```

## Memory bound

A line that has not yet seen its LF is held for at most 511 bytes (510 content bytes plus a CR; RFC 1459 and RFC 2812 section 2.3 allow 512 including CRLF). If a peer sends more than that without an LF, the partial line is dropped and the framer skips input up to the next LF, so a peer that never sends LF cannot grow memory. Call `frames()` after each read to keep complete lines from accumulating.
