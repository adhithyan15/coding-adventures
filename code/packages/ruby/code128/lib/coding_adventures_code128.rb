# frozen_string_literal: true

require "coding_adventures_barcode_layout_1d"
require_relative "coding_adventures/code128/version"

module CodingAdventures
  module Code128
    module_function

    DEFAULT_LAYOUT_CONFIG = CodingAdventures::BarcodeLayout1D::DEFAULT_BARCODE_1D_LAYOUT_CONFIG
    DEFAULT_RENDER_CONFIG = DEFAULT_LAYOUT_CONFIG

    START_B = 104
    STOP = 106

    PATTERNS = %w[
      11011001100 11001101100 11001100110 10010011000 10010001100 10001001100 10011001000 10011000100
      10001100100 11001001000 11001000100 11000100100 10110011100 10011011100 10011001110 10111001100
      10011101100 10011100110 11001110010 11001011100 11001001110 11011100100 11001110100 11101101110
      11101001100 11100101100 11100100110 11101100100 11100110100 11100110010 11011011000 11011000110
      11000110110 10100011000 10001011000 10001000110 10110001000 10001101000 10001100010 11010001000
      11000101000 11000100010 10110111000 10110001110 10001101110 10111011000 10111000110 10001110110
      11101110110 11010001110 11000101110 11011101000 11011100010 11011101110 11101011000 11101000110
      11100010110 11101101000 11101100010 11100011010 11101111010 11001000010 11110001010 10100110000
      10100001100 10010110000 10010000110 10000101100 10000100110 10110010000 10110000100 10011010000
      10011000010 10000110100 10000110010 11000010010 11001010000 11110111010 11000010100 10001111010
      10100111100 10010111100 10010011110 10111100100 10011110100 10011110010 11110100100 11110010100
      11110010010 11011011110 11011110110 11110110110 10101111000 10100011110 10001011110 10111101000
      10111100010 11110101000 11110100010 10111011110 10111101110 11101011110 11110101110 11010000100
      11010010000 11010011100 1100011101011
    ].freeze

    def normalize_code128_b(data)
      data.each_char do |char|
        code = char.ord
        next if code >= 32 && code <= 126

        raise ArgumentError, "Code 128 Code Set B supports printable ASCII characters only"
      end
      data
    end

    def value_for_code128_b_char(char)
      char.ord - 32
    end

    def compute_code128_checksum(values)
      (START_B + values.each_with_index.sum { |value, index| value * (index + 1) }) % 103
    end

    def encode_code128_b(data)
      normalized = normalize_code128_b(data)
      data_symbols = normalized.chars.each_with_index.map do |char, index|
        value = value_for_code128_b_char(char)
        {
          label: char,
          value: value,
          pattern: PATTERNS.fetch(value),
          source_index: index,
          role: "data",
        }
      end
      checksum = compute_code128_checksum(data_symbols.map { |symbol| symbol[:value] })

      [
        { label: "Start B", value: START_B, pattern: PATTERNS.fetch(START_B), source_index: -1, role: "start" },
        *data_symbols,
        {
          label: "Checksum #{checksum}",
          value: checksum,
          pattern: PATTERNS.fetch(checksum),
          source_index: normalized.length,
          role: "check",
        },
        { label: "Stop", value: STOP, pattern: PATTERNS.fetch(STOP), source_index: normalized.length + 1, role: "stop" },
      ]
    end

    def expand_code128_runs(data)
      encode_code128_b(data).flat_map do |symbol|
        retag_runs(
          CodingAdventures::BarcodeLayout1D.runs_from_binary_pattern(
            symbol[:pattern],
            source_char: symbol[:label],
            source_index: symbol[:source_index],
          ),
          symbol[:role],
        )
      end
    end

    def layout_code128(data, config = DEFAULT_LAYOUT_CONFIG)
      normalized = normalize_code128_b(data)
      checksum = encode_code128_b(normalized)[-2][:value]
      CodingAdventures::BarcodeLayout1D.draw_one_dimensional_barcode(
        expand_code128_runs(normalized),
        config,
        {
          metadata: {
            symbology: "code128",
            code_set: "B",
            checksum: checksum,
          },
        },
      )
    end

    def draw_code128(data, config = DEFAULT_LAYOUT_CONFIG)
      layout_code128(data, config)
    end

    def retag_runs(runs, role)
      runs.map do |run|
        run.merge(role: role, metadata: run.fetch(:metadata, {}).dup)
      end
    end
    private_class_method :retag_runs
  end
end
