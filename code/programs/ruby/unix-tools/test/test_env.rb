# frozen_string_literal: true

# test_env.rb -- Tests for the Ruby env tool
# ============================================
#
# === What These Tests Verify ===
#
# These tests exercise the env tool's environment manipulation
# and command execution. We test:
# - Parsing variable assignments vs command arguments
# - Building modified environments
# - Printing the environment
# - Ignore environment (-i)
# - Unset variables (-u)
# - Null-terminated output (-0)
# - Command execution with modified environment
# - CLI Builder integration

require "simplecov"
SimpleCov.start do
  add_filter "/test/"
  enable_coverage :branch
end

require "minitest/autorun"
require "tmpdir"
require "fileutils"
require "coding_adventures_cli_builder"

require_relative "../env_tool"

# ---------------------------------------------------------------------------
# Helper module
# ---------------------------------------------------------------------------

module EnvTestHelper
  ENV_TEST_SPEC = File.join(File.dirname(__FILE__), "..", "env.json")

  def parse_env_argv(argv)
    CodingAdventures::CliBuilder::Parser.new(ENV_TEST_SPEC, ["env"] + argv).parse
  end
end

# ===========================================================================
# Test: CLI Builder integration
# ===========================================================================

class TestEnvCliIntegration < Minitest::Test
  include EnvTestHelper

  def test_help_returns_help_result
    result = parse_env_argv(["--help"])
    assert_kind_of CodingAdventures::CliBuilder::HelpResult, result
  end

  def test_version_returns_version_result
    result = parse_env_argv(["--version"])
    assert_kind_of CodingAdventures::CliBuilder::VersionResult, result
    assert_equal "1.0.0", result.version
  end

  def test_ignore_environment_flag
    result = parse_env_argv(["-i"])
    assert result.flags["ignore_environment"]
  end

  def test_null_flag
    result = parse_env_argv(["-0"])
    assert result.flags["null"]
  end
end

# ===========================================================================
# Test: env_parse_assignments
# ===========================================================================

class TestEnvParseAssignments < Minitest::Test
  def test_no_args
    assignments, command = env_parse_assignments([])
    assert_empty assignments
    assert_empty command
  end

  def test_only_assignments
    assignments, command = env_parse_assignments(["FOO=bar", "BAZ=qux"])
    assert_equal({ "FOO" => "bar", "BAZ" => "qux" }, assignments)
    assert_empty command
  end

  def test_only_command
    assignments, command = env_parse_assignments(["ls", "-la"])
    assert_empty assignments
    assert_equal ["ls", "-la"], command
  end

  def test_assignments_then_command
    assignments, command = env_parse_assignments(["FOO=bar", "ls", "-la"])
    assert_equal({ "FOO" => "bar" }, assignments)
    assert_equal ["ls", "-la"], command
  end

  def test_assignment_with_equals_in_value
    assignments, command = env_parse_assignments(["PATH=/usr/bin:/bin"])
    assert_equal({ "PATH" => "/usr/bin:/bin" }, assignments)
    assert_empty command
  end

  def test_empty_value
    assignments, command = env_parse_assignments(["FOO="])
    assert_equal({ "FOO" => "" }, assignments)
  end

  def test_command_after_assignments_gets_all_remaining
    assignments, command = env_parse_assignments(["FOO=1", "cmd", "FOO=2"])
    assert_equal({ "FOO" => "1" }, assignments)
    assert_equal ["cmd", "FOO=2"], command
  end
end

# ===========================================================================
# Test: env_build_environment
# ===========================================================================

class TestEnvBuildEnvironment < Minitest::Test
  def test_empty_base_with_assignments
    env = env_build_environment({}, { "FOO" => "bar" })
    assert_equal "bar", env["FOO"]
  end

  def test_base_env_is_preserved
    base = { "HOME" => "/home/user", "PATH" => "/usr/bin" }
    env = env_build_environment(base, {})
    assert_equal "/home/user", env["HOME"]
    assert_equal "/usr/bin", env["PATH"]
  end

  def test_assignment_overrides_base
    base = { "FOO" => "old" }
    env = env_build_environment(base, { "FOO" => "new" })
    assert_equal "new", env["FOO"]
  end

  def test_unset_removes_variable
    base = { "FOO" => "bar", "BAZ" => "qux" }
    env = env_build_environment(base, {}, ["FOO"])
    refute env.key?("FOO")
    assert_equal "qux", env["BAZ"]
  end

  def test_unset_then_set
    base = { "FOO" => "old" }
    # Unset happens first, then assignment overrides
    env = env_build_environment(base, { "FOO" => "new" }, ["FOO"])
    assert_equal "new", env["FOO"]
  end

  def test_unset_nonexistent_is_harmless
    base = { "FOO" => "bar" }
    env = env_build_environment(base, {}, ["NONEXISTENT"])
    assert_equal "bar", env["FOO"]
  end

  def test_does_not_mutate_base
    base = { "FOO" => "bar" }
    env_build_environment(base, { "NEW" => "val" })
    refute base.key?("NEW")
  end
end

# ===========================================================================
# Test: env_print_environment
# ===========================================================================

class TestEnvPrintEnvironment < Minitest::Test
  def test_basic_output
    env = { "FOO" => "bar", "BAZ" => "qux" }
    output = env_print_environment(env)
    assert_includes output, "FOO=bar"
    assert_includes output, "BAZ=qux"
  end

  def test_newline_terminated
    env = { "FOO" => "bar" }
    output = env_print_environment(env)
    assert output.end_with?("\n")
  end

  def test_null_terminated
    env = { "FOO" => "bar", "BAZ" => "qux" }
    output = env_print_environment(env, null_terminated: true)
    assert_includes output, "\0"
    refute output.end_with?("\n")
    parts = output.split("\0")
    assert_includes parts, "FOO=bar"
    assert_includes parts, "BAZ=qux"
  end

  def test_empty_environment
    output = env_print_environment({})
    assert_equal "\n", output
  end

  def test_empty_environment_null
    output = env_print_environment({}, null_terminated: true)
    assert_equal "", output
  end
end

# ===========================================================================
# Test: env_execute
# ===========================================================================

class TestEnvExecute < Minitest::Test
  def test_successful_command
    code = env_execute(ENV.to_h, ["true"])
    assert_equal 0, code
  end

  def test_failing_command
    code = env_execute(ENV.to_h, ["false"])
    assert_equal 1, code
  end

  def test_nonexistent_command
    code = env_execute(ENV.to_h, ["nonexistent_command_xyz_12345"])
    assert_equal 127, code
  end

  def test_command_sees_custom_env
    # Run ruby to print an env var
    code = env_execute(
      { "TEST_VAR_XYZ" => "hello_from_env" },
      ["ruby", "-e", 'exit(ENV["TEST_VAR_XYZ"] == "hello_from_env" ? 0 : 1)']
    )
    assert_equal 0, code
  end

  def test_chdir_option
    Dir.mktmpdir("env_test") do |tmpdir|
      # Use File.realpath to handle macOS /private/var symlinks
      real_tmpdir = File.realpath(tmpdir)
      code = env_execute(
        ENV.to_h,
        ["ruby", "-e", "exit(File.realpath(Dir.pwd) == '#{real_tmpdir}' ? 0 : 1)"],
        chdir: tmpdir
      )
      assert_equal 0, code
    end
  end

  def test_empty_environment
    # Run with empty env -- command should still work
    code = env_execute({}, ["true"])
    assert_equal 0, code
  end

  def test_permission_denied
    Dir.mktmpdir("env_test") do |tmpdir|
      script = File.join(tmpdir, "noperm.sh")
      File.write(script, "#!/bin/sh\necho hi")
      File.chmod(0o000, script)
      code = env_execute(ENV.to_h, [script])
      assert_equal 126, code
      File.chmod(0o644, script)  # cleanup
    end
  end
end

# ===========================================================================
# Test: Integration -- full flow
# ===========================================================================

class TestEnvIntegration < Minitest::Test
  def test_print_environment_includes_known_var
    env = { "MY_TEST_VAR" => "hello123" }
    output = env_print_environment(env)
    assert_includes output, "MY_TEST_VAR=hello123"
  end

  def test_build_and_print_flow
    base = { "A" => "1", "B" => "2", "C" => "3" }
    env = env_build_environment(base, { "D" => "4" }, ["B"])
    output = env_print_environment(env)
    assert_includes output, "A=1"
    refute_includes output, "B=2"
    assert_includes output, "C=3"
    assert_includes output, "D=4"
  end

  def test_ignore_environment_with_assignments
    env = env_build_environment({}, { "ONLY_THIS" => "yes" })
    output = env_print_environment(env)
    assert_includes output, "ONLY_THIS=yes"
    # Should be the only variable
    lines = output.strip.split("\n")
    assert_equal 1, lines.length
  end

  def test_null_terminated_output_no_trailing_newline
    env = { "X" => "1" }
    output = env_print_environment(env, null_terminated: true)
    assert_equal "X=1\0", output
  end
end
