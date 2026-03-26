# frozen_string_literal: true

# test_chown.rb -- Tests for the Ruby chown tool
# ================================================
#
# === What These Tests Verify ===
#
# These tests exercise the chown tool's owner/group parsing and
# (limited) application. Since chown typically requires root privileges,
# most tests focus on parsing logic and graceful error handling.
#
# We test:
# - Owner:group string parsing (all forms)
# - UID/GID resolution
# - Graceful permission error handling
# - Verbose/changes output format
# - Recursive traversal logic
# - CLI Builder integration
#
# === Testing Strategy ===
#
# Because chown requires root to change ownership, we test:
# 1. Pure parsing functions -- no privileges needed
# 2. Applying chown to files WE own (setting to our own uid/gid)
# 3. Verifying permission errors are handled gracefully

require "simplecov"
SimpleCov.start do
  add_filter "/test/"
  enable_coverage :branch
end

require "minitest/autorun"
require "tmpdir"
require "fileutils"
require "etc"
require "coding_adventures_cli_builder"

require_relative "../chown_tool"

# ---------------------------------------------------------------------------
# Helper module
# ---------------------------------------------------------------------------

module ChownTestHelper
  CHOWN_TEST_SPEC = File.join(File.dirname(__FILE__), "..", "chown.json")

  def parse_chown_argv(argv)
    CodingAdventures::CliBuilder::Parser.new(CHOWN_TEST_SPEC, ["chown"] + argv).parse
  end

  # Current user's uid and gid for testing
  def current_uid
    Process.uid
  end

  def current_gid
    Process.gid
  end

  def current_username
    Etc.getpwuid(current_uid).name
  end
end

# ===========================================================================
# Test: CLI Builder integration
# ===========================================================================

class TestChownCliIntegration < Minitest::Test
  include ChownTestHelper

  def test_help_returns_help_result
    result = parse_chown_argv(["--help"])
    assert_kind_of CodingAdventures::CliBuilder::HelpResult, result
  end

  def test_version_returns_version_result
    result = parse_chown_argv(["--version"])
    assert_kind_of CodingAdventures::CliBuilder::VersionResult, result
    assert_equal "1.0.0", result.version
  end

  def test_recursive_flag
    result = parse_chown_argv(["-R", "user", "file"])
    assert result.flags["recursive"]
  end

  def test_verbose_flag
    result = parse_chown_argv(["-v", "user", "file"])
    assert result.flags["verbose"]
  end

  def test_no_dereference_flag
    result = parse_chown_argv(["-h", "user", "file"])
    assert result.flags["no_dereference"]
  end
end

# ===========================================================================
# Test: chown_parse_owner_group
# ===========================================================================

class TestChownParseOwnerGroup < Minitest::Test
  def test_owner_only
    result = chown_parse_owner_group("alice")
    assert_equal "alice", result[:owner]
    assert_nil result[:group]
  end

  def test_owner_colon_group
    result = chown_parse_owner_group("alice:staff")
    assert_equal "alice", result[:owner]
    assert_equal "staff", result[:group]
  end

  def test_owner_colon_only
    result = chown_parse_owner_group("alice:")
    assert_equal "alice", result[:owner]
    assert_equal :login_group, result[:group]
  end

  def test_colon_group_only
    result = chown_parse_owner_group(":staff")
    assert_nil result[:owner]
    assert_equal "staff", result[:group]
  end

  def test_owner_dot_group
    result = chown_parse_owner_group("alice.staff")
    assert_equal "alice", result[:owner]
    assert_equal "staff", result[:group]
  end

  def test_numeric_uid
    result = chown_parse_owner_group("1000")
    assert_equal "1000", result[:owner]
    assert_nil result[:group]
  end

  def test_numeric_uid_colon_gid
    result = chown_parse_owner_group("1000:20")
    assert_equal "1000", result[:owner]
    assert_equal "20", result[:group]
  end

  def test_colon_only
    result = chown_parse_owner_group(":")
    assert_nil result[:owner]
    assert_nil result[:group]
  end
end

# ===========================================================================
# Test: chown_resolve_uid
# ===========================================================================

class TestChownResolveUid < Minitest::Test
  include ChownTestHelper

  def test_nil_owner
    uid, err = chown_resolve_uid(nil)
    assert_nil uid
    assert_nil err
  end

  def test_numeric_uid
    uid, err = chown_resolve_uid("0")
    assert_equal 0, uid
    assert_nil err
  end

  def test_current_user_by_name
    uid, err = chown_resolve_uid(current_username)
    assert_equal current_uid, uid
    assert_nil err
  end

  def test_invalid_username
    uid, err = chown_resolve_uid("nonexistent_user_xyz_12345")
    assert_nil uid
    assert_includes err, "invalid user"
  end
end

# ===========================================================================
# Test: chown_resolve_gid
# ===========================================================================

class TestChownResolveGid < Minitest::Test
  def test_nil_group
    gid, err = chown_resolve_gid(nil)
    assert_nil gid
    assert_nil err
  end

  def test_numeric_gid
    gid, err = chown_resolve_gid("0")
    assert_equal 0, gid
    assert_nil err
  end

  def test_login_group_symbol
    gid, err = chown_resolve_gid(:login_group)
    assert_nil gid
    assert_nil err
  end

  def test_invalid_group
    gid, err = chown_resolve_gid("nonexistent_group_xyz_12345")
    assert_nil gid
    assert_includes err, "invalid group"
  end
end

# ===========================================================================
# Test: chown_apply
# ===========================================================================

class TestChownApply < Minitest::Test
  include ChownTestHelper

  def setup
    @tmpdir = Dir.mktmpdir("chown_test")
  end

  def teardown
    FileUtils.rm_rf(@tmpdir)
  end

  def test_apply_to_own_file
    path = File.join(@tmpdir, "file.txt")
    File.write(path, "hello")

    # Set to our own uid/gid -- should succeed without root
    msg, ok = chown_apply(path, current_uid, current_gid)
    assert ok
  end

  def test_apply_verbose
    path = File.join(@tmpdir, "file.txt")
    File.write(path, "hello")

    msg, ok = chown_apply(path, current_uid, current_gid, verbose: true)
    assert ok
    refute_nil msg
    assert_includes msg, "ownership of"
  end

  def test_apply_changes_when_unchanged
    path = File.join(@tmpdir, "file.txt")
    File.write(path, "hello")

    msg, ok = chown_apply(path, current_uid, current_gid, changes: true)
    assert ok
    # Setting to same owner/group = no change reported
    assert_nil msg
  end

  def test_nonexistent_file
    path = File.join(@tmpdir, "nonexistent.txt")
    msg, ok = chown_apply(path, current_uid, current_gid)
    refute ok
    assert_includes msg, "No such file"
  end

  def test_nonexistent_file_silent
    path = File.join(@tmpdir, "nonexistent.txt")
    msg, ok = chown_apply(path, current_uid, current_gid, silent: true)
    refute ok
    assert_nil msg
  end

  def test_nil_uid_and_gid
    path = File.join(@tmpdir, "file.txt")
    File.write(path, "hello")

    # Passing nil for both should use -1 (no change)
    msg, ok = chown_apply(path, nil, nil)
    assert ok
  end
end

# ===========================================================================
# Test: chown_recursive
# ===========================================================================

class TestChownRecursive < Minitest::Test
  include ChownTestHelper

  def setup
    @tmpdir = Dir.mktmpdir("chown_rec_test")
    @dir = File.join(@tmpdir, "testdir")
    FileUtils.mkdir_p(@dir)
    @file1 = File.join(@dir, "file1.txt")
    @file2 = File.join(@dir, "file2.txt")
    File.write(@file1, "hello")
    File.write(@file2, "world")
  end

  def teardown
    FileUtils.rm_rf(@tmpdir)
  end

  def test_recursive_applies_to_all
    msgs, ok = chown_recursive(@dir, current_uid, current_gid)
    assert ok
  end

  def test_recursive_verbose
    msgs, ok = chown_recursive(@dir, current_uid, current_gid, verbose: true)
    assert ok
    # Should have messages for dir + 2 files
    assert msgs.length >= 3
  end

  def test_recursive_with_subdirectory
    subdir = File.join(@dir, "sub")
    FileUtils.mkdir_p(subdir)
    File.write(File.join(subdir, "deep.txt"), "deep")

    msgs, ok = chown_recursive(@dir, current_uid, current_gid)
    assert ok
  end

  def test_recursive_on_single_file
    msgs, ok = chown_recursive(@file1, current_uid, current_gid)
    assert ok
  end
end

# ===========================================================================
# Test: Symlink handling
# ===========================================================================

class TestChownSymlink < Minitest::Test
  include ChownTestHelper

  def setup
    @tmpdir = Dir.mktmpdir("chown_symlink_test")
    @target = File.join(@tmpdir, "target.txt")
    @link = File.join(@tmpdir, "link.txt")
    File.write(@target, "hello")
    File.symlink(@target, @link)
  end

  def teardown
    FileUtils.rm_rf(@tmpdir)
  end

  def test_apply_to_symlink_follows_by_default
    msg, ok = chown_apply(@link, current_uid, current_gid)
    assert ok
  end

  def test_apply_to_symlink_no_dereference
    msg, ok = chown_apply(@link, current_uid, current_gid, no_dereference: true)
    assert ok
  end
end

# ===========================================================================
# Test: chown_parse_owner_group edge cases
# ===========================================================================

class TestChownParseOwnerGroupEdgeCases < Minitest::Test
  def test_dot_separator_without_group
    # "alice." is similar to "alice:" -- owner with login group
    result = chown_parse_owner_group("alice.")
    assert_equal "alice", result[:owner]
    assert_equal :login_group, result[:group]
  end

  def test_only_dot
    result = chown_parse_owner_group(".")
    assert_nil result[:owner]
    assert_nil result[:group]
  end

  def test_colon_takes_priority_over_dot
    # "alice:staff.dev" -- colon splits first
    result = chown_parse_owner_group("alice:staff.dev")
    assert_equal "alice", result[:owner]
    assert_equal "staff.dev", result[:group]
  end
end

# ===========================================================================
# Test: chown_resolve with real system data
# ===========================================================================

class TestChownResolveRealSystem < Minitest::Test
  include ChownTestHelper

  def test_resolve_root_uid
    uid, err = chown_resolve_uid("root")
    assert_equal 0, uid
    assert_nil err
  end

  def test_resolve_gid_for_wheel_or_root
    # On macOS it's "wheel", on Linux it's "root"
    begin
      gr = Etc.getgrnam("wheel")
      gid, err = chown_resolve_gid("wheel")
      assert_equal gr.gid, gid
      assert_nil err
    rescue ArgumentError
      gr = Etc.getgrnam("root")
      gid, err = chown_resolve_gid("root")
      assert_equal gr.gid, gid
      assert_nil err
    end
  end

  def test_resolve_large_numeric_uid
    uid, err = chown_resolve_uid("99999")
    assert_equal 99999, uid
    assert_nil err
  end

  def test_resolve_large_numeric_gid
    gid, err = chown_resolve_gid("99999")
    assert_equal 99999, gid
    assert_nil err
  end
end

# ===========================================================================
# Test: chown_apply verbose messages
# ===========================================================================

class TestChownApplyMessages < Minitest::Test
  include ChownTestHelper

  def setup
    @tmpdir = Dir.mktmpdir("chown_msg_test")
    @file = File.join(@tmpdir, "test.txt")
    File.write(@file, "hello")
  end

  def teardown
    FileUtils.rm_rf(@tmpdir)
  end

  def test_verbose_retained_message
    msg, ok = chown_apply(@file, current_uid, current_gid, verbose: true)
    assert ok
    assert_includes msg, "retained"
  end

  def test_no_verbose_no_message
    msg, ok = chown_apply(@file, current_uid, current_gid)
    assert ok
    assert_nil msg
  end

  def test_nil_uid_nil_gid_verbose
    msg, ok = chown_apply(@file, nil, nil, verbose: true)
    assert ok
    assert_includes msg, "retained"
  end
end
