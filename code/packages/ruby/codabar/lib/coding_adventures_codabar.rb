# frozen_string_literal: true

require "coding_adventures_barcode_layout_1d"
require_relative "coding_adventures/codabar/version"

module CodingAdventures
  module Codabar
    module_function

    DEFAULT_LAYOUT_CONFIG = CodingAdventures::BarcodeLayout1D::DEFAULT_BARCODE_1D_LAYOUT_CONFIG
    DEFAULT_RENDER_CONFIG = DEFAULT_LAYOUT_CONFIG

    GUARDS = %w[A B C D].freeze

    PATTERNS = {
      "0" => "101010011", "1" => "101011001", "2" => "101001011", "3" => "110010101",
      "4" => "101101001", "5" => "110101001", "6" => "100101011", "7" => "100101101",
      "8" => "100110101", "9" => "110100101", "-" => "101001101", "$" => "101100101",
      ":" => "1101011011", "/" => "1101101011", "." => "1101101101", "+" => "1011011011",
      "A" => "1011001001", "B" => "1001001011", "C" => "1010010011", "D" => "1010011001",
    }.freeze

    def normalize_codabar(data, start: "A", stop: "A")
      normalized = data.upcase

      if normalized.length >= 2 && guard?(normalized[0]) && guard?(normalized[-1])
        assert_body_chars!(normalized[1...-1])
        return normalized
      end

      raise ArgumentError, "Codabar guards must be one of A, B, C, or D" unless guard?(start) && guard?(stop)

      assert_body_chars!(normalized)
      "#{start}#{normalized}#{stop}"
    end

    def encode_codabar(data, start: "A", stop: "A")
      normalized = normalize_codabar(data, start: start, stop: stop)
      normalized.chars.each_with_index.map do |char, index|
        role = if index.zero?
          "start"
        elsif index == normalized.length - 1
          "stop"
        else
          "data"
        end

        {
          char: char,
          pattern: PATTERNS.fetch(char),
          source_index: index,
          role: role,
        }
      end
    end

    def expand_codabar_runs(data, options = {})
      start = options.fetch(:start, "A")
      stop = options.fetch(:stop, "A")
      encoded = encode_codabar(data, start: start, stop: stop)
      encoded.each_with_index.flat_map do |symbol, index|
        symbol_runs = retag_runs(
          CodingAdventures::BarcodeLayout1D.runs_from_binary_pattern(
            symbol[:pattern],
            source_char: symbol[:char],
            source_index: symbol[:source_index],
          ),
          symbol[:role],
        )

        if index < encoded.length - 1
          symbol_runs + [
            {
              color: "space",
              modules: 1,
              source_char: symbol[:char],
              source_index: symbol[:source_index],
              role: "inter-character-gap",
              metadata: {},
            },
          ]
        else
          symbol_runs
        end
      end
    end

    def layout_codabar(data, config = DEFAULT_LAYOUT_CONFIG, options = {})
      start = options.fetch(:start, "A")
      stop = options.fetch(:stop, "A")
      normalized = normalize_codabar(data, start: start, stop: stop)
      CodingAdventures::BarcodeLayout1D.draw_one_dimensional_barcode(
        expand_codabar_runs(normalized),
        config,
        {
          metadata: {
            symbology: "codabar",
            start: normalized[0],
            stop: normalized[-1],
          },
        },
      )
    end

    def draw_codabar(data, config = DEFAULT_LAYOUT_CONFIG, options = {})
      layout_codabar(data, config, options)
    end

    def guard?(char)
      GUARDS.include?(char)
    end
    private_class_method :guard?

    def assert_body_chars!(body)
      body.each_char do |char|
        raise ArgumentError, %(Invalid Codabar body character "#{char}") unless PATTERNS.key?(char) && !guard?(char)
      end
    end
    private_class_method :assert_body_chars!

    def retag_runs(runs, role)
      runs.map do |run|
        run.merge(role: role, metadata: run.fetch(:metadata, {}).dup)
      end
    end
    private_class_method :retag_runs
  end
end
