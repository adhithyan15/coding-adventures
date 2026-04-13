# frozen_string_literal: true

require "coding_adventures_barcode_layout_1d"
require_relative "coding_adventures/code39/version"

module CodingAdventures
  # Code 39 encoder that stops at a backend-neutral paint scene.
  module Code39
    module_function

    DEFAULT_LAYOUT_CONFIG = CodingAdventures::BarcodeLayout1D::DEFAULT_BARCODE_1D_LAYOUT_CONFIG
    DEFAULT_RENDER_CONFIG = DEFAULT_LAYOUT_CONFIG

    PATTERNS = {
      "0" => "bwbWBwBwb", "1" => "BwbWbwbwB", "2" => "bwBWbwbwB", "3" => "BwBWbwbwb",
      "4" => "bwbWBwbwB", "5" => "BwbWBwbwb", "6" => "bwBWBwbwb", "7" => "bwbWbwBwB",
      "8" => "BwbWbwBwb", "9" => "bwBWbwBwb", "A" => "BwbwbWbwB", "B" => "bwBwbWbwB",
      "C" => "BwBwbWbwb", "D" => "bwbwBWbwB", "E" => "BwbwBWbwb", "F" => "bwBwBWbwb",
      "G" => "bwbwbWBwB", "H" => "BwbwbWBwb", "I" => "bwBwbWBwb", "J" => "bwbwBWBwb",
      "K" => "BwbwbwbWB", "L" => "bwBwbwbWB", "M" => "BwBwbwbWb", "N" => "bwbwBwbWB",
      "O" => "BwbwBwbWb", "P" => "bwBwBwbWb", "Q" => "bwbwbwBWB", "R" => "BwbwbwBWb",
      "S" => "bwBwbwBWb", "T" => "bwbwBwBWb", "U" => "BWbwbwbwB", "V" => "bWBwbwbwB",
      "W" => "BWBwbwbwb", "X" => "bWbwBwbwB", "Y" => "BWbwBwbwb", "Z" => "bWBwBwbwb",
      "-" => "bWbwbwBwB", "." => "BWbwbwBwb", " " => "bWBwbwBwb", "$" => "bWbWbWbwb",
      "/" => "bWbWbwbWb", "+" => "bWbwbWbWb", "%" => "bwbWbWbWb", "*" => "bWbwBwBwb",
    }.freeze

    BAR_SPACE_COLORS = %w[bar space bar space bar space bar space bar].freeze

    def normalize_code39(data)
      normalized = data.upcase
      normalized.each_char do |char|
        raise ArgumentError, 'input must not contain "*" because it is reserved for start/stop' if char == "*"
        raise ArgumentError, %(invalid character: "#{char}" is not supported by Code 39) unless PATTERNS.key?(char)
      end
      normalized
    end

    def encode_code39_char(char)
      pattern = PATTERNS.fetch(char)
      {
        char: char,
        is_start_stop: char == "*",
        pattern: pattern.chars.map { |part| part == part.upcase ? "W" : "N" }.join,
      }
    end

    def encode_code39(data)
      normalized = normalize_code39(data)
      ("*" + normalized + "*").chars.map { |char| encode_code39_char(char) }
    end

    def expand_code39_runs(data)
      encoded = encode_code39(data)
      encoded.each_with_index.flat_map do |encoded_char, source_index|
        runs = CodingAdventures::BarcodeLayout1D.runs_from_width_pattern(
          encoded_char[:pattern],
          BAR_SPACE_COLORS,
          source_char: encoded_char[:char],
          source_index: source_index,
        )
        if source_index < encoded.length - 1
          runs << {
            color: "space",
            modules: 1,
            source_char: encoded_char[:char],
            source_index: source_index,
            role: "inter-character-gap",
            metadata: {},
          }
        end
        runs
      end
    end

    def layout_code39(data, config = DEFAULT_LAYOUT_CONFIG)
      normalized = normalize_code39(data)
      CodingAdventures::BarcodeLayout1D.layout_barcode_1d(
        expand_code39_runs(normalized),
        config,
        {
          fill: "#000000",
          background: "#ffffff",
          metadata: { symbology: "code39", data: normalized },
        },
      )
    end

    def draw_code39(data, config = DEFAULT_LAYOUT_CONFIG)
      layout_code39(data, config)
    end
  end
end
