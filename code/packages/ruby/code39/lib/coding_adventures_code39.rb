# frozen_string_literal: true

require "coding_adventures_draw_instructions"
require_relative "coding_adventures/code39/version"

module CodingAdventures
  # Code 39 encoder that stops at a backend-neutral draw scene.
  module Code39
    module_function

    DEFAULT_RENDER_CONFIG = {
      narrow_unit: 4,
      wide_unit: 12,
      bar_height: 120,
      quiet_zone_units: 10,
      include_human_readable_text: true,
    }.freeze

    TEXT_MARGIN = 8
    TEXT_FONT_SIZE = 16
    TEXT_BLOCK_HEIGHT = TEXT_MARGIN + TEXT_FONT_SIZE + 4

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
      colors = %w[bar space bar space bar space bar space bar]
      encoded.each_with_index.flat_map do |encoded_char, source_index|
        runs = encoded_char[:pattern].chars.each_with_index.map do |element, index|
          {
            color: colors[index],
            width: element == "W" ? "wide" : "narrow",
            source_char: encoded_char[:char],
            source_index: source_index,
            is_inter_character_gap: false,
          }
        end
        if source_index < encoded.length - 1
          runs << {
            color: "space",
            width: "narrow",
            source_char: encoded_char[:char],
            source_index: source_index,
            is_inter_character_gap: true,
          }
        end
        runs
      end
    end

    def draw_code39(data, config = DEFAULT_RENDER_CONFIG)
      normalized = normalize_code39(data)
      quiet_zone_width = config[:quiet_zone_units] * config[:narrow_unit]
      instructions = []
      cursor_x = quiet_zone_width

      expand_code39_runs(normalized).each do |run|
        width = run[:width] == "wide" ? config[:wide_unit] : config[:narrow_unit]
        if run[:color] == "bar"
          instructions << CodingAdventures::DrawInstructions.draw_rect(
            x: cursor_x,
            y: 0,
            width: width,
            height: config[:bar_height],
            metadata: { char: run[:source_char], index: run[:source_index] },
          )
        end
        cursor_x += width
      end

      if config[:include_human_readable_text]
        instructions << CodingAdventures::DrawInstructions.draw_text(
          x: (cursor_x + quiet_zone_width) / 2,
          y: config[:bar_height] + TEXT_MARGIN + TEXT_FONT_SIZE - 2,
          value: normalized,
          metadata: { role: "label" },
        )
      end

      CodingAdventures::DrawInstructions.create_scene(
        width: cursor_x + quiet_zone_width,
        height: config[:bar_height] + (config[:include_human_readable_text] ? TEXT_BLOCK_HEIGHT : 0),
        instructions: instructions,
        metadata: { label: "Code 39 barcode for #{normalized}", symbology: "code39" },
      )
    end

    def render_code39(data, renderer, config = DEFAULT_RENDER_CONFIG)
      renderer.render(draw_code39(data, config))
    end
  end
end
