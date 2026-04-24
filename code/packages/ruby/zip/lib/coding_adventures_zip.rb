# frozen_string_literal: true

# coding_adventures_zip.rb — CMP09: ZIP archive format (PKZIP, 1989).
#
# ZIP bundles one or more files into a single .zip archive, compressing each
# entry independently with DEFLATE (method 8) or storing it verbatim (method 0).
# The same format underlies Java JARs, Office Open XML (.docx/.xlsx), Android
# APKs (.apk), Python wheels (.whl), and many more.
#
# Architecture:
#
#   ┌─────────────────────────────────────────────────────┐
#   │  [Local File Header + File Data]  ← entry 1         │
#   │  [Local File Header + File Data]  ← entry 2         │
#   │  ...                                                │
#   │  ══════════ Central Directory ══════════            │
#   │  [Central Dir Header]  ← entry 1 (has local offset)│
#   │  [Central Dir Header]  ← entry 2                   │
#   │  [End of Central Directory Record]                  │
#   └─────────────────────────────────────────────────────┘
#
# DEFLATE Inside ZIP:
# ZIP method 8 stores raw RFC 1951 DEFLATE — no zlib wrapper. This
# implementation uses fixed Huffman blocks (BTYPE=01) and the
# coding_adventures_lzss gem for LZ77 match-finding.
#
# Series:
#   CMP02 (LZSS,    1982) — LZ77 + flag bits.  ← dependency
#   CMP05 (DEFLATE, 1996) — LZ77 + Huffman; ZIP/gzip/PNG/zlib.
#   CMP09 (ZIP,     1989) — DEFLATE container; universal archive. ← this file
#
# === Usage ===
#
#   require "coding_adventures_zip"
#
#   archive = CodingAdventures::Zip.zip([["hello.txt", "Hello!"]])
#   files   = CodingAdventures::Zip.unzip(archive)
#   # files["hello.txt"] => "Hello!"

require_relative "coding_adventures/zip/version"
require "coding_adventures_lzss"

module CodingAdventures
  module Zip
    # =========================================================================
    # CRC-32
    # =========================================================================
    #
    # CRC-32 uses polynomial 0xEDB88320 (reflected form of 0x04C11DB7).
    # The IEEE 802.3 CRC used by ZIP, gzip, PNG, and Ethernet.

    CRC_TABLE = Array.new(256) do |i|
      c = i
      8.times { c = (c & 1 == 1) ? (0xEDB88320 ^ (c >> 1)) : (c >> 1) }
      c
    end.freeze

    # Compute CRC-32 over +data+ (binary String), starting from +initial+.
    # Pass a previous result as +initial+ for incremental updates.
    #
    #   crc32("hello world") # => 0x0D4A1185
    def self.crc32(data, initial: 0)
      crc = initial ^ 0xFFFFFFFF
      data.each_byte { |b| crc = CRC_TABLE[(crc ^ b) & 0xFF] ^ (crc >> 8) }
      crc ^ 0xFFFFFFFF
    end

    # =========================================================================
    # RFC 1951 DEFLATE — Bit I/O
    # =========================================================================
    #
    # RFC 1951 packs bits LSB-first. Huffman codes are written MSB-first
    # logically, so they are bit-reversed before writing LSB-first.

    # Reverse the lowest +nbits+ bits of +value+.
    def self.reverse_bits(value, nbits)
      result = 0
      nbits.times do
        result = (result << 1) | (value & 1)
        value >>= 1
      end
      result
    end

    # Accumulates bits LSB-first and flushes whole bytes to an array.
    class BitWriter
      def initialize
        @buf = 0
        @bits = 0
        @out = []
      end

      # Write +nbits+ of +value+, LSB first.
      def write_lsb(value, nbits)
        @buf |= (value & ((1 << nbits) - 1)) << @bits
        @bits += nbits
        while @bits >= 8
          @out << (@buf & 0xFF)
          @buf >>= 8
          @bits -= 8
        end
      end

      # Write a Huffman code by bit-reversing it first (MSB → LSB order).
      def write_huffman(code, nbits)
        write_lsb(CodingAdventures::Zip.reverse_bits(code, nbits), nbits)
      end

      # Pad to byte boundary, discarding partial bits.
      def align
        return if @bits.zero?
        @out << (@buf & 0xFF)
        @buf = 0
        @bits = 0
      end

      # Flush remaining bits and return the output as a binary String.
      def finish
        align
        @out.pack("C*")
      end
    end

    # Reads bits LSB-first from a binary String.
    class BitReader
      def initialize(data)
        @data = data.bytes
        @pos = 0
        @buf = 0
        @bits = 0
      end

      # Fill the buffer with at least +need+ bits. Returns false on EOF.
      def fill(need)
        while @bits < need
          return false if @pos >= @data.length
          @buf |= @data[@pos] << @bits
          @pos += 1
          @bits += 8
        end
        true
      end
      private :fill

      # Read +nbits+ from the stream (LSB first). Returns nil on EOF.
      def read_lsb(nbits)
        return 0 if nbits.zero?
        return nil unless fill(nbits)
        val = @buf & ((1 << nbits) - 1)
        @buf >>= nbits
        @bits -= nbits
        val
      end

      # Read +nbits+ and bit-reverse (for reading Huffman codes MSB first).
      def read_msb(nbits)
        v = read_lsb(nbits)
        v.nil? ? nil : CodingAdventures::Zip.reverse_bits(v, nbits)
      end

      # Discard partial byte bits to align to a byte boundary.
      def align
        discard = @bits % 8
        return if discard.zero?
        @buf >>= discard
        @bits -= discard
      end
    end

    # =========================================================================
    # RFC 1951 DEFLATE — Fixed Huffman Tables (§3.2.6)
    # =========================================================================
    #
    # Code lengths per symbol range:
    #   Symbols   0–143: 8-bit codes, starting at 0b00110000 (= 48)
    #   Symbols 144–255: 9-bit codes, starting at 0b110010000 (= 400)
    #   Symbols 256–279: 7-bit codes, starting at 0b0000000 (= 0)
    #   Symbols 280–287: 8-bit codes, starting at 0b11000000 (= 192)
    # Distance codes 0–29: 5-bit codes equal to the code number.

    # Return [code, nbits] for a literal/length symbol.
    def self.fixed_ll_encode(sym)
      return [0b00110000 + sym, 8] if sym <= 143
      return [0b110010000 + (sym - 144), 9] if sym <= 255
      return [sym - 256, 7] if sym <= 279
      return [0b11000000 + (sym - 280), 8] if sym <= 287
      raise ArgumentError, "fixed_ll_encode: invalid symbol #{sym}"
    end

    # Decode one symbol from +br+ using the fixed Huffman table.
    # Returns nil on EOF.
    def self.fixed_ll_decode(br)
      v7 = br.read_msb(7)
      return nil if v7.nil?
      return v7 + 256 if v7 <= 23   # 7-bit codes: 256-279

      extra = br.read_lsb(1)
      return nil if extra.nil?
      v8 = (v7 << 1) | extra

      return v8 - 48 if v8.between?(48, 191)   # literals 0-143
      return v8 + 88 if v8.between?(192, 199)  # symbols 280-287

      extra2 = br.read_lsb(1)
      return nil if extra2.nil?
      v9 = (v8 << 1) | extra2
      return v9 - 256 if v9.between?(400, 511)  # literals 144-255
      nil
    end

    # =========================================================================
    # RFC 1951 DEFLATE — Length / Distance Tables
    # =========================================================================
    #
    # Each entry is [base_value, extra_bits]. The length symbol is 257 + index.

    LENGTH_TABLE = [
      [3, 0], [4, 0], [5, 0], [6, 0], [7, 0], [8, 0], [9, 0], [10, 0],
      [11, 1], [13, 1], [15, 1], [17, 1],
      [19, 2], [23, 2], [27, 2], [31, 2],
      [35, 3], [43, 3], [51, 3], [59, 3],
      [67, 4], [83, 4], [99, 4], [115, 4],
      [131, 5], [163, 5], [195, 5], [227, 5]
    ].freeze

    DIST_TABLE = [
      [1, 0], [2, 0], [3, 0], [4, 0],
      [5, 1], [7, 1], [9, 2], [13, 2],
      [17, 3], [25, 3], [33, 4], [49, 4],
      [65, 5], [97, 5], [129, 6], [193, 6],
      [257, 7], [385, 7], [513, 8], [769, 8],
      [1025, 9], [1537, 9], [2049, 10], [3073, 10],
      [4097, 11], [6145, 11], [8193, 12], [12289, 12],
      [16385, 13], [24577, 13]
    ].freeze

    # Find the length symbol (257+) and extra bits for a match +length+.
    def self.encode_length(length)
      (LENGTH_TABLE.length - 1).downto(0) do |i|
        base, extra = LENGTH_TABLE[i]
        return [257 + i, base, extra] if length >= base
      end
      raise "encode_length: unreachable for length=#{length}"
    end

    # Find the distance code and extra bits for a back-reference +offset+.
    def self.encode_dist(offset)
      (DIST_TABLE.length - 1).downto(0) do |i|
        base, extra = DIST_TABLE[i]
        return [i, base, extra] if offset >= base
      end
      raise "encode_dist: unreachable for offset=#{offset}"
    end

    # =========================================================================
    # RFC 1951 DEFLATE — Compress (fixed Huffman, BTYPE=01)
    # =========================================================================

    # Maximum decompressed output size (256 MiB) — prevents zip-bomb expansion.
    MAX_OUTPUT = 256 * 1024 * 1024

    # Compress +data+ (binary String) into raw RFC 1951 DEFLATE bytes.
    # Uses fixed Huffman codes (BTYPE=01) and LZSS for LZ77 match-finding.
    def self.deflate_compress(data)
      bw = BitWriter.new

      # Empty input → stored block (BTYPE=00) shortcut.
      if data.empty?
        bw.write_lsb(1, 1)       # BFINAL=1
        bw.write_lsb(0, 2)       # BTYPE=00 (stored)
        bw.align
        bw.write_lsb(0x0000, 16) # LEN=0
        bw.write_lsb(0xFFFF, 16) # NLEN=~0
        return bw.finish
      end

      tokens = CodingAdventures::LZSS.encode(data, window_size: 32768, max_match: 255, min_match: 3)

      bw.write_lsb(1, 1) # BFINAL=1
      bw.write_lsb(1, 1) # BTYPE bit 0 = 1
      bw.write_lsb(0, 1) # BTYPE bit 1 = 0  → BTYPE = 01 (fixed Huffman)

      tokens.each do |tok|
        case tok
        when CodingAdventures::LZSS::Literal
          code, nbits = fixed_ll_encode(tok.byte)
          bw.write_huffman(code, nbits)
        when CodingAdventures::LZSS::Match
          sym, base_len, extra_len_bits = encode_length(tok.length)
          code, nbits = fixed_ll_encode(sym)
          bw.write_huffman(code, nbits)
          bw.write_lsb(tok.length - base_len, extra_len_bits) if extra_len_bits > 0

          dist_code, base_dist, extra_dist_bits = encode_dist(tok.offset)
          bw.write_huffman(dist_code, 5)
          bw.write_lsb(tok.offset - base_dist, extra_dist_bits) if extra_dist_bits > 0
        end
      end

      eob_code, eob_bits = fixed_ll_encode(256)
      bw.write_huffman(eob_code, eob_bits)
      bw.finish
    end
    # deflate_compress is called by ZipWriter (a nested class), so it cannot

    # =========================================================================
    # RFC 1951 DEFLATE — Decompress
    # =========================================================================

    # Decompress raw RFC 1951 DEFLATE bytes to a binary String.
    def self.deflate_decompress(data)
      br = BitReader.new(data)
      out = []

      loop do
        bfinal = br.read_lsb(1)
        raise "deflate: unexpected EOF reading BFINAL" if bfinal.nil?
        btype = br.read_lsb(2)
        raise "deflate: unexpected EOF reading BTYPE" if btype.nil?

        case btype
        when 0
          # Stored block
          br.align
          len_val = br.read_lsb(16)
          raise "deflate: EOF reading stored LEN" if len_val.nil?
          nlen = br.read_lsb(16)
          raise "deflate: EOF reading stored NLEN" if nlen.nil?
          raise "deflate: LEN/NLEN mismatch" if (nlen ^ 0xFFFF) != len_val
          raise "deflate: output size limit exceeded" if out.length + len_val > MAX_OUTPUT
          len_val.times do
            b = br.read_lsb(8)
            raise "deflate: EOF inside stored block data" if b.nil?
            out << b
          end
        when 1
          # Fixed Huffman block
          loop do
            sym = fixed_ll_decode(br)
            raise "deflate: EOF decoding fixed Huffman symbol" if sym.nil?
            if sym < 256
              raise "deflate: output size limit exceeded" if out.length >= MAX_OUTPUT
              out << sym
            elsif sym == 256
              break
            elsif sym.between?(257, 285)
              idx = sym - 257
              base_len, extra_len_bits = LENGTH_TABLE[idx]
              extra_len = br.read_lsb(extra_len_bits)
              raise "deflate: EOF reading length extra bits" if extra_len.nil?
              length = base_len + extra_len

              dist_code = br.read_msb(5)
              raise "deflate: EOF reading distance code" if dist_code.nil?
              base_dist, extra_dist_bits = DIST_TABLE[dist_code]
              raise "deflate: invalid dist code #{dist_code}" if base_dist.nil?
              extra_dist = br.read_lsb(extra_dist_bits)
              raise "deflate: EOF reading distance extra bits" if extra_dist.nil?
              offset = base_dist + extra_dist

              raise "deflate: back-reference offset #{offset} > output len #{out.length}" if offset > out.length
              raise "deflate: output size limit exceeded" if out.length + length > MAX_OUTPUT
              length.times { |i| out << out[out.length - offset] }
            else
              raise "deflate: invalid LL symbol #{sym}"
            end
          end
        when 2
          raise "deflate: dynamic Huffman blocks (BTYPE=10) not supported"
        else
          raise "deflate: reserved BTYPE=11"
        end

        break if bfinal == 1
      end

      out.pack("C*")
    end
    # deflate_decompress is called by ZipReader (a nested class), so it cannot

    # =========================================================================
    # MS-DOS Date / Time Encoding
    # =========================================================================

    # Encode a calendar date/time into the 32-bit MS-DOS datetime used by ZIP.
    # Returns a 32-bit integer: high 16 bits = date, low 16 bits = time.
    #
    #   dos_datetime(1980, 1, 1)          # => 0x00210000
    def self.dos_datetime(year, month, day, hour = 0, minute = 0, second = 0)
      t = (hour << 11) | (minute << 5) | (second >> 1)
      d = ([year - 1980, 0].max << 9) | (month << 5) | day
      ((d & 0xFFFF) << 16) | (t & 0xFFFF)
    end

    # Fixed timestamp 1980-01-01 00:00:00 used for all entries.
    DOS_EPOCH = dos_datetime(1980, 1, 1)

    # =========================================================================
    # ZIP Write — ZipWriter
    # =========================================================================

    # Builds a ZIP archive incrementally in memory.
    class ZipWriter
      CdRecord = Struct.new(:name_bytes, :method, :crc, :compressed_size,
        :uncompressed_size, :local_offset, :external_attrs)

      def initialize
        @buf = String.new("", encoding: "BINARY")
        @entries = []
      end

      # Add a file entry. Compress with DEFLATE if it reduces size.
      def add_file(name, data, compress: true)
        add_entry(name, data, compress, 0o100644)
      end

      # Add a directory entry (name should end with '/').
      def add_directory(name)
        add_entry(name, "".b, false, 0o040755)
      end

      # Append Central Directory + EOCD and return the archive as a binary String.
      def finish
        cd_offset = @buf.bytesize
        cd_start = @buf.bytesize

        @entries.each do |e|
          version_needed = (e.method == 8) ? 20 : 10
          @buf << pack_le32(0x02014B50)
          @buf << pack_le16(0x031E)                          # version_made_by
          @buf << pack_le16(version_needed)
          @buf << pack_le16(0x0800)                          # flags (UTF-8)
          @buf << pack_le16(e.method)
          @buf << pack_le16(DOS_EPOCH & 0xFFFF)              # mod_time
          @buf << pack_le16((DOS_EPOCH >> 16) & 0xFFFF)      # mod_date
          @buf << pack_le32(e.crc)
          @buf << pack_le32(e.compressed_size)
          @buf << pack_le32(e.uncompressed_size)
          @buf << pack_le16(e.name_bytes.bytesize)
          @buf << pack_le16(0)   # extra_len
          @buf << pack_le16(0)   # comment_len
          @buf << pack_le16(0)   # disk_start
          @buf << pack_le16(0)   # internal_attrs
          @buf << pack_le32(e.external_attrs)
          @buf << pack_le32(e.local_offset)
          @buf << e.name_bytes
        end

        cd_size = @buf.bytesize - cd_start

        @buf << pack_le32(0x06054B50)   # EOCD signature
        raise "zip: entry count #{@entries.length} exceeds ZIP limit of 65535" if @entries.length > 65535

        @buf << pack_le16(0)
        @buf << pack_le16(0)
        @buf << pack_le16(@entries.length)
        @buf << pack_le16(@entries.length)
        @buf << pack_le32(cd_size)
        @buf << pack_le32(cd_offset)
        @buf << pack_le16(0)   # comment_len

        @buf.dup
      end

      private

      def add_entry(name, data, compress, unix_mode)
        name_bytes = name.encode("UTF-8").b
        checksum = CodingAdventures::Zip.crc32(data)
        uncompressed_size = data.bytesize

        if compress && !data.empty?
          compressed = CodingAdventures::Zip.deflate_compress(data)
          if compressed.bytesize < data.bytesize
            method = 8
            file_data = compressed
          else
            method = 0
            file_data = data
          end
        else
          method = 0
          file_data = data
        end

        compressed_size = file_data.bytesize
        local_offset = @buf.bytesize
        version_needed = (method == 8) ? 20 : 10

        # Local File Header
        @buf << pack_le32(0x04034B50)
        @buf << pack_le16(version_needed)
        @buf << pack_le16(0x0800)              # flags (UTF-8)
        @buf << pack_le16(method)
        @buf << pack_le16(DOS_EPOCH & 0xFFFF)  # mod_time
        @buf << pack_le16((DOS_EPOCH >> 16) & 0xFFFF) # mod_date
        @buf << pack_le32(checksum)
        @buf << pack_le32(compressed_size)
        @buf << pack_le32(uncompressed_size)
        @buf << pack_le16(name_bytes.bytesize)
        @buf << pack_le16(0)   # extra_field_length
        @buf << name_bytes
        @buf << file_data

        @entries << CdRecord.new(name_bytes, method, checksum,
          compressed_size, uncompressed_size,
          local_offset, (unix_mode << 16) & 0xFFFFFFFF)
      end

      def pack_le16(v) = [v & 0xFFFF].pack("v")
      def pack_le32(v) = [v & 0xFFFFFFFF].pack("V")
    end

    # =========================================================================
    # ZIP Read — ZipEntry and ZipReader
    # =========================================================================

    # Metadata for a single entry inside a ZIP archive.
    ZipEntry = Struct.new(:name, :size, :compressed_size, :method, :crc32,
      :is_directory, :local_offset) do
      def directory? = is_directory
    end

    # Reads entries from an in-memory ZIP archive (binary String).
    class ZipReader
      def initialize(data)
        @data = data.b
        eocd_off = find_eocd
        raise "zip: no End of Central Directory record found" if eocd_off.nil?

        cd_size = read_le32(eocd_off + 12)
        cd_offset = read_le32(eocd_off + 16)
        raise "zip: Central Directory out of bounds" if cd_offset + cd_size > @data.bytesize

        @entries = []
        pos = cd_offset

        while pos + 4 <= cd_offset + cd_size
          break if read_le32(pos) != 0x02014B50

          # Guard: minimum CD header is 46 bytes before the variable-length fields.
          raise "zip: CD entry header out of bounds" if pos + 46 > @data.bytesize

          method = read_le16(pos + 10)
          crc32v = read_le32(pos + 16)
          compressed_size = read_le32(pos + 20)
          size = read_le32(pos + 24)
          name_len = read_le16(pos + 28)
          extra_len = read_le16(pos + 30)
          comment_len = read_le16(pos + 32)
          local_offset = read_le32(pos + 42)

          # Validate that all fixed fields were readable (nil = byteslice out of bounds).
          raise "zip: CD entry fields truncated" if [method, crc32v, compressed_size,
            size, name_len, extra_len, comment_len, local_offset].any?(&:nil?)

          name_start = pos + 46
          name_end = name_start + name_len

          raise "zip: CD entry name out of bounds" if name_end > @data.bytesize

          # force_encoding relabels the bytes; scrub replaces any invalid UTF-8
          # sequences with U+FFFD rather than raising on attacker-crafted names.
          name = @data.byteslice(name_start, name_len).force_encoding("UTF-8").scrub

          @entries << ZipEntry.new(name, size, compressed_size, method,
            crc32v, name.end_with?("/"), local_offset)

          # Guard: ensure pos advancement stays within declared CD region.
          next_pos = name_end + extra_len + comment_len
          raise "zip: CD entry advance out of bounds" if next_pos > cd_offset + cd_size
          pos = next_pos
        end
      end

      # Returns a copy of all ZipEntry objects.
      def entries = @entries.dup

      # Decompress and return one entry's data as a binary String.
      def read(entry)
        return "".b if entry.directory?

        local_flags = read_le16(entry.local_offset + 6)
        raise "zip: local header out of bounds" if local_flags.nil?
        raise "zip: entry '#{entry.name}' is encrypted" if local_flags & 1 != 0

        lh_name_len = read_le16(entry.local_offset + 26)
        lh_extra_len = read_le16(entry.local_offset + 28)
        raise "zip: local header fields out of bounds for '#{entry.name}'" if lh_name_len.nil? || lh_extra_len.nil?
        data_start = entry.local_offset + 30 + lh_name_len + lh_extra_len
        data_end = data_start + entry.compressed_size
        raise "zip: entry '#{entry.name}' data out of bounds" if data_end > @data.bytesize

        compressed = @data.byteslice(data_start, entry.compressed_size)

        decompressed = case entry.method
        when 0 then compressed
        when 8 then CodingAdventures::Zip.deflate_decompress(compressed)
        else raise "zip: unsupported compression method #{entry.method} for '#{entry.name}'"
        end

        decompressed = decompressed.byteslice(0, entry.size) if decompressed.bytesize > entry.size

        actual_crc = CodingAdventures::Zip.crc32(decompressed)
        if actual_crc != entry.crc32
          raise "zip: CRC-32 mismatch for '#{entry.name}': " \
                "expected #{entry.crc32.to_s(16)}, got #{actual_crc.to_s(16)}"
        end

        decompressed
      end

      # Convenience: find and read an entry by name.
      def read_by_name(name)
        entry = @entries.find { |e| e.name == name }
        raise "zip: entry '#{name}' not found" if entry.nil?
        read(entry)
      end

      private

      # Scan backwards from end of file for the EOCD signature.
      # The EOCD can have a variable-length comment (max 65535 bytes).
      def find_eocd
        eocd_sig = 0x06054B50
        eocd_min = 22
        max_comment = 65535

        return nil if @data.bytesize < eocd_min

        scan_start = [@data.bytesize - eocd_min - max_comment, 0].max
        i = @data.bytesize - eocd_min

        while i >= scan_start
          if read_le32(i) == eocd_sig
            comment_len = read_le16(i + 20)
            return i if i + eocd_min + comment_len == @data.bytesize
          end
          i -= 1
        end
        nil
      end

      def read_le16(off) = @data.byteslice(off, 2)&.unpack1("v")
      def read_le32(off) = @data.byteslice(off, 4)&.unpack1("V")
    end

    # =========================================================================
    # Convenience Functions
    # =========================================================================

    # Compress an array of +[name, data]+ pairs into a ZIP archive.
    # Returns a binary String.
    #
    #   archive = CodingAdventures::Zip.zip([["hello.txt", "Hello!"]])
    def self.zip(entries, compress: true)
      w = ZipWriter.new
      entries.each { |name, data| w.add_file(name, data, compress: compress) }
      w.finish
    end

    # Decompress all file entries from a ZIP archive.
    # Returns a Hash of +name => data+ (binary Strings).
    #
    #   files = CodingAdventures::Zip.unzip(archive)
    #   files["hello.txt"]  # => "Hello!"
    def self.unzip(data)
      reader = ZipReader.new(data)
      reader.entries.each_with_object({}) do |entry, h|
        h[entry.name] = reader.read(entry) unless entry.directory?
      end
    end
  end
end
