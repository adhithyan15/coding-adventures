# protobuf (C++)

A **zero-dependency Protocol Buffers wire-format codec**, header-only, pure ISO
C++17. A faithful port of the Rust [`protobuf`](../../rust/protobuf) crate, in
namespace `ca::protobuf`. It implements just the
[wire format](https://protobuf.dev/programming-guides/encoding/) — no `.proto`
compiler, no codegen; you hand-write the few encode/decode calls you need.

## The wire format

A message is a flat sequence of `(tag, value)` records with no framing. Each tag
is a varint: `tag = (field_number << 3) | wire_type`.

| wire type | name | payload |
|-----------|------|---------|
| 0 | `Varint`          | one LEB128 varint |
| 1 | `Fixed64`         | 8 little-endian bytes |
| 2 | `LengthDelimited` | a varint length, then that many bytes |
| 5 | `Fixed32`         | 4 little-endian bytes |

## API

```cpp
#include "protobuf.hpp"
using namespace ca::protobuf;

// Encode (chainable)
Writer w;
w.varint(1, 150).string(2, "hi").fixed32(3, 0xdeadbeef);
std::vector<std::uint8_t> msg = w.into_bytes();

// Decode — next_field() yields std::optional<Field>, throws Error on bad input
Reader r(msg);
while (auto f = r.next_field()) {
    if (f->number == 1) { auto v = f->value.as_varint(); /* std::optional */ }
}
```

- **`Writer`** — chainable `varint` / `bytes` / `string` / `message` /
  `fixed32` / `fixed64` / `write_varint`; `into_bytes()` moves out the buffer.
- **`Reader`** — `next_field()` returns `std::optional<Field>` (`nullopt` at end)
  and **throws `ca::protobuf::Error`** on malformed input; unknown field numbers
  are yielded for forward compatibility. Length-delimited payloads are returned
  as a non-owning `ByteView` borrowing the input.
- **`Value`** — tagged by `WireType`, with `as_varint()` / `as_bytes()`
  (`std::optional`) and value equality; `Field { number, value }`.
- **`Error`** — a `std::runtime_error` subclass carrying an `ErrorKind`
  (`TruncatedVarint`, `UnexpectedEof`, `UnknownWireType`, `ZeroFieldNumber`).

> `ByteView` is a small hand-rolled `{ptr, len}` view rather than
> `std::basic_string_view<std::uint8_t>`, whose `char_traits<unsigned char>` is
> non-standard and rejected under `-Werror`.

## Building

Builds through the shared [`iso-harness`](../../c/iso-harness) engine under every
ISO C++ compiler on `PATH` with `-std=c++17 -pedantic-errors -Wall -Wextra
-Werror`:

```sh
sh BUILD          # POSIX: g++ and/or clang++
```

Each compiler prints `N checks, 0 failed`.
