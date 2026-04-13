# frozen_string_literal: true

require "coding_adventures_paint_instructions"
require_relative "coding_adventures/barcode_layout_1d/version"

module CodingAdventures
  module BarcodeLayout1D
    module_function

    DEFAULT_BARCODE_1D_LAYOUT_CONFIG = {
      module_unit: 4,
      bar_height: 120,
      quiet_zone_modules: 10,
    }.freeze

    DEFAULT_PAINT_BARCODE_1D_OPTIONS = {
      fill: "#000000",
      background: "#ffffff",
      metadata: {},
    }.freeze

    class BarcodeError < StandardError; end
    class InvalidConfigurationError < BarcodeError; end

    def runs_from_binary_pattern(pattern, bar_char: "1", space_char: "0", source_char: "", source_index: 0, metadata: {})
      return [] if pattern.empty?

      runs = []
      current = pattern[0]
      count = 1

      flush = lambda do |token, modules|
        color =
          if token == bar_char
            "bar"
          elsif token == space_char
            "space"
          else
            raise InvalidConfigurationError, "binary pattern contains unsupported token: #{token.inspect}"
          end

        runs << {
          color: color,
          modules: modules,
          source_char: source_char,
          source_index: source_index,
          role: "data",
          metadata: metadata.dup,
        }
      end

      pattern[1..].each_char do |token|
        if token == current
          count += 1
        else
          flush.call(current, count)
          current = token
          count = 1
        end
      end

      flush.call(current, count)
      runs
    end

    def runs_from_width_pattern(pattern, colors, source_char:, source_index:, narrow_modules: 1, wide_modules: 3, role: "data", metadata: {})
      raise InvalidConfigurationError, "pattern length must match colors length" unless pattern.length == colors.length
      raise InvalidConfigurationError, "module widths must be positive" unless narrow_modules.positive? && wide_modules.positive?

      pattern.chars.each_with_index.map do |element, index|
        raise InvalidConfigurationError, "width pattern contains unsupported token: #{element.inspect}" unless %w[N W].include?(element)

        {
          color: colors[index],
          modules: element == "W" ? wide_modules : narrow_modules,
          source_char: source_char,
          source_index: source_index,
          role: role,
          metadata: metadata.dup,
        }
      end
    end

    def layout_barcode_1d(runs, config = DEFAULT_BARCODE_1D_LAYOUT_CONFIG, options = DEFAULT_PAINT_BARCODE_1D_OPTIONS)
      validate_layout_config!(config)

      quiet_zone_width = config[:quiet_zone_modules] * config[:module_unit]
      cursor_x = quiet_zone_width
      instructions = []

      runs.each do |run|
        validate_run!(run)
        width = run[:modules] * config[:module_unit]
        if run[:color] == "bar"
          instructions << CodingAdventures::PaintInstructions.paint_rect(
            x: cursor_x,
            y: 0,
            width: width,
            height: config[:bar_height],
            fill: options[:fill],
            metadata: {
              source_char: run[:source_char],
              source_index: run[:source_index],
              modules: run[:modules],
              role: run[:role],
            }.merge(run.fetch(:metadata, {})),
          )
        end
        cursor_x += width
      end

      content_width = cursor_x - quiet_zone_width
      CodingAdventures::PaintInstructions.paint_scene(
        width: cursor_x + quiet_zone_width,
        height: config[:bar_height],
        instructions: instructions,
        background: options[:background],
        metadata: {
          content_width: content_width,
          quiet_zone_width: quiet_zone_width,
          module_unit: config[:module_unit],
          bar_height: config[:bar_height],
        }.merge(options.fetch(:metadata, {})),
      )
    end

    def draw_one_dimensional_barcode(runs, config = DEFAULT_BARCODE_1D_LAYOUT_CONFIG, options = DEFAULT_PAINT_BARCODE_1D_OPTIONS)
      layout_barcode_1d(runs, config, options)
    end

    def validate_layout_config!(config)
      raise InvalidConfigurationError, "module_unit must be positive" unless config[:module_unit].positive?
      raise InvalidConfigurationError, "bar_height must be positive" unless config[:bar_height].positive?
      raise InvalidConfigurationError, "quiet_zone_modules must be zero or positive" if config[:quiet_zone_modules].negative?
    end

    def validate_run!(run)
      raise InvalidConfigurationError, "run color must be 'bar' or 'space'" unless %w[bar space].include?(run[:color])
      raise InvalidConfigurationError, "run modules must be positive" unless run[:modules].positive?
    end
    private_class_method :validate_layout_config!, :validate_run!
  end
end
