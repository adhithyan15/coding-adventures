# coding_adventures/lz78/compressor.rb
#
# Core LZ78 compression logic.
#
# LZ78 (Lempel & Ziv, 1978) builds an explicit trie-based dictionary of byte
# sequences encountered during encoding. The encoder and decoder build the same
# dictionary independently — no dictionary is transmitted.
#
# === Token ===
#
# Each token is a [dict_index, next_char] pair:
#   dict_index  — ID of the longest dictionary prefix (0 = literal)
#   next_char   — byte following the match (0 = flush sentinel)
#
# === End-of-Stream ===
#
# If input ends mid-match, a flush token [current_id, 0] is emitted.
# The compress() API stores the original length in the wire format so
# decompress() can truncate and discard the sentinel byte.
#
# === Wire Format ===
#
#   4 bytes  original_length (big-endian uint32)
#   4 bytes  token_count (big-endian uint32)
#   N × 4    tokens: uint16 dict_index (BE) + uint8 next_char + uint8 0x00

module CodingAdventures
  module LZ78
    # Token is a (dict_index, next_char) pair.
    Token = Struct.new(:dict_index, :next_char)

    # Internal trie node used during encoding.
    # Each node stores a dict_id and a hash of byte → child node.
    class TrieNode
      attr_reader :dict_id, :children

      def initialize(dict_id)
        @dict_id = dict_id
        @children = {}
      end
    end

    module Compressor
      # Encodes bytes into an LZ78 token stream.
      #
      # @param data        [String]  binary string (encoding: ASCII-8BIT)
      # @param max_dict    [Integer] maximum dictionary size (default 65536)
      # @return            [Array<Token>]
      def self.encode(data, max_dict: 65536)
        root    = TrieNode.new(0)
        next_id = 1
        current = root
        tokens  = []

        data.each_byte do |byte|
          if (child = current.children[byte])
            current = child
          else
            tokens << Token.new(current.dict_id, byte)

            if next_id < max_dict
              current.children[byte] = TrieNode.new(next_id)
              next_id += 1
            end

            current = root
          end
        end

        # Flush partial match at end of stream.
        tokens << Token.new(current.dict_id, 0) if current != root

        tokens
      end

      # Decodes an LZ78 token stream back into the original bytes.
      #
      # @param tokens          [Array<Token>]
      # @param original_length [Integer, nil] if set, truncates output to this
      #                        length (strips flush sentinel). Pass nil to
      #                        return all bytes.
      # @return [String] binary string (encoding: ASCII-8BIT)
      def self.decode(tokens, original_length: nil)
        # dict_table[i] = [parent_id, byte]. Entry 0 is the root sentinel.
        dict_table = [[0, 0]]
        output = "".b

        tokens.each do |tok|
          seq = reconstruct(dict_table, tok.dict_index)
          output << seq

          if original_length.nil? || output.bytesize < original_length
            output << tok.next_char.chr
          end

          dict_table << [tok.dict_index, tok.next_char]
        end

        original_length ? output.byteslice(0, original_length) : output
      end

      # Serialises tokens to the CMP01 wire format.
      #
      # @param tokens          [Array<Token>]
      # @param original_length [Integer]
      # @return [String] binary string
      def self.serialise_tokens(tokens, original_length)
        buf = [original_length].pack("N")
        buf << [tokens.length].pack("N")
        tokens.each do |tok|
          buf << [tok.dict_index].pack("n")
          buf << [tok.next_char].pack("C")
          buf << "\x00"
        end
        buf
      end

      # Deserialises wire-format bytes back into (tokens, original_length).
      #
      # @param data [String] binary string
      # @return [Array(Array<Token>, Integer)]
      def self.deserialise_tokens(data)
        return [[], 0] if data.bytesize < 8

        original_length = data.byteslice(0, 4).unpack1("N")
        token_count     = data.byteslice(4, 4).unpack1("N")
        tokens = []

        token_count.times do |i|
          base = 8 + i * 4
          break if base + 4 > data.bytesize

          dict_index = data.byteslice(base, 2).unpack1("n")
          next_char  = data.byteslice(base + 2, 1).unpack1("C")
          tokens << Token.new(dict_index, next_char)
        end

        [tokens, original_length]
      end

      private_class_method def self.reconstruct(dict_table, index)
        return "".b if index == 0

        seq = []
        idx = index
        while idx != 0
          parent_id, byte = dict_table[idx]
          seq << byte
          idx = parent_id
        end
        seq.reverse.pack("C*")
      end
    end
  end
end
