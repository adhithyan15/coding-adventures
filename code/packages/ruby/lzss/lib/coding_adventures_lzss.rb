# coding_adventures_lzss.rb — LZSS lossless compression algorithm (1982).
#
# LZSS refines LZ77 by using flag bits to distinguish literals from
# back-references, eliminating the wasted next_char byte after every match.
#
# === Series ===
#
#   CMP00 (LZ77, 1977) — Sliding-window backreferences.
#   CMP01 (LZ78, 1978) — Explicit dictionary (trie).
#   CMP02 (LZSS, 1982) — LZ77 + flag bits. ← this module
#   CMP03 (LZW,  1984) — LZ78 + pre-initialised alphabet; GIF.
#   CMP04 (Huffman, 1952) — Entropy coding.
#   CMP05 (DEFLATE, 1996) — LZ77 + Huffman; ZIP/gzip/PNG/zlib.
#
# === Usage ===
#
#   require "coding_adventures_lzss"
#
#   data = "hello hello hello world"
#   compressed = CodingAdventures::LZSS.compress(data)
#   original   = CodingAdventures::LZSS.decompress(compressed)
#   # original == "hello hello hello world"

require_relative "coding_adventures/lzss/version"
require_relative "coding_adventures/lzss/compressor"

module CodingAdventures
  module LZSS
    # Encodes bytes into an LZSS token stream.
    def self.encode(data, **opts) = Compressor.encode(data, **opts)

    # Decodes an LZSS token stream back into bytes.
    def self.decode(tokens, **opts) = Compressor.decode(tokens, **opts)

    # Compresses bytes and returns the CMP02 wire format.
    def self.compress(data, **opts) = Compressor.compress(data, **opts)

    # Decompresses bytes produced by compress.
    def self.decompress(data) = Compressor.decompress(data)

    # Token types (re-exported for convenience).
    Literal = Compressor::Literal
    Match   = Compressor::Match
  end
end
