# frozen_string_literal: true

require "coding_adventures_barcode_layout_1d"
require_relative "coding_adventures/upc_a/version"

module CodingAdventures
  module UpcA
    module_function

    DEFAULT_LAYOUT_CONFIG = CodingAdventures::BarcodeLayout1D::DEFAULT_BARCODE_1D_LAYOUT_CONFIG
    DEFAULT_RENDER_CONFIG = DEFAULT_LAYOUT_CONFIG

    SIDE_GUARD = "101"
    CENTER_GUARD = "01010"

    DIGIT_PATTERNS = {
      "L" => %w[0001101 0011001 0010011 0111101 0100011 0110001 0101111 0111011 0110111 0001011],
      "R" => %w[1110010 1100110 1101100 1000010 1011100 1001110 1010000 1000100 1001000 1110100],
    }.freeze

    def compute_upc_a_check_digit(payload11)
      assert_digits!(payload11, [11])
      odd_sum = 0
      even_sum = 0

      payload11.chars.each_with_index do |digit, index|
        if index.even?
          odd_sum += digit.to_i
        else
          even_sum += digit.to_i
        end
      end

      ((10 - (((odd_sum * 3) + even_sum) % 10)) % 10).to_s
    end

    def normalize_upc_a(data)
      assert_digits!(data, [11, 12])
      return "#{data}#{compute_upc_a_check_digit(data)}" if data.length == 11

      expected = compute_upc_a_check_digit(data[0, 11])
      actual = data[11]
      raise ArgumentError, "Invalid UPC-A check digit: expected #{expected} but received #{actual}" unless expected == actual

      data
    end

    def encode_upc_a(data)
      normalized = normalize_upc_a(data)
      normalized.chars.each_with_index.map do |digit, index|
        encoding = index < 6 ? "L" : "R"
        {
          digit: digit,
          encoding: encoding,
          pattern: DIGIT_PATTERNS.fetch(encoding).fetch(digit.to_i),
          source_index: index,
          role: index == 11 ? "check" : "data",
        }
      end
    end

    def expand_upc_a_runs(data)
      encoded_digits = encode_upc_a(data)
      runs = []
      runs.concat retag_runs(CodingAdventures::BarcodeLayout1D.runs_from_binary_pattern(SIDE_GUARD, source_char: "start", source_index: -1), "guard")

      encoded_digits.first(6).each do |entry|
        runs.concat retag_runs(
          CodingAdventures::BarcodeLayout1D.runs_from_binary_pattern(
            entry[:pattern],
            source_char: entry[:digit],
            source_index: entry[:source_index],
          ),
          entry[:role],
        )
      end

      runs.concat retag_runs(CodingAdventures::BarcodeLayout1D.runs_from_binary_pattern(CENTER_GUARD, source_char: "center", source_index: -2), "guard")

      encoded_digits.drop(6).each do |entry|
        runs.concat retag_runs(
          CodingAdventures::BarcodeLayout1D.runs_from_binary_pattern(
            entry[:pattern],
            source_char: entry[:digit],
            source_index: entry[:source_index],
          ),
          entry[:role],
        )
      end

      runs.concat retag_runs(CodingAdventures::BarcodeLayout1D.runs_from_binary_pattern(SIDE_GUARD, source_char: "end", source_index: -3), "guard")
      runs
    end

    def layout_upc_a(data, config = DEFAULT_LAYOUT_CONFIG)
      normalized = normalize_upc_a(data)
      CodingAdventures::BarcodeLayout1D.draw_one_dimensional_barcode(
        expand_upc_a_runs(normalized),
        config,
        {
          metadata: {
            symbology: "upc-a",
            content_modules: 95,
          },
        },
      )
    end

    def draw_upc_a(data, config = DEFAULT_LAYOUT_CONFIG)
      layout_upc_a(data, config)
    end

    def assert_digits!(data, lengths)
      raise ArgumentError, "UPC-A input must contain digits only" unless /\A\d+\z/.match?(data)
      raise ArgumentError, "UPC-A input must contain 11 digits or 12 digits" unless lengths.include?(data.length)
    end
    private_class_method :assert_digits!

    def retag_runs(runs, role)
      runs.map do |run|
        run.merge(role: role, metadata: run.fetch(:metadata, {}).dup)
      end
    end
    private_class_method :retag_runs
  end
end
