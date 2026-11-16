defmodule CodingAdventures.CanonicalCbor.Constants do
  @moduledoc """
  Protocol constants, major types, additional info codes, bit masks, limits,
  and wire bytes for deterministic canonical CBOR (RFC 8949 CBR01 profile).
  """

  # Maximum allowed container/tag recursion depth per CBR01 specification.
  @spec max_nesting_depth() :: non_neg_integer()
  def max_nesting_depth, do: 128

  # Maximum allowed encoded or decoded byte size (1 MiB).
  @spec max_encoded_bytes() :: non_neg_integer()
  def max_encoded_bytes, do: 1_048_576

  # Initial nesting depth for root items.
  @spec root_depth() :: non_neg_integer()
  def root_depth, do: 0

  # Minimum bytes required per item in container pre-allocation guards.
  @spec min_bytes_per_raw_byte() :: non_neg_integer()
  def min_bytes_per_raw_byte, do: 1

  @spec min_bytes_per_array_item() :: non_neg_integer()
  def min_bytes_per_array_item, do: 1

  @spec min_bytes_per_map_entry() :: non_neg_integer()
  def min_bytes_per_map_entry, do: 2

  # CBOR Major Types (RFC 8949 Section 3.1).
  @spec major_unsigned() :: non_neg_integer()
  def major_unsigned, do: 0

  @spec major_negative() :: non_neg_integer()
  def major_negative, do: 1

  @spec major_bytes() :: non_neg_integer()
  def major_bytes, do: 2

  @spec major_text() :: non_neg_integer()
  def major_text, do: 3

  @spec major_array() :: non_neg_integer()
  def major_array, do: 4

  @spec major_map() :: non_neg_integer()
  def major_map, do: 5

  @spec major_tag() :: non_neg_integer()
  def major_tag, do: 6

  @spec major_simple_or_float() :: non_neg_integer()
  def major_simple_or_float, do: 7

  # Major type bit shift and mask.
  @spec major_type_shift() :: non_neg_integer()
  def major_type_shift, do: 5

  @spec major_type_mask() :: non_neg_integer()
  def major_type_mask, do: 0x07

  @spec additional_info_mask() :: non_neg_integer()
  def additional_info_mask, do: 0x1F

  # Additional information integer lengths.
  @spec info_inline_max() :: non_neg_integer()
  def info_inline_max, do: 23

  @spec info_1_byte() :: non_neg_integer()
  def info_1_byte, do: 24

  @spec info_2_bytes() :: non_neg_integer()
  def info_2_bytes, do: 25

  @spec info_4_bytes() :: non_neg_integer()
  def info_4_bytes, do: 26

  @spec info_8_bytes() :: non_neg_integer()
  def info_8_bytes, do: 27

  @spec info_reserved_28() :: non_neg_integer()
  def info_reserved_28, do: 28

  @spec info_reserved_29() :: non_neg_integer()
  def info_reserved_29, do: 29

  @spec info_reserved_30() :: non_neg_integer()
  def info_reserved_30, do: 30

  @spec info_indefinite() :: non_neg_integer()
  def info_indefinite, do: 31

  # Argument widths in bytes for multi-byte integer decoding.
  @spec arg_width_1_byte() :: non_neg_integer()
  def arg_width_1_byte, do: 1

  @spec arg_width_2_bytes() :: non_neg_integer()
  def arg_width_2_bytes, do: 2

  @spec arg_width_4_bytes() :: non_neg_integer()
  def arg_width_4_bytes, do: 4

  @spec arg_width_8_bytes() :: non_neg_integer()
  def arg_width_8_bytes, do: 8

  # Integer range bounds for canonical minimal representation verification (§4.2.1).
  @spec max_uint8() :: non_neg_integer()
  def max_uint8, do: 0xFF

  @spec max_uint16() :: non_neg_integer()
  def max_uint16, do: 0xFFFF

  @spec max_uint32() :: non_neg_integer()
  def max_uint32, do: 0xFFFFFFFF

  @spec max_uint64() :: non_neg_integer()
  def max_uint64, do: 18_446_744_073_709_551_615

  # Wire bytes for CBR01 simple values (§3.3).
  @spec wire_byte_false() :: non_neg_integer()
  def wire_byte_false, do: 0xF4

  @spec wire_byte_true() :: non_neg_integer()
  def wire_byte_true, do: 0xF5

  @spec wire_byte_null() :: non_neg_integer()
  def wire_byte_null, do: 0xF6

  # Additional info codes for simple values and floats.
  @spec simple_false() :: non_neg_integer()
  def simple_false, do: 20

  @spec simple_true() :: non_neg_integer()
  def simple_true, do: 21

  @spec simple_null() :: non_neg_integer()
  def simple_null, do: 22

  @spec min_1_byte_simple_value() :: non_neg_integer()
  def min_1_byte_simple_value, do: 32

  @spec simple_float_16() :: non_neg_integer()
  def simple_float_16, do: 25

  @spec simple_float_32() :: non_neg_integer()
  def simple_float_32, do: 26

  @spec simple_float_64() :: non_neg_integer()
  def simple_float_64, do: 27
end
