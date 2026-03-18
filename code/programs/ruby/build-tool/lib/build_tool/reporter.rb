# frozen_string_literal: true

# reporter.rb -- Build Report Formatting
# =======================================
#
# This module formats and prints a summary table of build results. The output
# is human-readable and designed for terminal display.
#
# Output format
# -------------
#
#     Build Report
#     ============
#     Package                    Status     Duration
#     python/logic-gates         SKIPPED    -
#     python/arithmetic          BUILT      2.3s
#     python/arm-simulator       FAILED     0.5s
#     python/riscv-simulator     DEP-SKIP   - (dep failed)
#
#     Total: 21 packages | 5 built | 14 skipped | 1 failed | 1 dep-skipped

require "stringio"

module BuildTool
  module Reporter
    # STATUS_DISPLAY -- Human-readable names for each build status.
    #
    # The keys match the status strings used in BuildResult, and the values
    # are the uppercase labels shown in the report table.
    STATUS_DISPLAY = {
      "built"       => "BUILT",
      "failed"      => "FAILED",
      "skipped"     => "SKIPPED",
      "dep-skipped" => "DEP-SKIP",
      "would-build" => "WOULD-BUILD"
    }.freeze

    module_function

    # format_duration -- Format a duration for display.
    #
    # Returns "-" for zero/negligible durations (< 0.01s), otherwise "X.Ys".
    # This matches the Python implementation's formatting.
    #
    # @param seconds [Float] The duration in seconds.
    # @return [String]
    def format_duration(seconds)
      return "-" if seconds < 0.01

      format("%.1fs", seconds)
    end

    # format_report -- Format a build report as a string.
    #
    # Produces a table with Package, Status, and Duration columns, followed
    # by a summary line counting each status category. Results are sorted
    # by package name for consistent output.
    #
    # @param results [Hash<String, BuildResult>] Package name -> result.
    # @return [String] The formatted report.
    def format_report(results)
      buf = StringIO.new

      buf.puts
      buf.puts "Build Report"
      buf.puts "============"

      if results.empty?
        buf.puts "No packages processed."
        return buf.string
      end

      # Calculate column widths -- the package name column is at least as
      # wide as the word "Package" and at most as wide as the longest name.
      max_name_len = [results.keys.map(&:length).max, "Package".length].max

      # Header row.
      buf.puts format("%-#{max_name_len}s   %-12s %s", "Package", "Status", "Duration")

      # One row per package, sorted by name.
      results.keys.sort.each do |name|
        result = results[name]
        status = STATUS_DISPLAY.fetch(result.status, result.status.upcase)
        duration = format_duration(result.duration)
        duration = "- (dep failed)" if result.status == "dep-skipped"

        buf.puts format("%-#{max_name_len}s   %-12s %s", name, status, duration)
      end

      # Summary line with counts for each status category.
      total       = results.size
      built       = results.values.count { |r| r.status == "built" }
      skipped     = results.values.count { |r| r.status == "skipped" }
      failed      = results.values.count { |r| r.status == "failed" }
      dep_skipped = results.values.count { |r| r.status == "dep-skipped" }
      would_build = results.values.count { |r| r.status == "would-build" }

      buf.print "\nTotal: #{total} packages"
      buf.print " | #{built} built"          if built > 0
      buf.print " | #{skipped} skipped"      if skipped > 0
      buf.print " | #{failed} failed"        if failed > 0
      buf.print " | #{dep_skipped} dep-skipped"  if dep_skipped > 0
      buf.print " | #{would_build} would-build"  if would_build > 0
      buf.puts

      buf.string
    end

    # print_report -- Print a summary table of build results.
    #
    # @param results [Hash<String, BuildResult>] Package name -> result.
    # @param io [IO] Output stream (defaults to $stdout).
    def print_report(results, io: $stdout)
      io.print format_report(results)
    end
  end
end
