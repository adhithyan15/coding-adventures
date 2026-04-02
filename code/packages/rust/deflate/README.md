# deflate

Zero-dependency implementation of DEFLATE (RFC 1951) and ZLIB (RFC 1950)
compression.  Uses LZ77 with fixed Huffman codes.  Includes Adler-32
checksum for the ZLIB wrapper.

Built from scratch — no external compression libraries.
