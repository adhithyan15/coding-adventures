# frozen_string_literal: true

require_relative "test_helper"

# Tests for SpecLoader — the component that reads, parses, and validates
# a CLI Builder JSON spec file.
#
# We cover:
#   - Loading a valid minimal spec (echo)
#   - Loading a valid full-featured spec (git-style)
#   - All 9 validation error cases
class TestSpecLoader < Minitest::Test
  include CodingAdventures::CliBuilder

  # ---------------------------------------------------------------------------
  # Helpers
  # ---------------------------------------------------------------------------

  # Write a spec hash to disk, load it, and return the result.
  def load_spec(hash)
    with_spec_file(hash) { |path| SpecLoader.new(path).load }
  end

  # Assert that loading the given spec hash raises a SpecError matching the pattern.
  def assert_spec_error(hash, pattern = nil)
    err = assert_raises(SpecError) { load_spec(hash) }
    assert_match(pattern, err.message) if pattern
    err
  end

  # ---------------------------------------------------------------------------
  # Valid spec: echo — minimal spec with variadic arg and flag conflicts
  # ---------------------------------------------------------------------------

  ECHO_SPEC = {
    "cli_builder_spec_version" => "1.0",
    "name" => "echo",
    "description" => "Display a line of text",
    "version" => "8.32",
    "flags" => [
      {
        "id" => "no-newline",
        "short" => "n",
        "description" => "Do not output the trailing newline",
        "type" => "boolean"
      },
      {
        "id" => "enable-escapes",
        "short" => "e",
        "description" => "Enable interpretation of backslash escapes",
        "type" => "boolean",
        "conflicts_with" => ["disable-escapes"]
      },
      {
        "id" => "disable-escapes",
        "short" => "E",
        "description" => "Disable interpretation of backslash escapes (default)",
        "type" => "boolean",
        "conflicts_with" => ["enable-escapes"]
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
        "variadic_min" => 0
      }
    ]
  }.freeze

  def test_load_echo_spec_returns_hash
    spec = load_spec(ECHO_SPEC)
    assert_equal "echo", spec["name"]
    assert_equal "Display a line of text", spec["description"]
    assert_equal "8.32", spec["version"]
    assert_equal "gnu", spec["parsing_mode"]
  end

  def test_load_echo_spec_normalizes_flags
    spec = load_spec(ECHO_SPEC)
    flag = spec["flags"].find { |f| f["id"] == "no-newline" }
    refute_nil flag
    assert_equal false, flag["required"]
    assert_equal false, flag["repeatable"]
    assert_equal [], flag["conflicts_with"]
    assert_equal [], flag["requires"]
  end

  def test_load_echo_spec_normalizes_arguments
    spec = load_spec(ECHO_SPEC)
    arg = spec["arguments"].first
    assert_equal "string", arg["id"]
    assert_equal false, arg["required"]
    assert_equal true, arg["variadic"]
    assert_equal 0, arg["variadic_min"]
  end

  def test_load_echo_spec_has_builtin_flags
    spec = load_spec(ECHO_SPEC)
    assert_equal true, spec["builtin_flags"]["help"]
    assert_equal true, spec["builtin_flags"]["version"]
  end

  # ---------------------------------------------------------------------------
  # Valid spec: git-style with subcommands and global flags
  # ---------------------------------------------------------------------------

  GIT_SPEC = {
    "cli_builder_spec_version" => "1.0",
    "name" => "git",
    "description" => "The stupid content tracker",
    "version" => "2.39.0",
    "global_flags" => [
      {
        "id" => "verbose",
        "short" => "v",
        "long" => "verbose",
        "description" => "Be more verbose",
        "type" => "boolean"
      }
    ],
    "commands" => [
      {
        "id" => "cmd-remote",
        "name" => "remote",
        "description" => "Manage set of tracked repositories",
        "flags" => [],
        "commands" => [
          {
            "id" => "cmd-remote-add",
            "name" => "add",
            "aliases" => ["a"],
            "description" => "Add a named remote",
            "flags" => [
              {
                "id" => "fetch",
                "short" => "f",
                "description" => "Fetch after adding",
                "type" => "boolean"
              }
            ],
            "arguments" => [
              {
                "id" => "name",
                "name" => "NAME",
                "description" => "Remote name",
                "type" => "string",
                "required" => true
              },
              {
                "id" => "url",
                "name" => "URL",
                "description" => "Remote URL",
                "type" => "string",
                "required" => true
              }
            ]
          }
        ]
      }
    ]
  }.freeze

  def test_load_git_spec_returns_hash
    spec = load_spec(GIT_SPEC)
    assert_equal "git", spec["name"]
    assert_equal 1, spec["commands"].length
  end

  def test_load_git_spec_global_flags_normalized
    spec = load_spec(GIT_SPEC)
    gf = spec["global_flags"].first
    assert_equal "verbose", gf["id"]
    assert_equal false, gf["required"]
  end

  def test_load_git_spec_nested_commands_normalized
    spec = load_spec(GIT_SPEC)
    remote = spec["commands"].first
    assert_equal "remote", remote["name"]
    add = remote["commands"].first
    assert_equal "add", add["name"]
    assert_equal ["a"], add["aliases"]
    arg = add["arguments"].first
    assert_equal true, arg["required"]
  end

  # ---------------------------------------------------------------------------
  # Validation Error 1: missing version field
  # ---------------------------------------------------------------------------

  def test_error_missing_version_field
    assert_spec_error(ECHO_SPEC.reject { |k, _| k == "cli_builder_spec_version" },
      /missing required field: cli_builder_spec_version/i)
  end

  def test_error_wrong_version
    spec = ECHO_SPEC.merge("cli_builder_spec_version" => "2.0")
    assert_spec_error(spec, /unsupported spec version/i)
  end

  # ---------------------------------------------------------------------------
  # Validation Error 2: missing required top-level fields
  # ---------------------------------------------------------------------------

  def test_error_missing_name
    spec = ECHO_SPEC.reject { |k, _| k == "name" }
    assert_spec_error(spec, /missing required top-level field: name/i)
  end

  def test_error_missing_description
    spec = ECHO_SPEC.reject { |k, _| k == "description" }
    assert_spec_error(spec, /missing required top-level field: description/i)
  end

  # ---------------------------------------------------------------------------
  # Validation Error 3: duplicate IDs
  # ---------------------------------------------------------------------------

  def test_error_duplicate_flag_id
    spec = ECHO_SPEC.merge("flags" => [
      {"id" => "verbose", "short" => "v", "description" => "x", "type" => "boolean"},
      {"id" => "verbose", "short" => "q", "description" => "y", "type" => "boolean"}
    ])
    assert_spec_error(spec, /duplicate flag id/i)
  end

  def test_error_duplicate_command_name
    spec = {
      "cli_builder_spec_version" => "1.0",
      "name" => "myapp",
      "description" => "test",
      "commands" => [
        {"id" => "cmd-a", "name" => "run", "description" => "run it"},
        {"id" => "cmd-b", "name" => "run", "description" => "run it again"}
      ]
    }
    assert_spec_error(spec, /duplicate command name\/alias/i)
  end

  # ---------------------------------------------------------------------------
  # Validation Error 4: flag with no short/long/single_dash_long
  # ---------------------------------------------------------------------------

  def test_error_flag_without_name
    spec = ECHO_SPEC.merge("flags" => [
      {"id" => "verbose", "description" => "Be verbose", "type" => "boolean"}
    ])
    assert_spec_error(spec, /no short, long, or single_dash_long/i)
  end

  # ---------------------------------------------------------------------------
  # Validation Error 5: unknown flag references in conflicts_with / requires
  # ---------------------------------------------------------------------------

  def test_error_conflicts_with_unknown_id
    spec = ECHO_SPEC.merge("flags" => [
      {
        "id" => "foo",
        "short" => "f",
        "description" => "foo",
        "type" => "boolean",
        "conflicts_with" => ["nonexistent"]
      }
    ])
    assert_spec_error(spec, /conflicts_with reference to unknown id/i)
  end

  def test_error_requires_unknown_id
    spec = ECHO_SPEC.merge("flags" => [
      {
        "id" => "foo",
        "short" => "f",
        "description" => "foo",
        "type" => "boolean",
        "requires" => ["nonexistent"]
      }
    ])
    assert_spec_error(spec, /requires reference to unknown id/i)
  end

  # ---------------------------------------------------------------------------
  # Validation Error 6: exclusive group references unknown flag
  # ---------------------------------------------------------------------------

  def test_error_exclusive_group_unknown_flag
    spec = {
      "cli_builder_spec_version" => "1.0",
      "name" => "myapp",
      "description" => "test",
      "flags" => [
        {"id" => "foo", "short" => "f", "description" => "f", "type" => "boolean"}
      ],
      "mutually_exclusive_groups" => [
        {"id" => "grp", "flag_ids" => ["foo", "nonexistent"]}
      ]
    }
    assert_spec_error(spec, /references unknown flag id/i)
  end

  def test_error_exclusive_group_too_small
    spec = {
      "cli_builder_spec_version" => "1.0",
      "name" => "myapp",
      "description" => "test",
      "flags" => [
        {"id" => "foo", "short" => "f", "description" => "f", "type" => "boolean"}
      ],
      "mutually_exclusive_groups" => [
        {"id" => "grp", "flag_ids" => ["foo"]}
      ]
    }
    assert_spec_error(spec, /must contain at least 2/i)
  end

  # ---------------------------------------------------------------------------
  # Validation Error 7: enum without values
  # ---------------------------------------------------------------------------

  def test_error_enum_without_values
    spec = ECHO_SPEC.merge("flags" => [
      {
        "id" => "format",
        "long" => "format",
        "description" => "Output format",
        "type" => "enum",
        "enum_values" => []
      }
    ])
    assert_spec_error(spec, /type 'enum' but no enum_values/i)
  end

  def test_error_enum_without_values_key
    spec = ECHO_SPEC.merge("flags" => [
      {
        "id" => "format",
        "long" => "format",
        "description" => "Output format",
        "type" => "enum"
      }
    ])
    assert_spec_error(spec, /type 'enum' but no enum_values/i)
  end

  # ---------------------------------------------------------------------------
  # Validation Error 8: circular requires dependency
  # ---------------------------------------------------------------------------

  def test_error_circular_requires
    spec = {
      "cli_builder_spec_version" => "1.0",
      "name" => "myapp",
      "description" => "test",
      "flags" => [
        {
          "id" => "verbose",
          "short" => "v",
          "description" => "verbose",
          "type" => "boolean",
          "requires" => ["quiet"]
        },
        {
          "id" => "quiet",
          "short" => "q",
          "description" => "quiet",
          "type" => "boolean",
          "requires" => ["verbose"]
        }
      ]
    }
    assert_spec_error(spec, /circular requires dependency/i)
  end

  # ---------------------------------------------------------------------------
  # Validation Error: multiple variadic arguments
  # ---------------------------------------------------------------------------

  def test_error_multiple_variadic
    spec = {
      "cli_builder_spec_version" => "1.0",
      "name" => "myapp",
      "description" => "test",
      "arguments" => [
        {"id" => "a", "name" => "A", "description" => "a", "type" => "string", "variadic" => true},
        {"id" => "b", "name" => "B", "description" => "b", "type" => "string", "variadic" => true}
      ]
    }
    assert_spec_error(spec, /at most one is allowed/i)
  end

  # ---------------------------------------------------------------------------
  # File I/O errors
  # ---------------------------------------------------------------------------

  def test_error_file_not_found
    err = assert_raises(SpecError) { SpecLoader.new("/nonexistent/path.json").load }
    assert_match(/not found/i, err.message)
  end

  def test_error_invalid_json
    f = Tempfile.new(["bad", ".json"])
    f.write("{ this is not json }")
    f.close
    err = assert_raises(SpecError) { SpecLoader.new(f.path).load }
    assert_match(/invalid json/i, err.message)
  ensure
    f.unlink
  end

  # ---------------------------------------------------------------------------
  # Validation Error 5: required_unless references unknown id
  # ---------------------------------------------------------------------------

  def test_error_required_unless_unknown_id
    spec = ECHO_SPEC.merge("flags" => [
      {
        "id" => "foo",
        "short" => "f",
        "description" => "foo",
        "type" => "boolean",
        "required_unless" => ["nonexistent"]
      }
    ])
    assert_spec_error(spec, /required_unless reference to unknown id/i)
  end

  # ---------------------------------------------------------------------------
  # Validation Error 3: duplicate argument id
  # ---------------------------------------------------------------------------

  def test_error_duplicate_argument_id
    spec = {
      "cli_builder_spec_version" => "1.0",
      "name" => "myapp",
      "description" => "test",
      "arguments" => [
        {"id" => "file", "name" => "FILE", "description" => "a", "type" => "string"},
        {"id" => "file", "name" => "FILE2", "description" => "b", "type" => "string"}
      ]
    }
    assert_spec_error(spec, /duplicate argument id/i)
  end

  # ---------------------------------------------------------------------------
  # Validation Error 3: duplicate command id
  # ---------------------------------------------------------------------------

  def test_error_duplicate_command_id
    spec = {
      "cli_builder_spec_version" => "1.0",
      "name" => "myapp",
      "description" => "test",
      "commands" => [
        {"id" => "cmd-a", "name" => "run", "description" => "run it"},
        {"id" => "cmd-a", "name" => "build", "description" => "build it"}
      ]
    }
    assert_spec_error(spec, /duplicate command id/i)
  end

  # ---------------------------------------------------------------------------
  # Validation Error 9: self-loop cycle (A requires A)
  # ---------------------------------------------------------------------------

  def test_error_self_loop_requires
    spec = {
      "cli_builder_spec_version" => "1.0",
      "name" => "myapp",
      "description" => "test",
      "flags" => [
        {
          "id" => "verbose",
          "short" => "v",
          "description" => "verbose",
          "type" => "boolean",
          "requires" => ["verbose"]
        }
      ]
    }
    assert_spec_error(spec, /circular requires dependency/i)
  end

  # ---------------------------------------------------------------------------
  # Validation: enum argument (not just flag) must have enum_values
  # ---------------------------------------------------------------------------

  def test_error_enum_argument_without_values
    spec = {
      "cli_builder_spec_version" => "1.0",
      "name" => "myapp",
      "description" => "test",
      "arguments" => [
        {
          "id" => "mode",
          "name" => "MODE",
          "description" => "run mode",
          "type" => "enum",
          "enum_values" => []
        }
      ]
    }
    assert_spec_error(spec, /type 'enum' but no enum_values/i)
  end

  # ---------------------------------------------------------------------------
  # Valid: inherit_global_flags false suppresses global flags from scope
  # ---------------------------------------------------------------------------

  def test_inherit_global_flags_false_normalizes
    spec = {
      "cli_builder_spec_version" => "1.0",
      "name" => "myapp",
      "description" => "test",
      "global_flags" => [
        {"id" => "verbose", "short" => "v", "description" => "verbose", "type" => "boolean"}
      ],
      "commands" => [
        {
          "id" => "cmd-run",
          "name" => "run",
          "description" => "run",
          "inherit_global_flags" => false
        }
      ]
    }
    loaded = load_spec(spec)
    cmd = loaded["commands"].first
    assert_equal false, cmd["inherit_global_flags"]
  end

  # ---------------------------------------------------------------------------
  # Normalization: variadic_min defaults based on required field
  # ---------------------------------------------------------------------------

  def test_normalize_variadic_min_defaults_to_0_when_not_required
    spec = {
      "cli_builder_spec_version" => "1.0",
      "name" => "myapp",
      "description" => "test",
      "arguments" => [
        {
          "id" => "files",
          "name" => "FILES",
          "description" => "files",
          "type" => "string",
          "required" => false,
          "variadic" => true
        }
      ]
    }
    loaded = load_spec(spec)
    arg = loaded["arguments"].first
    assert_equal 0, arg["variadic_min"]
  end

  def test_normalize_variadic_min_defaults_to_1_when_required
    spec = {
      "cli_builder_spec_version" => "1.0",
      "name" => "myapp",
      "description" => "test",
      "arguments" => [
        {
          "id" => "files",
          "name" => "FILES",
          "description" => "files",
          "type" => "string",
          "required" => true,
          "variadic" => true
        }
      ]
    }
    loaded = load_spec(spec)
    arg = loaded["arguments"].first
    assert_equal 1, arg["variadic_min"]
  end

  # ---------------------------------------------------------------------------
  # Validation: command-level scope validates cross-references too
  # ---------------------------------------------------------------------------

  def test_error_command_scope_requires_unknown_id
    spec = {
      "cli_builder_spec_version" => "1.0",
      "name" => "myapp",
      "description" => "test",
      "commands" => [
        {
          "id" => "cmd-run",
          "name" => "run",
          "description" => "run",
          "flags" => [
            {
              "id" => "fast",
              "short" => "f",
              "description" => "fast",
              "type" => "boolean",
              "requires" => ["nonexistent"]
            }
          ]
        }
      ]
    }
    assert_spec_error(spec, /requires reference to unknown id/i)
  end

  # ---------------------------------------------------------------------------
  # Validation: command name alias duplication
  # ---------------------------------------------------------------------------

  def test_error_command_alias_clashes_with_sibling_name
    spec = {
      "cli_builder_spec_version" => "1.0",
      "name" => "myapp",
      "description" => "test",
      "commands" => [
        {"id" => "cmd-a", "name" => "run", "description" => "run it"},
        {"id" => "cmd-b", "name" => "build", "aliases" => ["run"], "description" => "build it"}
      ]
    }
    assert_spec_error(spec, /duplicate command name\/alias/i)
  end
end
