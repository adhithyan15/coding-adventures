# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).

## [0.1.0] - 2026-04-11

### Added

- Initial implementation of LZ78 encoding and decoding (CMP01 spec).
- `encode(data, max_dict_size=65536) -> list[Token]` — byte-by-byte trie traversal
  producing `Token(dict_index, next_char)` stream.
- `decode(tokens, original_length=-1) -> bytes` — parent-chain reconstruction from
  dictionary. `original_length` truncation strips the flush sentinel byte.
- `compress(data, max_dict_size=65536) -> bytes` — encode + serialise to CMP01
  wire format (8-byte header: original_length + token_count; 4 bytes/token).
- `decompress(data) -> bytes` — deserialise + decode with automatic sentinel strip.
- End-of-stream flush token: if input ends mid-dictionary-match, a flush token
  `Token(current_id, 0)` is emitted; the sentinel byte is stripped by
  `decompress` using the stored original length.
- Embedded byte-indexed trie (`_TrieNode`) for O(1) child lookup during encoding.
- 90%+ test coverage across 7 test classes covering spec vectors, round-trips,
  parameter handling, edge cases, and serialisation.
