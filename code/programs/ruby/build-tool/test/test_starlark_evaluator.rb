# frozen_string_literal: true

# test_starlark_evaluator.rb -- Tests for Starlark BUILD file evaluation
# =======================================================================
#
# These tests verify three main areas:
#
#   1. Detection: starlark_build?() correctly distinguishes Starlark BUILD
#      files from shell BUILD files based on the first significant line.
#
#   2. Command generation: generate_commands() maps each rule type to the
#      correct shell commands, matching the Go build tool's behavior.
#
#   3. Target extraction: extract_targets() converts the _targets variable
#      from the interpreter's result into Target structs.

require_relative "test_helper"
require_relative "../lib/build_tool/starlark_evaluator"

class TestStarlarkEvaluator < Minitest::Test
  include TestHelper

  # ==========================================================================
  # starlark_build? detection tests
  # ==========================================================================
  #
  # The detection heuristic looks at the first non-blank, non-comment line
  # of the BUILD file. If it matches a Starlark pattern (load(), def, or a
  # known rule call), the file is Starlark. Otherwise it's shell.

  def test_detects_load_statement_as_starlark
    content = <<~BUILD
      load("//rules.star", "py_library")

      py_library(name = "foo")
    BUILD
    assert BuildTool::StarlarkEvaluator.starlark_build?(content)
  end

  def test_detects_def_statement_as_starlark
    content = <<~BUILD
      def custom_rule(name):
          pass
    BUILD
    assert BuildTool::StarlarkEvaluator.starlark_build?(content)
  end

  def test_detects_py_library_as_starlark
    content = <<~BUILD
      py_library(name = "logic-gates", srcs = ["src/**/*.py"])
    BUILD
    assert BuildTool::StarlarkEvaluator.starlark_build?(content)
  end

  def test_detects_py_binary_as_starlark
    content = "py_binary(name = \"build-tool\")\n"
    assert BuildTool::StarlarkEvaluator.starlark_build?(content)
  end

  def test_detects_go_library_as_starlark
    content = "go_library(name = \"directed-graph\")\n"
    assert BuildTool::StarlarkEvaluator.starlark_build?(content)
  end

  def test_detects_go_binary_as_starlark
    content = "go_binary(name = \"build-tool\")\n"
    assert BuildTool::StarlarkEvaluator.starlark_build?(content)
  end

  def test_detects_ruby_library_as_starlark
    content = "ruby_library(name = \"logic_gates\")\n"
    assert BuildTool::StarlarkEvaluator.starlark_build?(content)
  end

  def test_detects_ruby_binary_as_starlark
    content = "ruby_binary(name = \"build-tool\")\n"
    assert BuildTool::StarlarkEvaluator.starlark_build?(content)
  end

  def test_detects_ts_library_as_starlark
    content = "ts_library(name = \"lexer\")\n"
    assert BuildTool::StarlarkEvaluator.starlark_build?(content)
  end

  def test_detects_ts_binary_as_starlark
    content = "ts_binary(name = \"app\")\n"
    assert BuildTool::StarlarkEvaluator.starlark_build?(content)
  end

  def test_detects_rust_library_as_starlark
    content = "rust_library(name = \"vm\")\n"
    assert BuildTool::StarlarkEvaluator.starlark_build?(content)
  end

  def test_detects_rust_binary_as_starlark
    content = "rust_binary(name = \"compiler\")\n"
    assert BuildTool::StarlarkEvaluator.starlark_build?(content)
  end

  def test_detects_elixir_library_as_starlark
    content = "elixir_library(name = \"parser\")\n"
    assert BuildTool::StarlarkEvaluator.starlark_build?(content)
  end

  def test_detects_elixir_binary_as_starlark
    content = "elixir_binary(name = \"server\")\n"
    assert BuildTool::StarlarkEvaluator.starlark_build?(content)
  end

  def test_detects_shell_echo_as_not_starlark
    content = "echo \"hello world\"\n"
    refute BuildTool::StarlarkEvaluator.starlark_build?(content)
  end

  def test_detects_shell_pip_as_not_starlark
    content = "pip install -e .\npytest\n"
    refute BuildTool::StarlarkEvaluator.starlark_build?(content)
  end

  def test_detects_shell_bundle_as_not_starlark
    content = "bundle install --quiet\nbundle exec rake test\n"
    refute BuildTool::StarlarkEvaluator.starlark_build?(content)
  end

  def test_detects_shell_go_as_not_starlark
    content = "go test ./... -v -cover\n"
    refute BuildTool::StarlarkEvaluator.starlark_build?(content)
  end

  def test_skips_comments_and_blanks_before_detecting
    # Comments and blank lines at the top should be ignored. The first
    # significant line is "py_library(" which is Starlark.
    content = <<~BUILD
      # This is a comment
      # Another comment

      py_library(name = "foo")
    BUILD
    assert BuildTool::StarlarkEvaluator.starlark_build?(content)
  end

  def test_skips_comments_before_shell
    # Comments followed by a shell command should detect as shell.
    content = <<~BUILD
      # Build script
      echo "building"
    BUILD
    refute BuildTool::StarlarkEvaluator.starlark_build?(content)
  end

  def test_empty_content_is_not_starlark
    refute BuildTool::StarlarkEvaluator.starlark_build?("")
  end

  def test_only_comments_is_not_starlark
    content = "# just a comment\n# another\n"
    refute BuildTool::StarlarkEvaluator.starlark_build?(content)
  end

  def test_only_blanks_is_not_starlark
    content = "\n\n  \n\t\n"
    refute BuildTool::StarlarkEvaluator.starlark_build?(content)
  end

  # ==========================================================================
  # generate_commands tests
  # ==========================================================================
  #
  # Each rule type maps to specific shell commands. These tests verify
  # every supported rule type and the fallback for unknown rules.

  def test_generate_commands_py_library_default_pytest
    target = BuildTool::Target.new(rule: "py_library", name: "logic-gates")
    commands = BuildTool::StarlarkEvaluator.generate_commands(target)

    assert_equal 2, commands.size
    assert_equal 'uv pip install --system -e ".[dev]"', commands[0]
    assert_equal "python -m pytest --cov --cov-report=term-missing", commands[1]
  end

  def test_generate_commands_py_library_explicit_pytest
    target = BuildTool::Target.new(
      rule: "py_library", name: "foo", test_runner: "pytest"
    )
    commands = BuildTool::StarlarkEvaluator.generate_commands(target)

    assert_equal 2, commands.size
    assert_includes commands[1], "pytest"
  end

  def test_generate_commands_py_library_unittest
    target = BuildTool::Target.new(
      rule: "py_library", name: "foo", test_runner: "unittest"
    )
    commands = BuildTool::StarlarkEvaluator.generate_commands(target)

    assert_equal 2, commands.size
    assert_includes commands[1], "unittest discover"
  end

  def test_generate_commands_py_binary
    target = BuildTool::Target.new(rule: "py_binary", name: "build-tool")
    commands = BuildTool::StarlarkEvaluator.generate_commands(target)

    assert_equal 2, commands.size
    assert_includes commands[0], "uv pip install"
    assert_includes commands[1], "pytest"
  end

  def test_generate_commands_go_library
    target = BuildTool::Target.new(rule: "go_library", name: "graph")
    commands = BuildTool::StarlarkEvaluator.generate_commands(target)

    assert_equal 3, commands.size
    assert_equal "go build ./...", commands[0]
    assert_equal "go test ./... -v -cover", commands[1]
    assert_equal "go vet ./...", commands[2]
  end

  def test_generate_commands_go_binary
    target = BuildTool::Target.new(rule: "go_binary", name: "build-tool")
    commands = BuildTool::StarlarkEvaluator.generate_commands(target)

    assert_equal 3, commands.size
    assert_equal "go build ./...", commands[0]
  end

  def test_generate_commands_ruby_library
    target = BuildTool::Target.new(rule: "ruby_library", name: "logic_gates")
    commands = BuildTool::StarlarkEvaluator.generate_commands(target)

    assert_equal 2, commands.size
    assert_equal "bundle install --quiet", commands[0]
    assert_equal "bundle exec rake test", commands[1]
  end

  def test_generate_commands_ruby_binary
    target = BuildTool::Target.new(rule: "ruby_binary", name: "tool")
    commands = BuildTool::StarlarkEvaluator.generate_commands(target)

    assert_equal 2, commands.size
    assert_equal "bundle install --quiet", commands[0]
  end

  def test_generate_commands_ts_library
    target = BuildTool::Target.new(rule: "ts_library", name: "lexer")
    commands = BuildTool::StarlarkEvaluator.generate_commands(target)

    assert_equal 2, commands.size
    assert_equal "npm install --silent", commands[0]
    assert_equal "npx vitest run --coverage", commands[1]
  end

  def test_generate_commands_ts_binary
    target = BuildTool::Target.new(rule: "ts_binary", name: "app")
    commands = BuildTool::StarlarkEvaluator.generate_commands(target)

    assert_equal 2, commands.size
    assert_equal "npm install --silent", commands[0]
  end

  def test_generate_commands_rust_library
    target = BuildTool::Target.new(rule: "rust_library", name: "vm")
    commands = BuildTool::StarlarkEvaluator.generate_commands(target)

    assert_equal 2, commands.size
    assert_equal "cargo build", commands[0]
    assert_equal "cargo test", commands[1]
  end

  def test_generate_commands_rust_binary
    target = BuildTool::Target.new(rule: "rust_binary", name: "tool")
    commands = BuildTool::StarlarkEvaluator.generate_commands(target)

    assert_equal 2, commands.size
    assert_equal "cargo build", commands[0]
  end

  def test_generate_commands_elixir_library
    target = BuildTool::Target.new(rule: "elixir_library", name: "parser")
    commands = BuildTool::StarlarkEvaluator.generate_commands(target)

    assert_equal 2, commands.size
    assert_equal "mix deps.get", commands[0]
    assert_equal "mix test --cover", commands[1]
  end

  def test_generate_commands_elixir_binary
    target = BuildTool::Target.new(rule: "elixir_binary", name: "server")
    commands = BuildTool::StarlarkEvaluator.generate_commands(target)

    assert_equal 2, commands.size
    assert_equal "mix deps.get", commands[0]
  end

  def test_generate_commands_unknown_rule
    target = BuildTool::Target.new(rule: "haskell_library", name: "weird")
    commands = BuildTool::StarlarkEvaluator.generate_commands(target)

    assert_equal 1, commands.size
    assert_includes commands[0], "Unknown rule: haskell_library"
  end

  # ==========================================================================
  # Target struct tests
  # ==========================================================================
  #
  # Verify that Target is an immutable Data.define value object with
  # sensible defaults.

  def test_target_defaults
    target = BuildTool::Target.new(rule: "py_library", name: "foo")

    assert_equal "py_library", target.rule
    assert_equal "foo", target.name
    assert_equal [], target.srcs
    assert_equal [], target.deps
    assert_equal "", target.test_runner
    assert_equal "", target.entry_point
  end

  def test_target_with_all_fields
    target = BuildTool::Target.new(
      rule: "go_binary",
      name: "build-tool",
      srcs: ["*.go"],
      deps: ["go/directed-graph"],
      test_runner: "go-test",
      entry_point: "main.go"
    )

    assert_equal "go_binary", target.rule
    assert_equal "build-tool", target.name
    assert_equal ["*.go"], target.srcs
    assert_equal ["go/directed-graph"], target.deps
    assert_equal "go-test", target.test_runner
    assert_equal "main.go", target.entry_point
  end

  def test_build_file_result_defaults
    result = BuildTool::BuildFileResult.new
    assert_equal [], result.targets
  end

  def test_build_file_result_with_targets
    targets = [BuildTool::Target.new(rule: "py_library", name: "a")]
    result = BuildTool::BuildFileResult.new(targets: targets)
    assert_equal 1, result.targets.size
  end

  # ==========================================================================
  # extract_targets tests
  # ==========================================================================
  #
  # These test the private extract_targets method by calling it via send().
  # We test it directly because it contains the core dict-to-struct conversion
  # logic and we want thorough coverage of edge cases.

  def test_extract_targets_with_valid_targets
    variables = {
      "_targets" => [
        {
          "rule" => "py_library",
          "name" => "logic-gates",
          "srcs" => ["src/**/*.py"],
          "deps" => ["python/grammar-tools"],
          "test_runner" => "pytest",
          "entry_point" => ""
        }
      ]
    }

    targets = BuildTool::StarlarkEvaluator.send(:extract_targets, variables)

    assert_equal 1, targets.size
    t = targets.first
    assert_equal "py_library", t.rule
    assert_equal "logic-gates", t.name
    assert_equal ["src/**/*.py"], t.srcs
    assert_equal ["python/grammar-tools"], t.deps
    assert_equal "pytest", t.test_runner
    assert_equal "", t.entry_point
  end

  def test_extract_targets_returns_empty_when_no_targets_variable
    variables = { "x" => 42 }
    targets = BuildTool::StarlarkEvaluator.send(:extract_targets, variables)
    assert_equal [], targets
  end

  def test_extract_targets_returns_empty_for_empty_list
    variables = { "_targets" => [] }
    targets = BuildTool::StarlarkEvaluator.send(:extract_targets, variables)
    assert_equal [], targets
  end

  def test_extract_targets_raises_when_targets_not_a_list
    variables = { "_targets" => "not a list" }
    assert_raises(RuntimeError) do
      BuildTool::StarlarkEvaluator.send(:extract_targets, variables)
    end
  end

  def test_extract_targets_raises_when_element_not_a_dict
    variables = { "_targets" => ["not a dict"] }
    assert_raises(RuntimeError) do
      BuildTool::StarlarkEvaluator.send(:extract_targets, variables)
    end
  end

  def test_extract_targets_handles_missing_optional_fields
    # A target dict with only rule and name -- srcs, deps, test_runner,
    # and entry_point should default to empty values.
    variables = {
      "_targets" => [
        { "rule" => "go_binary", "name" => "tool" }
      ]
    }

    targets = BuildTool::StarlarkEvaluator.send(:extract_targets, variables)

    assert_equal 1, targets.size
    t = targets.first
    assert_equal "go_binary", t.rule
    assert_equal "tool", t.name
    assert_equal [], t.srcs
    assert_equal [], t.deps
    assert_equal "", t.test_runner
    assert_equal "", t.entry_point
  end

  def test_extract_targets_multiple_targets
    variables = {
      "_targets" => [
        { "rule" => "py_library", "name" => "lib-a" },
        { "rule" => "py_binary", "name" => "bin-b" }
      ]
    }

    targets = BuildTool::StarlarkEvaluator.send(:extract_targets, variables)
    assert_equal 2, targets.size
    assert_equal "lib-a", targets[0].name
    assert_equal "bin-b", targets[1].name
  end

  # ==========================================================================
  # get_string / get_string_list helper tests
  # ==========================================================================

  def test_get_string_returns_value_when_present
    assert_equal "hello", BuildTool::StarlarkEvaluator.send(:get_string, { "k" => "hello" }, "k")
  end

  def test_get_string_returns_empty_for_missing_key
    assert_equal "", BuildTool::StarlarkEvaluator.send(:get_string, {}, "k")
  end

  def test_get_string_returns_empty_for_non_string_value
    assert_equal "", BuildTool::StarlarkEvaluator.send(:get_string, { "k" => 42 }, "k")
  end

  def test_get_string_list_returns_value_when_present
    result = BuildTool::StarlarkEvaluator.send(:get_string_list, { "k" => ["a", "b"] }, "k")
    assert_equal ["a", "b"], result
  end

  def test_get_string_list_returns_empty_for_missing_key
    result = BuildTool::StarlarkEvaluator.send(:get_string_list, {}, "k")
    assert_equal [], result
  end

  def test_get_string_list_returns_empty_for_non_array_value
    result = BuildTool::StarlarkEvaluator.send(:get_string_list, { "k" => "not array" }, "k")
    assert_equal [], result
  end

  def test_get_string_list_filters_non_strings
    # Non-string elements in the array should be silently skipped.
    result = BuildTool::StarlarkEvaluator.send(:get_string_list, { "k" => ["a", 42, "b", nil] }, "k")
    assert_equal ["a", "b"], result
  end

  # ==========================================================================
  # evaluate_build_file tests
  # ==========================================================================
  #
  # These tests require the starlark_interpreter gem. We test with simple
  # Starlark source that sets _targets directly, avoiding the need for
  # load() and complex rule definitions.

  def test_evaluate_build_file_raises_for_missing_file
    assert_raises(RuntimeError) do
      BuildTool::StarlarkEvaluator.evaluate_build_file(
        "/nonexistent/BUILD", "/nonexistent", "/nonexistent"
      )
    end
  end

  def test_evaluate_build_file_returns_empty_when_no_targets
    # This test requires the starlark_interpreter gem. If it's not
    # installed, we skip rather than fail -- the gem is only available
    # when the full dependency chain is set up.
    begin
      require "coding_adventures_starlark_interpreter"
    rescue LoadError
      skip "starlark_interpreter gem not available"
    end

    dir = create_temp_dir
    build_file = dir / "BUILD"
    # A valid Starlark file that doesn't declare any targets.
    write_file(build_file, "x = 42\n")

    result = BuildTool::StarlarkEvaluator.evaluate_build_file(
      build_file.to_s, dir.to_s, dir.to_s
    )

    assert_instance_of BuildTool::BuildFileResult, result
    assert_equal [], result.targets
  ensure
    FileUtils.rm_rf(dir) if dir
  end
end
