# frozen_string_literal: true

# test_id.rb -- Tests for the Ruby id tool
# ==========================================
#
# === What These Tests Verify ===
#
# These tests exercise the id tool's user/group information lookup
# and formatting. We test:
# - get_user_info for current user
# - format_id_default output format
# - collect_user_groups
# - CLI Builder integration
# - Main function with various flags

require "simplecov"
SimpleCov.start do
  add_filter "/test/"
  enable_coverage :branch
end

require "minitest/autorun"
require "etc"
require "rbconfig"
require "coding_adventures_cli_builder"

require_relative "../id_tool"

if RbConfig::CONFIG['host_os'] !~ /mswin|mingw|cygwin/

# ---------------------------------------------------------------------------
# Helper module
# ---------------------------------------------------------------------------

module IdTestHelper
  ID_TEST_SPEC = File.join(File.dirname(__FILE__), "..", "id.json")

  def parse_id_argv(argv)
    CodingAdventures::CliBuilder::Parser.new(ID_TEST_SPEC, ["id"] + argv).parse
  end
end

# ===========================================================================
# Test: CLI Builder integration
# ===========================================================================

class TestIdCliIntegration < Minitest::Test
  include IdTestHelper

  def test_no_flags_returns_parse_result
    result = parse_id_argv([])
    assert_kind_of CodingAdventures::CliBuilder::ParseResult, result
  end

  def test_help_returns_help_result
    result = parse_id_argv(["--help"])
    assert_kind_of CodingAdventures::CliBuilder::HelpResult, result
    assert_includes result.text, "id"
  end

  def test_version_returns_version_result
    result = parse_id_argv(["--version"])
    assert_kind_of CodingAdventures::CliBuilder::VersionResult, result
    assert_equal "1.0.0", result.version
  end

  def test_user_flag
    result = parse_id_argv(["-u"])
    assert result.flags["user"]
  end

  def test_group_flag
    result = parse_id_argv(["-g"])
    assert result.flags["group"]
  end

  def test_groups_flag
    result = parse_id_argv(["-G"])
    assert result.flags["groups"]
  end
end

# ===========================================================================
# Test: get_user_info for current user
# ===========================================================================

class TestGetUserInfoCurrent < Minitest::Test
  def test_returns_hash
    info = get_user_info(nil)
    assert_kind_of Hash, info
  end

  def test_uid_matches_process
    info = get_user_info(nil)
    assert_equal Process.euid, info[:uid]
  end

  def test_gid_matches_process
    info = get_user_info(nil)
    assert_equal Process.egid, info[:gid]
  end

  def test_username_present
    info = get_user_info(nil)
    refute_nil info[:username]
    refute_empty info[:username]
  end

  def test_groupname_present
    info = get_user_info(nil)
    refute_nil info[:groupname]
    refute_empty info[:groupname]
  end

  def test_groups_present
    info = get_user_info(nil)
    assert_kind_of Array, info[:groups]
    refute_empty info[:groups]
  end

  def test_groups_are_pairs
    info = get_user_info(nil)
    info[:groups].each do |entry|
      assert_kind_of Array, entry
      assert_equal 2, entry.length
    end
  end
end

# ===========================================================================
# Test: get_user_info for specific user
# ===========================================================================

class TestGetUserInfoSpecific < Minitest::Test
  def test_current_user_by_name
    current_name = Etc.getpwuid(Process.euid).name
    info = get_user_info(current_name)
    refute_nil info
    assert_equal current_name, info[:username]
  end

  def test_nonexistent_user_returns_nil
    info = get_user_info("nonexistent_user_xyz_12345")
    assert_nil info
  end

  def test_nonexistent_user_prints_error
    _out, err = capture_io { get_user_info("nonexistent_user_xyz_12345") }
    assert_includes err, "no such user"
  end
end

# ===========================================================================
# Test: get_real_ids
# ===========================================================================

class TestGetRealIds < Minitest::Test
  def test_returns_hash
    ids = get_real_ids
    assert_kind_of Hash, ids
  end

  def test_has_uid
    ids = get_real_ids
    assert_kind_of Integer, ids[:uid]
  end

  def test_has_gid
    ids = get_real_ids
    assert_kind_of Integer, ids[:gid]
  end
end

# ===========================================================================
# Test: collect_user_groups
# ===========================================================================

class TestCollectUserGroups < Minitest::Test
  def test_includes_primary_group
    current_name = Etc.getpwuid(Process.euid).name
    primary_gid = Etc.getpwnam(current_name).gid
    groups = collect_user_groups(current_name, primary_gid)

    # Primary group should be first
    assert_equal primary_gid, groups[0][0]
  end

  def test_returns_array_of_pairs
    current_name = Etc.getpwuid(Process.euid).name
    primary_gid = Etc.getpwnam(current_name).gid
    groups = collect_user_groups(current_name, primary_gid)

    groups.each do |entry|
      assert_kind_of Array, entry
      assert_equal 2, entry.length
      assert_kind_of Integer, entry[0]
      assert_kind_of String, entry[1]
    end
  end
end

# ===========================================================================
# Test: format_id_default
# ===========================================================================

class TestFormatIdDefault < Minitest::Test
  def test_format_includes_uid
    info = {
      uid: 501, username: "alice",
      gid: 20, groupname: "staff",
      groups: [[20, "staff"], [80, "admin"]]
    }
    result = format_id_default(info)
    assert_includes result, "uid=501(alice)"
  end

  def test_format_includes_gid
    info = {
      uid: 501, username: "alice",
      gid: 20, groupname: "staff",
      groups: [[20, "staff"]]
    }
    result = format_id_default(info)
    assert_includes result, "gid=20(staff)"
  end

  def test_format_includes_groups
    info = {
      uid: 501, username: "alice",
      gid: 20, groupname: "staff",
      groups: [[20, "staff"], [80, "admin"]]
    }
    result = format_id_default(info)
    assert_includes result, "groups=20(staff),80(admin)"
  end

  def test_format_structure
    info = {
      uid: 1000, username: "bob",
      gid: 1000, groupname: "bob",
      groups: [[1000, "bob"]]
    }
    result = format_id_default(info)
    assert_match(/\Auid=\d+\(\w+\) gid=\d+\(\w+\) groups=/, result)
  end
end

# ===========================================================================
# Test: Main function integration
# ===========================================================================

class TestIdMainFunction < Minitest::Test
  include IdTestHelper

  def test_main_default_output
    old_argv = ARGV.dup
    ARGV.replace([])
    output = capture_io { id_main }[0]
    assert_includes output, "uid="
    assert_includes output, "gid="
    assert_includes output, "groups="
  ensure
    ARGV.replace(old_argv)
  end

  def test_main_user_flag
    old_argv = ARGV.dup
    ARGV.replace(["-u"])
    output = capture_io { id_main }[0].strip
    assert_match(/\A\d+\z/, output)
  ensure
    ARGV.replace(old_argv)
  end

  def test_main_user_name_flag
    old_argv = ARGV.dup
    ARGV.replace(["-u", "-n"])
    output = capture_io { id_main }[0].strip
    expected = Etc.getpwuid(Process.euid).name
    assert_equal expected, output
  ensure
    ARGV.replace(old_argv)
  end

  def test_main_group_flag
    old_argv = ARGV.dup
    ARGV.replace(["-g"])
    output = capture_io { id_main }[0].strip
    assert_match(/\A\d+\z/, output)
  ensure
    ARGV.replace(old_argv)
  end

  def test_main_groups_flag
    old_argv = ARGV.dup
    ARGV.replace(["-G"])
    output = capture_io { id_main }[0].strip
    # Should be space-separated group IDs
    parts = output.split(" ")
    assert parts.length >= 1
    parts.each { |p| assert_match(/\A\d+\z/, p) }
  ensure
    ARGV.replace(old_argv)
  end

  def test_main_help_exits_zero
    old_argv = ARGV.dup
    ARGV.replace(["--help"])
    err = assert_raises(SystemExit) { capture_io { id_main } }
    assert_equal 0, err.status
  ensure
    ARGV.replace(old_argv)
  end

  def test_main_version_exits_zero
    old_argv = ARGV.dup
    ARGV.replace(["--version"])
    err = assert_raises(SystemExit) { capture_io { id_main } }
    assert_equal 0, err.status
  ensure
    ARGV.replace(old_argv)
  end
end
end
