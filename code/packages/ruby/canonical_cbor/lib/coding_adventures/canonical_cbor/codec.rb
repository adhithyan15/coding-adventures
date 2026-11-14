# frozen_string_literal: true

require_relative "constants"
require_relative "types"

module CodingAdventures
  module CanonicalCbor
    module Codec
      EncodedMapEntry = Data.define(:key_buf, :value)
      private_constant :EncodedMapEntry

      class ByteCursor
        attr_reader :data, :pos

        def initialize(data)
          @data = data.b
          @pos = 0
        end

        def remaining
          @data.bytesize - @pos
        end

        def read_u8
          if @pos >= @data.bytesize
            raise CborError.new(:unexpected_eof)
          end
          b = @data.getbyte(@pos)
          @pos += 1
          b
        end

        def read_n(n)
          end_pos = @pos + n
          if end_pos > @data.bytesize
            raise CborError.new(:unexpected_eof)
          end
          slice = @data.byteslice(@pos, n)
          @pos = end_pos
          slice
        end
      end
      private_constant :ByteCursor

      def self.encode_checked(value)
        out = String.new(encoding: Encoding::BINARY)
        encode_into_buffer(value, out, Constants::ROOT_DEPTH)
        out.freeze
      end

      def self.encode_into_checked(value, dst)
        unless dst.is_a?(String) && !dst.frozen? && dst.encoding == Encoding::BINARY
          raise CborError.new(:unsupported_simple)
        end
        encoded = encode_checked(value)
        dst << encoded
      end

      def self.encode(value)
        encode_checked(value)
      end

      def self.encode_into(value, dst)
        encode_into_checked(value, dst)
      end

      def self.decode(data)
        bytes = normalize_bytes(data)
        reader = ByteCursor.new(bytes)
        value = read_value(reader, Constants::ROOT_DEPTH)
        if reader.pos != reader.data.bytesize
          raise CborError.new(:trailing_bytes)
        end
        value
      end

      def self.decode_map_spans(data)
        bytes = normalize_bytes(data)
        if bytes.empty?
          raise CborError.new(:unexpected_eof)
        end
        if (bytes.getbyte(0) >> Constants::MAJOR_TYPE_SHIFT) != Constants::MAJOR_MAP
          decode(bytes)
          return DecodeMapSpansResult.non_map
        end

        reader = ByteCursor.new(bytes)
        _, _, arg = read_header(reader)
        count = check_length(arg, reader.remaining, Constants::MIN_BYTES_PER_MAP_ENTRY)
        entries = read_spanned_map_entries(reader, Constants::ROOT_DEPTH, count)

        if reader.pos != reader.data.bytesize
          raise CborError.new(:trailing_bytes)
        end
        DecodeMapSpansResult.map(entries)
      end

      class << self
        private

        def normalize_bytes(data)
          CanonicalCbor.normalize_bytes(data, :unexpected_eof)
        end

        def encode_into_buffer(v, out, depth)
          if depth > Constants::MAX_NESTING_DEPTH
            raise CborError.new(:encode_too_deep)
          end
          unless v.is_a?(CborValue)
            raise CborError.new(:unsupported_simple)
          end

          case v.kind
          when :unsigned
            write_type_and_argument(out, Constants::MAJOR_UNSIGNED, v.value)
          when :negative
            write_type_and_argument(out, Constants::MAJOR_NEGATIVE, v.value)
          when :bytes
            b = v.bytes
            write_type_and_argument(out, Constants::MAJOR_BYTES, b.bytesize)
            append_checked(out, b)
          when :text
            t = v.text
            utf8_bytes = t.b
            write_type_and_argument(out, Constants::MAJOR_TEXT, utf8_bytes.bytesize)
            append_checked(out, utf8_bytes)
          when :array
            items = v.items
            write_type_and_argument(out, Constants::MAJOR_ARRAY, items.length)
            items.each do |item|
              encode_into_buffer(item, out, depth + 1)
            end
          when :map
            encode_map(v.entries, out, depth)
          when :tag
            write_type_and_argument(out, Constants::MAJOR_TAG, v.tag)
            encode_into_buffer(v.value, out, depth + 1)
          when :boolean
            push_byte_checked(out, v.value ? Constants::WIRE_BYTE_TRUE : Constants::WIRE_BYTE_FALSE)
          when :null
            push_byte_checked(out, Constants::WIRE_BYTE_NULL)
          else
            raise CborError.new(:unsupported_simple)
          end
        end

        def encode_map(entries, out, depth)
          encoded_entries = entries.map do |entry|
            key_buf = String.new(encoding: Encoding::BINARY)
            encode_into_buffer(entry.key, key_buf, depth + 1)
            EncodedMapEntry.new(key_buf.freeze, entry.value)
          end

          encoded_entries.sort! do |a, b|
            len_cmp = a.key_buf.bytesize <=> b.key_buf.bytesize
            if len_cmp != 0
              len_cmp
            else
              a.key_buf <=> b.key_buf
            end
          end

          (1...encoded_entries.length).each do |i|
            if encoded_entries[i - 1].key_buf == encoded_entries[i].key_buf
              raise CborError.new(:duplicate_map_key)
            end
          end

          write_type_and_argument(out, Constants::MAJOR_MAP, encoded_entries.length)
          encoded_entries.each do |entry|
            append_checked(out, entry.key_buf)
            encode_into_buffer(entry.value, out, depth + 1)
          end
        end

        def write_type_and_argument(out, major, arg)
          major_bits = major << Constants::MAJOR_TYPE_SHIFT

          if arg <= Constants::INFO_INLINE_MAX
            push_byte_checked(out, major_bits | arg)
          elsif arg <= Constants::MAX_UINT8
            push_byte_checked(out, major_bits | Constants::INFO_1_BYTE)
            push_byte_checked(out, arg)
          elsif arg <= Constants::MAX_UINT16
            push_byte_checked(out, major_bits | Constants::INFO_2_BYTES)
            write_big_endian_uint(out, arg, Constants::ARG_WIDTH_2_BYTES)
          elsif arg <= Constants::MAX_UINT32
            push_byte_checked(out, major_bits | Constants::INFO_4_BYTES)
            write_big_endian_uint(out, arg, Constants::ARG_WIDTH_4_BYTES)
          else
            push_byte_checked(out, major_bits | Constants::INFO_8_BYTES)
            write_big_endian_uint(out, arg, Constants::ARG_WIDTH_8_BYTES)
          end
        end

        def write_big_endian_uint(out, value, byte_count)
          (byte_count - 1).downto(0) do |i|
            shift = i * Constants::BITS_PER_BYTE
            push_byte_checked(out, (value >> shift) & Constants::BYTE_MASK)
          end
        end

        def read_big_endian_uint(bytes, offset, byte_count)
          val = 0
          byte_count.times do |i|
            val = (val << Constants::BITS_PER_BYTE) | bytes.getbyte(offset + i)
          end
          val
        end

        def push_byte_checked(out, b)
          if out.bytesize >= Constants::MAX_ENCODED_BYTES
            raise CborError.new(:encode_too_large)
          end
          out << (b & Constants::BYTE_MASK)
        end

        def append_checked(out, data)
          if data.bytesize > Constants::MAX_ENCODED_BYTES - out.bytesize
            raise CborError.new(:encode_too_large)
          end
          out << data
        end

        def check_length(declared, remaining, min_bytes_per_unit)
          if declared > remaining || declared > remaining / min_bytes_per_unit
            raise CborError.new(:length_too_large)
          end
          declared
        end

        def validate_minimal_argument(major, info, arg)
          return if major == Constants::MAJOR_SIMPLE_OR_FLOAT

          case info
          when Constants::INFO_1_BYTE
            if arg <= Constants::INFO_INLINE_MAX
              raise CborError.new(:non_minimal_integer)
            end
          when Constants::INFO_2_BYTES
            if arg <= Constants::MAX_UINT8
              raise CborError.new(:non_minimal_integer)
            end
          when Constants::INFO_4_BYTES
            if arg <= Constants::MAX_UINT16
              raise CborError.new(:non_minimal_integer)
            end
          when Constants::INFO_8_BYTES
            if arg <= Constants::MAX_UINT32
              raise CborError.new(:non_minimal_integer)
            end
          end
        end

        def read_header(reader)
          header_byte = reader.read_u8
          major = header_byte >> Constants::MAJOR_TYPE_SHIFT
          info = header_byte & Constants::ADDITIONAL_INFO_MASK

          arg = if info <= Constants::INFO_INLINE_MAX
            info
          elsif info == Constants::INFO_1_BYTE
            reader.read_u8
          elsif info == Constants::INFO_2_BYTES
            bytes = reader.read_n(Constants::ARG_WIDTH_2_BYTES)
            read_big_endian_uint(bytes, 0, Constants::ARG_WIDTH_2_BYTES)
          elsif info == Constants::INFO_4_BYTES
            bytes = reader.read_n(Constants::ARG_WIDTH_4_BYTES)
            read_big_endian_uint(bytes, 0, Constants::ARG_WIDTH_4_BYTES)
          elsif info == Constants::INFO_8_BYTES
            bytes = reader.read_n(Constants::ARG_WIDTH_8_BYTES)
            read_big_endian_uint(bytes, 0, Constants::ARG_WIDTH_8_BYTES)
          elsif info.between?(Constants::INFO_RESERVED_28, Constants::INFO_RESERVED_30)
            raise CborError.new(:reserved)
          else
            # info == Constants::INFO_INDEFINITE
            raise CborError.new(:indefinite)
          end

          validate_minimal_argument(major, info, arg)
          [major, info, arg]
        end

        def key_strictly_less?(a, b)
          if a.bytesize != b.bytesize
            return a.bytesize < b.bytesize
          end
          a < b
        end

        def read_spanned_map_entries(reader, parent_depth, entry_count)
          entries = []
          prev_key_bytes = nil

          entry_count.times do
            key_start = reader.pos
            key = read_value(reader, parent_depth + 1)
            key_end = reader.pos
            cur_key_bytes = reader.data.byteslice(key_start, key_end - key_start)

            if prev_key_bytes && !key_strictly_less?(prev_key_bytes, cur_key_bytes)
              raise CborError.new(:non_canonical_map_order)
            end
            prev_key_bytes = cur_key_bytes

            val_start = reader.pos
            val = read_value(reader, parent_depth + 1)
            val_end = reader.pos

            entries << SpannedMapEntry.new(
              key: key,
              value: val,
              value_span: Span.new(val_start, val_end)
            )
          end
          entries
        end

        def read_array(reader, depth, arg)
          count = check_length(arg, reader.remaining, Constants::MIN_BYTES_PER_ELEMENT)
          items = Array.new(count) do
            read_value(reader, depth + 1)
          end
          CborArray.new(items)
        end

        def read_map(reader, depth, arg)
          count = check_length(arg, reader.remaining, Constants::MIN_BYTES_PER_MAP_ENTRY)
          spanned = read_spanned_map_entries(reader, depth, count)
          entries = spanned.map { |s| CborMapEntry.new(s.key, s.value) }
          CborMap.new(entries)
        end

        def read_value(reader, depth)
          if depth > Constants::MAX_NESTING_DEPTH
            raise CborError.new(:too_deep)
          end

          major, info, arg = read_header(reader)

          case major
          when Constants::MAJOR_UNSIGNED
            CborUnsigned.new(arg)
          when Constants::MAJOR_NEGATIVE
            CborNegative.new(arg)
          when Constants::MAJOR_BYTES
            length = check_length(arg, reader.remaining, Constants::MIN_BYTES_PER_ELEMENT)
            bs = reader.read_n(length)
            CborBytes.new(bs)
          when Constants::MAJOR_TEXT
            length = check_length(arg, reader.remaining, Constants::MIN_BYTES_PER_ELEMENT)
            bs = reader.read_n(length)
            str = bs.dup.force_encoding(Encoding::UTF_8)
            unless str.valid_encoding?
              raise CborError.new(:invalid_utf8)
            end
            CborText.new(str)
          when Constants::MAJOR_ARRAY
            read_array(reader, depth, arg)
          when Constants::MAJOR_MAP
            read_map(reader, depth, arg)
          when Constants::MAJOR_TAG
            inner = read_value(reader, depth + 1)
            CborTag.new(arg, inner)
          else
            case info
            when Constants::SIMPLE_FALSE
              CborBool.new(false)
            when Constants::SIMPLE_TRUE
              CborBool.new(true)
            when Constants::SIMPLE_NULL
              CborNull.new
            when Constants::SIMPLE_FLOAT_16, Constants::SIMPLE_FLOAT_32, Constants::SIMPLE_FLOAT_64
              raise CborError.new(:float_not_supported)
            else
              raise CborError.new(:unsupported_simple)
            end
          end
        end
      end
    end
  end
end
