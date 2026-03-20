# frozen_string_literal: true

require "optparse"
require "json"

# ============================================================================
# CLI — Command-Line Interface for the Capability Analyzer
# ============================================================================
#
# Provides three commands:
#
#   detect  — Scan Ruby files for capability usage
#   check   — Compare detected capabilities against a manifest
#   banned  — Scan Ruby files for banned constructs
#
# ## Usage Examples
#
#   # Detect capabilities in a single file:
#   ca-capability-analyzer detect lib/my_module.rb
#
#   # Detect capabilities in a directory (excluding tests):
#   ca-capability-analyzer detect --exclude-tests lib/
#
#   # Check a directory against its manifest:
#   ca-capability-analyzer check --manifest required_capabilities.json lib/
#
#   # Scan for banned constructs:
#   ca-capability-analyzer banned lib/
#
#   # JSON output for CI integration:
#   ca-capability-analyzer detect --json lib/
# ============================================================================

module CA
  module CapabilityAnalyzer
    module CLI
      # Run the CLI with the given arguments.
      #
      # @param argv [Array<String>] command-line arguments.
      def self.run(argv)
        if argv.empty? || %w[-h --help help].include?(argv.first)
          print_usage
          return
        end

        command = argv.shift

        case command
        when "detect"
          run_detect(argv)
        when "check"
          run_check(argv)
        when "banned"
          run_banned(argv)
        else
          $stderr.puts "Unknown command: #{command}"
          $stderr.puts "Run with --help for usage."
          exit 1
        end
      end

      # ── detect command ─────────────────────────────────────────────

      def self.run_detect(argv)
        json_output = false
        exclude_tests = false

        parser = OptionParser.new do |opts|
          opts.banner = "Usage: ca-capability-analyzer detect [options] <path>"

          opts.on("--json", "Output results as JSON") do
            json_output = true
          end

          opts.on("--exclude-tests", "Skip test/ directories") do
            exclude_tests = true
          end
        end
        parser.parse!(argv)

        path = argv.first
        unless path
          $stderr.puts "Error: path argument required."
          $stderr.puts parser.help
          exit 1
        end

        detected = if File.directory?(path)
          CA::CapabilityAnalyzer.analyze_directory(path, exclude_tests: exclude_tests)
        else
          CA::CapabilityAnalyzer.analyze_file(path)
        end

        if json_output
          puts JSON.pretty_generate(detected.map(&:to_h))
        else
          if detected.empty?
            puts "No capabilities detected."
          else
            puts "Detected #{detected.length} capability(ies):\n\n"
            detected.each do |cap|
              puts "  #{cap.file}:#{cap.line}: #{cap} (#{cap.evidence})"
            end
          end
        end
      end

      # ── check command ──────────────────────────────────────────────

      def self.run_check(argv)
        manifest_path = nil
        exclude_tests = false

        parser = OptionParser.new do |opts|
          opts.banner = "Usage: ca-capability-analyzer check [options] <path>"

          opts.on("--manifest PATH", "Path to required_capabilities.json") do |p|
            manifest_path = p
          end

          opts.on("--exclude-tests", "Skip test/ directories") do
            exclude_tests = true
          end
        end
        parser.parse!(argv)

        path = argv.first
        unless path
          $stderr.puts "Error: path argument required."
          $stderr.puts parser.help
          exit 1
        end

        # Load manifest
        manifest = if manifest_path
          CA::CapabilityAnalyzer.load_manifest(manifest_path)
        else
          CA::CapabilityAnalyzer.default_manifest(path)
        end

        # Detect capabilities
        detected = if File.directory?(path)
          CA::CapabilityAnalyzer.analyze_directory(path, exclude_tests: exclude_tests)
        else
          CA::CapabilityAnalyzer.analyze_file(path)
        end

        # Compare
        result = CA::CapabilityAnalyzer.compare_capabilities(detected, manifest)
        puts result.summary

        exit(result.passed ? 0 : 1)
      end

      # ── banned command ─────────────────────────────────────────────

      def self.run_banned(argv)
        json_output = false

        parser = OptionParser.new do |opts|
          opts.banner = "Usage: ca-capability-analyzer banned [options] <path>"

          opts.on("--json", "Output results as JSON") do
            json_output = true
          end
        end
        parser.parse!(argv)

        path = argv.first
        unless path
          $stderr.puts "Error: path argument required."
          $stderr.puts parser.help
          exit 1
        end

        violations = if File.directory?(path)
          CA::CapabilityAnalyzer.detect_banned_in_directory(path)
        else
          CA::CapabilityAnalyzer.detect_banned(path)
        end

        if json_output
          data = violations.map do |v|
            {construct: v.construct, file: v.file, line: v.line, evidence: v.evidence}
          end
          puts JSON.pretty_generate(data)
        else
          if violations.empty?
            puts "No banned constructs detected."
          else
            puts "Found #{violations.length} banned construct(s):\n\n"
            violations.each do |v|
              puts "  #{v}"
            end
            exit 1
          end
        end
      end

      # ── Help text ──────────────────────────────────────────────────

      def self.print_usage
        puts <<~USAGE
          ca-capability-analyzer — Static capability analyzer for Ruby

          Commands:
            detect <path>   Scan Ruby files for capability usage
            check <path>    Compare detected capabilities against a manifest
            banned <path>   Scan Ruby files for banned constructs

          Options:
            --json            Output results as JSON
            --exclude-tests   Skip test/ directories
            --manifest PATH   Path to required_capabilities.json (check only)
            -h, --help        Show this help message

          Examples:
            ca-capability-analyzer detect lib/
            ca-capability-analyzer detect --json lib/my_module.rb
            ca-capability-analyzer check --manifest caps.json lib/
            ca-capability-analyzer banned lib/
        USAGE
      end
    end
  end
end
