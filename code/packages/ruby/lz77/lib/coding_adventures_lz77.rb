# frozen_string_literal: true

# The main entry point for the coding_adventures_lz77 gem.
#
# Provides the LZ77 sliding-window compression algorithm (CMP00 specification).
# Require this file to access CodingAdventures::LZ77.encode, .decode,
# .compress, and .decompress.

require_relative "coding_adventures/lz77/version"
require_relative "coding_adventures/lz77/token"
require_relative "coding_adventures/lz77/compressor"

module CodingAdventures
  # LZ77 sliding-window compression (Lempel & Ziv, 1977).
  #
  # == Public API
  #
  # Token-level (streaming):
  #   CodingAdventures::LZ77.encode(bytes, window_size: 4096, max_match: 255, min_match: 3)
  #   CodingAdventures::LZ77.decode(tokens, initial_buffer: [])
  #
  # Byte-level (one-shot):
  #   CodingAdventures::LZ77.compress(string_or_bytes, window_size: 4096, max_match: 255, min_match: 3)
  #   CodingAdventures::LZ77.decompress(binary_string)
  #
  # == Example
  #
  #   data = "hello hello hello world"
  #   compressed = CodingAdventures::LZ77.compress(data)
  #   CodingAdventures::LZ77.decompress(compressed)  # => "hello hello hello world"
  module LZ77
    # encode encodes data into an LZ77 token stream.
    #
    # @param data [String, Array<Integer>] input — String is converted via #bytes
    # @param window_size [Integer] maximum lookback distance (default 4096)
    # @param max_match [Integer] maximum match length (default 255)
    # @param min_match [Integer] minimum match length for backreference (default 3)
    # @return [Array<Token>] compressed token stream
    def self.encode(data, window_size: 4096, max_match: 255, min_match: 3)
      bytes = data.is_a?(String) ? data.bytes : data
      Compressor.encode(bytes, window_size: window_size, max_match: max_match, min_match: min_match)
    end

    # decode decodes an LZ77 token stream back to the original bytes.
    #
    # @param tokens [Array<Token>] token stream (output of encode)
    # @param initial_buffer [Array<Integer>] optional seed for search buffer
    # @return [String] reconstructed data (ASCII-8BIT / binary encoding)
    def self.decode(tokens, initial_buffer: [])
      bytes = Compressor.decode(tokens, initial_buffer: initial_buffer)
      bytes.pack("C*").b
    end

    # compress compresses data using LZ77 and serialises to binary.
    #
    # @param data [String, Array<Integer>] input data
    # @param window_size [Integer] maximum lookback distance (default 4096)
    # @param max_match [Integer] maximum match length (default 255)
    # @param min_match [Integer] minimum match length for backreference (default 3)
    # @return [String] compressed binary string
    def self.compress(data, window_size: 4096, max_match: 255, min_match: 3)
      tokens = encode(data, window_size: window_size, max_match: max_match, min_match: min_match)
      Compressor.serialise_tokens(tokens)
    end

    # decompress decompresses data that was compressed with compress.
    #
    # @param data [String] compressed binary string
    # @return [String] original data (binary encoding)
    def self.decompress(data)
      tokens = Compressor.deserialise_tokens(data)
      decode(tokens)
    end
  end
end
