# java/zip — CMP09: ZIP Archive Format

Part of the **coding-adventures** compression series.

## What it does

ZIP bundles one or more files into a single `.zip` archive, compressing each
entry independently with DEFLATE (method 8) or storing it verbatim (method 0).
The same container format underlies Java JARs, Office Open XML (`.docx`),
Android APKs, Python wheels, and many other formats.

## Series context

| ID    | Algorithm | Year | Description                               |
|-------|-----------|------|-------------------------------------------|
| CMP00 | LZ77      | 1977 | Sliding-window back-references            |
| CMP01 | LZ78      | 1978 | Explicit dictionary (trie)                |
| CMP02 | LZSS      | 1982 | LZ77 + flag bits                          |
| CMP03 | LZW       | 1984 | LZ78 + pre-initialized alphabet; GIF      |
| CMP04 | Huffman   | 1952 | Entropy coding                            |
| CMP05 | DEFLATE   | 1996 | LZ77 + Huffman; ZIP/gzip/PNG/zlib         |
| **CMP09** | **ZIP** | **1989** | **DEFLATE container; universal archive** |

## Architecture

```
┌──────────────────────────────────────────────────────────┐
│  [Local File Header + File Data]  ← entry 1              │
│  [Local File Header + File Data]  ← entry 2              │
│  ...                                                     │
│  ══════════ Central Directory ══════════                 │
│  [Central Dir Header]  ← entry 1  (has local offset)    │
│  [Central Dir Header]  ← entry 2                        │
│  [End of Central Directory Record]                       │
└──────────────────────────────────────────────────────────┘
```

The dual-header design supports two workflows:

- **Sequential write**: append Local Headers + data, write Central Directory
  at the end.
- **Random-access read**: seek to the EOCD, parse the Central Directory, then
  jump directly to any entry.

## Usage

```java
import com.codingadventures.zip.Zip;
import java.nio.charset.StandardCharsets;
import java.util.List;

// ── Write ──────────────────────────────────────────────────────────────────
Zip.ZipWriter w = new Zip.ZipWriter();
w.addFile("hello.txt", "hello, world!".getBytes(StandardCharsets.UTF_8), true);
w.addDirectory("mydir/");
w.addFile("mydir/data.bin", someBytes, true); // auto-compresses
byte[] archive = w.finish();

// ── Read ───────────────────────────────────────────────────────────────────
Zip.ZipReader r = new Zip.ZipReader(archive);
for (Zip.ZipEntry entry : r.entries()) {
    System.out.println(entry.name());
}
byte[] data = r.read("hello.txt");

// ── Convenience ────────────────────────────────────────────────────────────
byte[] zipped = Zip.zip(List.of(
    new Zip.ZipEntry("a.txt", "AAA".getBytes()),
    new Zip.ZipEntry("b.txt", "BBB".getBytes())
));
List<Zip.ZipEntry> entries = Zip.unzip(zipped);
```

## Design decisions

### DEFLATE via LZSS

Rather than shipping a separate DEFLATE library, the compressor re-uses the
`com.codingadventures:lzss` package for match-finding and emits a single
fixed-Huffman block (BTYPE=01).  This keeps the dependency graph explicit and
the implementation self-contained.

### Auto-compression policy

If `compress=true` but the DEFLATE output is >= the original size (common for
already-compressed formats such as JPEG, PNG, or nested ZIP), the entry is
silently stored verbatim (method 0).  This prevents bloat without requiring
callers to pre-analyse their data.

### Security

- **CRC-32 verification** on every read — corrupt data raises `IOException`.
- **256 MB decompression limit** — guards against decompression bombs.
- **LEN/NLEN validation** on stored DEFLATE blocks.
- **Encryption rejection** — encrypted entries (GP flag bit 0) raise
  `IOException` rather than producing garbage.
- **EOCD scan limit** — backwards scan is bounded to the last 65 557 bytes
  (22-byte EOCD + 65 535-byte max comment) to prevent unbounded loops on
  malformed archives.

## Building and testing

```bash
cd code/packages/java/zip
gradle test
```

Requires Java 21 and Gradle (or the Gradle wrapper from the parent repo).
The `lzss` package is resolved via a Gradle composite build (`includeBuild`),
so no separate install step is needed.

## Dependencies

| Dependency                        | Purpose                         |
|-----------------------------------|---------------------------------|
| `com.codingadventures:lzss`       | LZSS match-finding for DEFLATE  |
| `org.junit.jupiter:junit-jupiter` | Unit tests (JUnit 5)            |
