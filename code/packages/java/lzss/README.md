# java/lzss — CMP02 LZSS Compression

A pure-Java implementation of the LZSS (Lempel–Ziv–Storer–Szymanski, 1982) lossless compression algorithm, part of the **CMP** compression-algorithm series.

## What is LZSS?

LZSS is a refinement of LZ77. Both algorithms compress data by replacing repeated byte sequences with compact back-references into a *sliding window* of recently seen output. LZSS improves on LZ77 by replacing the mandatory `(offset, length, next_char)` triple with a *flag-bit scheme*:

- Tokens are grouped into blocks of 8.
- Each block starts with a 1-byte **flag**. Bit *i* = 0 means token *i* is a **Literal** (1 byte); bit *i* = 1 means token *i* is a **Match** (3 bytes: 2-byte BE offset + 1-byte length).
- A match is only emitted when its length is at least 3 bytes (the break-even point where the 3-byte Match costs the same as 3 Literals).

## CMP02 Wire Format

```
Bytes 0–3:  original_length  (big-endian uint32)
Bytes 4–7:  block_count      (big-endian uint32)
Bytes 8+:   blocks
  Each block: [1-byte flag][symbol data per token]
    Literal symbol: 1 byte
    Match symbol:   2-byte BE offset + 1-byte length
```

## Series

| ID     | Algorithm | Year | Notes                          |
|--------|-----------|------|--------------------------------|
| CMP00  | LZ77      | 1977 | Sliding-window back-references |
| CMP01  | LZ78      | 1978 | Explicit dictionary (trie)     |
| CMP02  | **LZSS**  | 1982 | LZ77 + flag bits ← this pkg    |
| CMP03  | LZW       | 1984 | LZ78 + pre-init alphabet; GIF  |
| CMP04  | Huffman   | 1952 | Entropy coding                 |
| CMP05  | DEFLATE   | 1996 | LZ77 + Huffman; ZIP/gzip/PNG   |

## Usage

```java
import com.codingadventures.lzss.Lzss;
import java.nio.charset.StandardCharsets;

byte[] data       = "hello hello hello".getBytes(StandardCharsets.UTF_8);
byte[] compressed = Lzss.compress(data);
byte[] restored   = Lzss.decompress(compressed);

assert java.util.Arrays.equals(data, restored);
```

### Token-level API

```java
import com.codingadventures.lzss.*;
import java.util.List;

// Encode to tokens
List<LzssToken> tokens = Lzss.encode(
    data,
    Lzss.DEFAULT_WINDOW_SIZE,   // 4096
    Lzss.DEFAULT_MAX_MATCH,     // 255
    Lzss.DEFAULT_MIN_MATCH      // 3
);

// Inspect tokens
for (LzssToken tok : tokens) {
    switch (tok) {
        case LzssToken.Literal lit  -> System.out.println("Literal: " + lit.value());
        case LzssToken.Match   match -> System.out.println("Match:  offset=" + match.offset() + " length=" + match.length());
    }
}

// Decode from tokens
byte[] decoded = Lzss.decode(tokens, data.length);
```

## Build & Test

```bash
cd code/packages/java/lzss
gradle test
```

Requires Java 21 and Gradle on `$PATH` (or use the Gradle wrapper if present).

## Package layout

```
lzss/
├── BUILD                         # build-tool entry point
├── BUILD_windows
├── CHANGELOG.md
├── README.md
├── required_capabilities.json
├── build.gradle.kts
├── settings.gradle.kts
└── src/
    ├── main/java/com/codingadventures/lzss/
    │   ├── LzssToken.java        # sealed interface: Literal | Match
    │   └── Lzss.java             # encoder, decoder, serialiser, one-shot API
    └── test/java/com/codingadventures/lzss/
        └── LzssTest.java         # 15 JUnit 5 tests
```
