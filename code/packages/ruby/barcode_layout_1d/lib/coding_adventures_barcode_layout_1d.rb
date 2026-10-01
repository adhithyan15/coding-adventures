# frozen_string_literal: true

require "coding_adventures_paint_instructions"
require_relative "coding_adventures/barcode_layout_1d/version"

module CodingAdventures
  module BarcodeLayout1D
    module_function

    DEFAULT_BARCODE_1D_LAYOUT_CONFIG = {
      module_unit: 4,
      bar_height: 120,
      quiet_zone_modules: 10
    }.freeze

    DEFAULT_PAINT_BARCODE_1D_OPTIONS = {
      fill: "#000000",
      background: "#ffffff",
      metadata: {}
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
          metadata: metadata.dup
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
          modules: (element == "W") ? wide_modules : narrow_modules,
          source_char: source_char,
          source_index: source_index,
          role: role,
          metadata: metadata.dup
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
              role: run[:role]
            }.merge(run.fetch(:metadata, {}))
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
          bar_height: config[:bar_height]
        }.merge(options.fetch(:metadata, {}))
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

    # ----------------------------------------------------------------------
    # Strict, language-neutral v1 adapter
    # ----------------------------------------------------------------------
    #
    # The legacy helpers above remain intact for existing barcode encoders.
    # These parallel entry points implement the closed integer contract from
    # code/specs/barcode-layout-1d-v1.md. Keeping the boundary additive lets a
    # caller opt into stable errors and hard ceilings without silently changing
    # the historical zero-quiet-zone and metadata behavior of old consumers.

    MAX_PATTERN_SCALARS = 65_567
    MAX_RUNS = 40_979
    MAX_CONTENT_MODULES = 65_567
    MAX_QUIET_ZONE_MODULES = 4_096
    MAX_SYMBOLS = 40_979
    MAX_LABEL_SCALARS = 4_096
    MAX_METADATA_ENTRIES = 64
    MAX_METADATA_KEY_SCALARS = 128
    MAX_METADATA_VALUE_SCALARS = 4_096
    MAX_METADATA_UTF8_BYTES = 65_536
    MAX_MODULE_WIDTH = 8_192
    MAX_BAR_HEIGHT = 8_192
    MAX_COLOR_SCALARS = 128

    V1_ROLES = %w[data start stop guard check inter-character-gap].freeze
    V1_SYMBOL_ROLES = %w[data start stop guard check].freeze

    class BarcodeV1Error < BarcodeError
      attr_reader :error_id

      def initialize(error_id)
        @error_id = error_id
        super
      end
    end

    def v1_fail!(error_id)
      raise BarcodeV1Error, error_id
    end

    def v1_scalar_length(value, error_id)
      v1_fail!(error_id) unless value.is_a?(String) && value.valid_encoding?

      value.each_codepoint.count
    end

    def v1_integer?(value)
      value.is_a?(Integer)
    end

    def validate_v1_source!(label, index)
      v1_fail!("invalid-source-attribution") if
        v1_scalar_length(label, "invalid-source-attribution") > MAX_LABEL_SCALARS ||
          !v1_integer?(index) ||
          !index.between?(-(2**31), (2**31) - 1)
    end

    def v1_value(hash, key, default = :__missing__)
      return hash[key] if hash.key?(key)

      string_key = key.to_s
      return hash[string_key] if hash.key?(string_key)
      return default unless default == :__missing__

      raise KeyError, key
    end

    def expand_binary_v1(pattern, source_label:, source_index:, role:)
      length = v1_scalar_length(pattern, "invalid-binary-token")
      v1_fail!("pattern-too-long") if length > MAX_PATTERN_SCALARS
      v1_fail!("empty-pattern") if length.zero?
      v1_fail!("invalid-binary-token") unless pattern.each_char.all? { |token| %w[0 1].include?(token) }
      validate_v1_source!(source_label, source_index)
      v1_fail!("invalid-source-attribution") unless V1_ROLES.include?(role)

      result = []
      current = pattern[0]
      count = 1
      pattern[1..].each_char do |token|
        if token == current
          count += 1
          next
        end
        v1_fail!("too-many-runs") if result.length >= MAX_RUNS
        result << v1_run((current == "1") ? "bar" : "space", count, source_label, source_index, role)
        current = token
        count = 1
      end
      v1_fail!("too-many-runs") if result.length >= MAX_RUNS
      result << v1_run((current == "1") ? "bar" : "space", count, source_label, source_index, role)
      result
    end

    def expand_width_v1(
      pattern,
      source_label:,
      source_index:,
      role:,
      narrow_marker: "N",
      wide_marker: "W",
      narrow_modules: 1,
      wide_modules: 3,
      starting_color: "bar"
    )
      length = v1_scalar_length(pattern, "invalid-width-token")
      v1_fail!("pattern-too-long") if length > MAX_PATTERN_SCALARS
      v1_fail!("empty-pattern") if length.zero?
      if v1_scalar_length(narrow_marker, "invalid-marker-configuration") != 1 ||
          v1_scalar_length(wide_marker, "invalid-marker-configuration") != 1 ||
          narrow_marker == wide_marker
        v1_fail!("invalid-marker-configuration")
      end
      markers = [narrow_marker, wide_marker]
      v1_fail!("invalid-width-token") unless pattern.each_char.all? { |token| markers.include?(token) }
      validate_v1_source!(source_label, source_index)
      v1_fail!("invalid-source-attribution") unless V1_ROLES.include?(role)
      unless v1_integer?(narrow_modules) && v1_integer?(wide_modules) &&
          narrow_modules.positive? && wide_modules.positive?
        v1_fail!("invalid-module-count")
      end
      v1_fail!("invalid-marker-configuration") unless %w[bar space].include?(starting_color)
      v1_fail!("too-many-runs") if length > MAX_RUNS

      content = 0
      pattern.each_char.with_index.map do |token, index|
        modules = (token == wide_marker) ? wide_modules : narrow_modules
        v1_fail!("content-too-wide") if modules > MAX_CONTENT_MODULES - content
        content += modules
        color = if index.even?
          starting_color
        else
          (starting_color == "bar") ? "space" : "bar"
        end
        v1_run(color, modules, source_label, source_index, role)
      end
    end

    def compute_layout_v1(runs, quiet_zone_modules, symbols = nil)
      v1_fail!("too-many-runs") if runs.length > MAX_RUNS
      content = 0
      previous = nil
      copied_runs = runs.map do |run|
        color = v1_value(run, :color)
        modules = v1_value(run, :modules)
        label = v1_value(run, :source_label)
        index = v1_value(run, :source_index)
        role = v1_value(run, :role)
        v1_fail!("invalid-source-attribution") unless %w[bar space].include?(color) && V1_ROLES.include?(role)
        v1_fail!("invalid-module-count") unless v1_integer?(modules) && modules.positive?
        validate_v1_source!(label, index)
        v1_fail!("content-too-wide") if modules > MAX_CONTENT_MODULES - content
        content += modules
        v1_fail!("non-alternating-runs") if previous == color
        previous = color
        v1_run(color, modules, label, index, role)
      end

      unless v1_integer?(quiet_zone_modules) && quiet_zone_modules.between?(1, MAX_QUIET_ZONE_MODULES)
        v1_fail!("invalid-quiet-zone")
      end

      symbol_layouts = if symbols.nil?
        infer_v1_symbols(copied_runs)
      else
        explicit_v1_symbols(symbols, content)
      end
      total = quiet_zone_modules + content + quiet_zone_modules
      {
        "leftQuietZoneModules" => quiet_zone_modules,
        "rightQuietZoneModules" => quiet_zone_modules,
        "contentModules" => content,
        "totalModules" => total,
        "symbolLayouts" => symbol_layouts
      }
    end

    def project_scene_v1(runs, quiet_zone_modules, options = {})
      render = v1_value(options, :render_config, {})
      include_text = v1_value(render, :include_human_readable_text, false)
      human_text = v1_value(options, :human_readable_text, nil)
      v1_fail!("human-readable-text-unsupported") if include_text || !human_text.nil?

      module_width = v1_value(render, :module_width, 4)
      bar_height = v1_value(render, :bar_height, 120)
      foreground = v1_value(render, :foreground, "#000000")
      background = v1_value(render, :background, "#ffffff")
      unless v1_integer?(module_width) && module_width.between?(1, MAX_MODULE_WIDTH) &&
          v1_integer?(bar_height) && bar_height.between?(1, MAX_BAR_HEIGHT) &&
          v1_scalar_length(foreground, "invalid-render-config") <= MAX_COLOR_SCALARS &&
          v1_scalar_length(background, "invalid-render-config") <= MAX_COLOR_SCALARS
        v1_fail!("invalid-render-config")
      end

      symbols = v1_value(options, :symbols, nil)
      layout = compute_layout_v1(runs, quiet_zone_modules, symbols)
      metadata = validate_v1_metadata!(v1_value(options, :metadata, {}))
      label = v1_value(options, :label, "1D barcode")
      v1_fail!("metadata-too-large") if v1_scalar_length(label, "metadata-too-large") > MAX_LABEL_SCALARS

      instructions = []
      cursor = quiet_zone_modules
      runs.each do |run|
        modules = v1_value(run, :modules)
        ending = cursor + modules
        if v1_value(run, :color) == "bar"
          instructions << CodingAdventures::PaintInstructions.paint_rect(
            x: cursor * module_width,
            y: 0,
            width: modules * module_width,
            height: bar_height,
            fill: foreground.dup,
            metadata: {
              "sourceLabel" => v1_value(run, :source_label).dup,
              "sourceIndex" => v1_value(run, :source_index).to_s,
              "role" => v1_value(run, :role).dup,
              "moduleStart" => cursor.to_s,
              "moduleEnd" => ending.to_s
            }
          )
        end
        cursor = ending
      end
      scene_width = layout.fetch("totalModules") * module_width
      canonical = {
        "label" => label.dup,
        "leftQuietZoneModules" => layout.fetch("leftQuietZoneModules").to_s,
        "rightQuietZoneModules" => layout.fetch("rightQuietZoneModules").to_s,
        "contentModules" => layout.fetch("contentModules").to_s,
        "totalModules" => layout.fetch("totalModules").to_s,
        "moduleWidthPx" => module_width.to_s,
        "barHeightPx" => bar_height.to_s,
        "sceneWidthPx" => scene_width.to_s,
        "sceneHeightPx" => bar_height.to_s,
        "symbolCount" => layout.fetch("symbolLayouts").length.to_s
      }
      CodingAdventures::PaintInstructions.paint_scene(
        width: scene_width,
        height: bar_height,
        instructions: instructions,
        background: background.dup,
        metadata: metadata.merge(canonical)
      )
    end

    def v1_run(color, modules, label, index, role)
      {
        color: color.dup,
        modules: modules,
        source_label: label.dup,
        source_index: index,
        role: role.dup
      }
    end

    def infer_v1_symbols(runs)
      layouts = []
      cursor = 0
      active = nil
      active_start = 0
      runs.each do |run|
        ending = cursor + run.fetch(:modules)
        unless run.fetch(:role) == "inter-character-gap"
          candidate = [run.fetch(:source_label), run.fetch(:source_index), run.fetch(:role)]
          if candidate != active
            layouts << v1_symbol_layout(active, active_start, cursor) unless active.nil?
            active = candidate
            active_start = cursor
          end
        end
        cursor = ending
      end
      layouts << v1_symbol_layout(active, active_start, cursor) unless active.nil?
      v1_fail!("too-many-symbols") if layouts.length > MAX_SYMBOLS
      layouts
    end

    def explicit_v1_symbols(symbols, content)
      v1_fail!("too-many-symbols") if symbols.length > MAX_SYMBOLS
      cursor = 0
      layouts = symbols.map do |symbol|
        modules = v1_value(symbol, :modules)
        label = v1_value(symbol, :label)
        index = v1_value(symbol, :source_index, v1_value(symbol, :sourceIndex, nil))
        role = v1_value(symbol, :role)
        v1_fail!("invalid-module-count") unless v1_integer?(modules) && modules.positive?
        validate_v1_source!(label, index)
        v1_fail!("invalid-source-attribution") unless V1_SYMBOL_ROLES.include?(role)
        v1_fail!("symbol-width-mismatch") if modules > MAX_CONTENT_MODULES - cursor
        ending = cursor + modules
        layout = v1_symbol_layout([label, index, role], cursor, ending)
        cursor = ending
        layout
      end
      v1_fail!("symbol-width-mismatch") unless cursor == content
      layouts
    end

    def v1_symbol_layout(tuple, start_module, end_module)
      {
        "label" => tuple[0].dup,
        "startModule" => start_module,
        "endModule" => end_module,
        "sourceIndex" => tuple[1],
        "role" => tuple[2].dup
      }
    end

    def validate_v1_metadata!(metadata)
      v1_fail!("metadata-too-large") unless metadata.is_a?(Hash) && metadata.length <= MAX_METADATA_ENTRIES
      total = 0
      metadata.each_with_object({}) do |(key, value), copied|
        unless key.is_a?(String) && value.is_a?(String) && key.valid_encoding? && value.valid_encoding?
          v1_fail!("metadata-too-large")
        end
        if v1_scalar_length(key, "metadata-too-large") > MAX_METADATA_KEY_SCALARS ||
            v1_scalar_length(value, "metadata-too-large") > MAX_METADATA_VALUE_SCALARS
          v1_fail!("metadata-too-large")
        end
        total += key.bytesize + value.bytesize
        v1_fail!("metadata-too-large") if total > MAX_METADATA_UTF8_BYTES
        copied[key.dup] = value.dup
      end
    end

    private_class_method :v1_fail!, :v1_scalar_length, :v1_integer?,
      :validate_v1_source!, :v1_value, :v1_run, :infer_v1_symbols,
      :explicit_v1_symbols, :v1_symbol_layout, :validate_v1_metadata!
  end
end
