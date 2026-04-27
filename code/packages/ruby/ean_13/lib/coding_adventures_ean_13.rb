# frozen_string_literal: true

require "coding_adventures_barcode_layout_1d"
require_relative "coding_adventures/ean_13/version"

module CodingAdventures
  module Ean13
    module_function

    DEFAULT_LAYOUT_CONFIG = CodingAdventures::BarcodeLayout1D::DEFAULT_BARCODE_1D_LAYOUT_CONFIG
    DEFAULT_RENDER_CONFIG = DEFAULT_LAYOUT_CONFIG

    SIDE_GUARD = "101"
    CENTER_GUARD = "01010"

    DIGIT_PATTERNS = {
      "L" => %w[0001101 0011001 0010011 0111101 0100011 0110001 0101111 0111011 0110111 0001011],
      "G" => %w[0100111 0110011 0011011 0100001 0011101 0111001 0000101 0010001 0001001 0010111],
      "R" => %w[1110010 1100110 1101100 1000010 1011100 1001110 1010000 1000100 1001000 1110100],
    }.freeze

    LEFT_PARITY_PATTERNS = %w[
      LLLLLL LLGLGG LLGGLG LLGGGL LGLLGG LGGLLG LGGGLL LGLGLG LGLGGL LGGLGL
    ].freeze

    def compute_ean_13_check_digit(payload12)
      assert_digits!(payload12, [12])
      total = payload12.chars.reverse.each_with_index.sum do |digit, index|
        digit.to_i * (index.even? ? 3 : 1)
      end
      ((10 - (total % 10)) % 10).to_s
    end

    def normalize_ean_13(data)
      assert_digits!(data, [12, 13])
      return "#{data}#{compute_ean_13_check_digit(data)}" if data.length == 12

      expected = compute_ean_13_check_digit(data[0, 12])
      actual = data[12]
      raise ArgumentError, "Invalid EAN-13 check digit: expected #{expected} but received #{actual}" unless expected == actual

      data
    end

    def left_parity_pattern(data)
      normalized = normalize_ean_13(data)
      LEFT_PARITY_PATTERNS.fetch(normalized[0].to_i)
    end

    def encode_ean_13(data)
      normalized = normalize_ean_13(data)
      parity = left_parity_pattern(normalized)
      digits = normalized.chars

      left_digits = digits[1, 6].each_with_index.map do |digit, offset|
        encoding = parity[offset]
        {
          digit: digit,
          encoding: encoding,
          pattern: DIGIT_PATTERNS.fetch(encoding).fetch(digit.to_i),
          source_index: offset + 1,
          role: "data",
        }
      end

      right_digits = digits[7, 6].each_with_index.map do |digit, offset|
        {
          digit: digit,
          encoding: "R",
          pattern: DIGIT_PATTERNS.fetch("R").fetch(digit.to_i),
          source_index: offset + 7,
          role: offset == 5 ? "check" : "data",
        }
      end

      left_digits + right_digits
    end

    def expand_ean_13_runs(data)
      encoded_digits = encode_ean_13(data)
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

    def layout_ean_13(data, config = DEFAULT_LAYOUT_CONFIG)
      normalized = normalize_ean_13(data)
      CodingAdventures::BarcodeLayout1D.draw_one_dimensional_barcode(
        expand_ean_13_runs(normalized),
        config,
        {
          metadata: {
            symbology: "ean-13",
            leading_digit: normalized[0],
            left_parity: left_parity_pattern(normalized),
            content_modules: 95,
          },
        },
      )
    end

    def draw_ean_13(data, config = DEFAULT_LAYOUT_CONFIG)
      layout_ean_13(data, config)
    end

    def assert_digits!(data, lengths)
      raise ArgumentError, "EAN-13 input must contain digits only" unless /\A\d+\z/.match?(data)
      raise ArgumentError, "EAN-13 input must contain 12 digits or 13 digits" unless lengths.include?(data.length)
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
