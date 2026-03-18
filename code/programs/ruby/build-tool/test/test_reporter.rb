# frozen_string_literal: true

# test_reporter.rb -- Tests for build report formatting
# =====================================================
#
# These tests verify the report table layout, status labels, duration
# formatting, and the summary line.

require_relative "test_helper"

class TestReporter < Minitest::Test
  # -- format_duration tests ---------------------------------------------------

  def test_format_duration_zero
    assert_equal "-", BuildTool::Reporter.format_duration(0.0)
  end

  def test_format_duration_negligible
    assert_equal "-", BuildTool::Reporter.format_duration(0.005)
  end

  def test_format_duration_normal
    assert_equal "2.3s", BuildTool::Reporter.format_duration(2.3)
  end

  def test_format_duration_large
    assert_equal "123.5s", BuildTool::Reporter.format_duration(123.456)
  end

  # -- format_report tests -----------------------------------------------------

  def test_format_report_empty
    report = BuildTool::Reporter.format_report({})
    assert_includes report, "No packages processed."
  end

  def test_format_report_built
    results = {
      "python/pkg-a" => BuildTool::BuildResult.new(
        package_name: "python/pkg-a", status: "built", duration: 1.5
      )
    }
    report = BuildTool::Reporter.format_report(results)

    assert_includes report, "Build Report"
    assert_includes report, "BUILT"
    assert_includes report, "1.5s"
    assert_includes report, "1 built"
  end

  def test_format_report_failed
    results = {
      "python/pkg-a" => BuildTool::BuildResult.new(
        package_name: "python/pkg-a", status: "failed", duration: 0.5
      )
    }
    report = BuildTool::Reporter.format_report(results)
    assert_includes report, "FAILED"
    assert_includes report, "1 failed"
  end

  def test_format_report_skipped
    results = {
      "python/pkg-a" => BuildTool::BuildResult.new(
        package_name: "python/pkg-a", status: "skipped"
      )
    }
    report = BuildTool::Reporter.format_report(results)
    assert_includes report, "SKIPPED"
    assert_includes report, "1 skipped"
  end

  def test_format_report_dep_skipped
    results = {
      "python/pkg-a" => BuildTool::BuildResult.new(
        package_name: "python/pkg-a", status: "dep-skipped"
      )
    }
    report = BuildTool::Reporter.format_report(results)
    assert_includes report, "DEP-SKIP"
    assert_includes report, "- (dep failed)"
    assert_includes report, "1 dep-skipped"
  end

  def test_format_report_would_build
    results = {
      "python/pkg-a" => BuildTool::BuildResult.new(
        package_name: "python/pkg-a", status: "would-build"
      )
    }
    report = BuildTool::Reporter.format_report(results)
    assert_includes report, "WOULD-BUILD"
    assert_includes report, "1 would-build"
  end

  def test_format_report_sorted_by_name
    results = {
      "python/zzz" => BuildTool::BuildResult.new(
        package_name: "python/zzz", status: "built", duration: 1.0
      ),
      "python/aaa" => BuildTool::BuildResult.new(
        package_name: "python/aaa", status: "built", duration: 2.0
      )
    }
    report = BuildTool::Reporter.format_report(results)
    aaa_pos = report.index("python/aaa")
    zzz_pos = report.index("python/zzz")
    assert aaa_pos < zzz_pos, "Expected python/aaa before python/zzz"
  end

  def test_format_report_mixed_statuses
    results = {
      "python/a" => BuildTool::BuildResult.new(package_name: "python/a", status: "built", duration: 1.0),
      "python/b" => BuildTool::BuildResult.new(package_name: "python/b", status: "skipped"),
      "python/c" => BuildTool::BuildResult.new(package_name: "python/c", status: "failed", duration: 0.5),
      "python/d" => BuildTool::BuildResult.new(package_name: "python/d", status: "dep-skipped")
    }
    report = BuildTool::Reporter.format_report(results)
    assert_includes report, "Total: 4 packages"
    assert_includes report, "1 built"
    assert_includes report, "1 skipped"
    assert_includes report, "1 failed"
    assert_includes report, "1 dep-skipped"
  end

  # -- print_report tests ------------------------------------------------------

  def test_print_report_writes_to_io
    results = {
      "test/pkg" => BuildTool::BuildResult.new(
        package_name: "test/pkg", status: "built", duration: 1.0
      )
    }
    output = StringIO.new
    BuildTool::Reporter.print_report(results, io: output)

    assert_includes output.string, "Build Report"
    assert_includes output.string, "BUILT"
  end
end
