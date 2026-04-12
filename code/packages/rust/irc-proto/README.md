# irc-proto

Pure IRC message parsing and serialization (RFC 1459). No I/O, no dependencies.

## Usage

```rust
use irc_proto::{parse, serialize, Message};

let msg = parse("NICK alice").unwrap();
assert_eq!(msg.command, "NICK");
let bytes = serialize(&msg);
assert_eq!(bytes, b"NICK alice\r\n");
```

## Running tests

```bash
cargo test -p irc-proto
```
