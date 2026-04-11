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

    # TrieCursor is a step-by-step cursor for navigating a byte-keyed trie.
    #
    # Unlike a full trie class (which operates on complete keys), TrieCursor
    # maintains a current position and advances one byte at a time. This is
    # the core abstraction for streaming dictionary algorithms: LZ78, LZW, etc.
    #
    # Usage in LZ78 encoding:
    #   cursor = TrieCursor.new
    #   data.each_byte do |byte|
    #     unless cursor.step(byte)
    #       emit Token(cursor.dict_id, byte)
    #       cursor.insert(byte, next_id)
    #       cursor.reset
    #     end
    #   end
    #   emit flush token unless cursor.at_root?
    class TrieCursor
      def initialize
        # Each node: { byte => {dict_id:, children:} }
        # We store the trie as nested hashes for simplicity and idiomatic Ruby.
        @root    = { dict_id: 0, children: {} }
        @current = @root
      end

      # Try to follow the child edge for +byte+ from current position.
      # Returns true and advances cursor if child exists; returns false otherwise
      # (cursor unchanged).
      def step(byte)
        child = @current[:children][byte]
        if child
          @current = child
          true
        else
          false
        end
      end

      # Add a child edge for +byte+ at current position with +dict_id+.
      # Does not advance the cursor — call reset to return to root.
      def insert(byte, dict_id)
        @current[:children][byte] = { dict_id: dict_id, children: {} }
      end

      # Return cursor to the root of the trie.
      def reset
        @current = @root
      end

      # Dictionary ID at the current cursor position.
      # Returns 0 when at root (empty sequence).
      def dict_id
        @current[:dict_id]
      end

      # True if cursor is at the root node.
      def at_root?
        @current.equal?(@root)
      end

      # Yield [path, dict_id] for every node that has a dict_id > 0 (DFS).
      def each(&block)
        each_node(@root, [], &block)
      end

      include Enumerable

      private

      def each_node(node, path, &block)
        yield path.dup, node[:dict_id] if node[:dict_id] > 0
        node[:children].each do |byte, child|
          path.push(byte)
          each_node(child, path, &block)
          path.pop
        end
      end
    end

    module Compressor
      # Encodes bytes into an LZ78 token stream.
      #
      # Uses TrieCursor to walk the dictionary one byte at a time.
      # When step(byte) returns false (no child edge), emits a token for the
      # current dict_id plus byte, records the new sequence, resets to root.
      #
      # @param data        [String]  binary string (encoding: ASCII-8BIT)
      # @param max_dict    [Integer] maximum dictionary size (default 65536)
      # @return            [Array<Token>]
      def self.encode(data, max_dict: 65536)
        cursor  = TrieCursor.new
        next_id = 1
        tokens  = []

        data.each_byte do |byte|
          unless cursor.step(byte)
            tokens << Token.new(cursor.dict_id, byte)

            if next_id < max_dict
              cursor.insert(byte, next_id)
              next_id += 1
            end

            cursor.reset
          end
        end

        # Flush partial match at end of stream.
        tokens << Token.new(cursor.dict_id, 0) unless cursor.at_root?

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
