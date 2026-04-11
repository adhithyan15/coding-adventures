# coding_adventures_lz78.rb — LZ78 lossless compression algorithm (1978).
#
# Public API wrapper for the CodingAdventures::LZ78 module.
# Delegates to the internal Compressor module.
#
# === Series ===
#
#   CMP00 (LZ77, 1977) — Sliding-window backreferences.
#   CMP01 (LZ78, 1978) — Explicit dictionary (trie). ← this module
#   CMP02 (LZSS, 1982) — LZ77 + flag bits.
#   CMP03 (LZW,  1984) — LZ78 + pre-initialised alphabet; GIF.
#   CMP04 (Huffman, 1952) — Entropy coding.
#   CMP05 (DEFLATE, 1996) — LZ77 + Huffman; ZIP/gzip/PNG/zlib.
#
# === Usage ===
#
#   require "coding_adventures_lz78"
#
#   data = "hello hello hello world"
#   compressed = CodingAdventures::LZ78.compress(data)
#   original   = CodingAdventures::LZ78.decompress(compressed)
#   # original == "hello hello hello world"

require_relative "coding_adventures/lz78/version"
require_relative "coding_adventures/lz78/compressor"

module CodingAdventures
  module LZ78
    # Encodes bytes into an LZ78 token stream.
    #
    # @param data     [String]  input bytes (ASCII-8BIT or any encoding)
    # @param max_dict [Integer] maximum dictionary size (default 65536)
    # @return         [Array<Token>]
    def self.encode(data, max_dict: 65536)
      Compressor.encode(data.b, max_dict: max_dict)
    end

    # Decodes an LZ78 token stream back into bytes.
    #
    # @param tokens          [Array<Token>]
    # @param original_length [Integer, nil]
    # @return [String] binary string (ASCII-8BIT)
    def self.decode(tokens, original_length: nil)
      Compressor.decode(tokens, original_length: original_length).b
    end

    # Compresses data using LZ78 and serialises to CMP01 wire format.
    #
    # @param data     [String]  input bytes
    # @param max_dict [Integer] maximum dictionary size (default 65536)
    # @return [String] compressed bytes (ASCII-8BIT)
    def self.compress(data, max_dict: 65536)
      raw    = data.b
      tokens = Compressor.encode(raw, max_dict: max_dict)
      Compressor.serialise_tokens(tokens, raw.bytesize)
    end

    # Decompresses data that was compressed with compress().
    #
    # @param data [String] compressed bytes
    # @return [String] original bytes (ASCII-8BIT)
    def self.decompress(data)
      tokens, original_length = Compressor.deserialise_tokens(data.b)
      Compressor.decode(tokens, original_length: original_length).b
    end
  end
end
