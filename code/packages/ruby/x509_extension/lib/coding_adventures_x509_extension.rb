# frozen_string_literal: true

require "coding_adventures_der_asn1"
require_relative "coding_adventures/x509_extension/version"

module CodingAdventures
  # Bounded, payload-blind decoding for the generic RFC 5280 Extension shape.
  module X509Extension
    construction_capability = Object.new.freeze
    extension_provenance = ObjectSpace::WeakMap.new

    Error = Class.new(StandardError) do
      attr_reader :kind, :offset, :asn1_kind, :framing_kind

      define_method(:initialize) do |capability, kind, offset, asn1_kind = nil, framing_kind = nil|
        raise ArgumentError, "Error construction is private" unless capability.equal?(construction_capability)

        @kind = kind
        @offset = offset
        @asn1_kind = asn1_kind
        @framing_kind = framing_kind
        super("X.509 extension error #{kind} at byte #{offset}")
      end
    end

    Extension = Class.new do
      define_method(:initialize) do |capability, extension_id, critical, extension_value|
        raise ArgumentError, "Extension construction is private" unless capability.equal?(construction_capability)

        @extension_id = extension_id
        @critical = critical
        @extension_value = extension_value.dup.freeze
        extension_provenance[self] = true
        freeze
      end

      define_method(:extension_id) do
        raise ArgumentError, "invalid Extension" unless extension_provenance.key?(self)
        @extension_id
      end

      define_method(:critical) do
        raise ArgumentError, "invalid Extension" unless extension_provenance.key?(self)
        @critical
      end

      define_method(:extension_value) do
        raise ArgumentError, "invalid Extension" unless extension_provenance.key?(self)
        @extension_value.dup.freeze
      end
    end

    module_function

    define_method(:decode_extension) do |decoder, root|
      value_offset = root.header.bytesize
      value_length = root.value.bytesize
      begin
        cursor = decoder.sequence(root)
      rescue DerAsn1::Error => error
        fail_with(construction_capability, "structure", error.offset, error)
      end

      extension_id_offset = child_offset(value_offset, value_length, cursor)
      extension_id_element = read_child(construction_capability, decoder, cursor, value_offset, extension_id_offset)
      fail_with(construction_capability, "missing-extension-id", extension_id_offset) unless extension_id_element
      extension_id = decode_typed(construction_capability, "invalid-extension-id", extension_id_offset) do
        DerAsn1.decode_object_identifier(extension_id_element, decoder.limits)
      end

      second_offset = child_offset(value_offset, value_length, cursor)
      second = read_child(construction_capability, decoder, cursor, value_offset, second_offset)
      fail_with(construction_capability, "missing-extension-value", second_offset) unless second

      is_critical = false
      value_element = second
      extension_value_offset = second_offset
      if second.tag.number == 1
        is_critical = decode_typed(construction_capability, "invalid-critical", second_offset) do
          DerAsn1.decode_boolean(second)
        end
        fail_with(construction_capability, "encoded-default-critical", second_offset) unless is_critical
        extension_value_offset = child_offset(value_offset, value_length, cursor)
        value_element = read_child(construction_capability, decoder, cursor, value_offset, extension_value_offset)
        fail_with(construction_capability, "missing-extension-value", extension_value_offset) unless value_element
      end

      extension_value = decode_typed(construction_capability, "invalid-extension-value", extension_value_offset) do
        DerAsn1.decode_octet_string(value_element)
      end
      trailing_offset = child_offset(value_offset, value_length, cursor)
      trailing = read_child(construction_capability, decoder, cursor, value_offset, trailing_offset)
      fail_with(construction_capability, "trailing-element", trailing_offset) if trailing

      Extension.__send__(:new, construction_capability, extension_id, is_critical, extension_value)
    end

    def child_offset(value_offset, value_length, cursor)
      value_offset + value_length - cursor.remaining.bytesize
    end
    private_class_method :child_offset

    define_method(:read_child) do |capability, decoder, cursor, value_offset, current_offset|
      raise ArgumentError, "cursor reading is private" unless capability.equal?(construction_capability)
      cursor.read(decoder)
    rescue DerAsn1::Error => error
      offset = if error.kind == "framing"
        value_offset + error.offset
      else
        current_offset + error.offset
      end
      fail_with(construction_capability, "structure", offset, error)
    end
    private_class_method :read_child

    define_method(:decode_typed) do |capability, kind, offset, &block|
      raise ArgumentError, "typed decoding is private" unless capability.equal?(construction_capability)
      block.call
    rescue DerAsn1::Error => error
      fail_with(construction_capability, kind, offset + error.offset, error)
    end
    private_class_method :decode_typed

    define_method(:fail_with) do |capability, kind, offset, asn1_error = nil|
      raise ArgumentError, "error creation is private" unless capability.equal?(construction_capability)
      raise Error.__send__(:new, construction_capability, kind, offset, asn1_error&.kind, asn1_error&.framing_kind)
    end
    private_class_method :fail_with

    [Extension, Error].each { |type| type.singleton_class.__send__(:private, :new) }
  end
end
