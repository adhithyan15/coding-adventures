# frozen_string_literal: true

require "coding_adventures_barcode_layout_1d"
require_relative "coding_adventures/itf/version"

module CodingAdventures
  module Itf
    module_function

    DEFAULT_LAYOUT_CONFIG = CodingAdventures::BarcodeLayout1D::DEFAULT_BARCODE_1D_LAYOUT_CONFIG
    DEFAULT_RENDER_CONFIG = DEFAULT_LAYOUT_CONFIG

    START_PATTERN = "1010"
    STOP_PATTERN = "11101"

    DIGIT_PATTERNS = %w[
      00110 10001 01001 11000 00101 10100 01100 00011 10010 01010
    ].freeze

    def normalize_itf(data)
      raise ArgumentError, "ITF input must contain digits only" unless /\A\d+\z/.match?(data)
      raise ArgumentError, "ITF input must contain an even number of digits" if data.empty? || data.length.odd?

      data
    end

    def encode_itf(data)
      normalized = normalize_itf(data)
      normalized.chars.each_slice(2).with_index.map do |pair_chars, index|
        pair = pair_chars.join
        bar_pattern = DIGIT_PATTERNS.fetch(pair[0].to_i)
        space_pattern = DIGIT_PATTERNS.fetch(pair[1].to_i)
        binary_pattern = bar_pattern.chars.zip(space_pattern.chars).map do |bar_marker, space_marker|
          "#{bar_marker == "1" ? "111" : "1"}#{space_marker == "1" ? "000" : "0"}"
        end.join

        {
          pair: pair,
          bar_pattern: bar_pattern,
          space_pattern: space_pattern,
          binary_pattern: binary_pattern,
          source_index: index,
        }
      end
    end

    def expand_itf_runs(data)
      encoded_pairs = encode_itf(data)
      runs = []
      runs.concat retag_runs(CodingAdventures::BarcodeLayout1D.runs_from_binary_pattern(START_PATTERN, source_char: "start", source_index: -1), "start")
      encoded_pairs.each do |entry|
        runs.concat retag_runs(
          CodingAdventures::BarcodeLayout1D.runs_from_binary_pattern(
            entry[:binary_pattern],
            source_char: entry[:pair],
            source_index: entry[:source_index],
          ),
          "data",
        )
      end
      runs.concat retag_runs(CodingAdventures::BarcodeLayout1D.runs_from_binary_pattern(STOP_PATTERN, source_char: "stop", source_index: -2), "stop")
      runs
    end

    def layout_itf(data, config = DEFAULT_LAYOUT_CONFIG)
      normalized = normalize_itf(data)
      CodingAdventures::BarcodeLayout1D.draw_one_dimensional_barcode(
        expand_itf_runs(normalized),
        config,
        {
          metadata: {
            symbology: "itf",
            pair_count: normalized.length / 2,
          },
        },
      )
    end

    def draw_itf(data, config = DEFAULT_LAYOUT_CONFIG)
      layout_itf(data, config)
    end

    def retag_runs(runs, role)
      runs.map do |run|
        run.merge(role: role, metadata: run.fetch(:metadata, {}).dup)
      end
    end
    private_class_method :retag_runs
  end
end
