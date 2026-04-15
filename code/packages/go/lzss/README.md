# lzss — LZSS Compression Algorithm (CMP02)

LZSS (Lempel-Ziv-Storer-Szymanski, 1982) is an improvement on LZ77 that uses
flag bits to distinguish literals from back-references. Literals cost 1 byte;
matches cost 3 bytes. No next_char waste.

## Usage

```go
import "github.com/adhithyan15/coding-adventures/code/packages/go/lzss"

compressed := lzss.Compress(data)
original   := lzss.Decompress(compressed)
```

## API

- `Compress(data []byte) []byte` — encode and serialise to CMP02 wire format
- `Decompress(data []byte) []byte` — deserialise and decode
- `Encode(data []byte, windowSize, maxMatch, minMatch int) []Token` — token-level encoding
- `Decode(tokens []Token, originalLength int) []byte` — token-level decoding
- `Literal(b byte) Token` / `Match(offset uint16, length uint8) Token` — token constructors
