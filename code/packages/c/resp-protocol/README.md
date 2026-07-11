# resp-protocol (C)

A pure ISO **C17** implementation of **RESP** (the REdis Serialization Protocol,
v2). A faithful port of the Rust `resp-protocol` crate.

It compiles clean under **GCC, Clang, and MSVC** with `-std=c17
-pedantic-errors -Wall -Wextra -Werror` (and `/std:c17 /permissive- /W4 /WX` on
MSVC), via the shared [`iso-harness`](../iso-harness/). Standard library only.

## What it is

RESP is the line protocol Redis speaks. A value is one of five frame types,
each prefixed by one byte and terminated by CRLF:

| Prefix | Type | Example |
|---|---|---|
| `+` | simple string | `+OK\r\n` |
| `-` | error | `-ERR boom\r\n` |
| `:` | integer (i64) | `:-42\r\n` |
| `$` | bulk string | `$3\r\nfoo\r\n` (`$-1` = null) |
| `*` | array | `*2\r\n:1\r\n:2\r\n` (`*-1` = null) |

A bare line with no known prefix is parsed as an **inline command**: split on
ASCII whitespace, each token becomes a bulk string in an array.

## API

Values are a heap-allocated tagged union (`RespValue`); build with the `resp_*`
constructors and release with `resp_free` (which recurses into arrays).

```c
#include "resp_protocol.h"

/* encode */
RespValue *v = resp_integer(-42);
unsigned char *bytes; size_t len;
if (resp_encode(v, &bytes, &len) == RESP_ENCODE_OK) {
    /* bytes = ":-42\r\n" */
    free(bytes);
}
resp_free(v);

/* decode one frame */
RespValue *out; size_t consumed;
RespDecodeStatus st = resp_decode((const unsigned char *)"+OK\r\n", 5, &out, &consumed);
/* st == RESP_DECODE_OK, consumed == 5 */
resp_free(out);
```

- `resp_encode` → `RESP_ENCODE_OK`, or `RESP_ENCODE_ERR_SIMPLE_NEWLINE` (a simple
  string held CR/LF), or `RESP_ENCODE_ERR_ALLOC`.
- `resp_decode` / `resp_decode_all` → `RESP_DECODE_OK` (with `*consumed`),
  `RESP_DECODE_INCOMPLETE` (need more bytes), or `RESP_DECODE_ERROR` (malformed).
- Streaming `RespDecoder` (`resp_decoder_new` / `_feed` / `_has_message` /
  `_get_message` / `_decode_all` / `_has_error` / `_free`) accumulates bytes
  across feeds and queues whole messages, latching an error on a bad frame.
- Error values carry the `resp_error_type` / `resp_error_detail` split.

All allocating calls are overflow-guarded (`calloc`'s checked multiply for
arrays, guarded doubling for the encode buffer) and return `NULL` /
`RESP_*_ERROR` on allocation failure. Bulk strings hold arbitrary bytes.

## Building & testing

```sh
sh BUILD          # POSIX: compiles + runs the tests under every compiler found
```

Tests use the crate's own encode/decode vectors — every frame type, nested
arrays, the newline-rejection and negative-length errors, incomplete inputs,
invalid UTF-8, `decode_all`, and the streaming decoder.
