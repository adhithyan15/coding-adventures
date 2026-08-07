# lzw (C)

**LZW** compression with variable-width codes (9→16 bits), in pure ISO C17. A
faithful port of the Rust `lzw` crate. CLEAR/STOP control codes, dictionary-full
reset, LSB-first bit packing, and a 4-byte length header.

Compiles and runs under **GCC, Clang, and MSVC** with strict ISO-conformance
flags, via the shared [`iso-harness`](../iso-harness/README.md).

## Usage

```c
#include "lzw.h"

uint8_t *packed = NULL, *restored = NULL;
size_t packed_len = 0, restored_len = 0;

lzw_compress(data, len, &packed, &packed_len);
lzw_decompress(packed, packed_len, &restored, &restored_len);  /* == data */
free(packed);
free(restored);
```

`lzw_decompress` returns 0 on a malformed stream. Both functions allocate their
output (overflow-guarded) and report the length through an out-parameter; the
caller frees it.

## Development

```bash
sh BUILD   # compile + run the tests under every C compiler present (strict ISO)
```

Ports `code/packages/rust/lzw`. The encoder keys its dictionary on
`(prefix_code, byte)`, giving the same code assignments as the crate's
byte-sequence map. See also the [C++ port](../../cpp/lzw/README.md).
