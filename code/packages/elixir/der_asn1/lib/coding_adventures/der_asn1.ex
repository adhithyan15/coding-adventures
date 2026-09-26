defmodule CodingAdventures.DerAsn1 do
  @moduledoc """
  Bounded, payload-blind typed ASN.1 DER decoding above `CodingAdventures.DerTlv`.

  Decoder, element, cursor, and typed values are authenticated opaque closures.
  Shared atomics make decoder budgets and cursor progress replay-safe while
  normal BEAM value sharing keeps parsing free of ambient authority.
  """

  import Bitwise
  alias CodingAdventures.DerTlv

  @u64_max 0xFFFF_FFFF_FFFF_FFFF
  @host_max 0x7FFF_FFFF_FFFF_FFFF
  @default_limits %{
    der: %{
      max_input_len: 1_048_576,
      max_value_len: 1_048_576,
      max_elements: 4_096,
      max_tag_number: 0xFFFF_FFFF
    },
    max_depth: 32,
    max_total_elements: 16_384,
    max_oid_arcs: 128
  }

  @on_load :initialize_seal

  defp initialize_seal do
    :persistent_term.put({__MODULE__, :seal_secret}, {make_ref(), make_ref()})
    :ok
  end

  defmodule Decoder do
    @moduledoc "Opaque immutable decoder state produced by `new_decoder/1`."
    @opaque t :: (term() -> term())
  end

  defmodule Element do
    @moduledoc "Opaque validated element state returned by `decode_exact/2`."
    @opaque t :: (term() -> term())
  end

  defmodule Cursor do
    @moduledoc "Opaque immutable sibling cursor returned by `sequence/2` or `set/2`."
    @opaque t :: (term() -> term())
  end

  defmodule Error do
    @moduledoc "A stable payload-blind typed DER error."
    defexception [:kind, :offset, :framing_kind]

    @impl true
    def message(%__MODULE__{kind: kind, offset: offset, framing_kind: nil}),
      do: "DER ASN.1 error #{kind} at byte #{offset}"

    def message(%__MODULE__{kind: kind, offset: offset, framing_kind: framing}),
      do: "DER ASN.1 error #{kind} (#{framing}) at byte #{offset}"
  end

  defmodule IntegerValue do
    @moduledoc "A validated canonical DER INTEGER."
    @opaque t :: (term() -> term())
  end

  defmodule BitString do
    @moduledoc "A validated DER BIT STRING."
    @opaque t :: (term() -> term())
  end

  defmodule ObjectIdentifier do
    @moduledoc "A completely validated DER OBJECT IDENTIFIER."
    @opaque t :: (term() -> term())
  end

  def default_limits, do: @default_limits

  def new_decoder(limits \\ %{}) do
    with {:ok, normalized} <- normalize_limits(limits) do
      budget = :atomics.new(1, signed: false)
      {:ok, sealed_fun(:decoder, %{owner: make_ref(), limits: normalized, budget: budget})}
    end
  end

  def decoder_limits(decoder), do: decoder_data!(decoder).limits
  def elements_read(decoder), do: decoder_data!(decoder).budget |> :atomics.get(1)

  def decode_exact(decoder, input) when is_binary(input) do
    data = decoder_data!(decoder)

    cond do
      data.limits.max_depth == 0 ->
        {:error, error("depth-limit-exceeded", 0)}

      true ->
        with :ok <- reserve_budget(data, 0) do
          cond do
            byte_size(input) > data.limits.der.max_input_len ->
              release_budget(data)
              {:error, error("framing", 0, "input-limit-exceeded")}

            true ->
              case DerTlv.decode_exact(:binary.copy(input), data.limits.der) do
                {:ok, framed} ->
                  element = wrap_element(framed, 0, data.owner, data.budget)
                  {:ok, element, decoder}

                {:error, failure} ->
                  release_budget(data)
                  {:error, framing(failure)}
              end
          end
        end
    end
  end

  def decode_exact(_decoder, _input),
    do: {:error, %ArgumentError{message: "input must be binary"}}

  def element_tag(element), do: element_data!(element).tag
  def element_header(element), do: element_data!(element).header
  def element_value(element), do: element_data!(element).value
  def element_encoded(element), do: element_data!(element).encoded
  def element_depth(element), do: element_data!(element).depth

  def sequence(decoder, element), do: container(decoder, element, "universal", true, 16)
  def set(decoder, element), do: container(decoder, element, "universal", true, 17)

  def explicit(decoder, element, tag_number) do
    with {:ok, tag_number} <- validate_tag_number(tag_number),
         {:ok, decoder_data, element_data} <- owned(decoder, element),
         :ok <- expect_tag(element_data, "context-specific", true, tag_number),
         :ok <- check_child_depth(element_data.depth, decoder_data.limits),
         :ok <- reserve_budget(decoder_data, element_data.value_offset) do
      case DerTlv.decode_exact(element_data.value, decoder_data.limits.der) do
        {:ok, framed} ->
          child =
            wrap_element(
              framed,
              element_data.depth + 1,
              decoder_data.owner,
              decoder_data.budget
            )

          {:ok, child, decoder}

        {:error, failure} ->
          release_budget(decoder_data)
          {:error, framing(failure)}
      end
    end
  end

  def cursor_remaining(cursor) do
    data = cursor_data!(cursor)
    offset = :atomics.get(data.progress, 1)
    binary_part(data.input, offset, byte_size(data.input) - offset)
  end

  def cursor_finish(cursor) do
    if byte_size(cursor_remaining(cursor)) == 0,
      do: :ok,
      else: {:error, error("framing", data_prefix(cursor), "trailing-data")}
  end

  def cursor_read(cursor, decoder) do
    cursor_data = cursor_data!(cursor)
    decoder_data = decoder_data!(decoder)

    with_cursor_lock(cursor_data, fn ->
      offset = :atomics.get(cursor_data.progress, 1)
      lower_count = :atomics.get(cursor_data.progress, 2)

      cond do
        offset == byte_size(cursor_data.input) ->
          {:end, cursor}

        decoder_data.owner != cursor_data.owner or
            decoder_data.budget != cursor_data.budget ->
          {:error, error("decoder-limit-mismatch", 0), cursor, decoder}

        true ->
          with :ok <- reserve_budget(decoder_data, 0) do
            framing = %DerTlv.Cursor{
              input: cursor_data.input,
              limits: cursor_data.limits,
              offset: offset,
              elements_read: lower_count
            }

            case DerTlv.read(framing) do
              {:ok, framed, next_framing} ->
                :atomics.put(cursor_data.progress, 1, next_framing.offset)
                :atomics.put(cursor_data.progress, 2, next_framing.elements_read)

                child =
                  wrap_element(
                    framed,
                    cursor_data.depth,
                    cursor_data.owner,
                    cursor_data.budget
                  )

                {:ok, child, cursor, decoder}

              {:error, failure, _same} ->
                release_budget(decoder_data)
                {:error, framing(failure), cursor, decoder}
            end
          else
            {:error, failure} -> {:error, failure, cursor, decoder}
          end
      end
    end)
  end

  def decode_boolean(element) do
    with {:ok, value, offset} <- primitive(element, "universal", 1) do
      case value do
        <<0>> -> {:ok, false}
        <<0xFF>> -> {:ok, true}
        <<_>> -> {:error, error("invalid-boolean-value", offset)}
        _ -> {:error, error("invalid-boolean-length", offset)}
      end
    end
  end

  def decode_integer(element) do
    with {:ok, value, offset} <- primitive(element, "universal", 2),
         :ok <- validate_integer(value, offset) do
      <<first, _::binary>> = value

      {:ok,
       sealed_fun(:integer_value, %{
         signed_bytes: value,
         negative: (first &&& 0x80) != 0,
         value_offset: offset
       })}
    end
  end

  def integer_signed_bytes(integer), do: typed_data!(integer, :integer_value).signed_bytes
  def integer_negative?(integer), do: typed_data!(integer, :integer_value).negative

  def integer_to_u64(integer) do
    data = typed_data!(integer, :integer_value)

    if data.negative do
      {:error, error("negative-integer", data.value_offset)}
    else
      bytes =
        case data.signed_bytes do
          <<0, rest::binary>> when byte_size(rest) > 0 -> rest
          value -> value
        end

      if byte_size(bytes) > 8,
        do: {:error, error("integer-overflow", data.value_offset)},
        else: {:ok, :binary.decode_unsigned(bytes)}
    end
  rescue
    ArgumentError -> {:error, %ArgumentError{message: "expected validated INTEGER"}}
  end

  def decode_bit_string(element) do
    with {:ok, value, offset} <- primitive(element, "universal", 3) do
      case value do
        <<>> ->
          {:error, error("missing-unused-bit-count", offset)}

        <<unused, payload::binary>> when unused <= 7 ->
          cond do
            byte_size(payload) == 0 and unused != 0 ->
              {:error, error("invalid-unused-bit-count", offset)}

            byte_size(payload) > 0 and unused > 0 and
                (:binary.last(payload) &&& (1 <<< unused) - 1) != 0 ->
              {:error, error("non-zero-bit-padding", offset + byte_size(value) - 1)}

            byte_size(payload) > div(@host_max + unused, 8) ->
              {:error, error("bit-length-overflow", offset)}

            true ->
              {:ok,
               sealed_fun(:bit_string, %{
                 bytes: payload,
                 unused_bits: unused,
                 bit_length: byte_size(payload) * 8 - unused
               })}
          end

        _ ->
          {:error, error("invalid-unused-bit-count", offset)}
      end
    end
  end

  def bit_string_bytes(bits), do: typed_data!(bits, :bit_string).bytes
  def bit_string_unused_bits(bits), do: typed_data!(bits, :bit_string).unused_bits
  def bit_string_bit_length(bits), do: typed_data!(bits, :bit_string).bit_length

  def decode_octet_string(element), do: primitive_value(element, "universal", 4)

  def decode_implicit_octet_string(element, tag_number) do
    with {:ok, tag_number} <- validate_tag_number(tag_number),
         do: primitive_value(element, "context-specific", tag_number)
  end

  def decode_ia5_string(element) do
    with {:ok, value, offset} <- primitive(element, "universal", 22),
         do: decode_ia5(value, offset)
  end

  def decode_implicit_ia5_string(element, tag_number) do
    with {:ok, tag_number} <- validate_tag_number(tag_number),
         {:ok, value, offset} <- primitive(element, "context-specific", tag_number),
         do: decode_ia5(value, offset)
  end

  def decode_null(element) do
    with {:ok, value, offset} <- primitive(element, "universal", 5) do
      if value == <<>>, do: {:ok, nil}, else: {:error, error("non-empty-null", offset)}
    end
  end

  def decode_object_identifier(element, limits \\ %{}) do
    with {:ok, configured} <- normalize_limits(limits),
         {:ok, value, offset} <- primitive(element, "universal", 6),
         do: decode_oid(value, offset, configured.max_oid_arcs)
  end

  def decode_implicit_object_identifier(element, tag_number, limits \\ %{}) do
    with {:ok, configured} <- normalize_limits(limits),
         {:ok, tag_number} <- validate_tag_number(tag_number),
         {:ok, value, offset} <- primitive(element, "context-specific", tag_number),
         do: decode_oid(value, offset, configured.max_oid_arcs)
  end

  def oid_encoded(oid), do: typed_data!(oid, :object_identifier).encoded
  def oid_arcs(oid), do: typed_data!(oid, :object_identifier).arcs

  def oid_arc_count(oid),
    do: oid |> typed_data!(:object_identifier) |> Map.fetch!(:arcs) |> length()

  def oid_equals?(oid, expected),
    do: typed_data!(oid, :object_identifier).arcs == Enum.to_list(expected)

  defp container(decoder, element, tag_class, constructed, number) do
    with {:ok, decoder_data, element_data} <- owned(decoder, element),
         :ok <- expect_tag(element_data, tag_class, constructed, number),
         :ok <- check_child_depth(element_data.depth, decoder_data.limits) do
      {:ok, _framing} = DerTlv.new_cursor(element_data.value, decoder_data.limits.der)
      progress = :atomics.new(3, signed: false)

      {:ok,
       sealed_fun(:cursor, %{
         owner: decoder_data.owner,
         budget: decoder_data.budget,
         input: element_data.value,
         limits: decoder_data.limits.der,
         progress: progress,
         depth: element_data.depth + 1
       })}
    end
  end

  defp primitive_value(element, tag_class, number) do
    with {:ok, value, _offset} <- primitive(element, tag_class, number), do: {:ok, value}
  end

  defp primitive(element, tag_class, number) do
    data = element_data!(element)

    case expect_tag(data, tag_class, false, number) do
      :ok -> {:ok, data.value, data.value_offset}
      {:error, failure} -> {:error, failure}
    end
  end

  defp validate_integer(<<>>, offset), do: {:error, error("empty-integer", offset)}

  defp validate_integer(<<first, second, _::binary>>, offset)
       when (first == 0 and (second &&& 0x80) == 0) or
              (first == 0xFF and (second &&& 0x80) != 0),
       do: {:error, error("non-minimal-integer", offset)}

  defp validate_integer(_value, _offset), do: :ok

  defp decode_ia5(value, offset) do
    case Enum.find_index(:binary.bin_to_list(value), &(&1 > 0x7F)) do
      nil -> {:ok, value}
      index -> {:error, error("non-ascii-ia5-string", offset + index)}
    end
  end

  defp decode_oid(<<>>, offset, _max_arcs),
    do: {:error, error("empty-object-identifier", offset)}

  defp decode_oid(value, offset, max_arcs) do
    with {:ok, first, index} <- oid_subidentifier(value, 0, offset, @u64_max + 80) do
      arcs =
        cond do
          first < 40 -> [0, first]
          first < 80 -> [1, first - 40]
          true -> [2, first - 80]
        end

      if length(arcs) > max_arcs,
        do: {:error, error("oid-arc-limit-exceeded", offset)},
        else: decode_oid_tail(value, index, offset, max_arcs, 2, Enum.reverse(arcs))
    end
  end

  defp decode_oid_tail(value, index, _offset, _max_arcs, _count, reversed)
       when index == byte_size(value),
       do: {:ok, sealed_fun(:object_identifier, %{encoded: value, arcs: Enum.reverse(reversed)})}

  defp decode_oid_tail(value, index, offset, max_arcs, count, reversed) do
    with {:ok, arc, next} <- oid_subidentifier(value, index, offset, @u64_max) do
      if count >= max_arcs,
        do: {:error, error("oid-arc-limit-exceeded", offset + index)},
        else: decode_oid_tail(value, next, offset, max_arcs, count + 1, [arc | reversed])
    end
  end

  defp oid_subidentifier(value, start, offset, maximum) do
    if :binary.at(value, start) == 0x80,
      do: {:error, error("non-minimal-object-identifier", offset + start)},
      else: oid_loop(value, start, offset, 0, maximum)
  end

  defp oid_loop(value, index, offset, _number, _maximum) when index >= byte_size(value),
    do: {:error, error("unterminated-object-identifier", offset + index)}

  defp oid_loop(value, index, offset, number, maximum) do
    octet = :binary.at(value, index)
    payload = octet &&& 0x7F

    if number > div(maximum - payload, 128) do
      {:error, error("object-identifier-overflow", offset + index)}
    else
      next_number = number * 128 + payload

      if (octet &&& 0x80) == 0,
        do: {:ok, next_number, index + 1},
        else: oid_loop(value, index + 1, offset, next_number, maximum)
    end
  end

  defp owned(decoder, element) do
    decoder_data = decoder_data!(decoder)
    element_data = element_data!(element)

    if decoder_data.owner == element_data.owner and decoder_data.budget == element_data.budget,
      do: {:ok, decoder_data, element_data},
      else: {:error, %ArgumentError{message: "element belongs to another decoder"}}
  end

  defp expect_tag(data, tag_class, constructed, number) do
    tag = data.tag

    if tag.class == tag_class and tag.constructed == constructed and tag.number == number,
      do: :ok,
      else: {:error, error("unexpected-tag", 0)}
  end

  defp check_child_depth(depth, limits) do
    if depth + 1 >= limits.max_depth,
      do: {:error, error("depth-limit-exceeded", 0)},
      else: :ok
  end

  defp reserve_budget(data, offset) do
    current = :atomics.get(data.budget, 1)

    cond do
      current >= data.limits.max_total_elements ->
        {:error, error("element-limit-exceeded", offset)}

      :atomics.compare_exchange(data.budget, 1, current, current + 1) == :ok ->
        :ok

      true ->
        reserve_budget(data, offset)
    end
  end

  defp release_budget(data), do: :atomics.add(data.budget, 1, -1)

  defp validate_tag_number(value)
       when is_integer(value) and value >= 0 and value <= 0xFFFF_FFFF,
       do: {:ok, value}

  defp validate_tag_number(_),
    do: {:error, %ArgumentError{message: "schema tag number must fit u32"}}

  defp normalize_limits(limits) when is_map(limits) do
    allowed = MapSet.new([:der, :max_depth, :max_total_elements, :max_oid_arcs])

    if MapSet.subset?(Map.keys(limits) |> MapSet.new(), allowed),
      do: normalize_limits_with_der(limits, Map.get(limits, :der, %{})),
      else: {:error, %ArgumentError{message: "unknown ASN.1 limit"}}
  end

  defp normalize_limits(_), do: {:error, %ArgumentError{message: "limits must be a map"}}

  defp normalize_limits_with_der(limits, der) when is_map(der) do
    der_allowed = MapSet.new([:max_input_len, :max_value_len, :max_elements, :max_tag_number])

    if MapSet.subset?(Map.keys(der) |> MapSet.new(), der_allowed) do
      configured =
        %{@default_limits | der: Map.merge(@default_limits.der, der)}
        |> Map.merge(Map.delete(limits, :der))

      values = [configured.max_depth, configured.max_total_elements, configured.max_oid_arcs]
      der_values = Map.values(configured.der)

      if Enum.all?(values ++ der_values, &(is_integer(&1) and &1 >= 0 and &1 <= @host_max)) and
           configured.der.max_tag_number <= 0xFFFF_FFFF,
         do: {:ok, configured},
         else:
           {:error, %ArgumentError{message: "limits must be finite non-negative host integers"}}
    else
      {:error, %ArgumentError{message: "unknown DER limit"}}
    end
  end

  defp normalize_limits_with_der(_limits, _der),
    do: {:error, %ArgumentError{message: "DER limits must be a map"}}

  defp wrap_element(framed, depth, owner, budget) do
    header = :binary.copy(DerTlv.header(framed))
    value = :binary.copy(DerTlv.value(framed))
    encoded = :binary.copy(DerTlv.encoded(framed))

    sealed_fun(:element, %{
      owner: owner,
      budget: budget,
      tag: framed.tag,
      header: header,
      value: value,
      encoded: encoded,
      value_offset: byte_size(header),
      depth: depth
    })
  end

  defp with_cursor_lock(data, operation) do
    case :atomics.compare_exchange(data.progress, 3, 0, 1) do
      :ok ->
        try do
          operation.()
        after
          :atomics.put(data.progress, 3, 0)
        end

      _ ->
        :erlang.yield()
        with_cursor_lock(data, operation)
    end
  end

  defp data_prefix(cursor) do
    data = cursor_data!(cursor)
    :atomics.get(data.progress, 1)
  end

  defp seal_signature(kind, data) do
    secret = :persistent_term.get({__MODULE__, :seal_secret})

    {
      :erlang.phash2({secret, 1, kind, data}, 4_294_967_296),
      :erlang.phash2({data, kind, 2, secret}, 4_294_967_296),
      :erlang.phash2({kind, secret, data, 3}, 4_294_967_296),
      :erlang.phash2({4, data, secret, kind}, 4_294_967_296)
    }
  end

  defp sealed_fun(kind, data) do
    signature = seal_signature(kind, data)
    fn :__der_asn1_sealed__ -> {kind, data, signature} end
  end

  defp decoder_data!(value), do: sealed_data!(value, :decoder, "decoder")
  defp element_data!(value), do: sealed_data!(value, :element, "element")
  defp cursor_data!(value), do: sealed_data!(value, :cursor, "cursor")
  defp typed_data!(value, kind), do: sealed_data!(value, kind, Atom.to_string(kind))

  defp sealed_data!(value, kind, name) when is_function(value, 1) do
    sample = sealed_fun(:sample, %{})
    fields = [:module, :new_uniq, :new_index]

    if Enum.all?(fields, &(:erlang.fun_info(value, &1) == :erlang.fun_info(sample, &1))) do
      case value.(:__der_asn1_sealed__) do
        {^kind, data, signature} ->
          if signature == seal_signature(kind, data),
            do: data,
            else: raise(ArgumentError, "expected validated #{name}")

        _ ->
          raise ArgumentError, "expected validated #{name}"
      end
    else
      raise ArgumentError, "expected validated #{name}"
    end
  end

  defp sealed_data!(_value, _kind, name),
    do: raise(ArgumentError, "expected validated #{name}")

  defp framing(%DerTlv.Error{kind: kind, offset: offset}), do: error("framing", offset, kind)

  defp error(kind, offset, framing_kind \\ nil),
    do: %Error{kind: kind, offset: offset, framing_kind: framing_kind}
end
