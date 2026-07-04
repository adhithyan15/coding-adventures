# protobuf

A zero-dependency Rust codec for the **Protocol Buffers wire format** — the
byte-level encoding shared by every protobuf implementation (Google's `protoc`,
`prost`, and the messages Anki embeds in `.apkg` files).

It implements only the wire format: LEB128 varints, the four wire types
(`Varint`, `Fixed64`, `LengthDelimited`, `Fixed32`), and a `Writer`/`Reader`
pair. There is **no** `.proto` compiler and **no** `derive` macro — callers
hand-write the few `encode`/`decode` functions for the specific messages they
use. For small messages this is a handful of lines and keeps the crate tiny and
dependency-free, which is why it exists: it replaces the third-party `prost`
crate in the Engram Anki stack without pulling in a build-time code generator.

## Where it fits

```
engram-anki-package  →  protobuf (this crate)      ← replaces `prost`
   .apkg meta/media      wire-format read/write
```

Anki's `.apkg` archives store a `meta` message (package version) and a `media`
map (filename/size/sha1 per entry) as protobuf. `engram-anki-package` hand-codes
those messages against this crate so Engram can read files real Anki wrote and
write files real Anki can open, with no third-party dependency.

## Usage

```rust
use protobuf::{Writer, Reader, Value};

// Encode: message { field 1 = "hi"; field 2 = 42 }
let mut w = Writer::new();
w.string(1, "hi").varint(2, 42);
let bytes = w.into_bytes();

// Decode: iterate fields; unknown numbers can simply be ignored.
let mut r = Reader::new(&bytes);
while let Some(field) = r.next_field().unwrap() {
    match field.number {
        1 => assert_eq!(field.value.as_bytes(), Some(&b"hi"[..])),
        2 => assert_eq!(field.value.as_varint(), Some(42)),
        _ => {} // forward-compatible: skip unknown fields
    }
}
```

## Scope

Implemented: varints (unsigned LEB128), wire types 0/1/2/5, nested messages
(via length-delimited), unknown-field skipping. Not implemented (add if needed):
zig-zag `sint` helpers, packed repeated fields, the deprecated group wire types
(3/4), and any schema/reflection layer. See the module docs in `src/lib.rs` for
a full description of the wire format.
