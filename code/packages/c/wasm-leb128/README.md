# wasm-leb128 (C)

A pure ISO **C17** implementation of **LEB128** variable-length integer coding.
A faithful port of the Rust `wasm-leb128` crate.

It compiles clean under **GCC, Clang, and MSVC** with `-std=c17
-pedantic-errors -Wall -Wextra -Werror` (and `/std:c17 /permissive- /W4 /WX` on
MSVC), via the shared [`iso-harness`](../iso-harness/). Standard library only.

## What it is

LEB128 ("Little-Endian Base 128") is the varint format used by **WebAssembly**,
**DWARF** debug info, and Android DEX. Each byte carries 7 data bits; the high
bit (`0x80`) is a continuation flag, set on every byte but the last, and groups
are emitted least-significant first.

- **Unsigned** values are zero-extended: `624485 → E5 8E 26`.
- **Signed** values use two's complement with sign extension: `-2 → 7E`.

A `u64`/`i64` needs at most `LEB128_MAX_BYTES` (10) bytes.

## API

```c
#include "wasm_leb128.h"

unsigned char buf[LEB128_MAX_BYTES];
size_t n = leb128_encode_unsigned(624485, buf);   /* n=3, buf = E5 8E 26 */
size_t m = leb128_encode_signed(-2, buf);         /* m=1, buf = 7E */

unsigned long long uv; size_t used;
if (leb128_decode_unsigned(buf, n, 0, &uv, &used) == LEB128_OK) { /* uv, used */ }

long long sv;
leb128_decode_signed(buf, m, 0, &sv, &used);
```

Encoding writes into a caller buffer (≥ `LEB128_MAX_BYTES`) and returns the byte
count. Decoding takes `(data, len, offset)` and returns `LEB128_OK`,
`LEB128_ERR_OFFSET`, `LEB128_ERR_OVERFLOW`, or `LEB128_ERR_UNTERMINATED`
(`leb128_status_message` describes each). The signed-shift and two's-complement
reinterpretation are spelled so the result is well-defined on every target.

## Building & testing

```sh
sh BUILD          # POSIX: compiles + runs the tests under every compiler found
```

Tests use the crate's own WASM/DWARF vectors — zero, multi-byte, u32/i32 min &
max, offset decoding, the unterminated/out-of-bounds/overflow errors, and
encode↔decode round trips including `u64::MAX` / `i64::MIN`.
