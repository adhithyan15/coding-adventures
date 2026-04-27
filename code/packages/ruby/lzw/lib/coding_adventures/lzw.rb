# =============================================================================
# CodingAdventures::LZW
# =============================================================================
#
# LZW (Lempel-Ziv-Welch, 1984) lossless compression algorithm.
# Part of the CMP compression series in the coding-adventures monorepo.
#
# What Is LZW?
# ------------
#
# LZW is LZ78 with a pre-seeded dictionary: all 256 single-byte sequences are
# added before encoding begins (codes 0-255). This eliminates LZ78's mandatory
# next_char byte -- every symbol is already in the dictionary, so the encoder
# can emit pure codes.
#
# With only codes to transmit, LZW uses variable-width bit-packing: codes start
# at 9 bits and grow as the dictionary expands. This is exactly how GIF works.
#
# Reserved Codes
# --------------
#
#   0-255:  Pre-seeded single-byte entries.
#   256:    CLEAR_CODE -- reset to initial 256-entry state.
#   257:    STOP_CODE  -- end of code stream.
#   258+:   Dynamically added entries.
#
# Wire Format (CMP03)
# -------------------
#
#   Bytes 0-3:  original_length (big-endian uint32)
#   Bytes 4+:   bit-packed variable-width codes, LSB-first
#
# The Tricky Token
# ----------------
#
# During decoding, the decoder may receive code C == next_code (not yet added).
# This happens when the input has the form xyx...x. The fix:
#
#   entry = dict[prev_code] + dict[prev_code][0]
#
# The Series: CMP00 -> CMP05
# --------------------------
#
#   CMP00 (LZ77,    1977) -- Sliding-window backreferences.
#   CMP01 (LZ78,    1978) -- Explicit dictionary (trie).
#   CMP02 (LZSS,    1982) -- LZ77 + flag bits; no wasted literals.
#   CMP03 (LZW,     1984) -- LZ78 + pre-initialized dict; GIF. (this module)
#   CMP04 (Huffman, 1952) -- Entropy coding; prerequisite for DEFLATE.
#   CMP05 (DEFLATE, 1996) -- LZ77 + Huffman; ZIP/gzip/PNG/zlib.
# =============================================================================

require_relative "lzw/version"

module CodingAdventures
  module LZW
    # ---- Constants -----------------------------------------------------------

    CLEAR_CODE       = 256
    STOP_CODE        = 257
    INITIAL_NEXT     = 258
    INITIAL_CODE_SIZE = 9
    MAX_CODE_SIZE    = 16

    # ---- Bit I/O helpers -----------------------------------------------------

    # BitWriter accumulates variable-width codes into a byte string, LSB-first.
    #
    # Bits within each byte are filled from the least-significant end first.
    # This matches the GIF and Unix compress conventions.
    class BitWriter
      def initialize
        @buf     = 0   # bit accumulator (Integer, unbounded)
        @bit_pos = 0   # valid bits in @buf
        @output  = "".b
      end

      # Write +code+ using exactly +code_size+ bits.
      def write(code, code_size)
        @buf |= code << @bit_pos
        @bit_pos += code_size
        while @bit_pos >= 8
          @output << (@buf & 0xFF).chr
          @buf >>= 8
          @bit_pos -= 8
        end
      end

      # Flush any remaining bits as a final partial byte.
      def flush
        if @bit_pos > 0
          @output << (@buf & 0xFF).chr
          @buf     = 0
          @bit_pos = 0
        end
      end

      def bytes
        @output.dup
      end
    end

    # BitReader reads variable-width codes from a binary string, LSB-first.
    class BitReader
      def initialize(data)
        @data    = data.b
        @pos     = 0   # next byte index to consume
        @buf     = 0
        @bit_pos = 0
      end

      # Return the next +code_size+-bit code.
      # Raises EOFError if the stream is exhausted.
      def read(code_size)
        while @bit_pos < code_size
          raise EOFError, "unexpected end of bit stream" if @pos >= @data.bytesize
          @buf |= @data.getbyte(@pos) << @bit_pos
          @pos     += 1
          @bit_pos += 8
        end
        code = @buf & ((1 << code_size) - 1)
        @buf >>= code_size
        @bit_pos -= code_size
        code
      end

      def exhausted?
        @pos >= @data.bytesize && @bit_pos.zero?
      end
    end

    # ---- Encoder -------------------------------------------------------------

    # Encode +data+ (String or Array of ints) into an Array of LZW codes.
    #
    # Returns [codes, original_length]. The code array begins with CLEAR_CODE
    # and ends with STOP_CODE.
    #
    # The encode dictionary maps byte sequences (String keys) to integer codes.
    def self.encode_codes(data)
      bytes = data.is_a?(String) ? data.bytes : data
      original_length = bytes.size

      enc_dict = {}
      (0..255).each { |b| enc_dict[[b]] = b }
      next_code  = INITIAL_NEXT
      max_entries = 1 << MAX_CODE_SIZE

      codes = [CLEAR_CODE]
      w = []

      bytes.each do |b|
        wb = w + [b]
        if enc_dict.key?(wb)
          w = wb
        else
          codes << enc_dict[w]

          if next_code < max_entries
            enc_dict[wb] = next_code
            next_code += 1
          elsif next_code == max_entries
            codes << CLEAR_CODE
            enc_dict = {}
            (0..255).each { |i| enc_dict[[i]] = i }
            next_code = INITIAL_NEXT
          end

          w = [b]
        end
      end

      codes << enc_dict[w] unless w.empty?
      codes << STOP_CODE

      [codes, original_length]
    end

    # ---- Decoder -------------------------------------------------------------

    # Decode an Array of LZW codes back to a byte array.
    #
    # Handles CLEAR_CODE (reset), STOP_CODE (done), and the tricky-token
    # edge case (code == next_code).
    def self.decode_codes(codes)
      dec_dict = (0..255).map { |b| [b] }
      dec_dict << []   # 256 = CLEAR_CODE placeholder
      dec_dict << []   # 257 = STOP_CODE  placeholder
      next_code = INITIAL_NEXT

      output    = []
      prev_code = nil

      codes.each do |code|
        if code == CLEAR_CODE
          dec_dict = (0..255).map { |b| [b] }
          dec_dict << []
          dec_dict << []
          next_code = INITIAL_NEXT
          prev_code = nil
          next
        end

        break if code == STOP_CODE

        entry =
          if code < dec_dict.size
            dec_dict[code]
          elsif code == next_code
            # Tricky token: code not yet in dict.
            next unless prev_code  # malformed -- skip
            prev_entry = dec_dict[prev_code]
            prev_entry + [prev_entry[0]]
          else
            next  # invalid code -- skip
          end

        output.concat(entry)

        if prev_code && next_code < (1 << MAX_CODE_SIZE)
          prev_entry = dec_dict[prev_code]
          dec_dict << (prev_entry + [entry[0]])
          next_code += 1
        end

        prev_code = code
      end

      output
    end

    # ---- Serialisation -------------------------------------------------------

    # Pack an Array of LZW codes into the CMP03 wire format.
    #
    # Header: 4-byte big-endian original_length.
    # Body:   LSB-first variable-width bit-packed codes.
    def self.pack_codes(codes, original_length)
      writer    = BitWriter.new
      code_size = INITIAL_CODE_SIZE
      next_code = INITIAL_NEXT

      codes.each do |code|
        writer.write(code, code_size)

        if code == CLEAR_CODE
          code_size = INITIAL_CODE_SIZE
          next_code = INITIAL_NEXT
        elsif code != STOP_CODE
          if next_code < (1 << MAX_CODE_SIZE)
            next_code += 1
            code_size += 1 if next_code > (1 << code_size) && code_size < MAX_CODE_SIZE
          end
        end
      end
      writer.flush

      [original_length].pack("N") + writer.bytes
    end

    # Unpack CMP03 wire-format bytes into an Array of LZW codes.
    #
    # Returns [codes, original_length]. Stops on STOP_CODE or stream end.
    def self.unpack_codes(data)
      return [[CLEAR_CODE, STOP_CODE], 0] if data.bytesize < 4

      original_length = data.unpack1("N")
      reader = BitReader.new(data[4..])

      codes     = []
      code_size = INITIAL_CODE_SIZE
      next_code = INITIAL_NEXT

      until reader.exhausted?
        begin
          code = reader.read(code_size)
        rescue EOFError
          break
        end
        codes << code

        if code == STOP_CODE
          break
        elsif code == CLEAR_CODE
          code_size = INITIAL_CODE_SIZE
          next_code = INITIAL_NEXT
        elsif next_code < (1 << MAX_CODE_SIZE)
          next_code += 1
          code_size += 1 if next_code > (1 << code_size) && code_size < MAX_CODE_SIZE
        end
      end

      [codes, original_length]
    end

    # ---- Public API ----------------------------------------------------------

    # Compress +data+ (String) using LZW and return CMP03 wire-format bytes.
    def self.compress(data)
      bytes = data.is_a?(String) ? data.bytes : data
      codes, original_length = encode_codes(bytes)
      pack_codes(codes, original_length)
    end

    # Decompress CMP03 wire-format +data+ and return the original bytes as a
    # binary String.
    def self.decompress(data)
      codes, original_length = unpack_codes(data)
      result = decode_codes(codes)
      result = result[0, original_length] if result.size > original_length
      result.pack("C*")
    end
  end
end
