# frozen_string_literal: true

require "digest"
require "json"
require "minitest/autorun"
require "tempfile"
require "coding_adventures_barcode_layout_1d"

class TestBarcodeLayout1DConformance < Minitest::Test
  Implementation = CodingAdventures::BarcodeLayout1D
  FixtureError = Class.new(StandardError)
  MAX_FIXTURE_BYTES = 131_072
  MAX_FIXTURE_DEPTH = 8
  MAX_SCHEMA_DEPTH = 24
  FIXTURE_ROOT = File.expand_path(
    "../../../../specs/fixtures/barcode-layout-1d-v1",
    __dir__,
  )

  class UniqueObject < Hash
    def []=(key, value)
      raise FixtureError, "fixture-invalid-json" if key?(key)

      super
    end
  end

  class << self
    def read_bounded(path)
      File.open(path, "rb") do |file|
        raise FixtureError, "fixture-size-limit" if file.stat.size > MAX_FIXTURE_BYTES

        bytes = file.read(MAX_FIXTURE_BYTES + 1)
        raise FixtureError, "fixture-size-limit" if bytes.bytesize > MAX_FIXTURE_BYTES

        bytes
      end
    end

    def parse_bounded(bytes, depth_limit: MAX_FIXTURE_DEPTH)
      raise FixtureError, "fixture-size-limit" if bytes.bytesize > MAX_FIXTURE_BYTES

      text = bytes.dup.force_encoding(Encoding::UTF_8)
      raise FixtureError, "fixture-invalid-json" unless text.valid_encoding?

      value = JSON.parse(
        text,
        object_class: UniqueObject,
        array_class: Array,
        allow_nan: false,
        max_nesting: false,
      )
      validate_tree(value, depth_limit)
      value
    rescue JSON::ParserError, JSON::NestingError
      raise FixtureError, "fixture-invalid-json"
    end

    def validate_tree(root, depth_limit)
      stack = [[root, 0]]
      until stack.empty?
        value, depth = stack.pop
        raise FixtureError, "fixture-depth-limit" if depth > depth_limit

        case value
        when String
          raise FixtureError, "fixture-invalid-scalar" unless value.valid_encoding?
        when Float
          raise FixtureError, "fixture-invalid-json" unless value.finite?
        when Array
          value.each { |item| stack << [item, depth + 1] }
        when Hash
          value.each { |key, item| stack << [key, depth + 1] << [item, depth + 1] }
        end
      end
    end

    def validate_local_refs(root)
      stack = [root]
      until stack.empty?
        value = stack.pop
        case value
        when Array
          stack.concat(value)
        when Hash
          value.each do |key, item|
            if %w[$ref $dynamicRef].include?(key) && (!item.is_a?(String) || !item.start_with?("#/"))
              raise FixtureError, "fixture-schema-invalid"
            end
            stack << item
          end
        end
      end
    end

    def load_document(schema_bytes, document_bytes)
      schema = parse_bounded(schema_bytes, depth_limit: MAX_SCHEMA_DEPTH)
      document = parse_bounded(document_bytes)
      raise FixtureError, "fixture-schema-invalid" unless schema.is_a?(Hash) && document.is_a?(Hash)

      validate_local_refs(schema)
      cases = document["cases"]
      raise FixtureError, "fixture-schema-invalid" unless document["schema_version"] == 1 &&
        document["profile"] == "barcode-layout-1d-v1" &&
        document["limits"].is_a?(Hash) &&
        document["error_ids"].is_a?(Array) &&
        document["error_ids"].all? { |item| item.is_a?(String) } &&
        cases.is_a?(Array)
      raise FixtureError, "fixture-schema-invalid" unless cases.length.between?(1, 64)
      raise FixtureError, "fixture-schema-invalid" unless cases.all? { |item| valid_case_shape?(item) }
      raise FixtureError, "fixture-schema-invalid" unless cases.map { |item| item["id"] }.uniq.length == cases.length

      document
    end

    def valid_case_shape?(item)
      item.is_a?(Hash) &&
        item.keys.sort == %w[expected id input operation] &&
        item["id"].is_a?(String) &&
        %w[compute-layout expand-binary expand-width project-scene].include?(item["operation"]) &&
        item["input"].is_a?(Hash) &&
        item["expected"].is_a?(Hash)
    end

    def pattern(value)
      return value.fetch("pattern") if value.key?("pattern")

      repeat = value.fetch("repeat")
      count = repeat.fetch("count")
      token = repeat.fetch("token")
      suffix = repeat.fetch("suffix", "")
      unless count.is_a?(Integer) && count.between?(0, 65_569) &&
          token.is_a?(String) && token.length.between?(1, 2) &&
          suffix.is_a?(String) && suffix.length <= 1
        raise FixtureError, "fixture-schema-invalid"
      end
      (token * count) + suffix
    end

    def runs(value)
      rows = if value.key?("runs")
        value.fetch("runs")
      else
        repeated = value.fetch("repeatRuns")
        count = repeated.fetch("count")
        raise FixtureError, "fixture-schema-invalid" unless count.is_a?(Integer) && count.between?(0, 40_980)

        Array.new(count) do |index|
          first = repeated.fetch("firstColor")
          {
            "color" => (index.even? ? first : (first == "bar" ? "space" : "bar")),
            "modules" => repeated.fetch("modules"),
            "sourceLabel" => repeated.fetch("sourceLabel"),
            "sourceIndex" => repeated.fetch("sourceIndex"),
            "role" => repeated.fetch("role"),
          }
        end
      end
      rows.map do |row|
        {
          color: row.fetch("color"),
          modules: row.fetch("modules"),
          source_label: row.fetch("sourceLabel"),
          source_index: row.fetch("sourceIndex"),
          role: row.fetch("role"),
        }
      end
    end

    def symbols(value)
      return value.fetch("symbols").map(&:dup) if value.key?("symbols")
      return nil unless value.key?("repeatSymbols")

      repeated = value.fetch("repeatSymbols")
      count = repeated.fetch("count")
      raise FixtureError, "fixture-schema-invalid" unless count.is_a?(Integer) && count.between?(0, 40_980)

      Array.new(count) do |index|
        {
          "label" => repeated.fetch("label"),
          "modules" => repeated.fetch("modules"),
          "sourceIndex" => index,
          "role" => repeated.fetch("role"),
        }
      end
    end

    def execute(test_case)
      value = test_case.fetch("input")
      case test_case.fetch("operation")
      when "expand-binary"
        Implementation.expand_binary_v1(
          pattern(value),
          source_label: value.fetch("sourceLabel"),
          source_index: value.fetch("sourceIndex"),
          role: value.fetch("role"),
        )
      when "expand-width"
        Implementation.expand_width_v1(
          pattern(value),
          source_label: value.fetch("sourceLabel"),
          source_index: value.fetch("sourceIndex"),
          role: value.fetch("role"),
          narrow_marker: value.fetch("narrowMarker", "N"),
          wide_marker: value.fetch("wideMarker", "W"),
          narrow_modules: value.fetch("narrowModules", 1),
          wide_modules: value.fetch("wideModules", 3),
          starting_color: value.fetch("startingColor", "bar"),
        )
      when "compute-layout"
        Implementation.compute_layout_v1(runs(value), value.fetch("quietZoneModules"), symbols(value))
      when "project-scene"
        render = value.fetch("renderConfig", {})
        Implementation.project_scene_v1(
          runs(value),
          value.fetch("quietZoneModules"),
          {
            render_config: {
              module_width: render.fetch("moduleWidth", 4),
              bar_height: render.fetch("barHeight", 120),
              foreground: render.fetch("foreground", "#000000"),
              background: render.fetch("background", "#ffffff"),
              include_human_readable_text: render.fetch("includeHumanReadableText", false),
            },
            label: value.fetch("label", "1D barcode"),
            metadata: value.fetch("metadata", {}).dup,
            human_readable_text: value["humanReadableText"],
            symbols: symbols(value),
          },
        )
      else
        raise "unknown operation"
      end
    end

    def run_projection(run)
      {
        "color" => run.fetch(:color),
        "modules" => run.fetch(:modules),
        "sourceLabel" => run.fetch(:source_label),
        "sourceIndex" => run.fetch(:source_index),
        "role" => run.fetch(:role),
      }
    end

    def scene_projection(scene)
      {
        "width" => scene.width,
        "height" => scene.height,
        "background" => scene.background,
        "rectangles" => scene.instructions.map do |rect|
          {
            "x" => rect.x,
            "y" => rect.y,
            "width" => rect.width,
            "height" => rect.height,
            "fill" => rect.fill,
            "metadata" => rect.metadata,
          }
        end,
        "metadata" => scene.metadata,
      }
    end

    def canonical(value)
      normalize = lambda do |item|
        case item
        when Array then item.map { |child| normalize.call(child) }
        when Hash then item.keys.sort.to_h { |key| [key, normalize.call(item.fetch(key))] }
        else item
        end
      end
      normalized = normalize.call(value)
      JSON.generate(normalized, ascii_only: false)
    end
  end

  SCHEMA_BYTES = read_bounded(File.join(FIXTURE_ROOT, "schema.json"))
  CASE_BYTES = read_bounded(File.join(FIXTURE_ROOT, "cases.json"))
  DOCUMENT = load_document(SCHEMA_BYTES, CASE_BYTES)

  DOCUMENT.fetch("cases").each do |test_case|
    define_method("test_#{test_case.fetch('id').tr('-', '_')}") do
      expected = test_case.fetch("expected")
      if expected.key?("error")
        error = assert_raises(Implementation::BarcodeV1Error) { self.class.execute(test_case) }
        assert_equal expected.fetch("error"), error.error_id
        next
      end

      actual = self.class.execute(test_case)
      if expected.key?("runs")
        assert_equal expected.fetch("runs"), actual.map { |run| self.class.run_projection(run) }
      elsif expected.key?("runDigest")
        projected = actual.map { |run| self.class.run_projection(run) }
        digest = expected.fetch("runDigest")
        assert_equal digest.fetch("runCount"), projected.length
        assert_equal digest.fetch("contentModules"), projected.sum { |run| run.fetch("modules") }
        assert_equal digest.fetch("firstRun"), projected.first
        assert_equal digest.fetch("lastRun"), projected.last
        assert_equal digest.fetch("runsSha256"), Digest::SHA256.hexdigest(self.class.canonical(projected))
      elsif expected.key?("layout")
        assert_equal expected.fetch("layout"), JSON.parse(JSON.generate(actual))
      else
        assert_equal expected.fetch("scene"), self.class.scene_projection(actual)
      end
    end
  end

  def test_operation_counts
    counts = DOCUMENT.fetch("cases").group_by { |test_case| test_case.fetch("operation") }.transform_values(&:length)
    assert_equal({"expand-binary" => 12, "expand-width" => 12, "compute-layout" => 19, "project-scene" => 13}, counts)
  end

  def test_loader_rejects_hostile_documents
    hostile = [
      '{"duplicate":1,"duplicate":2}'.b,
      (("[" * 9) + "0" + ("]" * 9)).b,
      "\xff".b,
      CASE_BYTES.sub('"layout-v1-binary-basic"'.b, '"\\ud800"'.b),
      "[]".b,
      "{}".b,
    ]
    hostile.each { |bytes| assert_raises(FixtureError) { self.class.load_document(SCHEMA_BYTES, bytes) } }
  end

  def test_loader_accepts_maximum_bytes_and_rejects_one_more
    prefix = '{"padding":"'.b
    suffix = '"}'.b
    exact = prefix + ("x" * (MAX_FIXTURE_BYTES - prefix.bytesize - suffix.bytesize)) + suffix
    assert_equal({"padding" => "x" * (MAX_FIXTURE_BYTES - prefix.bytesize - suffix.bytesize)}, self.class.parse_bounded(exact))
    assert_raises(FixtureError) { self.class.parse_bounded(exact + " ") }
  end

  def test_repeat_forms_are_bounded_before_materialization
    assert_raises(FixtureError) { self.class.pattern("repeat" => {"token" => "1", "count" => 65_570}) }
    assert_raises(FixtureError) { self.class.runs("repeatRuns" => {"count" => 40_981}) }
    assert_raises(FixtureError) { self.class.symbols("repeatSymbols" => {"count" => 40_981}) }
  end

  def test_text_value_fails_before_native_resolution
    calls = 0
    error = assert_raises(Implementation::BarcodeV1Error) do
      Implementation.project_scene_v1(
        [{color: "bar", modules: 0, source_label: "A", source_index: 0, role: "data"}],
        0,
        {human_readable_text: "123", native_resolver: -> { calls += 1 }},
      )
    end
    assert_equal "human-readable-text-unsupported", error.error_id
    assert_equal 0, calls
  end

  def test_text_enabled_fails_before_native_resolution
    calls = 0
    error = assert_raises(Implementation::BarcodeV1Error) do
      Implementation.project_scene_v1(
        [{color: "bar", modules: 0, source_label: "A", source_index: 0, role: "data"}],
        0,
        {render_config: {module_width: 0, include_human_readable_text: true}, native_resolver: -> { calls += 1 }},
      )
    end
    assert_equal "human-readable-text-unsupported", error.error_id
    assert_equal 0, calls
  end

  def test_outputs_are_deep_owned
    metadata = {"caller" => "value"}
    runs = [{color: "bar", modules: 1, source_label: +"A", source_index: 0, role: "data"}]
    first = Implementation.project_scene_v1(runs, 1, {metadata: metadata})
    first.metadata["caller"] = "changed"
    first.instructions.first.metadata["sourceLabel"] = "changed"
    metadata["caller"] = "caller-changed"
    runs.first[:source_label].replace("changed")
    second = Implementation.project_scene_v1(
      [{color: "bar", modules: 1, source_label: "A", source_index: 0, role: "data"}],
      1,
      {metadata: {"caller" => "value"}},
    )
    assert_equal "value", second.metadata.fetch("caller")
    assert_equal "A", second.instructions.first.metadata.fetch("sourceLabel")
  end
end
