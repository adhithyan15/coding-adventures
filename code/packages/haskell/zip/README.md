# zip — Haskell ZIP archive format (CMP09)

An educational Haskell implementation of the ZIP archive format (PKZIP, 1989),
part of the CMP compression series. Compresses with RFC 1951 DEFLATE (fixed
Huffman, method 8) and falls back to Stored (method 0) when DEFLATE would not
reduce size.

## Position in the compression series

```
CMP00 (LZ77,    1977) — Sliding-window back-references.
CMP01 (LZ78,    1978) — Explicit dictionary (trie).
CMP02 (LZSS,    1982) — LZ77 + flag bits.
CMP03 (LZW,     1984) — LZ78 + pre-initialised alphabet; GIF.
CMP04 (Huffman, 1952) — Entropy coding.
CMP05 (DEFLATE, 1996) — LZ77 + Huffman; ZIP/gzip/PNG/zlib.
CMP09 (ZIP,     1989) — DEFLATE container; universal archive.  ← this package
```

## Quick start

```haskell
import Zip
import qualified Data.ByteString.Char8 as BC

-- Write
let archive = zip' [(BC.pack "hello.txt", BC.pack "Hello, world!")]

-- Read back
case unzip' archive of
    Left err    -> putStrLn ("Error: " ++ err)
    Right files -> mapM_ print files
-- ("hello.txt", "Hello, world!")
```

## API

### Write

```haskell
-- | Build an archive. (name, data, compress=True uses DEFLATE if helpful)
writeZip :: [(ByteString, ByteString, Bool)] -> ByteString

-- | Convenience: all entries compressed with auto-fallback.
zip' :: [(ByteString, ByteString)] -> ByteString
```

### Read

```haskell
-- | Parse all entries (EOCD-first strategy, CRC-32 verified).
readZip :: ByteString -> Either String [ZipEntry]

-- | Read a single file by name.
readEntry :: ByteString -> ByteString -> Either String ByteString

-- | Unzip all non-directory entries.
unzip' :: ByteString -> Either String [(ByteString, ByteString)]
```

### Entry type

```haskell
data ZipEntry = ZipEntry
    { entryName :: !ByteString   -- UTF-8 filename
    , entryData :: !ByteString   -- decompressed content
    }
```

## Wire format (all integers little-endian)

```
[Local File Header + data] × N
[Central Directory Header] × N
[End of Central Directory Record]
```

The dual-header design enables sequential write and random-access read. The
reader uses the EOCD to locate the Central Directory without scanning the whole
file.

## Dependencies

- `base`, `bytestring`, `array` — from the standard GHC distribution.
- `lzss` — local sibling package providing LZ77 tokenisation (`encode`).

## Running tests

```bash
cd code/packages/haskell/zip
mise exec -- cabal test all
```

All 12 test cases should pass in roughly 5 seconds.
