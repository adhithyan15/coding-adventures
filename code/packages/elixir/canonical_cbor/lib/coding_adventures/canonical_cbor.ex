defmodule CodingAdventures.CanonicalCbor do
  @moduledoc """
  Deterministic canonical CBOR encoder and decoder (RFC 8949 CBR01 profile).

  RFC 8949 Section 4.2.3 Length-First Map Key Ordering profile.
  Guarantees byte-exact reproducibility across all platforms, languages, and runs.
  """

  alias CodingAdventures.CanonicalCbor.{
    CborError,
    Codec,
    DecodeMapSpansResult,
    Span,
    SpannedMapEntry,
    Types
  }

  # Re-export types
  @type cbor_value :: Types.cbor_value()
  @type cbor_error :: CborError.t()
  @type span :: Span.t()
  @type spanned_map_entry :: SpannedMapEntry.t()
  @type decode_map_spans_result :: DecodeMapSpansResult.t()

  # ---------------------------------------------------------------------------
  # Codec Delegations
  # ---------------------------------------------------------------------------

  @doc """
  Encode a CBOR value into canonical CBOR binary.
  Returns `{:ok, binary}` on success, or `{:error, %CborError{}}` on failure.
  """
  defdelegate encode_checked(val), to: Codec

  @doc """
  Encode a CBOR value into canonical CBOR binary, raising `CborError` on failure.
  """
  defdelegate encode_checked!(val), to: Codec

  @doc """
  Encode a CBOR value and append it onto an existing binary buffer atomically.
  On failure, returns the original buffer unchanged.
  """
  defdelegate encode_into_checked(val, buffer), to: Codec

  @doc """
  Decode a canonical CBOR wire binary into a `CborValue`.
  Returns `{:ok, value}` on success, or `{:error, %CborError{}}` on failure.
  """
  defdelegate decode(wire_bytes), to: Codec

  @doc """
  Decode a canonical CBOR wire binary into a `CborValue`, raising `CborError` on failure.
  """
  defdelegate decode!(wire_bytes), to: Codec

  @doc """
  Decode canonical CBOR map entries with exact wire spans.
  Returns `{:ok, DecodeMapSpansResult.t()}` on success, or `{:error, %CborError{}}` on failure.
  """
  defdelegate decode_map_spans(wire_bytes), to: Codec

  @doc """
  Decode canonical CBOR map entries with exact wire spans, raising `CborError` on failure.
  """
  defdelegate decode_map_spans!(wire_bytes), to: Codec

  # ---------------------------------------------------------------------------
  # Constructor Delegations
  # ---------------------------------------------------------------------------

  defdelegate cbor_unsigned(val), to: Types
  defdelegate cbor_negative(val), to: Types
  defdelegate cbor_bytes(bytes), to: Types
  defdelegate cbor_text(text), to: Types
  defdelegate cbor_array(items), to: Types
  defdelegate cbor_map_entry(key, value), to: Types
  defdelegate cbor_map(entries), to: Types
  defdelegate cbor_tag(tag, value), to: Types
  defdelegate cbor_bool(val), to: Types
  defdelegate cbor_null(), to: Types

  @doc "Construct a wire byte offset span."
  def span(start, finish), do: Span.new(start, finish)

  @doc "Construct a spanned map entry."
  def spanned_map_entry(key, value, span), do: SpannedMapEntry.new(key, value, span)

  # ---------------------------------------------------------------------------
  # Predicate Delegations
  # ---------------------------------------------------------------------------

  defdelegate is_cbor_value?(v), to: Types
  defdelegate is_cbor_map_entry?(v), to: Types
  defdelegate is_span?(v), to: Types
  defdelegate is_spanned_map_entry?(v), to: Types
  defdelegate is_decode_map_spans_result?(v), to: Types
  defdelegate is_cbor_error?(v), to: Types
end
