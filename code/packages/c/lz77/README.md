# lz77 (C)

**LZ77** sliding-window compression, in pure ISO C17. A faithful port of the Rust
`lz77` crate. Emits `(offset, length, next_char)` tokens; `compress`/`decompress`
add a compact serialisation.

Compiles and runs under **GCC, Clang, and MSVC** with strict ISO-conformance
flags, via the shared [`iso-harness`](../iso-harness/README.md).

## Usage

```c
#include "lz77.h"

uint8_t *packed = NULL, *restored = NULL;
size_t packed_len = 0, restored_len = 0;

lz77_compress(data, len, LZ77_DEFAULT_WINDOW, LZ77_DEFAULT_MAX_MATCH,
              LZ77_DEFAULT_MIN_MATCH, &packed, &packed_len);
lz77_decompress(packed, packed_len, &restored, &restored_len);  /* == data */
free(packed);
free(restored);
```

Lower-level `lz77_encode`/`decode`/`serialise`/`deserialise` are also available.
Every function that produces a buffer allocates it (overflow-guarded) and reports
the length through an out-parameter; the caller frees it.

## Development

```bash
sh BUILD   # compile + run the tests under every C compiler present (strict ISO)
```

Ports `code/packages/rust/lz77`. See also the [C++ port](../../cpp/lz77/README.md).
