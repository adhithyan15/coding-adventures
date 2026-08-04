# huffman-compression (C++)

**Huffman** compression with canonical codes, in pure ISO C++17 (header-only). A
faithful port of the Rust `huffman-compression` crate (CMP04).

Compiles and runs under **GCC, Clang, and MSVC** with strict ISO-conformance
flags, via the shared [`iso-harness`](../../c/iso-harness/README.md).

## Usage

```cpp
#include "huffman_compression.hpp"

std::vector<std::uint8_t> packed = ca::huffman::compress(data);
std::vector<std::uint8_t> restored = ca::huffman::decompress(packed);   // == data
```

`decompress` throws `std::invalid_argument` on a malformed stream.

## Development

```bash
sh BUILD   # compile + run the tests under every C++ compiler present (strict ISO)
```

Ports `code/packages/rust/huffman-compression`. See also the
[C port](../../c/huffman-compression/README.md).
