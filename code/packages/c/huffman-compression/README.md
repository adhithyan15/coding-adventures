# huffman-compression (C)

**Huffman** compression with canonical codes, in pure ISO C17. A faithful port
of the Rust `huffman-compression` crate (the CMP04 wire format). The Huffman tree
is built in a fixed array — no dynamic tree, no pointer ownership to get wrong.

Compiles and runs under **GCC, Clang, and MSVC** with strict ISO-conformance
flags, via the shared [`iso-harness`](../iso-harness/README.md).

## Usage

```c
#include "huffman_compression.h"

uint8_t *packed = NULL, *restored = NULL;
size_t packed_len = 0, restored_len = 0;

huffman_compress(data, len, &packed, &packed_len);
huffman_decompress(packed, packed_len, &restored, &restored_len);  /* == data */
free(packed);
free(restored);
```

`huffman_decompress` returns 0 on a malformed stream. Both functions allocate
their output (overflow-guarded) and report the length through an out-parameter.

## Development

```bash
sh BUILD   # compile + run the tests under every C compiler present (strict ISO)
```

Ports `code/packages/rust/huffman-compression`. Uses canonical codes, so the
stream carries only a per-symbol lengths table. See also the
[C++ port](../../cpp/huffman-compression/README.md).
