defmodule CodingAdventures.DerTlv do
  import Bitwise

  @moduledoc """
  Bounded, payload-blind DER tag-length-value framing.

  The decoder returns sub-binaries of the caller's input and deliberately does
  not interpret ASN.1 value semantics or recurse through constructed values.
  """

  @default_limits %{
    max_input_len: 1_048_576,
    max_value_len: 1_048_576,
    max_elements: 4_096,
    max_tag_number: 0xFFFF_FFFF
  }
  @host_max 0x7FFF_FFFF_FFFF_FFFF

  defmodule Error do
    @moduledoc "A stable payload-blind DER framing error."
    defexception [:kind, :offset]

    @impl true
    def message(%__MODULE__{kind: kind, offset: offset}) do
      "DER framing error #{kind} at byte #{offset}"
    end
  end

  defmodule Tag do
    @moduledoc "A decoded DER identifier."
    @enforce_keys [:class, :constructed, :number]
    defstruct [:class, :constructed, :number]
  end

  defmodule Element do
    @moduledoc "A DER frame backed by sub-binaries of the supplied input."
    @enforce_keys [:tag, :input, :start, :header_len, :encoded_len]
    defstruct [:tag, :input, :start, :header_len, :encoded_len]
  end

  defmodule Cursor do
    @moduledoc "Immutable iterative sibling decoder."
    @enforce_keys [:input, :limits]
    defstruct [:input, :limits, offset: 0, elements_read: 0]
  end

  def default_limits, do: @default_limits

  def header(%Element{} = element) do
    binary_part(element.input, element.start, element.header_len)
  end

  def value(%Element{} = element) do
    binary_part(
      element.input,
      element.start + element.header_len,
      element.encoded_len - element.header_len
    )
  end

  def encoded(%Element{} = element) do
    binary_part(element.input, element.start, element.encoded_len)
  end

  def decode_one(input, limits \\ @default_limits) when is_binary(input) do
    with {:ok, normalized} <- normalize_limits(limits),
         {:ok, element, next_offset} <- decode_at(input, 0, byte_size(input), normalized) do
      {:ok, element, binary_part(input, next_offset, byte_size(input) - next_offset)}
    end
  end

  def decode_exact(input, limits \\ @default_limits) when is_binary(input) do
    with {:ok, normalized} <- normalize_limits(limits),
         {:ok, element, next_offset} <- decode_at(input, 0, byte_size(input), normalized) do
      if next_offset == byte_size(input) do
        {:ok, element}
      else
        {:error, error("trailing-data", next_offset)}
      end
    end
  end

  def new_cursor(input, limits \\ @default_limits) when is_binary(input) do
    with {:ok, normalized} <- normalize_limits(limits) do
      if byte_size(input) > normalized.max_input_len do
        {:error, error("input-limit-exceeded", 0)}
      else
        {:ok, %Cursor{input: input, limits: normalized}}
      end
    end
  end

  def read(%Cursor{} = cursor) do
    cond do
      cursor.offset == byte_size(cursor.input) ->
        {:end, cursor}

      cursor.elements_read >= cursor.limits.max_elements ->
        {:error, error("element-limit-exceeded", cursor.offset), cursor}

      true ->
        case decode_at(
               cursor.input,
               cursor.offset,
               byte_size(cursor.input) - cursor.offset,
               cursor.limits
             ) do
          {:ok, element, next_offset} ->
            {:ok, element,
             %{cursor | offset: next_offset, elements_read: cursor.elements_read + 1}}

          {:error, failure} ->
            {:error, failure, cursor}
        end
    end
  end

  def finish(%Cursor{} = cursor) do
    if cursor.offset == byte_size(cursor.input) do
      :ok
    else
      {:error, error("trailing-data", cursor.offset)}
    end
  end

  def remaining(%Cursor{} = cursor) do
    binary_part(cursor.input, cursor.offset, byte_size(cursor.input) - cursor.offset)
  end

  defp normalize_limits(limits) when is_map(limits) do
    normalized = Map.merge(@default_limits, limits)

    if Enum.all?(
         [:max_input_len, :max_value_len, :max_elements, :max_tag_number],
         &(is_integer(normalized[&1]) and normalized[&1] >= 0)
       ) and normalized.max_tag_number <= 0xFFFF_FFFF do
      {:ok, normalized}
    else
      {:error,
       %ArgumentError{message: "DER limits must be non-negative and tag limit must fit u32"}}
    end
  end

  defp decode_at(input, start, available, limits) do
    cond do
      available > limits.max_input_len ->
        {:error, error("input-limit-exceeded", start)}

      available == 0 ->
        {:error, error("empty-input", start)}

      true ->
        decode_identifier(input, start, available, limits)
    end
  end

  defp decode_identifier(input, start, available, limits) do
    first = :binary.at(input, start)
    class = Enum.at(["universal", "application", "context-specific", "private"], first >>> 6)
    constructed = (first &&& 0x20) != 0
    low = first &&& 0x1F

    tag_result =
      if low != 0x1F do
        if low > limits.max_tag_number do
          {:error, error("tag-limit-exceeded", start)}
        else
          {:ok, low, 1}
        end
      else
        decode_high_tag(input, start, available, limits, 1, 0)
      end

    with {:ok, number, identifier_len} <- tag_result do
      if class == "universal" and number == 0 do
        {:error, error("end-of-contents", start)}
      else
        decode_length_and_value(
          input,
          start,
          available,
          limits,
          class,
          constructed,
          number,
          identifier_len
        )
      end
    end
  end

  defp decode_high_tag(input, start, available, limits, index, number) do
    if index >= available do
      {:error, error("truncated-high-tag", start + index)}
    else
      octet = :binary.at(input, start + index)
      payload = octet &&& 0x7F

      cond do
        index == 1 and payload == 0 ->
          {:error, error("non-minimal-tag", start + index)}

        number * 128 + payload > 0xFFFF_FFFF ->
          {:error, error("tag-overflow", start + index)}

        number * 128 + payload > limits.max_tag_number ->
          {:error, error("tag-limit-exceeded", start + index)}

        (octet &&& 0x80) == 0 and number * 128 + payload < 31 ->
          {:error, error("non-minimal-tag", start)}

        (octet &&& 0x80) == 0 ->
          {:ok, number * 128 + payload, index + 1}

        true ->
          decode_high_tag(
            input,
            start,
            available,
            limits,
            index + 1,
            number * 128 + payload
          )
      end
    end
  end

  defp decode_length_and_value(
         input,
         start,
         available,
         limits,
         class,
         constructed,
         number,
         identifier_len
       ) do
    length_offset = start + identifier_len

    if identifier_len >= available do
      {:error, error("truncated-length", length_offset)}
    else
      case decode_length(input, start, available, identifier_len) do
        {:ok, value_len, length_len} ->
          cond do
            value_len > @host_max ->
              {:error, error("length-host-overflow", length_offset)}

            value_len > limits.max_value_len ->
              {:error, error("value-limit-exceeded", length_offset)}

            value_len > @host_max - identifier_len - length_len ->
              {:error, error("length-host-overflow", length_offset)}

            identifier_len + length_len + value_len > available ->
              {:error, error("truncated-value", start + available)}

            true ->
              header_len = identifier_len + length_len
              encoded_len = header_len + value_len

              {:ok,
               %Element{
                 tag: %Tag{class: class, constructed: constructed, number: number},
                 input: input,
                 start: start,
                 header_len: header_len,
                 encoded_len: encoded_len
               }, start + encoded_len}
          end

        {:error, failure} ->
          {:error, failure}
      end
    end
  end

  defp decode_length(input, start, available, identifier_len) do
    length_offset = start + identifier_len
    first = :binary.at(input, length_offset)

    cond do
      first < 0x80 ->
        {:ok, first, 1}

      first == 0x80 ->
        {:error, error("indefinite-length", length_offset)}

      first == 0xFF ->
        {:error, error("reserved-length", length_offset)}

      (first &&& 0x7F) > 8 ->
        {:error, error("length-too-wide", length_offset)}

      identifier_len + 1 + (first &&& 0x7F) > available ->
        {:error, error("truncated-length", start + available)}

      :binary.at(input, start + identifier_len + 1) == 0 ->
        {:error, error("non-minimal-length", start + identifier_len + 1)}

      true ->
        count = first &&& 0x7F

        value =
          Enum.reduce(0..(count - 1), 0, fn index, accumulator ->
            accumulator * 256 + :binary.at(input, start + identifier_len + 1 + index)
          end)

        if value < 128,
          do: {:error, error("non-minimal-length", length_offset)},
          else: {:ok, value, count + 1}
    end
  end

  defp error(kind, offset), do: %Error{kind: kind, offset: offset}
end
