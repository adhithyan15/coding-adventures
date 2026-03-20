# frozen_string_literal: true

require "test_helper"
require "tempfile"
require "json"

# ============================================================================
# Tests for the Manifest Loader and Capability Comparison
# ============================================================================
#
# These tests verify:
# 1. Loading manifests from JSON files
# 2. Creating default (empty) manifests
# 3. Target matching (glob-style)
# 4. Capability matching (category + action + target)
# 5. Full comparison: detected vs declared capabilities
# ============================================================================

class TestManifest < Minitest::Test
  # Helper: create a DetectedCapability for testing.
  def cap(category:, action:, target:, file: "test.rb", line: 1, evidence: "test")
    CA::CapabilityAnalyzer::DetectedCapability.new(
      category: category,
      action: action,
      target: target,
      file: file,
      line: line,
      evidence: evidence
    )
  end

  # Helper: create a temporary JSON manifest file.
  def with_manifest(data)
    file = Tempfile.new(["manifest", ".json"])
    file.write(JSON.generate(data))
    file.close
    yield file.path
  ensure
    file.unlink
  end

  # ── Manifest Loading ─────────────────────────────────────────────

  def test_load_manifest_from_json
    data = {
      "package" => "ruby/my-package",
      "capabilities" => [
        {"category" => "fs", "action" => "read", "target" => "*"}
      ],
      "justification" => "Reads config files."
    }

    with_manifest(data) do |path|
      manifest = CA::CapabilityAnalyzer.load_manifest(path)
      assert_equal "ruby/my-package", manifest.package
      assert_equal 1, manifest.capabilities.length
      assert_equal "Reads config files.", manifest.justification
      assert_equal path, manifest.path
    end
  end

  def test_load_manifest_with_banned_exceptions
    data = {
      "package" => "ruby/template-engine",
      "capabilities" => [],
      "justification" => "Template engine.",
      "banned_construct_exceptions" => [
        {"construct" => "eval", "justification" => "Needed for template compilation."}
      ]
    }

    with_manifest(data) do |path|
      manifest = CA::CapabilityAnalyzer.load_manifest(path)
      assert_equal 1, manifest.banned_construct_exceptions.length
      assert_equal "eval", manifest.banned_construct_exceptions.first["construct"]
    end
  end

  def test_load_manifest_missing_file_raises
    assert_raises(Errno::ENOENT) do
      CA::CapabilityAnalyzer.load_manifest("/nonexistent/path.json")
    end
  end

  def test_load_manifest_invalid_json_raises
    file = Tempfile.new(["bad", ".json"])
    file.write("not valid json {{{")
    file.close

    assert_raises(JSON::ParserError) do
      CA::CapabilityAnalyzer.load_manifest(file.path)
    end
  ensure
    file.unlink
  end

  # ── Default Manifest ─────────────────────────────────────────────

  def test_default_manifest_has_zero_capabilities
    manifest = CA::CapabilityAnalyzer.default_manifest("ruby/test-pkg")
    assert_equal "ruby/test-pkg", manifest.package
    assert_empty manifest.capabilities
    assert manifest.empty?
    assert_nil manifest.path
  end

  # ── Manifest Empty Check ─────────────────────────────────────────

  def test_manifest_with_capabilities_is_not_empty
    manifest = CA::CapabilityAnalyzer::Manifest.new(
      package: "test",
      capabilities: [{"category" => "fs", "action" => "read", "target" => "*"}],
      justification: "test"
    )
    refute manifest.empty?
  end

  # ── Target Matching ──────────────────────────────────────────────

  def test_wildcard_pattern_matches_anything
    assert CA::CapabilityAnalyzer.target_matches?("*", "anything")
  end

  def test_wildcard_actual_matches_any_pattern
    assert CA::CapabilityAnalyzer.target_matches?("specific.txt", "*")
  end

  def test_exact_match
    assert CA::CapabilityAnalyzer.target_matches?("config.yml", "config.yml")
  end

  def test_exact_mismatch
    refute CA::CapabilityAnalyzer.target_matches?("config.yml", "other.yml")
  end

  def test_glob_pattern_matches
    assert CA::CapabilityAnalyzer.target_matches?("*.yml", "config.yml")
  end

  def test_glob_pattern_no_match
    refute CA::CapabilityAnalyzer.target_matches?("*.yml", "config.json")
  end

  # ── Capability Matching ──────────────────────────────────────────

  def test_exact_capability_match
    declared = {"category" => "fs", "action" => "read", "target" => "config.yml"}
    detected = cap(category: "fs", action: "read", target: "config.yml")
    assert CA::CapabilityAnalyzer.capability_matches?(declared, detected)
  end

  def test_wildcard_action_matches
    declared = {"category" => "fs", "action" => "*", "target" => "*"}
    detected = cap(category: "fs", action: "write", target: "output.txt")
    assert CA::CapabilityAnalyzer.capability_matches?(declared, detected)
  end

  def test_different_category_no_match
    declared = {"category" => "net", "action" => "read", "target" => "*"}
    detected = cap(category: "fs", action: "read", target: "file.txt")
    refute CA::CapabilityAnalyzer.capability_matches?(declared, detected)
  end

  def test_different_action_no_match
    declared = {"category" => "fs", "action" => "read", "target" => "*"}
    detected = cap(category: "fs", action: "write", target: "file.txt")
    refute CA::CapabilityAnalyzer.capability_matches?(declared, detected)
  end

  def test_glob_target_match
    declared = {"category" => "fs", "action" => "read", "target" => "*.yml"}
    detected = cap(category: "fs", action: "read", target: "config.yml")
    assert CA::CapabilityAnalyzer.capability_matches?(declared, detected)
  end

  # ── Full Comparison ──────────────────────────────────────────────

  def test_all_capabilities_declared_passes
    manifest = CA::CapabilityAnalyzer::Manifest.new(
      package: "test",
      capabilities: [
        {"category" => "fs", "action" => "read", "target" => "*"}
      ],
      justification: "test"
    )

    detected = [cap(category: "fs", action: "read", target: "config.yml")]

    result = CA::CapabilityAnalyzer.compare_capabilities(detected, manifest)
    assert result.passed
    assert_empty result.errors
    assert_equal 1, result.matched.length
  end

  def test_undeclared_capability_fails
    manifest = CA::CapabilityAnalyzer::Manifest.new(
      package: "test",
      capabilities: [
        {"category" => "fs", "action" => "read", "target" => "*"}
      ],
      justification: "test"
    )

    detected = [
      cap(category: "fs", action: "read", target: "config.yml"),
      cap(category: "net", action: "connect", target: "*")
    ]

    result = CA::CapabilityAnalyzer.compare_capabilities(detected, manifest)
    refute result.passed
    assert_equal 1, result.errors.length
    assert_equal "net", result.errors.first.category
  end

  def test_unused_declaration_is_warning
    manifest = CA::CapabilityAnalyzer::Manifest.new(
      package: "test",
      capabilities: [
        {"category" => "fs", "action" => "read", "target" => "*"},
        {"category" => "net", "action" => "connect", "target" => "*"}
      ],
      justification: "test"
    )

    detected = [cap(category: "fs", action: "read", target: "config.yml")]

    result = CA::CapabilityAnalyzer.compare_capabilities(detected, manifest)
    assert result.passed
    assert_equal 1, result.warnings.length
    assert_equal "net", result.warnings.first["category"]
  end

  def test_empty_manifest_any_capability_is_error
    manifest = CA::CapabilityAnalyzer.default_manifest("test")
    detected = [cap(category: "fs", action: "read", target: "file.txt")]

    result = CA::CapabilityAnalyzer.compare_capabilities(detected, manifest)
    refute result.passed
    assert_equal 1, result.errors.length
  end

  def test_no_detections_passes
    manifest = CA::CapabilityAnalyzer::Manifest.new(
      package: "test",
      capabilities: [
        {"category" => "fs", "action" => "read", "target" => "*"}
      ],
      justification: "test"
    )

    result = CA::CapabilityAnalyzer.compare_capabilities([], manifest)
    assert result.passed
    assert_empty result.errors
    assert_equal 1, result.warnings.length  # unused declaration
  end

  def test_empty_manifest_no_detections_passes
    manifest = CA::CapabilityAnalyzer.default_manifest("test")
    result = CA::CapabilityAnalyzer.compare_capabilities([], manifest)
    assert result.passed
    assert_empty result.errors
    assert_empty result.warnings
  end

  # ── Summary Output ───────────────────────────────────────────────

  def test_summary_pass
    result = CA::CapabilityAnalyzer::ComparisonResult.new(
      passed: true,
      errors: [],
      warnings: [],
      matched: [cap(category: "fs", action: "read", target: "file.txt")]
    )
    summary = result.summary
    assert_includes summary, "PASS"
    assert_includes summary, "1 capability(ies)"
  end

  def test_summary_fail
    result = CA::CapabilityAnalyzer::ComparisonResult.new(
      passed: false,
      errors: [cap(category: "net", action: "connect", target: "*")],
      warnings: [],
      matched: []
    )
    summary = result.summary
    assert_includes summary, "FAIL"
    assert_includes summary, "1 undeclared"
  end

  def test_summary_with_warnings
    result = CA::CapabilityAnalyzer::ComparisonResult.new(
      passed: true,
      errors: [],
      warnings: [{"category" => "net", "action" => "connect", "target" => "*"}],
      matched: []
    )
    summary = result.summary
    assert_includes summary, "Unused declarations"
    assert_includes summary, "net:connect:*"
  end

  # ── Multiple Declaration Matching ────────────────────────────────

  def test_multiple_declarations_can_cover_different_capabilities
    manifest = CA::CapabilityAnalyzer::Manifest.new(
      package: "test",
      capabilities: [
        {"category" => "fs", "action" => "read", "target" => "*"},
        {"category" => "fs", "action" => "write", "target" => "output.txt"},
        {"category" => "net", "action" => "connect", "target" => "*"}
      ],
      justification: "test"
    )

    detected = [
      cap(category: "fs", action: "read", target: "config.yml"),
      cap(category: "fs", action: "write", target: "output.txt"),
      cap(category: "net", action: "connect", target: "*")
    ]

    result = CA::CapabilityAnalyzer.compare_capabilities(detected, manifest)
    assert result.passed
    assert_equal 3, result.matched.length
    assert_empty result.warnings
  end
end
