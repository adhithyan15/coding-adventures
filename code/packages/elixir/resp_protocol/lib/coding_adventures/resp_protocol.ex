defmodule CodingAdventures.RESPProtocol do
  @moduledoc """
  RESP2 codec for simple in-memory servers.
  """

  def simple_string(value), do: {:simple_string, to_string(value)}
  def error(value), do: {:error, to_string(value)}
  def integer(value) when is_integer(value), do: {:integer, value}
  def bulk_string(value) when is_binary(value), do: {:bulk_string, value}
  def bulk_string(value), do: {:bulk_string, to_string(value)}
  def null_bulk_string(), do: :null_bulk_string
  def array(values) when is_list(values), do: {:array, values}
  def null_array(), do: :null_array

  def encode(value), do: value |> encode_iodata() |> IO.iodata_to_binary()

  def decode(binary) when is_binary(binary) do
    case decode_value(binary) do
      {:ok, value, rest} -> {:ok, value, rest}
      {:error, reason} -> {:error, reason}
    end
  end

  defp encode_iodata({:simple_string, value}), do: ["+", value, "\r\n"]
  defp encode_iodata({:error, value}), do: ["-", value, "\r\n"]
  defp encode_iodata({:integer, value}), do: [":", Integer.to_string(value), "\r\n"]
  defp encode_iodata({:bulk_string, value}), do: ["$", Integer.to_string(byte_size(value)), "\r\n", value, "\r\n"]
  defp encode_iodata(:null_bulk_string), do: "$-1\r\n"
  defp encode_iodata({:array, values}), do: ["*", Integer.to_string(length(values)), "\r\n", Enum.map(values, &encode_iodata/1)]
  defp encode_iodata(:null_array), do: "*-1\r\n"
  defp encode_iodata(other), do: raise(ArgumentError, "cannot encode #{inspect(other)} as RESP")

  defp decode_value(<<"+", rest::binary>>), do: decode_line(rest, &{:simple_string, &1})
  defp decode_value(<<"-", rest::binary>>), do: decode_line(rest, &{:error, &1})
  defp decode_value(<<":", rest::binary>>) do
    decode_line(rest, fn line ->
      {value, ""} = Integer.parse(line)
      {:integer, value}
    end)
  end
  defp decode_value(<<"$", rest::binary>>) do
    case decode_line(rest, & &1) do
      {:ok, len_str, remaining} ->
        {len, ""} = Integer.parse(len_str)
        decode_bulk(len, remaining)

      other ->
        other
    end
  end
  defp decode_value(<<"*", rest::binary>>) do
    case decode_line(rest, & &1) do
      {:ok, count_str, remaining} ->
        {count, ""} = Integer.parse(count_str)
        decode_array(count, remaining, [])

      other ->
        other
    end
  end
  defp decode_value(_), do: {:error, :incomplete}

  defp decode_line(binary, wrap) do
    case :binary.match(binary, "\r\n") do
      {index, 2} ->
        <<line::binary-size(index), "\r\n", rest::binary>> = binary
        {:ok, wrap.(line), rest}

      :nomatch ->
        {:error, :incomplete}
    end
  end

  defp decode_bulk(-1, rest), do: {:ok, :null_bulk_string, rest}

  defp decode_bulk(len, rest) when len >= 0 do
    if byte_size(rest) < len + 2 do
      {:error, :incomplete}
    else
      <<payload::binary-size(len), "\r\n", remaining::binary>> = rest
      {:ok, {:bulk_string, payload}, remaining}
    end
  end

  defp decode_array(-1, rest, _acc), do: {:ok, :null_array, rest}
  defp decode_array(0, rest, acc), do: {:ok, {:array, Enum.reverse(acc)}, rest}

  defp decode_array(count, rest, acc) when count > 0 do
    case decode_value(rest) do
      {:ok, value, remaining} -> decode_array(count - 1, remaining, [value | acc])
      {:error, reason} -> {:error, reason}
    end
  end
end
