defmodule CodingAdventures.HttpCore do
  @moduledoc """
  Shared HTTP message types and helpers.
  """

  defmodule Header do
    @enforce_keys [:name, :value]
    defstruct [:name, :value]
  end

  defmodule HttpVersion do
    @enforce_keys [:major, :minor]
    defstruct [:major, :minor]

    def parse("HTTP/" <> rest) do
      case String.split(rest, ".", parts: 2) do
        [major_text, minor_text] ->
          with {major, ""} <- Integer.parse(major_text),
               {minor, ""} <- Integer.parse(minor_text) do
            {:ok, %__MODULE__{major: major, minor: minor}}
          else
            _ -> {:error, {:invalid_version, "HTTP/" <> rest}}
          end

        _ ->
          {:error, {:invalid_version, "HTTP/" <> rest}}
      end
    end

    def parse(other), do: {:error, {:invalid_version, other}}
  end

  defimpl String.Chars, for: HttpVersion do
    def to_string(version), do: "HTTP/#{version.major}.#{version.minor}"
  end

  defmodule BodyKind do
    @enforce_keys [:mode]
    defstruct [:mode, :length]

    def none, do: %__MODULE__{mode: :none, length: nil}
    def content_length(length), do: %__MODULE__{mode: :content_length, length: length}
    def until_eof, do: %__MODULE__{mode: :until_eof, length: nil}
    def chunked, do: %__MODULE__{mode: :chunked, length: nil}
  end

  defmodule RequestHead do
    @enforce_keys [:method, :target, :version, :headers]
    defstruct [:method, :target, :version, :headers]

    def header(head, name), do: CodingAdventures.HttpCore.find_header(head.headers, name)
    def content_length(head), do: CodingAdventures.HttpCore.parse_content_length(head.headers)
    def content_type(head), do: CodingAdventures.HttpCore.parse_content_type(head.headers)
  end

  defmodule ResponseHead do
    @enforce_keys [:version, :status, :reason, :headers]
    defstruct [:version, :status, :reason, :headers]

    def header(head, name), do: CodingAdventures.HttpCore.find_header(head.headers, name)
    def content_length(head), do: CodingAdventures.HttpCore.parse_content_length(head.headers)
    def content_type(head), do: CodingAdventures.HttpCore.parse_content_type(head.headers)
  end

  @spec find_header([Header.t()], String.t()) :: String.t() | nil
  def find_header(headers, name) do
    lowered = String.downcase(name)

    headers
    |> Enum.find(fn header -> String.downcase(header.name) == lowered end)
    |> case do
      nil -> nil
      header -> header.value
    end
  end

  @spec parse_content_length([Header.t()]) :: non_neg_integer() | nil
  def parse_content_length(headers) do
    case find_header(headers, "Content-Length") do
      nil ->
        nil

      value ->
        case Integer.parse(value) do
          {length, ""} when length >= 0 -> length
          _ -> nil
        end
    end
  end

  @spec parse_content_type([Header.t()]) :: {String.t(), String.t() | nil} | nil
  def parse_content_type(headers) do
    case find_header(headers, "Content-Type") do
      nil ->
        nil

      value ->
        [media_type | parameters] = value |> String.split(";") |> Enum.map(&String.trim/1)

        if media_type == "" do
          nil
        else
          charset =
            Enum.find_value(parameters, fn parameter ->
              case String.split(parameter, "=", parts: 2) do
                [key, raw_value] ->
                  if String.downcase(String.trim(key)) == "charset" do
                    raw_value |> String.trim() |> String.trim("\"")
                  end

                _ ->
                  nil
              end
            end)

          {media_type, charset}
        end
    end
  end
end
