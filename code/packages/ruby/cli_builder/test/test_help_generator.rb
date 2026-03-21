# frozen_string_literal: true

require_relative "test_helper"

# Tests for HelpGenerator — the component that generates formatted help text
# from a CLI spec.
class TestHelpGenerator < Minitest::Test
  include CodingAdventures::CliBuilder

  # ---------------------------------------------------------------------------
  # Helper
  # ---------------------------------------------------------------------------

  def generate(spec, command_path)
    HelpGenerator.new(spec, command_path).generate
  end

  # Normalize whitespace for comparison — collapse runs of spaces/newlines to single space
  def normalize(text)
    text.gsub(/[[:space:]]+/, " ").strip
  end

  # ---------------------------------------------------------------------------
  # Fixtures
  # ---------------------------------------------------------------------------

  SIMPLE_SPEC = {
    "cli_builder_spec_version" => "1.0",
    "name" => "echo",
    "description" => "Display a line of text",
    "version" => "8.32",
    "parsing_mode" => "gnu",
    "builtin_flags" => {"help" => true, "version" => true},
    "global_flags" => [],
    "flags" => [
      {
        "id" => "no-newline",
        "short" => "n",
        "description" => "Do not output the trailing newline",
        "type" => "boolean",
        "required" => false,
        "repeatable" => false,
        "conflicts_with" => [],
        "requires" => [],
        "required_unless" => [],
        "enum_values" => []
      }
    ],
    "arguments" => [
      {
        "id" => "string",
        "name" => "STRING",
        "description" => "Text to print",
        "type" => "string",
        "required" => false,
        "variadic" => true,
        "variadic_min" => 0,
        "variadic_max" => nil,
        "enum_values" => [],
        "required_unless_flag" => []
      }
    ],
    "commands" => [],
    "mutually_exclusive_groups" => []
  }.freeze

  GIT_SPEC = {
    "cli_builder_spec_version" => "1.0",
    "name" => "git",
    "description" => "The stupid content tracker",
    "version" => "2.39.0",
    "parsing_mode" => "gnu",
    "builtin_flags" => {"help" => true, "version" => true},
    "global_flags" => [
      {
        "id" => "verbose",
        "short" => "v",
        "long" => "verbose",
        "description" => "Be more verbose",
        "type" => "boolean",
        "required" => false,
        "repeatable" => false,
        "conflicts_with" => [],
        "requires" => [],
        "required_unless" => [],
        "enum_values" => []
      }
    ],
    "flags" => [],
    "arguments" => [],
    "commands" => [
      {
        "id" => "cmd-commit",
        "name" => "commit",
        "aliases" => [],
        "description" => "Record changes to the repository",
        "inherit_global_flags" => true,
        "flags" => [
          {
            "id" => "message",
            "short" => "m",
            "long" => "message",
            "description" => "Commit message",
            "type" => "string",
            "required" => true,
            "repeatable" => false,
            "conflicts_with" => [],
            "requires" => [],
            "required_unless" => [],
            "enum_values" => [],
            "value_name" => "MSG"
          },
          {
            "id" => "all",
            "short" => "a",
            "long" => "all",
            "description" => "Stage all tracked files",
            "type" => "boolean",
            "required" => false,
            "repeatable" => false,
            "conflicts_with" => [],
            "requires" => [],
            "required_unless" => [],
            "enum_values" => []
          }
        ],
        "arguments" => [],
        "commands" => [],
        "mutually_exclusive_groups" => []
      },
      {
        "id" => "cmd-remote",
        "name" => "remote",
        "aliases" => [],
        "description" => "Manage set of tracked repositories",
        "inherit_global_flags" => true,
        "flags" => [],
        "arguments" => [],
        "commands" => [],
        "mutually_exclusive_groups" => []
      }
    ],
    "mutually_exclusive_groups" => []
  }.freeze

  # ---------------------------------------------------------------------------
  # Root-level program help
  # ---------------------------------------------------------------------------

  def test_help_contains_usage_section
    text = generate(SIMPLE_SPEC, ["echo"])
    assert_match(/USAGE/i, text)
    assert_match(/echo/, text)
  end

  def test_help_contains_description
    text = generate(SIMPLE_SPEC, ["echo"])
    assert_match(/DESCRIPTION/i, text)
    assert_match(/Display a line of text/, text)
  end

  def test_help_contains_options_section
    text = generate(SIMPLE_SPEC, ["echo"])
    assert_match(/OPTIONS/i, text)
    assert_match(/-n/, text)
    assert_match(/Do not output the trailing newline/, text)
  end

  def test_help_contains_global_options_with_help
    text = generate(SIMPLE_SPEC, ["echo"])
    assert_match(/GLOBAL OPTIONS/i, text)
    assert_match(/-h/, text)
    assert_match(/--help/, text)
  end

  def test_help_contains_version_in_global_options
    text = generate(SIMPLE_SPEC, ["echo"])
    assert_match(/--version/, text)
    assert_match(/Show version/, text)
  end

  def test_help_contains_arguments_section
    text = generate(SIMPLE_SPEC, ["echo"])
    assert_match(/ARGUMENTS/i, text)
    # Variadic optional: [STRING...]
    assert_match(/STRING/, text)
  end

  def test_help_omits_commands_section_when_no_subcommands
    text = generate(SIMPLE_SPEC, ["echo"])
    refute_match(/^COMMANDS/i, text)
  end

  # ---------------------------------------------------------------------------
  # Program with subcommands
  # ---------------------------------------------------------------------------

  def test_git_root_help_contains_commands
    text = generate(GIT_SPEC, ["git"])
    assert_match(/COMMANDS/i, text)
    assert_match(/commit/, text)
    assert_match(/remote/, text)
    assert_match(/Record changes/, text)
  end

  def test_git_root_help_has_global_options
    text = generate(GIT_SPEC, ["git"])
    assert_match(/GLOBAL OPTIONS/i, text)
    assert_match(/-v.*--verbose/i, text)
  end

  def test_git_root_usage_includes_options_and_command
    text = generate(GIT_SPEC, ["git"])
    assert_match(/\[OPTIONS\]/, text)
    assert_match(/COMMAND/, text)
  end

  # ---------------------------------------------------------------------------
  # Subcommand-level help
  # ---------------------------------------------------------------------------

  def test_git_commit_help
    text = generate(GIT_SPEC, ["git", "commit"])
    assert_match(/USAGE/i, text)
    assert_match(/git commit/, text)
    assert_match(/OPTIONS/i, text)
    assert_match(/-m.*--message/i, text)
    assert_match(/Commit message/, text)
  end

  def test_git_commit_help_shows_value_name
    text = generate(GIT_SPEC, ["git", "commit"])
    # The message flag has value_name "MSG"
    assert_match(/MSG/, text)
  end

  def test_git_commit_help_command_path
    gen = HelpGenerator.new(GIT_SPEC, ["git", "commit"])
    text = gen.generate
    assert_match(/git commit/, text)
  end

  # ---------------------------------------------------------------------------
  # Argument formatting
  # ---------------------------------------------------------------------------

  def test_required_arg_formatting
    spec = SIMPLE_SPEC.merge("arguments" => [
      {
        "id" => "target",
        "name" => "TARGET",
        "description" => "Target file",
        "type" => "path",
        "required" => true,
        "variadic" => false,
        "variadic_min" => 1,
        "variadic_max" => nil,
        "enum_values" => [],
        "required_unless_flag" => []
      }
    ], "flags" => [])
    text = generate(spec, ["echo"])
    # Required non-variadic: <TARGET>
    assert_match(/<TARGET>/, text)
    assert_match(/Required/, text)
  end

  def test_optional_arg_formatting
    spec = SIMPLE_SPEC.merge("arguments" => [
      {
        "id" => "target",
        "name" => "TARGET",
        "description" => "Target file",
        "type" => "path",
        "required" => false,
        "variadic" => false,
        "variadic_min" => 0,
        "variadic_max" => nil,
        "enum_values" => [],
        "required_unless_flag" => []
      }
    ], "flags" => [])
    text = generate(spec, ["echo"])
    assert_match(/\[TARGET\]/, text)
    assert_match(/Optional/, text)
  end

  def test_variadic_required_arg_formatting
    spec = SIMPLE_SPEC.merge("arguments" => [
      {
        "id" => "files",
        "name" => "FILE",
        "description" => "Files",
        "type" => "path",
        "required" => true,
        "variadic" => true,
        "variadic_min" => 1,
        "variadic_max" => nil,
        "enum_values" => [],
        "required_unless_flag" => []
      }
    ], "flags" => [])
    text = generate(spec, ["echo"])
    assert_match(/<FILE>\.\.\./, text)
  end

  def test_variadic_optional_arg_formatting
    # SIMPLE_SPEC already has a variadic optional STRING arg
    text = generate(SIMPLE_SPEC, ["echo"])
    assert_match(/\[STRING\.\.\.?\]/, text)
  end

  # ---------------------------------------------------------------------------
  # Flag formatting edge cases
  # ---------------------------------------------------------------------------

  def test_default_value_shown_in_help
    spec = SIMPLE_SPEC.merge("flags" => [
      {
        "id" => "timeout",
        "long" => "timeout",
        "description" => "Timeout in seconds",
        "type" => "integer",
        "required" => false,
        "repeatable" => false,
        "conflicts_with" => [],
        "requires" => [],
        "required_unless" => [],
        "enum_values" => [],
        "default" => 30
      }
    ], "arguments" => [])
    text = generate(spec, ["echo"])
    assert_match(/--timeout/, text)
    assert_match(/\[default: 30\]/, text)
  end

  def test_single_dash_long_flag_in_help
    spec = SIMPLE_SPEC.merge("flags" => [
      {
        "id" => "classpath",
        "single_dash_long" => "classpath",
        "description" => "Class path",
        "type" => "string",
        "required" => false,
        "repeatable" => false,
        "conflicts_with" => [],
        "requires" => [],
        "required_unless" => [],
        "enum_values" => [],
        "value_name" => "PATH"
      }
    ], "arguments" => [])
    text = generate(spec, ["echo"])
    assert_match(/-classpath/, text)
    assert_match(/PATH/, text)
  end

  def test_help_result_has_correct_command_path
    gen = HelpGenerator.new(GIT_SPEC, ["git"])
    text = gen.generate
    # Command path appears in usage
    assert_match(/git/, text)
  end

  # ---------------------------------------------------------------------------
  # format_flag_name: type nil → "VALUE" fallback
  # ---------------------------------------------------------------------------

  def test_flag_without_type_uses_value_fallback
    spec = SIMPLE_SPEC.merge("flags" => [
      {
        "id" => "config",
        "long" => "config",
        "description" => "Config file",
        "required" => false,
        "repeatable" => false,
        "conflicts_with" => [],
        "requires" => [],
        "required_unless" => [],
        "enum_values" => []
        # no "type" key → nil
      }
    ], "arguments" => [])
    text = generate(spec, ["echo"])
    # Should render <VALUE> when type is nil
    assert_match(/VALUE/, text)
  end

  # ---------------------------------------------------------------------------
  # format_default: required flag gets no [default: ...] annotation
  # ---------------------------------------------------------------------------

  def test_required_flag_no_default_annotation
    spec = SIMPLE_SPEC.merge("flags" => [
      {
        "id" => "output",
        "long" => "output",
        "description" => "Output file",
        "type" => "string",
        "required" => true,
        "default" => "out.txt",
        "repeatable" => false,
        "conflicts_with" => [],
        "requires" => [],
        "required_unless" => [],
        "enum_values" => []
      }
    ], "arguments" => [])
    text = generate(spec, ["echo"])
    # Even though default is set, required flag should NOT show [default: ...]
    refute_match(/\[default:/, text)
  end

  # ---------------------------------------------------------------------------
  # No GLOBAL OPTIONS section when both builtins are disabled and no global flags
  # ---------------------------------------------------------------------------

  def test_no_global_options_when_builtins_disabled_and_no_global_flags
    spec = SIMPLE_SPEC.merge(
      "builtin_flags" => {"help" => false, "version" => false},
      "global_flags" => [],
      "flags" => [],
      "arguments" => []
    )
    text = generate(spec, ["echo"])
    refute_match(/GLOBAL OPTIONS/, text)
  end

  # ---------------------------------------------------------------------------
  # No version in GLOBAL OPTIONS when spec has no version
  # ---------------------------------------------------------------------------

  def test_no_version_in_global_options_when_no_version_field
    spec = SIMPLE_SPEC.merge("global_flags" => [], "flags" => [], "arguments" => [])
    spec = spec.reject { |k, _| k == "version" }
    text = generate(spec, ["echo"])
    refute_match(/Show version/, text)
  end

  # ---------------------------------------------------------------------------
  # COMMANDS section: multiple commands, alignment
  # ---------------------------------------------------------------------------

  def test_commands_section_alignment
    text = generate(GIT_SPEC, ["git"])
    # commit and remote should both appear aligned in the COMMANDS section
    assert_match(/commit/, text)
    assert_match(/remote/, text)
  end

  # ---------------------------------------------------------------------------
  # OPTIONS section absent when no local flags
  # ---------------------------------------------------------------------------

  def test_no_options_section_when_no_local_flags
    text = generate(GIT_SPEC, ["git", "remote"])
    # git remote has no flags in GIT_SPEC
    # The OPTIONS section should be absent (but GLOBAL OPTIONS may still appear)
    # We check that local OPTIONS header doesn't appear while COMMANDS might
    lines = text.split("\n")
    # There should be no "^OPTIONS" line (just GLOBAL OPTIONS is OK)
    refute(lines.any? { |l| l.strip == "OPTIONS" },
      "Expected no standalone OPTIONS section for git remote")
  end

  # ---------------------------------------------------------------------------
  # format_arg_usage: variadic required vs optional in usage line
  # ---------------------------------------------------------------------------

  def test_usage_line_includes_variadic_required_arg
    spec = SIMPLE_SPEC.merge(
      "arguments" => [
        {
          "id" => "files",
          "name" => "FILE",
          "description" => "Files to process",
          "type" => "path",
          "required" => true,
          "variadic" => true,
          "variadic_min" => 1,
          "variadic_max" => nil,
          "enum_values" => [],
          "required_unless_flag" => []
        }
      ],
      "flags" => []
    )
    text = generate(spec, ["echo"])
    # Required variadic: <FILE>...
    assert_match(/<FILE>\.\.\./, text)
  end

  def test_usage_line_includes_optional_non_variadic_arg
    spec = SIMPLE_SPEC.merge(
      "arguments" => [
        {
          "id" => "target",
          "name" => "TARGET",
          "description" => "Optional target",
          "type" => "string",
          "required" => false,
          "variadic" => false,
          "variadic_min" => 0,
          "variadic_max" => nil,
          "enum_values" => [],
          "required_unless_flag" => []
        }
      ],
      "flags" => []
    )
    text = generate(spec, ["echo"])
    # Optional non-variadic: [TARGET]
    assert_match(/\[TARGET\]/, text)
  end

  # ---------------------------------------------------------------------------
  # DESCRIPTION section: absent when empty
  # ---------------------------------------------------------------------------

  def test_no_description_section_when_empty
    spec = SIMPLE_SPEC.merge("description" => "", "flags" => [], "arguments" => [])
    text = generate(spec, ["echo"])
    refute_match(/^DESCRIPTION/, text)
  end
end
