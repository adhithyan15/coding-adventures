defmodule CodingAdventures.ImageCodecPng.Messages do
  @moduledoc """
  Standing Rule #20: 100% of error messages, diagnostic strings, and placeholders
  for IC18 PNG codec operations are strictly isolated in this module.
  All error messages are guaranteed payload-blind and free of sensitive data leakage.
  """

  @mime_type "image/png"
  @redacted_placeholder "<redacted>"
  @invalid_placeholder "<invalid>"
  @unknown_error_code "unknown-error"
  @unknown_error_message "image-codec-png: unknown error code"

  @error_codes [
    "invalid-max-pixels",
    "invalid-image-dimensions",
    "invalid-pixel-data-length",
    "file-too-short",
    "invalid-signature",
    "truncated-chunk",
    "invalid-chunk-type",
    "chunk-crc-mismatch",
    "chunk-before-ihdr",
    "duplicate-ihdr",
    "invalid-ihdr-length",
    "invalid-dimensions",
    "dimension-limit",
    "pixel-limit",
    "unsupported-feature",
    "invalid-plte",
    "invalid-trns",
    "nonconsecutive-idat",
    "invalid-iend",
    "trailing-data",
    "unknown-critical-chunk",
    "missing-required-chunk",
    "invalid-zlib-header",
    "preset-dictionary",
    "inflate-failed",
    "inflated-length-mismatch",
    "idat-cavity",
    "adler-mismatch",
    "invalid-filter"
  ]

  @error_messages %{
    "invalid-max-pixels" => "image-codec-png: max_pixels must be a positive integer <= 33554432",
    "invalid-image-dimensions" =>
      "image-codec-png: image dimensions must be positive integers within limit",
    "invalid-pixel-data-length" =>
      "image-codec-png: pixel buffer length does not match width * height * 4",
    "file-too-short" => "image-codec-png: input too short to contain PNG signature",
    "invalid-signature" => "image-codec-png: invalid PNG signature",
    "truncated-chunk" => "image-codec-png: chunk header or payload truncated",
    "invalid-chunk-type" => "image-codec-png: invalid chunk type name",
    "chunk-crc-mismatch" => "image-codec-png: chunk CRC mismatch",
    "chunk-before-ihdr" => "image-codec-png: chunk found before IHDR",
    "duplicate-ihdr" => "image-codec-png: duplicate IHDR chunk",
    "invalid-ihdr-length" => "image-codec-png: IHDR length must be exactly 13 bytes",
    "invalid-dimensions" => "image-codec-png: image dimensions must be non-zero",
    "dimension-limit" => "image-codec-png: image dimensions exceed limit",
    "pixel-limit" => "image-codec-png: image pixel count exceeds limit",
    "unsupported-feature" => "image-codec-png: unsupported PNG feature or APNG chunk",
    "invalid-plte" => "image-codec-png: invalid, duplicate, or misplaced PLTE chunk",
    "invalid-trns" => "image-codec-png: invalid, duplicate, or misplaced tRNS chunk",
    "nonconsecutive-idat" => "image-codec-png: non-consecutive IDAT chunks",
    "invalid-iend" => "image-codec-png: IEND chunk must be empty",
    "trailing-data" => "image-codec-png: trailing data found after IEND chunk",
    "unknown-critical-chunk" => "image-codec-png: unknown critical chunk",
    "missing-required-chunk" => "image-codec-png: missing required chunk (IHDR, IDAT, or IEND)",
    "invalid-zlib-header" => "image-codec-png: invalid zlib header in IDAT",
    "preset-dictionary" => "image-codec-png: preset dictionary is unsupported",
    "inflate-failed" => "image-codec-png: DEFLATE decompression failed",
    "inflated-length-mismatch" =>
      "image-codec-png: decompressed length does not match expected scanline size",
    "idat-cavity" => "image-codec-png: unconsumed bytes between compressed data and Adler-32",
    "adler-mismatch" => "image-codec-png: Adler-32 checksum mismatch",
    "invalid-filter" => "image-codec-png: invalid scanline filter type"
  }

  @doc "The IANA MIME type for PNG."
  def mime_type, do: @mime_type

  @doc "Redacted placeholder string."
  def redacted_placeholder, do: @redacted_placeholder

  @doc "Invalid placeholder string."
  def invalid_placeholder, do: @invalid_placeholder

  @doc "List of all 29 closed IC18 error codes."
  def error_codes, do: @error_codes

  @doc "Map closed IC18 error atom to hyphenated error string."
  def error_code(:invalid_max_pixels), do: "invalid-max-pixels"
  def error_code(:invalid_image_dimensions), do: "invalid-image-dimensions"
  def error_code(:invalid_pixel_data_length), do: "invalid-pixel-data-length"
  def error_code(:file_too_short), do: "file-too-short"
  def error_code(:invalid_signature), do: "invalid-signature"
  def error_code(:truncated_chunk), do: "truncated-chunk"
  def error_code(:invalid_chunk_type), do: "invalid-chunk-type"
  def error_code(:chunk_crc_mismatch), do: "chunk-crc-mismatch"
  def error_code(:chunk_before_ihdr), do: "chunk-before-ihdr"
  def error_code(:duplicate_ihdr), do: "duplicate-ihdr"
  def error_code(:invalid_ihdr_length), do: "invalid-ihdr-length"
  def error_code(:invalid_dimensions), do: "invalid-dimensions"
  def error_code(:dimension_limit), do: "dimension-limit"
  def error_code(:pixel_limit), do: "pixel-limit"
  def error_code(:unsupported_feature), do: "unsupported-feature"
  def error_code(:invalid_plte), do: "invalid-plte"
  def error_code(:invalid_trns), do: "invalid-trns"
  def error_code(:nonconsecutive_idat), do: "nonconsecutive-idat"
  def error_code(:invalid_iend), do: "invalid-iend"
  def error_code(:trailing_data), do: "trailing-data"
  def error_code(:unknown_critical_chunk), do: "unknown-critical-chunk"
  def error_code(:missing_required_chunk), do: "missing-required-chunk"
  def error_code(:invalid_zlib_header), do: "invalid-zlib-header"
  def error_code(:preset_dictionary), do: "preset-dictionary"
  def error_code(:inflate_failed), do: "inflate-failed"
  def error_code(:inflated_length_mismatch), do: "inflated-length-mismatch"
  def error_code(:idat_cavity), do: "idat-cavity"
  def error_code(:adler_mismatch), do: "adler-mismatch"
  def error_code(:invalid_filter), do: "invalid-filter"
  def error_code(code) when is_binary(code) and code in @error_codes, do: code
  def error_code(_), do: @unknown_error_code

  @doc "Returns the payload-blind diagnostic message for an error code or reason atom."
  def message_for(reason) when is_atom(reason) do
    message_for(error_code(reason))
  end

  def message_for(code) when is_binary(code) do
    Map.get(@error_messages, code, @unknown_error_message)
  end

  def message_for(_), do: @unknown_error_message
end
