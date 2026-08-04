# resp-protocol (C++)

A pure ISO **C++17**, header-only implementation of **RESP** (the REdis
Serialization Protocol, v2), in namespace `ca::resp`. A faithful port of the
Rust `resp-protocol` crate.

It compiles clean under **GCC, Clang, and MSVC** with `-std=c++17
-pedantic-errors -Wall -Wextra -Werror` (and `/std:c++17 /permissive- /W4 /WX`
on MSVC), via the shared [`iso-harness`](../../c/iso-harness/). Standard library
only.

## What it is

RESP is the line protocol Redis speaks. A value is one of five frame types, each
prefixed by one byte and terminated by CRLF: `+` simple string, `-` error, `:`
integer, `$` bulk string (`$-1` = null), `*` array (`*-1` = null). A bare line
with no known prefix is parsed as an inline command (whitespace-split tokens as
bulk strings in an array).

## API

`ca::resp::Value` is a recursive value with value semantics (copy/move/compare).

```cpp
#include "resp_protocol.hpp"
namespace resp = ca::resp;
using resp::Value;

// encode -> std::optional (nullopt iff a simple string had CR/LF)
auto bytes = resp::encode(Value::make_integer(-42));   // ":-42\r\n"

// decode one frame -> DecodeResult { is_value / is_incomplete / is_error }
resp::DecodeResult r = resp::decode(*bytes);
if (r.is_value()) { /* r.value, r.consumed */ }

// nested value
Value v = Value::make_array({Value::simple_string("OK"), Value::make_integer(7)});
```

- Factories: `simple_string`, `make_error`, `make_integer`, `bulk_string`,
  `bulk_string_null`, `make_array`, `make_array_null`; error split via
  `error_type()` / `error_detail()`.
- `encode` → `std::optional<std::vector<std::uint8_t>>`.
- `decode` → `DecodeResult`; `decode_all` → `DecodeAllResult { ok, values,
  consumed, error }`.
- Streaming `Decoder`: `feed`, `has_message`, `get_message()` →
  `std::optional<Value>`, `decode_all`, `has_error`.

## Building & testing

```sh
sh BUILD          # POSIX: compiles + runs the tests under every compiler found
```

Tests use the crate's own encode/decode vectors — every frame type, nested
arrays, the newline and negative-length errors, incomplete inputs, invalid
UTF-8, `decode_all`, and the streaming decoder.
