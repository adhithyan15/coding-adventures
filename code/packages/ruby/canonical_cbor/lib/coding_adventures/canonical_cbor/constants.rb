# frozen_string_literal: true

module CodingAdventures
  module CanonicalCbor
    module Constants
      # Maximum allowed container/tag recursion depth per CBR01 specification.
      MAX_NESTING_DEPTH = 128

      # Maximum serialized bytes for a single canonical CBOR item (1 MiB / 1,048,576 bytes).
      MAX_ENCODED_BYTES = 1_048_576

      # Initial recursion depth of the root CBOR item.
      ROOT_DEPTH = 0

      # Bit shift to place the 3-bit major type into bits 7..5 of a CBOR header byte.
      MAJOR_TYPE_SHIFT = 5

      # Bit mask to extract the lower 5 bits (additional information) of a header byte.
      ADDITIONAL_INFO_MASK = 0x1F

      # Bits per byte for big-endian serialization.
      BITS_PER_BYTE = 8

      # 8-bit mask.
      BYTE_MASK = 0xFF

      # RFC 8949 Section 3.1 Major Types:
      MAJOR_UNSIGNED = 0
      MAJOR_NEGATIVE = 1
      MAJOR_BYTES = 2
      MAJOR_TEXT = 3
      MAJOR_ARRAY = 4
      MAJOR_MAP = 5
      MAJOR_TAG = 6
      MAJOR_SIMPLE_OR_FLOAT = 7

      # Additional-info representation codes for argument lengths:
      INFO_INLINE_MAX = 23
      INFO_1_BYTE = 24
      INFO_2_BYTES = 25
      INFO_4_BYTES = 26
      INFO_8_BYTES = 27
      INFO_RESERVED_28 = 28
      INFO_RESERVED_29 = 29
      INFO_RESERVED_30 = 30
      INFO_INDEFINITE = 31

      # Multi-byte header argument widths in bytes:
      ARG_WIDTH_1_BYTE = 1
      ARG_WIDTH_2_BYTES = 2
      ARG_WIDTH_4_BYTES = 4
      ARG_WIDTH_8_BYTES = 8

      # Maximum unsigned integer boundaries for CBOR argument width selection:
      MAX_UINT8 = 0xFF
      MAX_UINT16 = 0xFFFF
      MAX_UINT32 = 0xFFFF_FFFF
      MAX_UINT64 = 0xFFFF_FFFF_FFFF_FFFF

      # Simple value codes for major type 7 (RFC 8949 Section 3.3):
      SIMPLE_FALSE = 20
      SIMPLE_TRUE = 21
      SIMPLE_NULL = 22
      SIMPLE_FLOAT_16 = 25
      SIMPLE_FLOAT_32 = 26
      SIMPLE_FLOAT_64 = 27

      # Exact on-wire byte values for closed canonical simple values:
      WIRE_BYTE_FALSE = (MAJOR_SIMPLE_OR_FLOAT << MAJOR_TYPE_SHIFT) | SIMPLE_FALSE
      WIRE_BYTE_TRUE = (MAJOR_SIMPLE_OR_FLOAT << MAJOR_TYPE_SHIFT) | SIMPLE_TRUE
      WIRE_BYTE_NULL = (MAJOR_SIMPLE_OR_FLOAT << MAJOR_TYPE_SHIFT) | SIMPLE_NULL

      # Minimum wire bytes required per element to prevent hostile pre-allocation DoS:
      MIN_BYTES_PER_ELEMENT = 1
      MIN_BYTES_PER_MAP_ENTRY = 2
    end
  end
end
