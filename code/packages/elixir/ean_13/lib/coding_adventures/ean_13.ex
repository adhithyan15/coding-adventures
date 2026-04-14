defmodule CodingAdventures.Ean13 do
  @moduledoc """
  Dependency-free EAN-13 encoder that emits backend-neutral paint scenes.
  """

  alias CodingAdventures.BarcodeLayout1D

  @default_layout_config %{
    module_unit: 4,
    bar_height: 120,
    quiet_zone_modules: 10
  }

  @side_guard "101"
  @center_guard "01010"

  @digit_patterns %{
    "L" => ~w[0001101 0011001 0010011 0111101 0100011 0110001 0101111 0111011 0110111 0001011],
    "G" => ~w[0100111 0110011 0011011 0100001 0011101 0111001 0000101 0010001 0001001 0010111],
    "R" => ~w[1110010 1100110 1101100 1000010 1011100 1001110 1010000 1000100 1001000 1110100]
  }

  @left_parity_patterns ~w[
    LLLLLL LLGLGG LLGGLG LLGGGL LGLLGG LGGLLG LGGGLL LGLGLG LGLGGL LGGLGL
  ]

  def default_layout_config, do: @default_layout_config
  def default_render_config, do: @default_layout_config

  def compute_ean_13_check_digit(payload12) do
    assert_digits!(payload12, [12], "EAN-13 input must contain 12 digits or 13 digits")

    total =
      payload12
      |> String.graphemes()
      |> Enum.reverse()
      |> Enum.with_index()
      |> Enum.map(fn {digit, index} ->
        String.to_integer(digit) * if(rem(index, 2) == 0, do: 3, else: 1)
      end)
      |> Enum.sum()

    Integer.to_string(rem(10 - rem(total, 10), 10))
  end

  def normalize_ean_13(data) do
    assert_digits!(data, [12, 13], "EAN-13 input must contain 12 digits or 13 digits")

    if String.length(data) == 12 do
      data <> compute_ean_13_check_digit(data)
    else
      expected = compute_ean_13_check_digit(String.slice(data, 0, 12))
      actual = String.at(data, 12)

      if expected == actual do
        data
      else
        raise ArgumentError, "Invalid EAN-13 check digit: expected #{expected} but received #{actual}"
      end
    end
  end

  def left_parity_pattern(data) do
    normalized = normalize_ean_13(data)
    Enum.at(@left_parity_patterns, String.at(normalized, 0) |> String.to_integer())
  end

  def encode_ean_13(data) do
    normalized = normalize_ean_13(data)
    parity = left_parity_pattern(normalized)
    digits = String.graphemes(normalized)

    left_digits =
      digits
      |> Enum.slice(1, 6)
      |> Enum.with_index()
      |> Enum.map(fn {digit, offset} ->
        encoding = String.at(parity, offset)

        %{
          digit: digit,
          encoding: encoding,
          pattern: @digit_patterns |> Map.fetch!(encoding) |> Enum.at(String.to_integer(digit)),
          source_index: offset + 1,
          role: "data"
        }
      end)

    right_digits =
      digits
      |> Enum.slice(7, 6)
      |> Enum.with_index()
      |> Enum.map(fn {digit, offset} ->
        %{
          digit: digit,
          encoding: "R",
          pattern: @digit_patterns |> Map.fetch!("R") |> Enum.at(String.to_integer(digit)),
          source_index: offset + 7,
          role: if(offset == 5, do: "check", else: "data")
        }
      end)

    left_digits ++ right_digits
  end

  def expand_ean_13_runs(data) do
    encoded_digits = encode_ean_13(data)

    retag_runs(BarcodeLayout1D.runs_from_binary_pattern(@side_guard, source_char: "start", source_index: -1), "guard") ++
      expand_ean_13_side(Enum.take(encoded_digits, 6)) ++
      retag_runs(BarcodeLayout1D.runs_from_binary_pattern(@center_guard, source_char: "center", source_index: -2), "guard") ++
      expand_ean_13_side(Enum.drop(encoded_digits, 6)) ++
      retag_runs(BarcodeLayout1D.runs_from_binary_pattern(@side_guard, source_char: "end", source_index: -3), "guard")
  end

  def layout_ean_13(data, config \\ @default_layout_config) do
    normalized = normalize_ean_13(data)

    BarcodeLayout1D.layout_barcode_1d(
      expand_ean_13_runs(normalized),
      config,
      %{
        fill: "#000000",
        background: "#ffffff",
        metadata: %{
          symbology: "ean-13",
          leading_digit: String.at(normalized, 0),
          left_parity: left_parity_pattern(normalized),
          content_modules: 95
        }
      }
    )
  end

  def draw_ean_13(data, config \\ @default_layout_config), do: layout_ean_13(data, config)

  defp expand_ean_13_side(entries) do
    Enum.flat_map(entries, fn entry ->
      BarcodeLayout1D.runs_from_binary_pattern(
        entry.pattern,
        source_char: entry.digit,
        source_index: entry.source_index
      )
      |> retag_runs(entry.role)
    end)
  end

  defp assert_digits!(data, lengths, length_message) do
    unless String.match?(data, ~r/^\d+$/) do
      raise ArgumentError, "EAN-13 input must contain digits only"
    end

    unless String.length(data) in lengths do
      raise ArgumentError, length_message
    end
  end

  defp retag_runs(runs, role) do
    Enum.map(runs, fn run -> %{run | role: role, metadata: Map.new(Map.get(run, :metadata, %{}))} end)
  end
end
