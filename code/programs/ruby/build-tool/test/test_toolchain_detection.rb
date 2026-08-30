# frozen_string_literal: true

# test_toolchain_detection.rb -- Pure extra-CI toolchain snapshot tests
# =====================================================================
#
# The shared corpus is the behavior contract, but this suite deliberately
# drives the Ruby module directly. That keeps conformance evidence native to
# this engine and proves that no CLI, subprocess, Git checkout, or host probe is
# hiding behind the adapter.

require_relative "test_helper"
require_relative "../lib/build_tool/toolchain_detection"

class TestToolchainDetection < Minitest::Test
  REPO_ROOT = Pathname(__dir__).parent.parent.parent.parent.parent
  CONFORMANCE_CASES = REPO_ROOT / "code/specs/fixtures/build-tool-v1/cases"
  EXPECTED_FIXTURES = %w[
    toolchain-detection-affected-only.json
    toolchain-detection-crlf-grammar.json
    toolchain-detection-declarations.json
    toolchain-detection-empty.json
    toolchain-detection-force-full.json
    toolchain-detection-null-all.json
    toolchain-detection-platform-darwin.json
    toolchain-detection-platform-linux.json
    toolchain-detection-platform-windows.json
    toolchain-detection-shared.json
    toolchain-detection-unsupported.json
  ].freeze

  def package(name: "rust/app", language: "rust", build_files: nil)
    {
      "name" => name,
      "language" => language,
      "build_files" => build_files || {"BUILD" => ""}
    }
  end

  def evaluate(platform: "linux", force_full: false, packages: [package],
               scheduled_packages: nil, forced_toolchains: [])
    BuildTool::ToolchainDetection.evaluate_snapshot(
      platform,
      force_full,
      packages,
      scheduled_packages,
      forced_toolchains
    )
  end

  def test_independently_consumes_every_neutral_toolchain_fixture
    fixture_paths = Dir.glob(CONFORMANCE_CASES / "toolchain-detection-*.json").sort
    assert_equal EXPECTED_FIXTURES, fixture_paths.map { |path| File.basename(path) }

    fixture_paths.each do |path|
      fixture = JSON.parse(File.read(path, encoding: "UTF-8"))
      options = fixture.fetch("input").fetch("options")
      expected = fixture.fetch("expected")

      actual = BuildTool::ToolchainDetection.evaluate_snapshot(
        options.fetch("platform"),
        options.fetch("force_full"),
        options.fetch("packages"),
        options["scheduled_packages"],
        options.fetch("forced_toolchains")
      )

      assert_equal expected.fetch("outcome"), actual.fetch("outcome"), fixture.fetch("id")
      expected_toolchains = expected.fetch("result", {}).fetch("toolchains", {})
      assert_equal expected_toolchains, actual.fetch("toolchains"), fixture.fetch("id")
      assert_equal expected.fetch("diagnostics"), actual.fetch("diagnostics"), fixture.fetch("id")
    end
  end

  def test_enforces_exact_encoded_utf8_byte_ceiling
    assert_equal "ok", evaluate(packages: [package(build_files: {"BUILD" => "x" * 65_536})]).fetch("outcome")
    assert_equal "ok", evaluate(packages: [package(build_files: {"BUILD" => "é" * 32_768})]).fetch("outcome")

    error = assert_raises(ArgumentError) do
      evaluate(packages: [package(build_files: {"BUILD" => "x" * 65_537})])
    end
    assert_match(/per-file resource ceiling/, error.message)

    error = assert_raises(ArgumentError) do
      evaluate(packages: [package(build_files: {"BUILD" => "é" * 32_769})])
    end
    assert_match(/per-file resource ceiling/, error.message)
  end

  def test_transcodes_a_copy_before_counting_utf8_bytes
    content = ("é" * 32_769).encode(Encoding::ISO_8859_1)
    before = content.dup
    before_encoding = content.encoding

    error = assert_raises(ArgumentError) do
      evaluate(packages: [package(build_files: {"BUILD" => content})])
    end

    assert_match(/per-file resource ceiling/, error.message)
    assert_equal before, content
    assert_equal before_encoding, content.encoding
  end

  def test_enforces_exact_logical_line_ceiling
    assert_equal "ok", evaluate(packages: [package(build_files: {"BUILD" => "\n" * 4_095})]).fetch("outcome")

    error = assert_raises(ArgumentError) do
      evaluate(packages: [package(build_files: {"BUILD" => "\n" * 4_096})])
    end
    assert_match(/per-file resource ceiling/, error.message)
  end

  def test_enforces_aggregate_ceiling_across_every_front
    exact = (0...16).to_h { |index| ["BUILD_#{index}", "x" * 65_536] }
    assert_equal "ok", evaluate(packages: [package(build_files: exact)]).fetch("outcome")

    oversized = (0...17).to_h { |index| ["BUILD_#{index}", "x" * 65_536] }
    error = assert_raises(ArgumentError) do
      evaluate(packages: [package(build_files: oversized)])
    end
    assert_match(/aggregate resource ceiling/, error.message)

    error = assert_raises(ArgumentError) do
      evaluate(packages: [package(build_files: {
        "BUILD" => "",
        "BUILD_windows" => "x" * 65_537
      })])
    end
    assert_match(/per-file resource ceiling/, error.message)
  end

  def test_keeps_declaration_grammar_byte_exact_across_crlf_and_lone_cr
    declarations = BuildTool::ToolchainDetection.parse_extra_toolchains(
      "  # needs-toolchain: python  \r\n\t# needs-toolchain:\tjava\t\r\n"
    )
    assert_equal %w[python java], declarations
    assert_empty BuildTool::ToolchainDetection.parse_extra_toolchains("# needs-toolchain: python\r")
    assert_empty BuildTool::ToolchainDetection.parse_extra_toolchains("# needs-toolchain: lua\r  ")
    assert_empty BuildTool::ToolchainDetection.parse_extra_toolchains("# needs-toolchain: swift\r\r\n")
  end

  def test_stably_deduplicates_only_exact_canonical_declarations
    content = [
      "# needs-toolchain: python",
      "# needs-toolchain:\tjava",
      "# needs-toolchain: python",
      "# needs-toolchain: Python",
      "# needs-toolchain:zig",
      "# needs-toolchain: java suffix"
    ].join("\n")

    assert_equal %w[python java], BuildTool::ToolchainDetection.parse_extra_toolchains(content)
  end

  def test_preserves_empty_front_precedence_and_caller_owned_inputs
    packages = [package(build_files: {
      "BUILD" => "# needs-toolchain: java\n",
      "BUILD_windows" => ""
    })]
    before = Marshal.load(Marshal.dump(packages))

    actual = evaluate(platform: "windows", packages: packages, forced_toolchains: ["kotlin"])

    assert actual.fetch("toolchains").fetch("rust")
    assert actual.fetch("toolchains").fetch("kotlin")
    refute actual.fetch("toolchains").fetch("java")
    assert_equal before, packages
  end

  def test_nil_and_empty_schedules_remain_distinct
    packages = [package]

    all_packages = evaluate(packages: packages, scheduled_packages: nil)
    no_packages = evaluate(packages: packages, scheduled_packages: [])

    assert all_packages.fetch("toolchains").fetch("rust")
    refute no_packages.fetch("toolchains").values.any?
  end

  def test_registry_is_deeply_frozen_and_results_are_fresh_complete_maps
    registry = BuildTool::ToolchainDetection::CANONICAL_TOOLCHAINS
    assert registry.frozen?
    assert registry.all?(&:frozen?)
    assert_equal registry.sort, registry
    assert_equal 16, registry.length

    first = evaluate
    first.fetch("toolchains")["cpp"] = true
    second = evaluate

    refute_same first.fetch("toolchains"), second.fetch("toolchains")
    assert_equal registry, second.fetch("toolchains").keys
    refute second.fetch("toolchains").fetch("cpp")
  end

  def test_force_full_and_forced_toolchains_union_without_host_state
    full = evaluate(force_full: true, packages: [package], forced_toolchains: [])
    assert full.fetch("toolchains").values.all?

    forced = evaluate(packages: [package], scheduled_packages: [], forced_toolchains: %w[python java])
    assert forced.fetch("toolchains").fetch("python")
    assert forced.fetch("toolchains").fetch("java")
    assert_equal 2, forced.fetch("toolchains").values.count(true)
  end

  def test_keeps_unsupported_diagnostics_and_precedence_stable
    unsupported_package = evaluate(
      force_full: true,
      packages: [package(name: "zig/app", language: "zig")]
    )
    assert_equal [
      {"code" => "TOOLCHAIN_UNSUPPORTED", "severity" => "error", "package" => "zig/app"}
    ], unsupported_package.fetch("diagnostics")

    unsupported_forced = evaluate(
      packages: [package],
      scheduled_packages: [],
      forced_toolchains: ["zig"]
    )
    assert_equal [
      {"code" => "TOOLCHAIN_UNSUPPORTED", "severity" => "error"}
    ], unsupported_forced.fetch("diagnostics")

    both_invalid = evaluate(
      packages: [package(name: "zig/app", language: "zig")],
      forced_toolchains: ["zig"]
    )
    assert_equal [
      {"code" => "TOOLCHAIN_UNSUPPORTED", "severity" => "error", "package" => "zig/app"}
    ], both_invalid.fetch("diagnostics")
  end
end
