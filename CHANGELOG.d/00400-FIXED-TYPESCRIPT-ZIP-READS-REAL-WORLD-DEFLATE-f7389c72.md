### Fixed — TypeScript ZIP Reads Real-World DEFLATE
- `zip`'s inflater rejected dynamic Huffman blocks (BTYPE=10) outright, which is
  what zlib and Info-ZIP emit for anything but the smallest input — so the reader
  failed on most archives the world actually produces. It now decodes all three
  RFC 1951 block types via a canonical Huffman table builder, the code-length
  alphabet with its run-length escapes, and the permuted code-length order.
- Length symbol 285 was missing from the length table. RFC 1951 spells length 258
  either as symbol 284 with five extra bits or as symbol 285 with none, and a run
  of identical bytes reliably produces the cheaper form, which was rejected as an
  invalid symbol.
- Decoder conformance is now checked against Node's `zlib` as an oracle, because
  round-tripping our own encoder through our own decoder only proves the two
  agree with each other.
- Huffman tables are now checked against Kraft's inequality. Over-subscribed
  tables are rejected outright and incomplete tables everywhere RFC 1951 forbids
  them, so the decoder no longer accepts streams zlib refuses -- a difference
  between two readers of the same bytes is the shape of a content-inspection
  bypass.
- The inflate output cap now counts bytes. It previously counted elements of a
  `number[]`, where V8 spends four to eight bytes each, so a 256 MB ceiling
  allowed one to two gigabytes of backing store and the process died before the
  limit was reached. `rawInflate` takes a caller-supplied ceiling, and
  `ZipReader.read` passes the SMALLER of the entry's declared uncompressed size
  and the reader's own — the declared size is four bytes the archive chose, so
  trusting it alone would swap a fixed limit for an attacker-chosen one.

