defmodule CodingAdventures.Http1 do
  @moduledoc """
  HTTP/1 request and response head parsing.

  The parser stops at the boundary shared by higher-level clients:
  it extracts the start line and headers, reports the body offset,
  and describes how the payload should be framed.
  """

  alias CodingAdventures.HttpCore
  alias CodingAdventures.HttpCore.BodyKind
  alias CodingAdventures.HttpCore.Header
  alias CodingAdventures.HttpCore.HttpVersion
  alias CodingAdventures.HttpCore.RequestHead
  alias CodingAdventures.HttpCore.ResponseHead

  defmodule ParsedRequestHead do
    @enforce_keys [:head, :body_offset, :body_kind]
    defstruct [:head, :body_offset, :body_kind]
  end

  defmodule ParsedResponseHead do
    @enforce_keys [:head, :body_offset, :body_kind]
    defstruct [:head, :body_offset, :body_kind]
  end

  def parse_request_head(input) when is_binary(input) do
    with {:ok, lines, body_offset} <- split_head_lines(input),
         {:ok, method, target, version} <- parse_request_line(lines),
         {:ok, headers} <- parse_headers(Enum.drop(lines, 1)),
         {:ok, body_kind} <- request_body_kind(headers) do
      {:ok,
       %ParsedRequestHead{
         head: %RequestHead{method: method, target: target, version: version, headers: headers},
         body_offset: body_offset,
         body_kind: body_kind
       }}
    end
  end

  def parse_response_head(input) when is_binary(input) do
    with {:ok, lines, body_offset} <- split_head_lines(input),
         {:ok, version, status, reason} <- parse_status_line(lines),
         {:ok, headers} <- parse_headers(Enum.drop(lines, 1)),
         {:ok, body_kind} <- response_body_kind(status, headers) do
      {:ok,
       %ParsedResponseHead{
         head: %ResponseHead{version: version, status: status, reason: reason, headers: headers},
         body_offset: body_offset,
         body_kind: body_kind
       }}
    end
  end

  defp split_head_lines(input) do
    {trimmed, offset} = trim_leading_blank_lines(input, 0)
    do_split_head_lines(trimmed, offset, [])
  end

  defp trim_leading_blank_lines(<<"\r\n", rest::binary>>, offset),
    do: trim_leading_blank_lines(rest, offset + 2)

  defp trim_leading_blank_lines(<<"\n", rest::binary>>, offset),
    do: trim_leading_blank_lines(rest, offset + 1)

  defp trim_leading_blank_lines(rest, offset), do: {rest, offset}

  defp do_split_head_lines(input, offset, lines) do
    case :binary.match(input, "\n") do
      :nomatch ->
        {:error, :incomplete_head}

      {newline_index, 1} ->
        raw_line = binary_part(input, 0, newline_index)

        line =
          if String.ends_with?(raw_line, "\r") do
            binary_part(raw_line, 0, byte_size(raw_line) - 1)
          else
            raw_line
          end

        remainder = binary_part(input, newline_index + 1, byte_size(input) - newline_index - 1)
        next_offset = offset + newline_index + 1

        if line == "" do
          {:ok, Enum.reverse(lines), next_offset}
        else
          do_split_head_lines(remainder, next_offset, [line | lines])
        end
    end
  end

  defp parse_request_line([line | _]) do
    case String.split(line, ~r/\s+/, trim: true) do
      [method, target, version_text] ->
        with {:ok, version} <- HttpVersion.parse(version_text) do
          {:ok, method, target, version}
        end

      _ ->
        {:error, {:invalid_start_line, line}}
    end
  end

  defp parse_request_line([]), do: {:error, :invalid_start_line}

  defp parse_status_line([line | _]) do
    case String.split(line, ~r/\s+/, trim: true) do
      [version_text, status_text | reason_parts] ->
        with {:ok, version} <- HttpVersion.parse(version_text),
             {status, ""} <- Integer.parse(status_text) do
          {:ok, version, status, Enum.join(reason_parts, " ")}
        else
          :error -> {:error, {:invalid_status, status_text}}
          _ -> {:error, {:invalid_status_line, line}}
        end

      _ ->
        {:error, {:invalid_status_line, line}}
    end
  end

  defp parse_status_line([]), do: {:error, :invalid_status_line}

  defp parse_headers(lines) do
    Enum.reduce_while(lines, {:ok, []}, fn line, {:ok, headers} ->
      case String.split(line, ":", parts: 2) do
        [name, raw_value] ->
          trimmed_name = String.trim(name)

          if trimmed_name == "" do
            {:halt, {:error, {:invalid_header, line}}}
          else
            header = %Header{name: trimmed_name, value: String.trim(raw_value)}
            {:cont, {:ok, headers ++ [header]}}
          end

        _ ->
          {:halt, {:error, {:invalid_header, line}}}
      end
    end)
  end

  defp request_body_kind(headers) do
    cond do
      chunked_transfer_encoding?(headers) ->
        {:ok, BodyKind.chunked()}

      true ->
        with {:ok, length} <- declared_content_length(headers) do
          case length do
            nil -> {:ok, BodyKind.none()}
            0 -> {:ok, BodyKind.none()}
            value -> {:ok, BodyKind.content_length(value)}
          end
        end
    end
  end

  defp response_body_kind(status, headers) do
    cond do
      (status >= 100 and status < 200) or status in [204, 304] ->
        {:ok, BodyKind.none()}

      chunked_transfer_encoding?(headers) ->
        {:ok, BodyKind.chunked()}

      true ->
        with {:ok, length} <- declared_content_length(headers) do
          case length do
            nil -> {:ok, BodyKind.until_eof()}
            0 -> {:ok, BodyKind.none()}
            value -> {:ok, BodyKind.content_length(value)}
          end
        end
    end
  end

  defp declared_content_length(headers) do
    case HttpCore.find_header(headers, "Content-Length") do
      nil ->
        {:ok, nil}

      value ->
        case Integer.parse(value) do
          {length, ""} when length >= 0 -> {:ok, length}
          _ -> {:error, {:invalid_content_length, value}}
        end
    end
  end

  defp chunked_transfer_encoding?(headers) do
    headers
    |> Enum.filter(&(String.downcase(&1.name) == "transfer-encoding"))
    |> Enum.any?(fn header ->
      header.value
      |> String.split(",")
      |> Enum.any?(fn piece -> String.downcase(String.trim(piece)) == "chunked" end)
    end)
  end
end
