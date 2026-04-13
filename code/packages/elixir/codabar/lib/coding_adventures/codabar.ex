defmodule CodingAdventures.Codabar do
  @moduledoc """
  Dependency-free Codabar encoder that emits backend-neutral paint scenes.
  """

  alias CodingAdventures.BarcodeLayout1D

  @default_layout_config %{
    module_unit: 4,
    bar_height: 120,
    quiet_zone_modules: 10
  }

  @guards MapSet.new(~w[A B C D])

  @patterns %{
    "0" => "101010011",
    "1" => "101011001",
    "2" => "101001011",
    "3" => "110010101",
    "4" => "101101001",
    "5" => "110101001",
    "6" => "100101011",
    "7" => "100101101",
    "8" => "100110101",
    "9" => "110100101",
    "-" => "101001101",
    "$" => "101100101",
    ":" => "1101011011",
    "/" => "1101101011",
    "." => "1101101101",
    "+" => "1011011011",
    "A" => "1011001001",
    "B" => "1001001011",
    "C" => "1010010011",
    "D" => "1010011001"
  }

  def default_layout_config, do: @default_layout_config
  def default_render_config, do: @default_layout_config

  def normalize_codabar(data, opts \\ []) do
    normalized = String.upcase(data)

    cond do
      String.length(normalized) >= 2 and guard?(String.first(normalized)) and guard?(String.last(normalized)) ->
        assert_body_chars!(String.slice(normalized, 1, String.length(normalized) - 2))
        normalized

      true ->
        start = Keyword.get(opts, :start, "A")
        stop = Keyword.get(opts, :stop, "A")

        unless guard?(start) and guard?(stop) do
          raise ArgumentError, "Codabar guards must be one of A, B, C, or D"
        end

        assert_body_chars!(normalized)
        start <> normalized <> stop
    end
  end

  def encode_codabar(data, opts \\ []) do
    normalized = normalize_codabar(data, opts)

    normalized
    |> String.graphemes()
    |> Enum.with_index()
    |> Enum.map(fn {char, index} ->
      role =
        cond do
          index == 0 -> "start"
          index == String.length(normalized) - 1 -> "stop"
          true -> "data"
        end

      %{
        char: char,
        pattern: Map.fetch!(@patterns, char),
        source_index: index,
        role: role
      }
    end)
  end

  def expand_codabar_runs(data, opts \\ []) do
    encoded = encode_codabar(data, opts)

    encoded
    |> Enum.with_index()
    |> Enum.flat_map(fn {symbol, index} ->
      symbol_runs =
        BarcodeLayout1D.runs_from_binary_pattern(
          symbol.pattern,
          source_char: symbol.char,
          source_index: symbol.source_index
        )
        |> retag_runs(symbol.role)

      if index < length(encoded) - 1 do
        symbol_runs ++
          [
            %{
              color: "space",
              modules: 1,
              source_char: symbol.char,
              source_index: symbol.source_index,
              role: "inter-character-gap",
              metadata: %{}
            }
          ]
      else
        symbol_runs
      end
    end)
  end

  def layout_codabar(data, config \\ @default_layout_config, opts \\ []) do
    normalized = normalize_codabar(data, opts)

    BarcodeLayout1D.layout_barcode_1d(
      expand_codabar_runs(normalized),
      config,
      %{
        fill: "#000000",
        background: "#ffffff",
        metadata: %{
          symbology: "codabar",
          start: String.first(normalized),
          stop: String.last(normalized)
        }
      }
    )
  end

  def draw_codabar(data, config \\ @default_layout_config, opts \\ []),
    do: layout_codabar(data, config, opts)

  defp guard?(char), do: MapSet.member?(@guards, char)

  defp assert_body_chars!(body) do
    Enum.each(String.graphemes(body), fn char ->
      if not Map.has_key?(@patterns, char) or guard?(char) do
        raise ArgumentError, ~s(Invalid Codabar body character "#{char}")
      end
    end)
  end

  defp retag_runs(runs, role) do
    Enum.map(runs, fn run -> %{run | role: role, metadata: Map.new(Map.get(run, :metadata, %{}))} end)
  end
end
