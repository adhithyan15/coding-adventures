# DEFLATE (Java)

An RFC 1951 raw-DEFLATE implementation for [`CMP05-deflate.md`](../../../specs/CMP05-deflate.md).
The compressor constructs fixed- and dynamic-Huffman candidates from one repository LZSS
token stream. Its in-package package-merge planner enforces RFC code-length limits and compares
the candidates' exact bit costs before emitting one final block. The strict inflater accepts
stored, fixed, and dynamic blocks, rejects truncation and trailing bytes, and enforces a
configurable output limit.

```java
byte[] compressed = Deflate.compress(input);
byte[] original = Deflate.inflate(compressed, 1_000_000);
```

The wire format is raw DEFLATE: there is no zlib or gzip wrapper.
