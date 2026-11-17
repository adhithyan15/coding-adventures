package CodingAdventures::CanonicalCbor::Constants;

use strict;
use warnings;
use v5.28;

our $VERSION = '0.1.0';

# ============================================================================
# coding_adventures.canonical_cbor.constants
# ============================================================================
# Protocol constants, major types, additional info codes, bit masks, limits,
# and wire bytes for deterministic canonical CBOR (RFC 8949 CBR01 profile).
# ============================================================================

# Maximum allowed container/tag recursion depth per CBR01 specification.
use constant MAX_NESTING_DEPTH => 128;

# Maximum allowed encoded or decoded byte size (1 MiB).
use constant MAX_ENCODED_BYTES => 1048576;

# Initial nesting depth for root items.
use constant ROOT_DEPTH => 0;

# Minimum bytes required per item in container pre-allocation guards.
use constant MIN_BYTES_PER_ARRAY_ITEM => 1;
use constant MIN_BYTES_PER_MAP_ENTRY  => 2;

# CBOR Major Types (RFC 8949 Section 3.1).
use constant MAJOR_UNSIGNED        => 0;
use constant MAJOR_NEGATIVE        => 1;
use constant MAJOR_BYTES           => 2;
use constant MAJOR_TEXT            => 3;
use constant MAJOR_ARRAY           => 4;
use constant MAJOR_MAP             => 5;
use constant MAJOR_TAG             => 6;
use constant MAJOR_SIMPLE_OR_FLOAT => 7;

# Major type bit shift and mask.
use constant MAJOR_TYPE_SHIFT     => 5;
use constant MAJOR_TYPE_MASK      => 0x07;
use constant ADDITIONAL_INFO_MASK => 0x1F;

# Additional information integer lengths.
use constant INFO_INLINE_MAX  => 23;
use constant INFO_1_BYTE      => 24;
use constant INFO_2_BYTES     => 25;
use constant INFO_4_BYTES     => 26;
use constant INFO_8_BYTES     => 27;
use constant INFO_RESERVED_28 => 28;
use constant INFO_RESERVED_29 => 29;
use constant INFO_RESERVED_30 => 30;
use constant INFO_INDEFINITE  => 31;

# Argument widths in bytes for multi-byte integer decoding.
use constant ARG_WIDTH_1_BYTE  => 1;
use constant ARG_WIDTH_2_BYTES => 2;
use constant ARG_WIDTH_4_BYTES => 4;
use constant ARG_WIDTH_8_BYTES => 8;

# Minimum raw byte width.
use constant MIN_BYTES_PER_RAW_BYTE => 1;

# Integer range bounds for canonical minimal representation verification (§4.2.1).
use constant MAX_UINT8  => 0xFF;
use constant MAX_UINT16 => 0xFFFF;
use constant MAX_UINT32 => 0xFFFFFFFF;

# Wire bytes for CBR01 simple values (§3.3).
use constant WIRE_BYTE_FALSE => 0xF4;
use constant WIRE_BYTE_TRUE  => 0xF5;
use constant WIRE_BYTE_NULL  => 0xF6;

# Additional info codes for simple values and floats.
use constant SIMPLE_FALSE           => 20;
use constant SIMPLE_TRUE            => 21;
use constant SIMPLE_NULL            => 22;
use constant MIN_1_BYTE_SIMPLE_VALUE => 32;
use constant SIMPLE_FLOAT_16        => 25;
use constant SIMPLE_FLOAT_32 => 26;
use constant SIMPLE_FLOAT_64 => 27;

1;
