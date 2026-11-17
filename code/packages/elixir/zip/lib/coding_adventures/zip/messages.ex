defmodule CodingAdventures.Zip.Messages do
  @moduledoc """
  Standing Rule #20: 100% of error messages, diagnostic strings, and placeholders
  for CMP09 ZIP and raw RFC 1951 operations are strictly isolated in this module.
  All error messages are guaranteed payload-blind and free of sensitive data leakage.
  """

  @raw_inflate_error_codes [
    "invalid-output-limit",
    "unexpected-eof",
    "reserved-block-type",
    "stored-length-mismatch",
    "huffman-oversubscribed",
    "incomplete-code-length-tree",
    "incomplete-literal-length-tree",
    "incomplete-distance-tree",
    "repeat-without-previous",
    "repeat-overrun",
    "invalid-literal-length-symbol",
    "reserved-distance-symbol",
    "invalid-back-reference",
    "output-limit-exceeded"
  ]

  @doc "List of all 14 closed RFC 1951 error codes."
  def raw_inflate_error_codes, do: @raw_inflate_error_codes

  @doc "Map closed RFC 1951 error atom to hyphenated error string."
  def inflate_error_code(:invalid_output_limit), do: "invalid-output-limit"
  def inflate_error_code(:unexpected_eof), do: "unexpected-eof"
  def inflate_error_code(:reserved_block_type), do: "reserved-block-type"
  def inflate_error_code(:stored_length_mismatch), do: "stored-length-mismatch"
  def inflate_error_code(:huffman_oversubscribed), do: "huffman-oversubscribed"
  def inflate_error_code(:incomplete_code_length_tree), do: "incomplete-code-length-tree"
  def inflate_error_code(:incomplete_literal_length_tree), do: "incomplete-literal-length-tree"
  def inflate_error_code(:incomplete_distance_tree), do: "incomplete-distance-tree"
  def inflate_error_code(:repeat_without_previous), do: "repeat-without-previous"
  def inflate_error_code(:repeat_overrun), do: "repeat-overrun"
  def inflate_error_code(:invalid_literal_length_symbol), do: "invalid-literal-length-symbol"
  def inflate_error_code(:reserved_distance_symbol), do: "reserved-distance-symbol"
  def inflate_error_code(:invalid_back_reference), do: "invalid-back-reference"
  def inflate_error_code(:output_limit_exceeded), do: "output-limit-exceeded"
  def inflate_error_code(_), do: unknown_error()

  @doc "Documentation strings for sibling modules without inline string literals."
  def zip_moduledoc do
    "ZIP archive reader, writer, and raw RFC 1951 DEFLATE/INFLATE codec."
  end

  def constants_moduledoc do
    "Wire signatures, bitmasks, limits, and numeric constants for CMP09 ZIP and raw RFC 1951 DEFLATE operations."
  end

  @doc "Resolve RFC 1951 error message payload-blindly."
  def inflate_message(code) when code in @raw_inflate_error_codes, do: code
  def inflate_message(_), do: "unknown-error"

  # Diagnostic strings for ZIP archive structure validation
  def err_duplicate_entry, do: "zip: duplicate entry name"
  def err_local_header_out_of_bounds, do: "zip: local header out of bounds"
  def err_bad_local_header_sig, do: "zip: bad local header signature"
  def err_local_header_fields_out_of_bounds, do: "zip: local header fields out of bounds"
  def err_compressed_data_out_of_bounds, do: "zip: compressed data out of bounds"
  def err_trailing_bytes, do: "zip: compressed payload contains trailing bytes"
  def err_unsupported_compression_method, do: "zip: unsupported compression method"
  def err_uncompressed_size_mismatch, do: "zip: uncompressed size does not match the directory"
  def err_crc32_mismatch, do: "zip: CRC-32 mismatch"
  def err_entry_not_found, do: "zip: entry not found"
  def err_eocd_not_found, do: "zip: EOCD signature not found"
  def err_eocd_truncated, do: "zip: EOCD truncated"
  def err_entry_count_exceeded, do: "zip: entry count exceeds ZIP limit of 65535"
  def err_cd_offset_overlaps_eocd, do: "zip: CD offset overlaps EOCD"
  def err_cd_region_extends_into_eocd, do: "zip: CD region extends into EOCD"
  def err_cd_region_extends_beyond_archive, do: "zip: CD region extends beyond archive"
  def err_cd_entry_header_out_of_bounds, do: "zip: CD entry header out of bounds"
  def err_expected_cd_sig, do: "zip: expected CD signature"
  def err_cd_entry_fields_truncated, do: "zip: CD entry fields truncated"

  def err_cd_entry_name_extends_beyond_archive,
    do: "zip: CD entry name extends beyond archive data"

  def err_cd_entry_advance_out_of_bounds, do: "zip: CD entry advance out of bounds"
  def err_entry_name_invalid_encoding, do: "zip: entry name is not valid UTF-8 or Latin-1"
  def err_entry_name_null_byte, do: "zip: entry name contains null byte"
  def err_entry_name_backslash, do: "zip: entry name contains backslash"
  def err_entry_name_absolute, do: "zip: entry name is an absolute path"
  def err_entry_name_path_traversal, do: "zip: entry name contains path traversal (..)"
  def err_match_length_out_of_range, do: "zip: match length out of range"
  def err_distance_out_of_range, do: "zip: distance out of range"

  # Standard placeholders and fallbacks
  def redacted_placeholder, do: "<redacted>"
  def invalid_placeholder, do: "<invalid>"
  def unknown_error, do: "unknown-error"
  def unknown_error_message, do: "zip: unknown error"

  @doc "Enumerate all static diagnostic and error strings for verification."
  def all_diagnostic_messages do
    [
      err_duplicate_entry(),
      err_local_header_out_of_bounds(),
      err_bad_local_header_sig(),
      err_local_header_fields_out_of_bounds(),
      err_compressed_data_out_of_bounds(),
      err_trailing_bytes(),
      err_unsupported_compression_method(),
      err_uncompressed_size_mismatch(),
      err_crc32_mismatch(),
      err_entry_not_found(),
      err_eocd_not_found(),
      err_eocd_truncated(),
      err_entry_count_exceeded(),
      err_cd_offset_overlaps_eocd(),
      err_cd_region_extends_into_eocd(),
      err_cd_region_extends_beyond_archive(),
      err_cd_entry_header_out_of_bounds(),
      err_expected_cd_sig(),
      err_cd_entry_fields_truncated(),
      err_cd_entry_name_extends_beyond_archive(),
      err_cd_entry_advance_out_of_bounds(),
      err_entry_name_invalid_encoding(),
      err_entry_name_null_byte(),
      err_entry_name_backslash(),
      err_entry_name_absolute(),
      err_entry_name_path_traversal(),
      err_match_length_out_of_range(),
      err_distance_out_of_range(),
      redacted_placeholder(),
      invalid_placeholder(),
      unknown_error(),
      unknown_error_message()
    ]
  end
end
