# frozen_string_literal: true

# test_uname.rb -- Tests for the Ruby uname tool
# =================================================
#
# === What These Tests Verify ===
#
# These tests exercise the uname tool's system information gathering
# and formatting. We test:
# - get_system_info returns a populated hash
# - format_uname with various flag combinations
# - Default behavior (kernel name only)
# - -a flag (all information)
# - Individual flags (-s, -n, -r, -v, -m, -p, -i, -o)

require "simplecov"
SimpleCov.start do
  add_filter "/test/"
  enable_coverage :branch
end

require "minitest/autorun"
require "etc"
require "socket"
require "coding_adventures_cli_builder"

require_relative "../uname_tool"

# ---------------------------------------------------------------------------
# Helper module
# ---------------------------------------------------------------------------

module UnameTestHelper
  UNAME_TEST_SPEC = File.join(File.dirname(__FILE__), "..", "uname.json")

  def parse_uname_argv(argv)
    CodingAdventures::CliBuilder::Parser.new(UNAME_TEST_SPEC, ["uname"] + argv).parse
  end
end

# ===========================================================================
# Test: CLI Builder integration
# ===========================================================================

class TestUnameCliIntegration < Minitest::Test
  include UnameTestHelper

  def test_no_flags_returns_parse_result
    result = parse_uname_argv([])
    assert_kind_of CodingAdventures::CliBuilder::ParseResult, result
  end

  def test_help_returns_help_result
    result = parse_uname_argv(["--help"])
    assert_kind_of CodingAdventures::CliBuilder::HelpResult, result
    assert_includes result.text, "uname"
  end

  def test_version_returns_version_result
    result = parse_uname_argv(["--version"])
    assert_kind_of CodingAdventures::CliBuilder::VersionResult, result
    assert_equal "1.0.0", result.version
  end

  def test_all_flag
    result = parse_uname_argv(["-a"])
    assert result.flags["all"]
  end

  def test_kernel_name_flag
    result = parse_uname_argv(["-s"])
    assert result.flags["kernel_name"]
  end

  def test_nodename_flag
    result = parse_uname_argv(["-n"])
    assert result.flags["nodename"]
  end
end

# ===========================================================================
# Test: get_system_info
# ===========================================================================

class TestGetSystemInfo < Minitest::Test
  def test_returns_hash
    info = get_system_info
    assert_kind_of Hash, info
  end

  def test_kernel_name_present
    info = get_system_info
    refute_nil info[:kernel_name]
    refute_empty info[:kernel_name]
  end

  def test_nodename_present
    info = get_system_info
    refute_nil info[:nodename]
    refute_empty info[:nodename]
  end

  def test_kernel_release_present
    info = get_system_info
    refute_nil info[:kernel_release]
  end

  def test_kernel_version_present
    info = get_system_info
    refute_nil info[:kernel_version]
  end

  def test_machine_present
    info = get_system_info
    refute_nil info[:machine]
    refute_empty info[:machine]
  end

  def test_processor_present
    info = get_system_info
    refute_nil info[:processor]
  end

  def test_operating_system_present
    info = get_system_info
    refute_nil info[:operating_system]
    refute_empty info[:operating_system]
  end

  def test_nodename_matches_hostname
    info = get_system_info
    # Should match Socket.gethostname or Etc.uname[:nodename]
    expected = if Etc.respond_to?(:uname)
                 Etc.uname[:nodename]
               else
                 Socket.gethostname
               end
    assert_equal expected, info[:nodename]
  end
end

# ===========================================================================
# Test: detect_kernel_name
# ===========================================================================

class TestDetectKernelName < Minitest::Test
  def test_returns_string
    result = detect_kernel_name
    assert_kind_of String, result
  end

  def test_known_platform
    # Should return a recognized kernel name on any standard platform
    known = ["Darwin", "Linux", "Windows_NT", "FreeBSD", "unknown"]
    assert_includes known, detect_kernel_name
  end
end

# ===========================================================================
# Test: detect_machine
# ===========================================================================

class TestDetectMachine < Minitest::Test
  def test_returns_string
    result = detect_machine
    assert_kind_of String, result
  end

  def test_known_architecture
    known = ["arm64", "x86_64", "i686", "unknown"]
    assert_includes known, detect_machine
  end
end

# ===========================================================================
# Test: detect_operating_system
# ===========================================================================

class TestDetectOperatingSystem < Minitest::Test
  def test_linux
    assert_equal "GNU/Linux", detect_operating_system("Linux")
  end

  def test_darwin
    assert_equal "Darwin", detect_operating_system("Darwin")
  end

  def test_freebsd
    assert_equal "FreeBSD", detect_operating_system("FreeBSD")
  end

  def test_unknown
    assert_equal "UnknownOS", detect_operating_system("UnknownOS")
  end
end

# ===========================================================================
# Test: format_uname
# ===========================================================================

class TestFormatUname < Minitest::Test
  def setup
    @info = {
      kernel_name: "TestOS",
      nodename: "testhost",
      kernel_release: "5.0.0",
      kernel_version: "#1 SMP",
      machine: "x86_64",
      processor: "x86_64",
      hardware_platform: "x86_64",
      operating_system: "TestOS"
    }
  end

  def test_default_prints_kernel_name
    result = format_uname(@info, {})
    assert_equal "TestOS", result
  end

  def test_all_flag
    result = format_uname(@info, {"all" => true})
    # All 8 fields are present, joined by spaces. Note that kernel_version
    # may itself contain spaces (e.g., "#1 SMP"), so we check starts/ends.
    assert result.start_with?("TestOS testhost 5.0.0")
    assert result.end_with?("x86_64 TestOS")
    assert_includes result, "#1 SMP"
  end

  def test_kernel_name_flag
    result = format_uname(@info, {"kernel_name" => true})
    assert_equal "TestOS", result
  end

  def test_nodename_flag
    result = format_uname(@info, {"nodename" => true})
    assert_equal "testhost", result
  end

  def test_kernel_release_flag
    result = format_uname(@info, {"kernel_release" => true})
    assert_equal "5.0.0", result
  end

  def test_machine_flag
    result = format_uname(@info, {"machine" => true})
    assert_equal "x86_64", result
  end

  def test_operating_system_flag
    result = format_uname(@info, {"operating_system" => true})
    assert_equal "TestOS", result
  end

  def test_multiple_flags
    result = format_uname(@info, {"kernel_name" => true, "nodename" => true})
    assert_equal "TestOS testhost", result
  end
end

# ===========================================================================
# Test: no_uname_flags? helper
# ===========================================================================

class TestNoUnameFlags < Minitest::Test
  def test_no_flags
    assert no_uname_flags?({})
  end

  def test_with_all
    refute no_uname_flags?({"all" => true})
  end

  def test_with_kernel_name
    refute no_uname_flags?({"kernel_name" => true})
  end
end

# ===========================================================================
# Test: Main function integration
# ===========================================================================

class TestUnameMainFunction < Minitest::Test
  include UnameTestHelper

  def test_main_default_output
    old_argv = ARGV.dup
    ARGV.replace([])
    output = capture_io { uname_main }[0].strip
    refute_empty output
  ensure
    ARGV.replace(old_argv)
  end

  def test_main_all_flag
    old_argv = ARGV.dup
    ARGV.replace(["-a"])
    output = capture_io { uname_main }[0].strip
    # All output should have multiple space-separated fields
    assert output.split(" ").length >= 4
  ensure
    ARGV.replace(old_argv)
  end

  def test_main_help_exits_zero
    old_argv = ARGV.dup
    ARGV.replace(["--help"])
    err = assert_raises(SystemExit) { capture_io { uname_main } }
    assert_equal 0, err.status
  ensure
    ARGV.replace(old_argv)
  end

  def test_main_version_exits_zero
    old_argv = ARGV.dup
    ARGV.replace(["--version"])
    err = assert_raises(SystemExit) { capture_io { uname_main } }
    assert_equal 0, err.status
  ensure
    ARGV.replace(old_argv)
  end
end
