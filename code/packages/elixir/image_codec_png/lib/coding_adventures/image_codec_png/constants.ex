defmodule CodingAdventures.ImageCodecPng.Constants do
  @moduledoc """
  Standing Rule #20: Zero double-quote string literals in operational constants.
  Centralizes all wire signatures, framing sizes, limits, channel counts, filter
  types, and zlib constants for the IC18 PNG codec.
  """

  # PNG signature: 89 50 4E 47 0D 0A 1A 0A
  @png_signature <<137, 80, 78, 71, 13, 10, 26, 10>>
  @signature_length 8

  # Framing & chunk header/trailer sizes
  @chunk_length_size 4
  @chunk_type_size 4
  @chunk_header_size 8
  @chunk_crc_size 4
  @chunk_overhead 12
  @ancillary_bit_mask 0x20

  # Normative chunk 4CCs represented as byte binaries (zero string literals)
  @chunk_ihdr <<73, 72, 68, 82>>
  @chunk_plte <<80, 76, 84, 69>>
  @chunk_trns <<116, 82, 78, 83>>
  @chunk_idat <<73, 68, 65, 84>>
  @chunk_iend <<73, 69, 78, 68>>

  # APNG forbidden semantic chunk 4CCs
  @chunk_actl <<97, 99, 84, 76>>
  @chunk_fctl <<102, 99, 84, 76>>
  @chunk_fdat <<102, 100, 65, 84>>
  @apng_chunks [@chunk_actl, @chunk_fctl, @chunk_fdat]

  # Dimension and pixel bounds
  @png_max_dimension 16_384
  @png_max_pixels 33_554_432
  @default_max_pixels 33_554_432
  @ihdr_length 13
  @plte_min_length 3
  @plte_max_length 768
  @plte_entry_size 3
  @trns_grey_length 2
  @trns_rgb_length 6
  @max_sample_value 255

  # Colour types and channel counts
  @colour_type_grey 0
  @colour_type_rgb 2
  @colour_type_palette 3
  @colour_type_grey_alpha 4
  @colour_type_rgba 6
  @supported_colour_types [0, 2, 4, 6]

  @channels_grey 1
  @channels_rgb 3
  @channels_grey_alpha 2
  @channels_rgba 4

  @supported_bit_depth 8
  @compression_method_deflate 0
  @filter_method_adaptive 0
  @interlace_method_none 0

  # Scanline filter IDs
  @filter_none 0
  @filter_sub 1
  @filter_up 2
  @filter_average 3
  @filter_paeth 4
  @filter_count 5
  @max_filter_type 4

  # Zlib wrapper framing (RFC 1950)
  @zlib_min_stream_size 6
  @zlib_cmf_deflate 0x78
  @zlib_flg_default 0x9C
  @zlib_method_deflate 8
  @zlib_cinfo_max 7
  @zlib_cinfo_shift 4
  @zlib_cmf_method_mask 0x0F
  @zlib_fcheck_modulus 31
  @zlib_fdict_mask 0x20
  @zlib_header_size 2
  @zlib_trailer_size 4
  @zlib_overhead 6
  @average_divisor 2

  # Adler-32 constants
  @adler_initial_a 1
  @adler_initial_b 0
  @adler_base 65_521
  @adler_chunk_size 5552
  @adler_b_shift 16
  @uint32_mask 0xFFFFFFFF

  # Bit & byte arithmetic masks
  @byte_mask 0xFF
  @byte_midpoint 128
  @byte_modulus 256
  @bits_per_byte 8
  @bytes_per_rgba_pixel 4
  @transparent_alpha 0
  @opaque_alpha 255

  # Inlined accessors
  def png_signature, do: @png_signature
  def signature_length, do: @signature_length

  def chunk_length_size, do: @chunk_length_size
  def chunk_type_size, do: @chunk_type_size
  def chunk_header_size, do: @chunk_header_size
  def chunk_crc_size, do: @chunk_crc_size
  def chunk_overhead, do: @chunk_overhead
  def ancillary_bit_mask, do: @ancillary_bit_mask

  def chunk_ihdr, do: @chunk_ihdr
  def chunk_plte, do: @chunk_plte
  def chunk_trns, do: @chunk_trns
  def chunk_idat, do: @chunk_idat
  def chunk_iend, do: @chunk_iend
  def apng_chunks, do: @apng_chunks

  def png_max_dimension, do: @png_max_dimension
  def png_max_pixels, do: @png_max_pixels
  def default_max_pixels, do: @default_max_pixels
  def ihdr_length, do: @ihdr_length
  def plte_min_length, do: @plte_min_length
  def plte_max_length, do: @plte_max_length
  def plte_entry_size, do: @plte_entry_size
  def trns_grey_length, do: @trns_grey_length
  def trns_rgb_length, do: @trns_rgb_length
  def max_sample_value, do: @max_sample_value

  def colour_type_grey, do: @colour_type_grey
  def colour_type_rgb, do: @colour_type_rgb
  def colour_type_palette, do: @colour_type_palette
  def colour_type_grey_alpha, do: @colour_type_grey_alpha
  def colour_type_rgba, do: @colour_type_rgba
  def supported_colour_types, do: @supported_colour_types

  def channels_grey, do: @channels_grey
  def channels_rgb, do: @channels_rgb
  def channels_grey_alpha, do: @channels_grey_alpha
  def channels_rgba, do: @channels_rgba

  def channels_for_colour_type(@colour_type_grey), do: @channels_grey
  def channels_for_colour_type(@colour_type_rgb), do: @channels_rgb
  def channels_for_colour_type(@colour_type_grey_alpha), do: @channels_grey_alpha
  def channels_for_colour_type(@colour_type_rgba), do: @channels_rgba
  def channels_for_colour_type(_), do: nil

  def supported_bit_depth, do: @supported_bit_depth
  def compression_method_deflate, do: @compression_method_deflate
  def filter_method_adaptive, do: @filter_method_adaptive
  def interlace_method_none, do: @interlace_method_none

  def filter_none, do: @filter_none
  def filter_sub, do: @filter_sub
  def filter_up, do: @filter_up
  def filter_average, do: @filter_average
  def filter_paeth, do: @filter_paeth
  def filter_count, do: @filter_count
  def max_filter_type, do: @max_filter_type
  def average_divisor, do: @average_divisor

  def zlib_min_stream_size, do: @zlib_min_stream_size
  def zlib_cmf_deflate, do: @zlib_cmf_deflate
  def zlib_flg_default, do: @zlib_flg_default
  def zlib_method_deflate, do: @zlib_method_deflate
  def zlib_cinfo_max, do: @zlib_cinfo_max
  def zlib_cinfo_shift, do: @zlib_cinfo_shift
  def zlib_cmf_method_mask, do: @zlib_cmf_method_mask
  def zlib_fcheck_modulus, do: @zlib_fcheck_modulus
  def zlib_fdict_mask, do: @zlib_fdict_mask
  def zlib_header_size, do: @zlib_header_size
  def zlib_trailer_size, do: @zlib_trailer_size
  def zlib_overhead, do: @zlib_overhead

  def adler_initial_a, do: @adler_initial_a
  def adler_initial_b, do: @adler_initial_b
  def adler_base, do: @adler_base
  def adler_chunk_size, do: @adler_chunk_size
  def adler_b_shift, do: @adler_b_shift
  def uint32_mask, do: @uint32_mask

  def byte_mask, do: @byte_mask
  def byte_midpoint, do: @byte_midpoint
  def byte_modulus, do: @byte_modulus
  def bits_per_byte, do: @bits_per_byte
  def bytes_per_rgba_pixel, do: @bytes_per_rgba_pixel
  def transparent_alpha, do: @transparent_alpha
  def opaque_alpha, do: @opaque_alpha
end
