# frozen_string_literal: true

# coding_adventures_zstd.rb — CMP07: ZStd (RFC 8878) lossless compression.
#
# Zstandard is a high-ratio, high-speed compression format created by
# Yann Collet at Facebook (2015) and standardised as RFC 8878. It combines:
#
#   - LZ77 back-references (via LZSS token generation) to exploit repetition
#     in the data — the same "copy from earlier output" trick as DEFLATE.
#   - FSE (Finite State Entropy) coding instead of Huffman for sequence
#     descriptor symbols. FSE is an Asymmetric Numeral System that approaches
#     the Shannon entropy limit in a single pass.
#   - Predefined decode tables (RFC 8878 Appendix B) so short frames need no
#     table description overhead.
#
# === Series ===
#
#   CMP00 (LZ77)     — Sliding-window back-references
#   CMP02 (LZSS)     — LZ77 + flag bits  ← dependency
#   CMP05 (DEFLATE)  — LZ77 + Huffman; ZIP/gzip/PNG/zlib
#   CMP07 (ZStd)     — LZ77 + FSE; high ratio + speed  ← this gem
#
# === Usage ===
#
#   require "coding_adventures_zstd"
#
#   data       = "the quick brown fox jumps over the lazy dog " * 25
#   compressed = CodingAdventures::Zstd.compress(data)
#   original   = CodingAdventures::Zstd.decompress(compressed)
#   # original == data

require_relative "coding_adventures/zstd/version"
require_relative "coding_adventures/zstd"
