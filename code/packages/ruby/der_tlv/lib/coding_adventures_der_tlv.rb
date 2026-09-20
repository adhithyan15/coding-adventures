# frozen_string_literal: true

require_relative "coding_adventures/der_tlv/version"

module CodingAdventures
  # Bounded, payload-blind DER tag-length-value framing.
  module DerTlv
    DEFAULT_LIMITS = {
      max_input_len: 1_048_576,
      max_value_len: 1_048_576,
      max_elements: 4096,
      max_tag_number: 0xffff_ffff
    }.freeze
    HOST_MAX = (1 << ((0.size * 8) - 1)) - 1
    TAG_CLASSES = %w[universal application context-specific private].freeze

    class Error < StandardError
      attr_reader :kind, :offset

      def initialize(kind, offset)
        @kind = kind
        @offset = offset
        super("DER framing error #{kind} at byte #{offset}")
      end
    end

    Tag = Data.define(:tag_class, :constructed, :number)

    class Element
      attr_reader :tag

      def initialize(tag:, input:, start:, header_len:, encoded_len:)
        @tag = tag
        @input = input
        @start = start
        @header_len = header_len
        @encoded_len = encoded_len
      end

      def header
        @input.byteslice(@start, @header_len)
      end

      def value
        @input.byteslice(@start + @header_len, @encoded_len - @header_len)
      end

      def encoded
        @input.byteslice(@start, @encoded_len)
      end
    end

    class Cursor
      attr_reader :elements_read

      def initialize(input, limits = nil)
        @input = DerTlv.binary(input)
        @limits = DerTlv.normalize_limits(limits)
        DerTlv.fail_with("input-limit-exceeded", 0) if @input.bytesize > @limits[:max_input_len]

        @offset = 0
        @elements_read = 0
      end

      def remaining
        @input.byteslice(@offset..) || "".b
      end

      def read
        return nil if @offset == @input.bytesize

        if @elements_read >= @limits[:max_elements]
          DerTlv.fail_with("element-limit-exceeded", @offset)
        end

        element, next_offset = DerTlv.decode_at(
          @input,
          @offset,
          @input.bytesize - @offset,
          @limits
        )
        @offset = next_offset
        @elements_read += 1
        element
      end

      def finish
        DerTlv.fail_with("trailing-data", @offset) unless @offset == @input.bytesize
      end
    end

    module_function

    def decode_one(input, limits = nil)
      bytes = binary(input)
      element, next_offset = decode_at(bytes, 0, bytes.bytesize, normalize_limits(limits))
      [element, bytes.byteslice(next_offset..) || "".b]
    end

    def decode_exact(input, limits = nil)
      bytes = binary(input)
      element, next_offset = decode_at(bytes, 0, bytes.bytesize, normalize_limits(limits))
      fail_with("trailing-data", next_offset) unless next_offset == bytes.bytesize
      element
    end

    def binary(input)
      raise ArgumentError, "input must be a String" unless input.is_a?(String)

      (input.encoding == Encoding::BINARY) ? input : input.b
    end

    def normalize_limits(limits)
      values = DEFAULT_LIMITS.merge(limits || {})
      DEFAULT_LIMITS.each_key do |name|
        value = values.fetch(name)
        raise ArgumentError, "#{name} must be a non-negative Integer" unless value.is_a?(Integer) && value >= 0
      end
      raise ArgumentError, "max_tag_number must fit u32" if values[:max_tag_number] > 0xffff_ffff

      values.freeze
    end

    def fail_with(kind, offset)
      raise Error.new(kind, offset)
    end

    def decode_at(input, start, available, limits)
      fail_with("input-limit-exceeded", start) if available > limits[:max_input_len]
      fail_with("empty-input", start) if available.zero?

      first = input.getbyte(start)
      tag_class = TAG_CLASSES.fetch(first >> 6)
      constructed = (first & 0x20) != 0
      low = first & 0x1f

      if low != 0x1f
        number = low
        identifier_len = 1
        fail_with("tag-limit-exceeded", start) if number > limits[:max_tag_number]
      else
        number, identifier_len = decode_high_tag(input, start, available, limits)
      end

      fail_with("end-of-contents", start) if tag_class == "universal" && number.zero?

      value_len, length_len, length_offset = decode_length(input, start, available, identifier_len)
      fail_with("value-limit-exceeded", length_offset) if value_len > limits[:max_value_len]

      header_len = identifier_len + length_len
      fail_with("length-host-overflow", length_offset) if value_len > HOST_MAX - header_len

      encoded_len = header_len + value_len
      fail_with("truncated-value", start + available) if encoded_len > available

      element = Element.new(
        tag: Tag.new(tag_class: tag_class, constructed: constructed, number: number),
        input: input,
        start: start,
        header_len: header_len,
        encoded_len: encoded_len
      )
      [element, start + encoded_len]
    end

    def decode_high_tag(input, start, available, limits)
      number = 0
      index = 1
      loop do
        fail_with("truncated-high-tag", start + index) if index >= available

        octet = input.getbyte(start + index)
        payload = octet & 0x7f
        fail_with("non-minimal-tag", start + index) if index == 1 && payload.zero?

        candidate = (number * 128) + payload
        fail_with("tag-overflow", start + index) if candidate > 0xffff_ffff

        number = candidate
        fail_with("tag-limit-exceeded", start + index) if number > limits[:max_tag_number]

        index += 1
        break if (octet & 0x80).zero?
      end
      fail_with("non-minimal-tag", start) if number < 31

      [number, index]
    end

    def decode_length(input, start, available, identifier_len)
      length_offset = start + identifier_len
      fail_with("truncated-length", length_offset) if identifier_len >= available

      first = input.getbyte(length_offset)
      return [first, 1, length_offset] if first < 0x80

      fail_with("indefinite-length", length_offset) if first == 0x80
      fail_with("reserved-length", length_offset) if first == 0xff

      count = first & 0x7f
      fail_with("length-too-wide", length_offset) if count > 8
      fail_with("truncated-length", start + available) if identifier_len + 1 + count > available
      fail_with("non-minimal-length", length_offset + 1) if input.getbyte(length_offset + 1).zero?

      value = count.times.reduce(0) { |sum, index| (sum * 256) + input.getbyte(length_offset + 1 + index) }
      fail_with("non-minimal-length", length_offset) if value < 128
      fail_with("length-host-overflow", length_offset) if value > HOST_MAX

      [value, count + 1, length_offset]
    end
  end
end
