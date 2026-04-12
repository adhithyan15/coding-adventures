# LZ78 — Lossless Compression Algorithm (Swift)

Swift implementation of the LZ78 compression algorithm (Lempel & Ziv, 1978),
part of the CMP series in coding-adventures.

## Usage

```swift
import LZ78

// One-shot compress/decompress
let data       = [UInt8]("hello hello hello".utf8)
let compressed = compress(data)
let original   = decompress(compressed)
// original == data

// Token-level API
let tokens = encode(data)
let out    = decode(tokens, originalLength: data.count)
```

## TrieCursor

The `TrieCursor` struct is exported for reuse in streaming dictionary
algorithms like LZW (CMP03):

```swift
var cursor = TrieCursor()
cursor.insert(65, dictID: 1)           // root → 'A' → id=1
if cursor.step(65) {                   // true
    print(cursor.dictID)               // 1
}
cursor.reset()                         // back to root
```

## Development

```bash
swift test
```
