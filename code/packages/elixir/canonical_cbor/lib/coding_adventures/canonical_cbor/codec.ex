defmodule CodingAdventures.CanonicalCbor.Codec do
  @moduledoc """
  Deterministic canonical CBOR encoder and decoder (RFC 8949 CBR01 profile).

  Guarantees:
  1. Length-first canonical map key ordering (§4.2.3).
  2. Minimal integer argument representation (§4.2.1).
  3. 64-bit unsigned bounds checks to defeat integer overflow attacks.
  4. Pre-allocation bounds validation (check_length) to defeat OOM bombs.
  5. Strict nesting depth caps (MAX_NESTING_DEPTH = 128) against stack overflow.
  6. Atomic encode_into_checked rollback on failure.
  7. Zero inline diagnostic strings (all via Messages).
  """

  import Bitwise

  alias CodingAdventures.CanonicalCbor.{
    CborError,
    Constants,
    DecodeMapSpansResult,
    Span,
    SpannedMapEntry,
    Types
  }

  alias CodingAdventures.CanonicalCbor.Types.{
    CborArray,
    CborBool,
    CborBytes,
    CborMap,
    CborMapEntry,
    CborNegative,
    CborNull,
    CborTag,
    CborText,
    CborUnsigned
  }

  # ===========================================================================
  # Public API
  # ===========================================================================

  @doc """
  Encode a CBOR value into canonical CBOR binary.
  Returns `{:ok, binary}` on success, or `{:error, %CborError{}}` on failure.
  """
  @spec encode_checked(term()) :: {:ok, binary()} | {:error, CborError.t()}
  def encode_checked(val) do
    {:ok, encode_checked!(val)}
  rescue
    e in [CborError] -> {:error, e}
  end

  @doc """
  Encode a CBOR value into canonical CBOR binary, raising `CborError` on failure.
  """
  @spec encode_checked!(term()) :: binary()
  def encode_checked!(val) do
    encode_value(val, <<>>, Constants.root_depth(), Constants.max_encoded_bytes())
  end

  @doc """
  Encode a CBOR value and append it onto an existing binary buffer atomically.
  On failure, returns the original buffer unchanged.
  """
  @spec encode_into_checked(term(), term()) :: {:ok, binary()} | {:error, CborError.t()}
  def encode_into_checked(val, buffer) do
    limit = Constants.max_encoded_bytes()

    cond do
      not is_binary(buffer) ->
        {:error, CborError.exception(:unsupported_simple)}

      byte_size(buffer) > limit ->
        {:error, CborError.exception(:encode_too_large)}

      true ->
        try do
          {:ok, encode_value(val, buffer, Constants.root_depth(), limit)}
        rescue
          e in [CborError] -> {:error, e}
        end
    end
  end

  @doc """
  Decode a canonical CBOR wire binary into a `CborValue`.
  Returns `{:ok, value}` on success, or `{:error, %CborError{}}` on failure.
  """
  @spec decode(term()) :: {:ok, Types.cbor_value()} | {:error, CborError.t()}
  def decode(wire_bytes) do
    {:ok, decode!(wire_bytes)}
  rescue
    e in [CborError] -> {:error, e}
  end

  @doc """
  Decode a canonical CBOR wire binary into a `CborValue`, raising `CborError` on failure.
  """
  @spec decode!(term()) :: Types.cbor_value()
  def decode!(wire_bytes) do
    if not is_binary(wire_bytes) do
      raise CborError, :unsupported_simple
    end

    {val, rest} = decode_value(wire_bytes, wire_bytes, Constants.root_depth())

    if byte_size(rest) > 0 do
      raise CborError, :trailing_bytes
    end

    val
  end

  @doc """
  Decode canonical CBOR map entries with exact wire spans.
  Returns `{:ok, DecodeMapSpansResult.t()}` on success, or `{:error, %CborError{}}` on failure.
  """
  @spec decode_map_spans(term()) :: {:ok, DecodeMapSpansResult.t()} | {:error, CborError.t()}
  def decode_map_spans(wire_bytes) do
    {:ok, decode_map_spans!(wire_bytes)}
  rescue
    e in [CborError] -> {:error, e}
  end

  @doc """
  Decode canonical CBOR map entries with exact wire spans, raising `CborError` on failure.
  """
  @spec decode_map_spans!(term()) :: DecodeMapSpansResult.t()
  def decode_map_spans!(wire_bytes) do
    if not is_binary(wire_bytes) do
      raise CborError, :unsupported_simple
    end

    if byte_size(wire_bytes) == 0 do
      raise CborError, :unexpected_eof
    end

    {major, _info, arg, rest} = read_header(wire_bytes)

    if major == Constants.major_map() do
      {spanned_entries, remaining} =
        read_spanned_map_entries(wire_bytes, rest, Constants.root_depth(), arg)

      if byte_size(remaining) > 0 do
        raise CborError, :trailing_bytes
      end

      DecodeMapSpansResult.map(spanned_entries)
    else
      # Non-map item: validate that the entire wire is well-formed canonical CBOR
      {_val, remaining} = decode_value(wire_bytes, wire_bytes, Constants.root_depth())

      if byte_size(remaining) > 0 do
        raise CborError, :trailing_bytes
      end

      DecodeMapSpansResult.non_map()
    end
  end

  # ===========================================================================
  # Encoding Internals
  # ===========================================================================

  defp push_bytes(buf, chunk, limit) do
    if byte_size(buf) + byte_size(chunk) > limit do
      raise CborError.exception(:encode_too_large)
    else
      buf <> chunk
    end
  end

  defp encode_value(val, buffer, depth, limit) do
    if depth > Constants.max_nesting_depth() do
      raise CborError.exception(:encode_too_deep)
    end

    case val do
      %CborUnsigned{value: v} ->
        if is_integer(v) and v >= 0 and v <= Constants.max_uint64() do
          push_bytes(buffer, write_type_and_argument(Constants.major_unsigned(), v), limit)
        else
          raise CborError.exception(:unsupported_simple)
        end

      %CborNegative{value: v} ->
        if is_integer(v) and v >= 0 and v <= Constants.max_uint64() do
          push_bytes(buffer, write_type_and_argument(Constants.major_negative(), v), limit)
        else
          raise CborError.exception(:unsupported_simple)
        end

      %CborBytes{bytes: b} ->
        if is_binary(b) do
          header = write_type_and_argument(Constants.major_bytes(), byte_size(b))
          buf = push_bytes(buffer, header, limit)
          push_bytes(buf, b, limit)
        else
          raise CborError.exception(:unsupported_simple)
        end

      %CborText{text: t} ->
        if is_binary(t) and String.valid?(t) do
          header = write_type_and_argument(Constants.major_text(), byte_size(t))
          buf = push_bytes(buffer, header, limit)
          push_bytes(buf, t, limit)
        else
          if is_binary(t) do
            raise CborError.exception(:invalid_utf8)
          else
            raise CborError.exception(:unsupported_simple)
          end
        end

      %CborArray{items: it} ->
        if is_list(it) do
          header = write_type_and_argument(Constants.major_array(), Kernel.length(it))
          buf = push_bytes(buffer, header, limit)

          Enum.reduce(it, buf, fn item, acc ->
            encode_value(item, acc, depth + 1, limit)
          end)
        else
          raise CborError.exception(:unsupported_simple)
        end

      %CborMap{entries: entries} ->
        if is_list(entries) and Enum.all?(entries, &match?(%CborMapEntry{}, &1)) do
          encode_map(entries, buffer, depth, limit)
        else
          raise CborError.exception(:unsupported_simple)
        end

      %CborTag{tag: t, value: inner} ->
        if is_integer(t) and t >= 0 and t <= Constants.max_uint64() do
          header = write_type_and_argument(Constants.major_tag(), t)
          buf = push_bytes(buffer, header, limit)
          encode_value(inner, buf, depth + 1, limit)
        else
          raise CborError.exception(:unsupported_simple)
        end

      %CborBool{value: false} ->
        push_bytes(buffer, <<Constants.wire_byte_false()>>, limit)

      %CborBool{value: true} ->
        push_bytes(buffer, <<Constants.wire_byte_true()>>, limit)

      %CborNull{} ->
        push_bytes(buffer, <<Constants.wire_byte_null()>>, limit)

      _ ->
        raise CborError.exception(:unsupported_simple)
    end
  end

  defp encode_map(entries, buffer, depth, limit) do
    key_limit = Constants.max_encoded_bytes()

    # Encode all keys and collect pair descriptors at depth + 1
    pairs =
      Enum.map(entries, fn %CborMapEntry{key: k, value: v} ->
        k_wire = encode_value(k, <<>>, depth + 1, key_limit)
        {k_wire, k, v}
      end)

    # Canonical Length-First order (§4.2.3):
    # 1. Shorter byte length precedes longer byte length.
    # 2. Equal length sorted by lexicographical byte comparison.
    sorted_pairs =
      Enum.sort(pairs, fn {k1_wire, _, _}, {k2_wire, _, _} ->
        l1 = byte_size(k1_wire)
        l2 = byte_size(k2_wire)

        if l1 != l2 do
          l1 < l2
        else
          k1_wire < k2_wire
        end
      end)

    # Duplicate key detection
    check_duplicate_keys(sorted_pairs)

    header = write_type_and_argument(Constants.major_map(), Kernel.length(sorted_pairs))
    buf = push_bytes(buffer, header, limit)

    Enum.reduce(sorted_pairs, buf, fn {k_wire, _k, v}, acc ->
      acc_with_key = push_bytes(acc, k_wire, limit)
      encode_value(v, acc_with_key, depth + 1, limit)
    end)
  end

  defp check_duplicate_keys([]), do: :ok
  defp check_duplicate_keys([_single]), do: :ok

  defp check_duplicate_keys([{k1, _, _}, {k2, _, _} | rest]) do
    if k1 == k2 do
      raise CborError, :duplicate_map_key
    else
      check_duplicate_keys([{k2, nil, nil} | rest])
    end
  end

  defp write_type_and_argument(major, arg) do
    lead = major <<< Constants.major_type_shift()

    cond do
      arg <= Constants.info_inline_max() ->
        <<lead ||| arg>>

      arg <= Constants.max_uint8() ->
        <<lead ||| Constants.info_1_byte(), arg::8>>

      arg <= Constants.max_uint16() ->
        <<lead ||| Constants.info_2_bytes(), arg::16-big>>

      arg <= Constants.max_uint32() ->
        <<lead ||| Constants.info_4_bytes(), arg::32-big>>

      arg <= Constants.max_uint64() ->
        <<lead ||| Constants.info_8_bytes(), arg::64-big>>

      true ->
        raise CborError, :unsupported_simple
    end
  end

  # ===========================================================================
  # Decoding Internals
  # ===========================================================================

  defp decode_value(total_wire, rest, depth) do
    if depth > Constants.max_nesting_depth() do
      raise CborError, :too_deep
    end

    {major, info, arg, after_hdr} = read_header(rest)

    cond do
      major == Constants.major_unsigned() ->
        {%CborUnsigned{value: arg}, after_hdr}

      major == Constants.major_negative() ->
        {%CborNegative{value: arg}, after_hdr}

      major == Constants.major_bytes() ->
        check_length(arg, byte_size(after_hdr), Constants.min_bytes_per_raw_byte())

        if byte_size(after_hdr) < arg do
          raise CborError, :unexpected_eof
        end

        <<bytes::binary-size(arg), remaining::binary>> = after_hdr
        {%CborBytes{bytes: bytes}, remaining}

      major == Constants.major_text() ->
        check_length(arg, byte_size(after_hdr), Constants.min_bytes_per_raw_byte())

        if byte_size(after_hdr) < arg do
          raise CborError, :unexpected_eof
        end

        <<octets::binary-size(arg), remaining::binary>> = after_hdr

        if not String.valid?(octets) do
          raise CborError, :invalid_utf8
        end

        {%CborText{text: octets}, remaining}

      major == Constants.major_array() ->
        check_length(arg, byte_size(after_hdr), Constants.min_bytes_per_array_item())

        {items, remaining} =
          Enum.reduce(1..arg//1, {[], after_hdr}, fn _idx, {acc_items, curr_bin} ->
            {item, next_bin} = decode_value(total_wire, curr_bin, depth + 1)
            {[item | acc_items], next_bin}
          end)

        {%CborArray{items: Enum.reverse(items)}, remaining}

      major == Constants.major_map() ->
        check_length(arg, byte_size(after_hdr), Constants.min_bytes_per_map_entry())

        {spanned_entries, remaining} =
          read_spanned_map_entries(total_wire, after_hdr, depth, arg)

        cbor_entries =
          Enum.map(spanned_entries, fn %SpannedMapEntry{key: k, value: v} ->
            %CborMapEntry{key: k, value: v}
          end)

        {%CborMap{entries: cbor_entries}, remaining}

      major == Constants.major_tag() ->
        {inner, remaining} = decode_value(total_wire, after_hdr, depth + 1)
        {%CborTag{tag: arg, value: inner}, remaining}

      major == Constants.major_simple_or_float() ->
        cond do
          info == Constants.simple_false() ->
            {%CborBool{value: false}, after_hdr}

          info == Constants.simple_true() ->
            {%CborBool{value: true}, after_hdr}

          info == Constants.simple_null() ->
            {%CborNull{}, after_hdr}

          info in [
            Constants.simple_float_16(),
            Constants.simple_float_32(),
            Constants.simple_float_64()
          ] ->
            raise CborError, :float_not_supported

          true ->
            raise CborError, :unsupported_simple
        end
    end
  end

  defp read_header(bin) do
    if byte_size(bin) == 0 do
      raise CborError, :unexpected_eof
    end

    <<header_byte::8, rest::binary>> = bin

    major =
      header_byte >>> Constants.major_type_shift() &&&
        Constants.major_type_mask()

    info = header_byte &&& Constants.additional_info_mask()

    {arg, after_arg} =
      cond do
        info <= Constants.info_inline_max() ->
          {info, rest}

        info == Constants.info_1_byte() ->
          if byte_size(rest) < Constants.arg_width_1_byte() do
            raise CborError, :unexpected_eof
          end

          <<val::8, after_bytes::binary>> = rest
          {val, after_bytes}

        info == Constants.info_2_bytes() ->
          if byte_size(rest) < Constants.arg_width_2_bytes() do
            raise CborError, :unexpected_eof
          end

          <<val::16-big, after_bytes::binary>> = rest
          {val, after_bytes}

        info == Constants.info_4_bytes() ->
          if byte_size(rest) < Constants.arg_width_4_bytes() do
            raise CborError, :unexpected_eof
          end

          <<val::32-big, after_bytes::binary>> = rest
          {val, after_bytes}

        info == Constants.info_8_bytes() ->
          if byte_size(rest) < Constants.arg_width_8_bytes() do
            raise CborError, :unexpected_eof
          end

          <<val::64-big, after_bytes::binary>> = rest
          {val, after_bytes}

        info >= Constants.info_reserved_28() and info <= Constants.info_reserved_30() ->
          raise CborError, :reserved

        info == Constants.info_indefinite() ->
          raise CborError, :indefinite

        true ->
          raise CborError, :unsupported_simple
      end

    validate_minimal_argument(major, info, arg)
    {major, info, arg, after_arg}
  end

  defp validate_minimal_argument(major, info, arg) do
    cond do
      major == Constants.major_simple_or_float() ->
        if info == Constants.info_1_byte() and arg < Constants.min_1_byte_simple_value() do
          raise CborError, :non_minimal_integer
        end

      info == Constants.info_1_byte() ->
        if arg <= Constants.info_inline_max() do
          raise CborError, :non_minimal_integer
        end

      info == Constants.info_2_bytes() ->
        if arg <= Constants.max_uint8() do
          raise CborError, :non_minimal_integer
        end

      info == Constants.info_4_bytes() ->
        if arg <= Constants.max_uint16() do
          raise CborError, :non_minimal_integer
        end

      info == Constants.info_8_bytes() ->
        if arg <= Constants.max_uint32() do
          raise CborError, :non_minimal_integer
        end

      true ->
        :ok
    end
  end

  defp check_length(declared, remaining, min_bytes_per_unit) do
    max_allowed = div(remaining, min_bytes_per_unit)

    if declared > max_allowed do
      raise CborError, :length_too_large
    end

    declared
  end

  defp read_spanned_map_entries(total_wire, rest, depth, count) do
    if count == 0 do
      {[], rest}
    else
      total_len = byte_size(total_wire)

      {entries, remaining, _prev_k_wire} =
        Enum.reduce(1..count//1, {[], rest, nil}, fn _idx, {acc, curr_bin, prev_k_wire} ->
          k_wire_start = total_len - byte_size(curr_bin)

          {key, after_key_bin} = decode_value(total_wire, curr_bin, depth + 1)
          k_wire_finish = total_len - byte_size(after_key_bin)
          k_wire = binary_part(total_wire, k_wire_start, k_wire_finish - k_wire_start)

          # Verify canonical map key order (§4.2.3)
          if prev_k_wire != nil do
            l1 = byte_size(prev_k_wire)
            l2 = byte_size(k_wire)

            if l1 > l2 or (l1 == l2 and prev_k_wire >= k_wire) do
              raise CborError, :non_canonical_map_order
            end
          end

          v_start = total_len - byte_size(after_key_bin)
          {value, after_val_bin} = decode_value(total_wire, after_key_bin, depth + 1)
          v_finish = total_len - byte_size(after_val_bin)

          span = Span.new(v_start, v_finish)
          entry = SpannedMapEntry.new(key, value, span)

          {[entry | acc], after_val_bin, k_wire}
        end)

      {Enum.reverse(entries), remaining}
    end
  end
end
