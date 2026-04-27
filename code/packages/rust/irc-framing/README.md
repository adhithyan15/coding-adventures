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
