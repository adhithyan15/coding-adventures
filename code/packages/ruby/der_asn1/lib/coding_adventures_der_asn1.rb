# frozen_string_literal: true

require "coding_adventures_der_tlv"
require_relative "coding_adventures/der_asn1/version"

module CodingAdventures
  # A bounded, payload-blind typed layer over canonical DER framing.
  module DerAsn1
    U64_MAX = (1 << 64) - 1
    HOST_MAX = (1 << ((0.size * 8) - 1)) - 1
    DEFAULT_MAX_DEPTH = 32
    DEFAULT_MAX_TOTAL_ELEMENTS = 16_384
    DEFAULT_MAX_OID_ARCS = 128
    DEFAULT_LIMITS = {
      der: DerTlv::DEFAULT_LIMITS,
      max_depth: DEFAULT_MAX_DEPTH,
      max_total_elements: DEFAULT_MAX_TOTAL_ELEMENTS,
      max_oid_arcs: DEFAULT_MAX_OID_ARCS
    }.freeze
    construction_capability = Object.new.freeze
    element_provenance = ObjectSpace::WeakMap.new

    class Error < StandardError
      attr_reader :kind, :offset, :framing_kind

      def initialize(kind, offset, framing_kind = nil)
        @kind = kind
        @offset = offset
        @framing_kind = framing_kind
        suffix = framing_kind ? " (#{framing_kind})" : ""
        super("DER ASN.1 error #{kind}#{suffix} at byte #{offset}")
      end
    end

    Element = Class.new do
      attr_reader :tag, :header, :value, :encoded, :depth, :value_offset

      define_method(:initialize) do |capability, tag:, header:, value:, encoded:, depth:|
        raise ArgumentError, "Element construction is private" unless capability.equal?(construction_capability)

        @tag = tag
        @header = header.dup.freeze
        @value = value.dup.freeze
        @encoded = encoded.dup.freeze
        @depth = depth
        @value_offset = @header.bytesize
        element_provenance[self] = true
        freeze
      end

      define_method(:valid?) do
        element_provenance.key?(self)
      end
    end

    define_singleton_method(:registered_element?) do |element|
      element.instance_of?(Element) && element_provenance.key?(element)
    end
    private_class_method :registered_element?

    DerInteger = Class.new do
      attr_reader :signed_bytes

      define_method(:initialize) do |capability, signed_bytes, value_offset|
        raise ArgumentError, "DerInteger construction is private" unless capability.equal?(construction_capability)

        @signed_bytes = signed_bytes.dup.freeze
        @value_offset = value_offset
        freeze
      end

      def negative?
        (@signed_bytes.getbyte(0) & 0x80) != 0
      end

      alias_method :is_negative, :negative?

      def to_u64
        DerAsn1.fail_with("negative-integer", @value_offset) if negative?

        bytes = (@signed_bytes.bytesize > 1 && @signed_bytes.getbyte(0).zero?) ? @signed_bytes.byteslice(1..) : @signed_bytes
        DerAsn1.fail_with("integer-overflow", @value_offset) if bytes.bytesize > 8
        bytes.each_byte.reduce(0) { |value, octet| (value << 8) | octet }
      end
    end

    DerBitString = Class.new do
      attr_reader :bytes, :unused_bits, :bit_length

      define_method(:initialize) do |capability, bytes, unused_bits, bit_length|
        raise ArgumentError, "DerBitString construction is private" unless capability.equal?(construction_capability)

        @bytes = bytes.dup.freeze
        @unused_bits = unused_bits
        @bit_length = bit_length
        freeze
      end
    end

    ObjectIdentifier = Class.new do
      attr_reader :encoded, :arcs

      define_method(:initialize) do |capability, encoded, arcs|
        raise ArgumentError, "ObjectIdentifier construction is private" unless capability.equal?(construction_capability)

        @encoded = encoded.dup.freeze
        @arcs = arcs.dup.freeze
        freeze
      end

      def arc_count
        @arcs.length
      end

      def ==(other)
        other.is_a?(ObjectIdentifier) && other.encoded == @encoded && other.arcs == @arcs
      end

      def equals(expected_arcs)
        @arcs == expected_arcs.to_a
      end

      alias_method :eql?, :==

      def hash
        [@encoded, @arcs].hash
      end
    end

    Decoder = Class.new do
      attr_reader :limits, :elements_read

      def initialize(limits = nil)
        @limits = DerAsn1.normalize_limits(limits)
        @elements_read = 0
      end

      define_method(:decode_exact) do |input|
        raise ArgumentError, "input must be a String" unless input.is_a?(String)
        DerAsn1.fail_with("depth-limit-exceeded", 0) if @limits[:max_depth].zero?
        DerAsn1.fail_with("element-limit-exceeded", 0) if @elements_read >= @limits[:max_total_elements]
        DerAsn1.fail_with("framing", 0, "input-limit-exceeded") if input.bytesize > @limits[:der][:max_input_len]

        snapshot = input.b.dup.freeze
        framed = DerTlv.decode_exact(snapshot, @limits[:der])

        element = Element.__send__(
          :new,
          construction_capability,
          tag: framed.tag,
          header: framed.header,
          value: framed.value,
          encoded: framed.encoded,
          depth: 0
        )
        @elements_read += 1
        element
      rescue DerTlv::Error => error
        DerAsn1.fail_with("framing", error.offset, error.kind)
      end

      def sequence(element)
        container(element, "universal", true, 16)
      end

      def set(element)
        container(element, "universal", true, 17)
      end

      define_method(:explicit) do |element, tag_number|
        DerAsn1.__send__(:validate_element, element)
        DerAsn1.__send__(:expect_tag, element, "context-specific", true, tag_number)
        DerAsn1.__send__(:check_child_depth, element.depth, @limits)
        DerAsn1.fail_with("element-limit-exceeded", element.value_offset) if @elements_read >= @limits[:max_total_elements]

        framed = DerTlv.decode_exact(element.value, @limits[:der])
        child = Element.__send__(
          :new,
          construction_capability,
          tag: framed.tag,
          header: framed.header,
          value: framed.value,
          encoded: framed.encoded,
          depth: element.depth + 1
        )
        @elements_read += 1
        child
      rescue DerTlv::Error => error
        DerAsn1.fail_with("framing", error.offset, error.kind)
      end

      private

      define_method(:container) do |element, tag_class, constructed, number|
        DerAsn1.__send__(:validate_element, element)
        DerAsn1.__send__(:expect_tag, element, tag_class, constructed, number)
        DerAsn1.__send__(:check_child_depth, element.depth, @limits)
        Cursor.__send__(:new, construction_capability, element.value, element.depth + 1, @limits, self)
      end
    end

    Cursor = Class.new do
      define_method(:initialize) do |capability, contents, depth, limits, decoder|
        raise ArgumentError, "Cursor construction is private" unless capability.equal?(construction_capability)

        @framing = DerTlv::Cursor.new(contents, limits[:der])
        @depth = depth
        @limits = limits
        @decoder = decoder
      rescue DerTlv::Error => error
        DerAsn1.fail_with("framing", error.offset, error.kind)
      end

      def remaining
        @framing.remaining.dup.freeze
      end

      define_method(:read) do |decoder|
        raise ArgumentError, "decoder must be a Decoder" unless decoder.is_a?(Decoder)
        return nil if @framing.remaining.empty?
        DerAsn1.fail_with("decoder-limit-mismatch", 0) unless decoder.equal?(@decoder)
        DerAsn1.fail_with("element-limit-exceeded", 0) if decoder.elements_read >= @limits[:max_total_elements]

        framed = @framing.read
        return nil unless framed

        element = Element.__send__(
          :new,
          construction_capability,
          tag: framed.tag,
          header: framed.header,
          value: framed.value,
          encoded: framed.encoded,
          depth: @depth
        )
        decoder.instance_variable_set(:@elements_read, decoder.elements_read + 1)
        element
      rescue DerTlv::Error => error
        DerAsn1.fail_with("framing", error.offset, error.kind)
      end

      def finish
        @framing.finish
      rescue DerTlv::Error => error
        DerAsn1.fail_with("framing", error.offset, error.kind)
      end
    end

    module_function

    def decode_boolean(element)
      value = primitive(element, "universal", 1)
      fail_with("invalid-boolean-length", element.value_offset) unless value.bytesize == 1
      return false if value.getbyte(0).zero?
      return true if value.getbyte(0) == 0xff

      fail_with("invalid-boolean-value", element.value_offset)
    end

    define_method(:decode_integer) do |element|
      value = primitive(element, "universal", 2)
      validate_integer(value, element.value_offset)
      DerInteger.__send__(:new, construction_capability, value, element.value_offset)
    end

    define_method(:decode_bit_string) do |element|
      value = primitive(element, "universal", 3)
      fail_with("missing-unused-bit-count", element.value_offset) if value.empty?

      unused = value.getbyte(0)
      payload = value.byteslice(1..) || "".b
      fail_with("invalid-unused-bit-count", element.value_offset) if unused > 7 || (payload.empty? && !unused.zero?)
      if !payload.empty? && unused.positive? && (payload.getbyte(-1) & ((1 << unused) - 1)).positive?
        fail_with("non-zero-bit-padding", element.value_offset + value.bytesize - 1)
      end
      fail_with("bit-length-overflow", element.value_offset) if payload.bytesize > (HOST_MAX + unused) / 8
      DerBitString.__send__(:new, construction_capability, payload, unused, (payload.bytesize * 8) - unused)
    end

    def decode_octet_string(element)
      primitive(element, "universal", 4).dup.freeze
    end

    def decode_implicit_octet_string(element, tag_number)
      primitive(element, "context-specific", tag_number).dup.freeze
    end

    def decode_ia5_string(element)
      decode_ia5(primitive(element, "universal", 22), element.value_offset)
    end

    def decode_implicit_ia5_string(element, tag_number)
      decode_ia5(primitive(element, "context-specific", tag_number), element.value_offset)
    end

    def decode_null(element)
      value = primitive(element, "universal", 5)
      fail_with("non-empty-null", element.value_offset) unless value.empty?
      nil
    end

    define_method(:decode_object_identifier) do |element, limits = nil|
      configured = normalize_limits(limits)
      decode_oid(
        construction_capability,
        primitive(element, "universal", 6),
        element.value_offset,
        configured[:max_oid_arcs]
      )
    end

    define_method(:decode_implicit_object_identifier) do |element, tag_number, limits = nil|
      configured = normalize_limits(limits)
      decode_oid(
        construction_capability,
        primitive(element, "context-specific", tag_number),
        element.value_offset,
        configured[:max_oid_arcs]
      )
    end

    def normalize_limits(limits)
      supplied = (limits || {}).transform_keys(&:to_sym)
      unknown = supplied.keys - DEFAULT_LIMITS.keys
      raise ArgumentError, "unknown limit #{unknown.first}" unless unknown.empty?

      der = supplied.fetch(:der, DEFAULT_LIMITS[:der]).transform_keys(&:to_sym)
      unknown_der = der.keys - DerTlv::DEFAULT_LIMITS.keys
      raise ArgumentError, "unknown DER limit #{unknown_der.first}" unless unknown_der.empty?

      values = DEFAULT_LIMITS.merge(supplied).merge(der: DerTlv.normalize_limits(der))
      %i[max_depth max_total_elements max_oid_arcs].each do |name|
        value = values.fetch(name)
        raise ArgumentError, "#{name} must be a non-negative Integer" unless value.is_a?(Integer) && value >= 0
      end
      values.freeze
    end

    def validate_element(element)
      raise ArgumentError, "element must be a validated Element" unless registered_element?(element)
    end

    def expect_tag(element, tag_class, constructed, number)
      tag = element.tag
      return if tag.tag_class == tag_class && tag.constructed == constructed && tag.number == number
      fail_with("unexpected-tag", 0)
    end

    def check_child_depth(depth, limits)
      fail_with("depth-limit-exceeded", 0) if depth + 1 >= limits[:max_depth]
    end

    def primitive(element, tag_class, number)
      validate_element(element)
      expect_tag(element, tag_class, false, number)
      element.value
    end

    def validate_integer(value, value_offset)
      fail_with("empty-integer", value_offset) if value.empty?
      return if value.bytesize == 1

      first = value.getbyte(0)
      second = value.getbyte(1)
      redundant_positive = first.zero? && (second & 0x80).zero?
      redundant_negative = first == 0xff && (second & 0x80) != 0
      fail_with("non-minimal-integer", value_offset) if redundant_positive || redundant_negative
    end

    def decode_ia5(value, value_offset)
      value.each_byte.with_index do |octet, index|
        fail_with("non-ascii-ia5-string", value_offset + index) if octet > 0x7f
      end
      value.dup.force_encoding(Encoding::UTF_8).freeze
    end

    define_method(:decode_oid) do |capability, value, value_offset, max_arcs|
      raise ArgumentError, "raw OID decoding is private" unless capability.equal?(construction_capability)

      fail_with("empty-object-identifier", value_offset) if value.empty?
      first, index = oid_subidentifier(value, 0, value_offset)
      arcs = if first < 40
        [0, first]
      elsif first < 80
        [1, first - 40]
      else
        [2, first - 80]
      end
      fail_with("oid-arc-limit-exceeded", value_offset) if arcs.length > max_arcs
      while index < value.bytesize
        arc_start = index
        arc, index = oid_subidentifier(value, index, value_offset)
        fail_with("oid-arc-limit-exceeded", value_offset + arc_start) if arcs.length >= max_arcs
        arcs << arc
      end
      ObjectIdentifier.__send__(:new, construction_capability, value, arcs)
    end

    def oid_subidentifier(value, start, value_offset)
      fail_with("non-minimal-object-identifier", value_offset + start) if value.getbyte(start) == 0x80
      number = 0
      index = start
      loop do
        fail_with("unterminated-object-identifier", value_offset + index) if index >= value.bytesize
        octet = value.getbyte(index)
        payload = octet & 0x7f
        fail_with("object-identifier-overflow", value_offset + index) if number > (U64_MAX - payload) / 128
        number = (number * 128) + payload
        index += 1
        return [number, index] if (octet & 0x80).zero?
      end
    end

    def fail_with(kind, offset, framing_kind = nil)
      raise Error.new(kind, offset, framing_kind)
    end

    [Element, DerInteger, DerBitString, ObjectIdentifier, Cursor].each do |type|
      type.singleton_class.__send__(:private, :new)
    end
    private_class_method :validate_element, :expect_tag, :check_child_depth,
      :primitive, :validate_integer, :decode_ia5, :decode_oid, :oid_subidentifier
  end
end
