# zip — Haskell ZIP archive format (CMP09)

An educational Haskell implementation of the ZIP archive format (PKZIP, 1989),
part of the CMP compression series. Its encoder emits RFC 1951 stored or fixed
Huffman blocks and falls back to ZIP method 0 when compression would not reduce
size. Its decoder accepts stored, fixed, and dynamic Huffman blocks, including
multi-block streams and the full 32 KiB history window.

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

### Raw RFC 1951 and CRC-32

```haskell
rawDeflate :: ByteString -> ByteString
rawInflate :: ByteString -> Int -> Either RawInflateError ByteString
rawInflateCounted :: ByteString -> Int -> Either RawInflateError RawInflateResult
rawInflateMaxOutput :: Int

data RawInflateResult = RawInflateResult
    { rawInflateOutput :: ByteString
    , rawInflateBytesConsumed :: Int
    }
```

`rawInflateCounted` reports the exact byte reached at the final RFC 1951 block,
so ZIP readers can reject undeclared trailing compressed bytes. Output limits
are validated before decoding and cannot exceed 256 MiB. Failures use the
closed, payload-blind identifiers returned by `rawInflateErrorCode`.

CRC-32 detects accidental corruption; it is not authentication.

### Entry type

```haskell
data ZipEntry = ZipEntry
    { entryName :: !ByteString   -- UTF-8 filename
    , entryData :: !ByteString   -- decompressed content
    }
```

> **Security note (zip-slip):** `entryName` comes straight from the archive's
> Central Directory and is returned as-is — it is **not** validated or
> sanitised. This module never writes to disk itself, so there is no
> traversal bug here today, but any caller that joins `entryName` onto a
> filesystem path (e.g. to extract an archive) must first reject or
> normalise names containing `..` path segments, absolute paths, or
> drive-qualified Windows paths. This is the classic "zip-slip"
> vulnerability class — treat every `entryName` as attacker-controlled.

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

Production code is a pure in-memory byte transform. Fixture loading and the
independent Python `zlib` oracle exist only in the test suite.

- `base`, `bytestring`, `array`, `containers` — core Haskell libraries used
  for byte handling, canonical tables, and overlap-safe bounded output.
- `lzss` — local sibling package providing LZ77 tokenisation (`LZSS.encodeWith`).

## Running tests

```bash
cd code/packages/haskell/zip
cabal test
cabal build zip:lib:zip zip:test:spec --ghc-options="-Wall -Werror"
```

The suite includes the 34-case language-neutral raw RFC 1951/CRC-32 corpus,
foreign dynamic streams, exact-consumption container checks, a real 32 KiB
history-window stream, and the package's historical ZIP regressions.
