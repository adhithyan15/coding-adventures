# frozen_string_literal: true

# test_groups.rb -- Tests for the Ruby groups tool
# ===================================================
#
# === What These Tests Verify ===
#
# These tests exercise the groups tool's user group lookup and CLI
# Builder integration. We test:
# - get_user_groups for current user
# - get_user_groups for a specific user
# - Error handling for nonexistent users
# - Main function output

require "simplecov"
SimpleCov.start do
  add_filter "/test/"
  enable_coverage :branch
end

require "minitest/autorun"
require "etc"
require "rbconfig"
require "coding_adventures_cli_builder"

require_relative "../groups_tool"

if RbConfig::CONFIG['host_os'] !~ /mswin|mingw|cygwin/

# ---------------------------------------------------------------------------
# Helper module
# ---------------------------------------------------------------------------

module GroupsTestHelper
  GROUPS_TEST_SPEC = File.join(File.dirname(__FILE__), "..", "groups.json")

  def parse_groups_argv(argv)
    CodingAdventures::CliBuilder::Parser.new(GROUPS_TEST_SPEC, ["groups"] + argv).parse
  end
end

# ===========================================================================
# Test: CLI Builder integration
# ===========================================================================

class TestGroupsCliIntegration < Minitest::Test
  include GroupsTestHelper

  def test_no_flags_returns_parse_result
    result = parse_groups_argv([])
    assert_kind_of CodingAdventures::CliBuilder::ParseResult, result
  end

  def test_help_returns_help_result
    result = parse_groups_argv(["--help"])
    assert_kind_of CodingAdventures::CliBuilder::HelpResult, result
    assert_includes result.text, "groups"
  end

  def test_version_returns_version_result
    result = parse_groups_argv(["--version"])
    assert_kind_of CodingAdventures::CliBuilder::VersionResult, result
    assert_equal "1.0.0", result.version
  end
end

# ===========================================================================
# Test: get_user_groups for current user
# ===========================================================================

class TestGetUserGroupsCurrent < Minitest::Test
  def test_returns_array
    groups = get_user_groups(nil)
    assert_kind_of Array, groups
  end

  def test_returns_non_empty
    groups = get_user_groups(nil)
    refute_empty groups
  end

  def test_returns_strings
    groups = get_user_groups(nil)
    groups.each { |g| assert_kind_of String, g }
  end

  def test_includes_at_least_one_group
    groups = get_user_groups(nil)
    assert groups.length >= 1
  end
end

# ===========================================================================
# Test: get_user_groups for specific user
# ===========================================================================

class TestGetUserGroupsSpecific < Minitest::Test
  def test_current_user_by_name
    current_name = Etc.getpwuid(Process.euid).name
    groups = get_user_groups(current_name)
    refute_nil groups
    refute_empty groups
  end

  def test_includes_primary_group
    current_name = Etc.getpwuid(Process.euid).name
    primary_gid = Etc.getpwnam(current_name).gid
    primary_name = Etc.getgrgid(primary_gid).name
    groups = get_user_groups(current_name)
    assert_includes groups, primary_name
  end

  def test_nonexistent_user_returns_nil
    groups = get_user_groups("nonexistent_user_xyz_12345")
    assert_nil groups
  end

  def test_nonexistent_user_prints_error
    _out, err = capture_io { get_user_groups("nonexistent_user_xyz_12345") }
    assert_includes err, "no such user"
  end
end

# ===========================================================================
# Test: Main function integration
# ===========================================================================

class TestGroupsMainFunction < Minitest::Test
  include GroupsTestHelper

  def test_main_no_args
    old_argv = ARGV.dup
    ARGV.replace([])
    output = capture_io { groups_main }[0].strip
    refute_empty output
    # Should be space-separated group names
    parts = output.split(" ")
    assert parts.length >= 1
  ensure
    ARGV.replace(old_argv)
  end

  def test_main_current_user
    current_name = Etc.getpwuid(Process.euid).name
    old_argv = ARGV.dup
    ARGV.replace([current_name])
    output = capture_io { groups_main }[0].strip
    refute_empty output
  ensure
    ARGV.replace(old_argv)
  end

  def test_main_nonexistent_user
    old_argv = ARGV.dup
    ARGV.replace(["nonexistent_user_xyz_12345"])
    _out, err = capture_io { groups_main }
    assert_includes err, "no such user"
  ensure
    ARGV.replace(old_argv)
  end

  def test_main_help_exits_zero
    old_argv = ARGV.dup
    ARGV.replace(["--help"])
    err = assert_raises(SystemExit) { capture_io { groups_main } }
    assert_equal 0, err.status
  ensure
    ARGV.replace(old_argv)
  end

  def test_main_version_exits_zero
    old_argv = ARGV.dup
    ARGV.replace(["--version"])
    err = assert_raises(SystemExit) { capture_io { groups_main } }
    assert_equal 0, err.status
  ensure
    ARGV.replace(old_argv)
  end
end
end
