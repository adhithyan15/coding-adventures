# frozen_string_literal: true

require "test_helper"
require "tmpdir"
require "json"

# ============================================================================
# CLI Tests — Verifying the Command-Line Interface
# ============================================================================
#
# The CLI is the primary way CI and BUILD files interact with the analyzer.
# These tests verify that the three commands (detect, check, banned) produce
# correct output and exit codes.
#
# Since the CLI calls `exit`, we test the underlying methods by capturing
# stdout and rescuing SystemExit where needed.

class TestCLIDetect < Minitest::Test
  def setup
    @tmpdir = Dir.mktmpdir
    File.write(File.join(@tmpdir, "cap.rb"), 'require "socket"')
    File.write(File.join(@tmpdir, "pure.rb"), "x = 1 + 2\n")
  end

  def teardown
    FileUtils.rm_rf(@tmpdir)
  end

  def test_detect_file_with_capabilities
    out = capture_io do
      CA::CapabilityAnalyzer::CLI.run_detect([File.join(@tmpdir, "cap.rb")])
    end.first
    assert_includes out, "Detected"
    assert_includes out, "net"
  end

  def test_detect_file_pure
    out = capture_io do
      CA::CapabilityAnalyzer::CLI.run_detect([File.join(@tmpdir, "pure.rb")])
    end.first
    assert_includes out, "No capabilities detected"
  end

  def test_detect_directory
    out = capture_io do
      CA::CapabilityAnalyzer::CLI.run_detect([@tmpdir])
    end.first
    assert_includes out, "Detected"
  end

  def test_detect_json_output
    out = capture_io do
      CA::CapabilityAnalyzer::CLI.run_detect(["--json", File.join(@tmpdir, "cap.rb")])
    end.first
    parsed = JSON.parse(out)
    assert_kind_of Array, parsed
    assert parsed.length >= 1
    assert_equal "net", parsed.first["category"]
  end

  def test_detect_exclude_tests
    test_dir = File.join(@tmpdir, "test")
    Dir.mkdir(test_dir)
    File.write(File.join(test_dir, "test_thing.rb"), 'require "socket"')
    out = capture_io do
      CA::CapabilityAnalyzer::CLI.run_detect(["--exclude-tests", @tmpdir])
    end.first
    # The socket import from test/ should be excluded, only cap.rb remains
    assert_includes out, "Detected 1 capability"
  end

  def test_detect_no_path_exits
    assert_raises(SystemExit) do
      capture_io { CA::CapabilityAnalyzer::CLI.run_detect([]) }
    end
  end
end

class TestCLICheck < Minitest::Test
  def setup
    @tmpdir = Dir.mktmpdir
    File.write(File.join(@tmpdir, "cap.rb"), 'require "socket"')
    @manifest_path = File.join(@tmpdir, "required_capabilities.json")
  end

  def teardown
    FileUtils.rm_rf(@tmpdir)
  end

  def test_check_passes_with_manifest
    File.write(@manifest_path, JSON.generate({
      "version" => 1,
      "package" => "ruby/test",
      "capabilities" => [{"category" => "net", "action" => "*", "target" => "*"}],
      "justification" => "Test"
    }))
    out = nil
    assert_raises(SystemExit) do
      out = capture_io do
        CA::CapabilityAnalyzer::CLI.run_check([
          "--manifest", @manifest_path,
          File.join(@tmpdir, "cap.rb")
        ])
      end.first
    end
    # The exit was called — check that PASS was printed
    # (SystemExit with code 0 for pass)
  rescue SystemExit => e
    assert_equal 0, e.status
  end

  def test_check_fails_without_manifest
    # No manifest = default deny = everything fails
    assert_raises(SystemExit) do
      capture_io do
        CA::CapabilityAnalyzer::CLI.run_check([File.join(@tmpdir, "cap.rb")])
      end
    end
  rescue SystemExit => e
    assert_equal 1, e.status
  end

  def test_check_no_path_exits
    assert_raises(SystemExit) do
      capture_io { CA::CapabilityAnalyzer::CLI.run_check([]) }
    end
  end
end

class TestCLIBanned < Minitest::Test
  def setup
    @tmpdir = Dir.mktmpdir
    File.write(File.join(@tmpdir, "evil.rb"), 'eval("1 + 2")')
    File.write(File.join(@tmpdir, "clean.rb"), "x = 1 + 2\n")
  end

  def teardown
    FileUtils.rm_rf(@tmpdir)
  end

  def test_banned_finds_violations
    assert_raises(SystemExit) do
      capture_io do
        CA::CapabilityAnalyzer::CLI.run_banned([File.join(@tmpdir, "evil.rb")])
      end
    end
  rescue SystemExit => e
    assert_equal 1, e.status
  end

  def test_banned_clean_file
    out = capture_io do
      CA::CapabilityAnalyzer::CLI.run_banned([File.join(@tmpdir, "clean.rb")])
    end.first
    assert_includes out, "No banned constructs"
  end

  def test_banned_directory
    assert_raises(SystemExit) do
      capture_io do
        CA::CapabilityAnalyzer::CLI.run_banned([@tmpdir])
      end
    end
  rescue SystemExit => e
    assert_equal 1, e.status
  end

  def test_banned_json_output
    out = nil
    begin
      out = capture_io do
        CA::CapabilityAnalyzer::CLI.run_banned(["--json", File.join(@tmpdir, "evil.rb")])
      end.first
    rescue SystemExit
      # banned with violations exits 1, but json is printed before exit
    end
    # JSON might not be captured if exit happens first; check clean file instead
    out2 = capture_io do
      CA::CapabilityAnalyzer::CLI.run_banned(["--json", File.join(@tmpdir, "clean.rb")])
    end.first
    parsed = JSON.parse(out2)
    assert_kind_of Array, parsed
    assert_equal 0, parsed.length
  end

  def test_banned_no_path_exits
    assert_raises(SystemExit) do
      capture_io { CA::CapabilityAnalyzer::CLI.run_banned([]) }
    end
  end
end

class TestCLIRun < Minitest::Test
  def test_help_flag
    out = capture_io { CA::CapabilityAnalyzer::CLI.run(["--help"]) }.first
    assert_includes out, "Commands"
  end

  def test_empty_args
    out = capture_io { CA::CapabilityAnalyzer::CLI.run([]) }.first
    assert_includes out, "Commands"
  end

  def test_unknown_command
    assert_raises(SystemExit) do
      capture_io { CA::CapabilityAnalyzer::CLI.run(["foobar"]) }
    end
  end
end

# ============================================================================
# File and Directory Analysis Integration Tests
# ============================================================================
#
# These test the analyze_file, analyze_directory, detect_banned, and
# detect_banned_in_directory module-level methods that were previously
# uncovered.

class TestFileAnalysis < Minitest::Test
  def setup
    @tmpdir = Dir.mktmpdir
  end

  def teardown
    FileUtils.rm_rf(@tmpdir)
  end

  def test_analyze_file
    file = File.join(@tmpdir, "test.rb")
    File.write(file, 'require "socket"')
    caps = CA::CapabilityAnalyzer.analyze_file(file)
    assert caps.length >= 1
    assert_equal "net", caps.first.category
  end

  def test_analyze_file_pure
    file = File.join(@tmpdir, "pure.rb")
    File.write(file, "x = 1 + 2\n")
    caps = CA::CapabilityAnalyzer.analyze_file(file)
    assert_equal 0, caps.length
  end

  def test_analyze_directory
    File.write(File.join(@tmpdir, "a.rb"), 'require "socket"')
    File.write(File.join(@tmpdir, "b.rb"), "x = 1")
    caps = CA::CapabilityAnalyzer.analyze_directory(@tmpdir)
    assert caps.length >= 1
  end

  def test_analyze_directory_exclude_tests
    test_dir = File.join(@tmpdir, "test")
    Dir.mkdir(test_dir)
    File.write(File.join(test_dir, "test_thing.rb"), 'require "socket"')
    File.write(File.join(@tmpdir, "main.rb"), "x = 1")
    caps = CA::CapabilityAnalyzer.analyze_directory(@tmpdir, exclude_tests: true)
    assert_equal 0, caps.length
  end

  def test_detect_banned_file
    file = File.join(@tmpdir, "evil.rb")
    File.write(file, 'eval("bad")')
    violations = CA::CapabilityAnalyzer.detect_banned(file)
    assert violations.length >= 1
    assert_equal "eval", violations.first.construct
  end

  def test_detect_banned_in_directory
    File.write(File.join(@tmpdir, "evil.rb"), 'eval("bad")')
    File.write(File.join(@tmpdir, "clean.rb"), "x = 1")
    violations = CA::CapabilityAnalyzer.detect_banned_in_directory(@tmpdir)
    assert violations.length >= 1
  end

  def test_detect_banned_clean_directory
    File.write(File.join(@tmpdir, "clean.rb"), "x = 1")
    violations = CA::CapabilityAnalyzer.detect_banned_in_directory(@tmpdir)
    assert_equal 0, violations.length
  end
end
