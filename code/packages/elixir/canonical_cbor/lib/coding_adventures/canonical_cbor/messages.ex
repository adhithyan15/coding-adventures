defmodule CodingAdventures.CanonicalCbor.Messages do
  @moduledoc """
  Single source of truth for all 14 closed payload-blind CBR01 error diagnostics
  and security redaction placeholders (Standing Rule #20).

  No user-supplied data, keys, values, offsets, or sizes are ever reflected in
  error diagnostics, guaranteeing defense-in-depth against secret leakage.
  """

  @redacted_placeholder "<redacted>"
  @invalid_placeholder "<invalid>"
  @unknown_error_id_message "canonical-cbor: unknown error id"

  @doc """
  Fixed security redaction placeholder used in string representations.
  """
  @spec redacted_placeholder() :: String.t()
  def redacted_placeholder, do: @redacted_placeholder

  @doc """
  Fixed placeholder used in string representations when an invalid struct is inspected.
  """
  @spec invalid_placeholder() :: String.t()
  def invalid_placeholder, do: @invalid_placeholder

  @doc """
  Fixed payload-blind diagnostic when an invalid or unknown error ID is requested.
  """
  @spec unknown_error_id_message() :: String.t()
  def unknown_error_id_message, do: @unknown_error_id_message

  @doc """
  Return the payload-blind diagnostic message for a given error ID.
  Never reflects the input argument if invalid or unknown.
  Accepts either atom or string identifier without dynamic atom creation.
  """
  @spec message_for(atom() | String.t()) :: String.t()
  def message_for(id) do
    case id do
      :unexpected_eof -> "canonical-cbor: unexpected end of input"
      "unexpected-eof" -> "canonical-cbor: unexpected end of input"
      :trailing_bytes -> "canonical-cbor: trailing bytes after decoded item"
      "trailing-bytes" -> "canonical-cbor: trailing bytes after decoded item"
      :reserved -> "canonical-cbor: reserved additional-info value"
      "reserved" -> "canonical-cbor: reserved additional-info value"
      :indefinite -> "canonical-cbor: indefinite item rejected"
      "indefinite" -> "canonical-cbor: indefinite item rejected"
      :non_minimal_integer -> "canonical-cbor: argument is not in smallest form"
      "non-minimal-integer" -> "canonical-cbor: argument is not in smallest form"
      :invalid_utf8 -> "canonical-cbor: text is not valid UTF-8"
      "invalid-utf8" -> "canonical-cbor: text is not valid UTF-8"
      :non_canonical_map_order -> "canonical-cbor: map key order is not canonical"
      "non-canonical-map-order" -> "canonical-cbor: map key order is not canonical"
      :unsupported_simple -> "canonical-cbor: unsupported simple value"
      "unsupported-simple" -> "canonical-cbor: unsupported simple value"
      :float_not_supported -> "canonical-cbor: floats are not supported"
      "float-not-supported" -> "canonical-cbor: floats are not supported"
      :too_deep -> "canonical-cbor: decoded nesting is too deep"
      "too-deep" -> "canonical-cbor: decoded nesting is too deep"
      :length_too_large -> "canonical-cbor: declared length is too large"
      "length-too-large" -> "canonical-cbor: declared length is too large"
      :duplicate_map_key -> "canonical-cbor: duplicate canonical map key"
      "duplicate-map-key" -> "canonical-cbor: duplicate canonical map key"
      :encode_too_deep -> "canonical-cbor: encoded nesting is too deep"
      "encode-too-deep" -> "canonical-cbor: encoded nesting is too deep"
      :encode_too_large -> "canonical-cbor: encoded item is too large"
      "encode-too-large" -> "canonical-cbor: encoded item is too large"
      _ -> @unknown_error_id_message
    end
  end

  @doc """
  List of all 14 canonical error ID atoms.
  """
  @spec all_error_ids() :: [atom()]
  def all_error_ids do
    [
      :unexpected_eof,
      :trailing_bytes,
      :reserved,
      :indefinite,
      :non_minimal_integer,
      :invalid_utf8,
      :non_canonical_map_order,
      :unsupported_simple,
      :float_not_supported,
      :too_deep,
      :length_too_large,
      :duplicate_map_key,
      :encode_too_deep,
      :encode_too_large
    ]
  end
end
