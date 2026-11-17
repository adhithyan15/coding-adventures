-- ============================================================================
-- coding_adventures.canonical_cbor.constants
-- ============================================================================
-- Zero-magic-constant definitions for deterministic canonical CBOR (RFC 8949
-- Section 4.2.3 Length-First Map Key Ordering profile CBR01).
--
-- All values are immutable integers or booleans, protected by a read-only
-- metatable to prevent runtime tampering.
-- ============================================================================

local messages = require("coding_adventures.canonical_cbor.messages")

local M = {}

-- Maximum allowed container/tag recursion depth per CBR01 specification.
M.MAX_NESTING_DEPTH = 128

-- Maximum serialized bytes for a single canonical CBOR item (1 MiB / 1,048,576 bytes).
M.MAX_ENCODED_BYTES = 1048576

-- Initial recursion depth of the root CBOR item.
M.ROOT_DEPTH = 0

-- Bit shift to place the 3-bit major type into bits 7..5 of a CBOR header byte.
M.MAJOR_TYPE_SHIFT = 5

-- Bit mask to extract the lower 5 bits (additional information) of a header byte.
M.ADDITIONAL_INFO_MASK = 0x1F

-- Bits per byte for big-endian serialization and bitwise shifting.
M.BITS_PER_BYTE = 8

-- 8-bit mask (0xFF) for byte extraction.
M.BYTE_MASK = 0xFF

-- RFC 8949 Section 3.1 Major Types:
M.MAJOR_UNSIGNED = 0
M.MAJOR_NEGATIVE = 1
M.MAJOR_BYTES = 2
M.MAJOR_TEXT = 3
M.MAJOR_ARRAY = 4
M.MAJOR_MAP = 5
M.MAJOR_TAG = 6
M.MAJOR_SIMPLE_OR_FLOAT = 7

-- Additional-info representation codes for argument lengths:
M.INFO_INLINE_MAX = 23
M.INFO_1_BYTE = 24
M.INFO_2_BYTES = 25
M.INFO_4_BYTES = 26
M.INFO_8_BYTES = 27
M.INFO_RESERVED_28 = 28
M.INFO_RESERVED_29 = 29
M.INFO_RESERVED_30 = 30
M.INFO_INDEFINITE = 31

-- Multi-byte header argument widths in bytes:
M.ARG_WIDTH_1_BYTE = 1
M.ARG_WIDTH_2_BYTES = 2
M.ARG_WIDTH_4_BYTES = 4
M.ARG_WIDTH_8_BYTES = 8

-- Maximum unsigned integer boundaries for CBOR argument width selection:
M.MAX_UINT8 = 0xFF
M.MAX_UINT16 = 0xFFFF
M.MAX_UINT32 = 0xFFFFFFFF
-- In Lua 5.4, 64-bit unsigned maximum 0xFFFFFFFFFFFFFFFF wraps to -1.
M.MAX_UINT64 = -1

-- Simple value codes for major type 7 (RFC 8949 Section 3.3):
M.SIMPLE_FALSE = 20
M.SIMPLE_TRUE = 21
M.SIMPLE_NULL = 22
M.SIMPLE_FLOAT_16 = 25
M.SIMPLE_FLOAT_32 = 26
M.SIMPLE_FLOAT_64 = 27

-- Exact on-wire byte values for closed canonical simple values:
M.WIRE_BYTE_FALSE = 0xF4
M.WIRE_BYTE_TRUE = 0xF5
M.WIRE_BYTE_NULL = 0xF6

-- Minimum wire bytes required per element to prevent hostile pre-allocation DoS:
M.MIN_BYTES_PER_ELEMENT = 1
M.MIN_BYTES_PER_MAP_ENTRY = 2

return setmetatable(M, {
  __newindex = function()
    error(messages.READ_ONLY_MODULE_MESSAGE, 0)
  end,
  __metatable = false,
})
