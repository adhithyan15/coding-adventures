defmodule CodingAdventures.CliBuilder.V11FeaturesTest do
  @moduledoc """
  Tests for CLI Builder v1.1 features:

  1. **Count type** — a flag type that increments a counter on each occurrence.
  2. **Enum optional values (default_when_present)** — enum flags that can be
     used without a value, falling back to a configured default.
  3. **Flag presence detection (explicit_flags)** — track which flags were
     explicitly set by the user.
  4. **int64 range validation** — reject integers outside [-2^63, 2^63-1].
  """
  use ExUnit.Case, async: true

  alias CodingAdventures.CliBuilder.{Parser, Validator}

  # ===========================================================================
  # Embedded JSON specs
  # ===========================================================================

  # A spec with a count-type verbosity flag, used for Feature 1 tests.
  @count_spec """
  {
    "cli_builder_spec_version": "1.0",
    "name": "myapp",
    "description": "An app with count flags",
    "version": "1.0",
    "flags": [
      {"id": "verbose", "short": "v", "long": "verbose", "description": "Increase verbosity", "type": "count"},
      {"id": "quiet", "short": "q", "long": "quiet", "description": "Decrease verbosity", "type": "count"},
      {"id": "debug", "short": "d", "description": "Debug mode", "type": "boolean"}
    ],
    "arguments": []
  }
  """

  # A spec with an enum flag that has default_when_present, used for Feature 2 tests.
  @enum_dwp_spec """
  {
    "cli_builder_spec_version": "1.0",
    "name": "ls",
    "description": "List directory contents",
    "version": "1.0",
    "flags": [
      {
        "id": "color",
        "long": "color",
        "short": "c",
        "description": "Colourize output",
        "type": "enum",
        "enum_values": ["always", "auto", "never"],
        "default": "auto",
        "default_when_present": "always"
      },
      {"id": "all", "short": "a", "long": "all", "description": "Show hidden", "type": "boolean"}
    ],
    "arguments": [
      {"id": "path", "display_name": "PATH", "description": "Directory to list", "type": "path", "required": false}
    ]
  }
  """

  # A spec with integer flags, used for Feature 4 tests.
  @integer_spec """
  {
    "cli_builder_spec_version": "1.0",
    "name": "numtool",
    "description": "A tool with integer flags",
    "version": "1.0",
    "flags": [
      {"id": "count", "long": "count", "short": "n", "description": "How many", "type": "integer"},
      {"id": "verbose", "short": "v", "long": "verbose", "description": "Verbosity", "type": "boolean"}
    ],
    "arguments": []
  }
  """

  # A basic spec for explicit_flags testing.
  @basic_spec """
  {
    "cli_builder_spec_version": "1.0",
    "name": "basic",
    "description": "A basic tool",
    "version": "1.0",
    "flags": [
      {"id": "verbose", "short": "v", "long": "verbose", "description": "Verbose", "type": "boolean"},
      {"id": "output", "short": "o", "long": "output", "description": "Output file", "type": "string"},
      {"id": "format", "long": "format", "description": "Format", "type": "enum", "enum_values": ["json", "csv", "xml"]}
    ],
    "arguments": [
      {"id": "file", "display_name": "FILE", "description": "Input file", "type": "string", "required": false}
    ]
  }
  """

  # ===========================================================================
  # Feature 1: Count Type
  # ===========================================================================

  describe "Feature 1: count type" do
    test "count flag defaults to 0 when absent" do
      {:ok, result} = Parser.parse_string(@count_spec, [])
      assert result.flags["verbose"] == 0
      assert result.flags["quiet"] == 0
    end

    test "single --verbose gives count 1" do
      {:ok, result} = Parser.parse_string(@count_spec, ["--verbose"])
      assert result.flags["verbose"] == 1
    end

    test "double --verbose --verbose gives count 2" do
      {:ok, result} = Parser.parse_string(@count_spec, ["--verbose", "--verbose"])
      assert result.flags["verbose"] == 2
    end

    test "triple --verbose gives count 3" do
      {:ok, result} =
        Parser.parse_string(@count_spec, ["--verbose", "--verbose", "--verbose"])

      assert result.flags["verbose"] == 3
    end

    test "short flag -v gives count 1" do
      {:ok, result} = Parser.parse_string(@count_spec, ["-v"])
      assert result.flags["verbose"] == 1
    end

    test "stacked short flags -vvv gives count 3" do
      {:ok, result} = Parser.parse_string(@count_spec, ["-vvv"])
      assert result.flags["verbose"] == 3
    end

    test "stacked short flags -vvvvv gives count 5" do
      {:ok, result} = Parser.parse_string(@count_spec, ["-vvvvv"])
      assert result.flags["verbose"] == 5
    end

    test "mixed stacking and separate: -vv --verbose gives count 3" do
      {:ok, result} = Parser.parse_string(@count_spec, ["-vv", "--verbose"])
      assert result.flags["verbose"] == 3
    end

    test "multiple different count flags: -v -q" do
      {:ok, result} = Parser.parse_string(@count_spec, ["-v", "-q"])
      assert result.flags["verbose"] == 1
      assert result.flags["quiet"] == 1
    end

    test "count flag can be stacked with boolean: -vvd" do
      {:ok, result} = Parser.parse_string(@count_spec, ["-vvd"])
      assert result.flags["verbose"] == 2
      assert result.flags["debug"] == true
    end

    test "count flag can be stacked with boolean in any order: -dvv" do
      {:ok, result} = Parser.parse_string(@count_spec, ["-dvv"])
      assert result.flags["debug"] == true
      assert result.flags["verbose"] == 2
    end

    test "count and boolean can interleave in a stack: -vdv" do
      {:ok, result} = Parser.parse_string(@count_spec, ["-vdv"])
      assert result.flags["verbose"] == 2
      assert result.flags["debug"] == true
    end

    test "count flag stacked with multiple different counts: -vvqq" do
      {:ok, result} = Parser.parse_string(@count_spec, ["-vvqq"])
      assert result.flags["verbose"] == 2
      assert result.flags["quiet"] == 2
    end
  end

  # ===========================================================================
  # Feature 2: Enum Optional Values (default_when_present)
  # ===========================================================================

  describe "Feature 2: default_when_present" do
    test "--color without value uses default_when_present (always)" do
      {:ok, result} = Parser.parse_string(@enum_dwp_spec, ["--color"])
      assert result.flags["color"] == "always"
    end

    test "--color=auto uses the explicit value" do
      {:ok, result} = Parser.parse_string(@enum_dwp_spec, ["--color=auto"])
      assert result.flags["color"] == "auto"
    end

    test "--color=never uses the explicit value" do
      {:ok, result} = Parser.parse_string(@enum_dwp_spec, ["--color=never"])
      assert result.flags["color"] == "never"
    end

    test "--color followed by valid enum value consumes it" do
      {:ok, result} = Parser.parse_string(@enum_dwp_spec, ["--color", "never"])
      assert result.flags["color"] == "never"
    end

    test "--color followed by non-enum token uses default_when_present" do
      {:ok, result} = Parser.parse_string(@enum_dwp_spec, ["--color", "/tmp"])
      assert result.flags["color"] == "always"
      # /tmp should be treated as a positional argument
      assert result.arguments["path"] == "/tmp"
    end

    test "--color followed by another flag uses default_when_present" do
      {:ok, result} = Parser.parse_string(@enum_dwp_spec, ["--color", "--all"])
      assert result.flags["color"] == "always"
      assert result.flags["all"] == true
    end

    test "--color at end of argv uses default_when_present" do
      {:ok, result} = Parser.parse_string(@enum_dwp_spec, ["--color"])
      assert result.flags["color"] == "always"
    end

    test "absent --color uses spec default (auto)" do
      {:ok, result} = Parser.parse_string(@enum_dwp_spec, [])
      assert result.flags["color"] == "auto"
    end

    test "short flag -c without value uses default_when_present" do
      {:ok, result} = Parser.parse_string(@enum_dwp_spec, ["-c"])
      assert result.flags["color"] == "always"
    end

    test "short flag -c followed by valid enum value consumes it" do
      {:ok, result} = Parser.parse_string(@enum_dwp_spec, ["-c", "never"])
      assert result.flags["color"] == "never"
    end

    test "short flag -c followed by non-enum uses default_when_present" do
      {:ok, result} = Parser.parse_string(@enum_dwp_spec, ["-c", "/tmp"])
      assert result.flags["color"] == "always"
      assert result.arguments["path"] == "/tmp"
    end

    test "spec validation rejects default_when_present on non-enum type" do
      invalid_spec = """
      {
        "cli_builder_spec_version": "1.0",
        "name": "bad",
        "description": "Bad spec",
        "flags": [
          {"id": "level", "long": "level", "description": "Level", "type": "string", "default_when_present": "high"}
        ]
      }
      """

      result = Validator.validate_spec_string(invalid_spec)
      assert result.valid == false
      assert hd(result.errors) =~ "default_when_present"
      assert hd(result.errors) =~ "must be \"enum\""
    end

    test "spec validation rejects default_when_present value not in enum_values" do
      invalid_spec = """
      {
        "cli_builder_spec_version": "1.0",
        "name": "bad",
        "description": "Bad spec",
        "flags": [
          {"id": "color", "long": "color", "description": "Color", "type": "enum",
           "enum_values": ["always", "never"],
           "default_when_present": "auto"}
        ]
      }
      """

      result = Validator.validate_spec_string(invalid_spec)
      assert result.valid == false
      assert hd(result.errors) =~ "default_when_present"
      assert hd(result.errors) =~ "not in enum_values"
    end
  end

  # ===========================================================================
  # Feature 3: Flag Presence Detection (explicit_flags)
  # ===========================================================================

  describe "Feature 3: explicit_flags" do
    test "explicit_flags is empty when no flags are provided" do
      {:ok, result} = Parser.parse_string(@basic_spec, [])
      assert result.explicit_flags == []
    end

    test "explicit_flags contains flag ID when --verbose is used" do
      {:ok, result} = Parser.parse_string(@basic_spec, ["--verbose"])
      assert "verbose" in result.explicit_flags
    end

    test "explicit_flags contains flag ID when -v is used" do
      {:ok, result} = Parser.parse_string(@basic_spec, ["-v"])
      assert "verbose" in result.explicit_flags
    end

    test "explicit_flags contains flag ID when --output is used with value" do
      {:ok, result} = Parser.parse_string(@basic_spec, ["--output", "file.txt"])
      assert "output" in result.explicit_flags
    end

    test "explicit_flags contains flag ID for --output=value form" do
      {:ok, result} = Parser.parse_string(@basic_spec, ["--output=file.txt"])
      assert "output" in result.explicit_flags
    end

    test "explicit_flags contains all explicitly set flags" do
      {:ok, result} = Parser.parse_string(@basic_spec, ["--verbose", "--output", "out.txt"])
      assert "verbose" in result.explicit_flags
      assert "output" in result.explicit_flags
    end

    test "explicit_flags does not contain flag IDs for defaulted flags" do
      {:ok, result} = Parser.parse_string(@basic_spec, ["--verbose"])
      # "output" was not explicitly set
      refute "output" in result.explicit_flags
      # "format" was not explicitly set
      refute "format" in result.explicit_flags
    end

    test "count flags appear once per occurrence in explicit_flags" do
      {:ok, result} = Parser.parse_string(@count_spec, ["-vvv"])
      # "verbose" should appear 3 times (once per v in the stack)
      verbose_count = Enum.count(result.explicit_flags, &(&1 == "verbose"))
      assert verbose_count == 3
    end

    test "count flags from separate args appear in explicit_flags" do
      {:ok, result} = Parser.parse_string(@count_spec, ["--verbose", "--verbose"])
      verbose_count = Enum.count(result.explicit_flags, &(&1 == "verbose"))
      assert verbose_count == 2
    end

    test "explicit_flags tracks enum flags with default_when_present" do
      {:ok, result} = Parser.parse_string(@enum_dwp_spec, ["--color"])
      assert "color" in result.explicit_flags
    end

    test "explicit_flags tracks enum flags with explicit value" do
      {:ok, result} = Parser.parse_string(@enum_dwp_spec, ["--color=never"])
      assert "color" in result.explicit_flags
    end

    test "explicit_flags for short flag with inline value -ofile.txt" do
      {:ok, result} = Parser.parse_string(@basic_spec, ["-ofile.txt"])
      assert "output" in result.explicit_flags
    end

    test "stacked boolean flags each appear in explicit_flags" do
      spec = """
      {
        "cli_builder_spec_version": "1.0",
        "name": "tool",
        "description": "A tool",
        "flags": [
          {"id": "a-flag", "short": "a", "description": "A", "type": "boolean"},
          {"id": "b-flag", "short": "b", "description": "B", "type": "boolean"},
          {"id": "c-flag", "short": "c", "description": "C", "type": "boolean"}
        ],
        "arguments": []
      }
      """

      {:ok, result} = Parser.parse_string(spec, ["-abc"])
      assert "a-flag" in result.explicit_flags
      assert "b-flag" in result.explicit_flags
      assert "c-flag" in result.explicit_flags
    end
  end

  # ===========================================================================
  # Feature 4: int64 Range Validation
  # ===========================================================================

  describe "Feature 4: int64 range validation" do
    test "normal integer values are accepted" do
      {:ok, result} = Parser.parse_string(@integer_spec, ["--count", "42"])
      assert result.flags["count"] == 42
    end

    test "zero is accepted" do
      {:ok, result} = Parser.parse_string(@integer_spec, ["--count", "0"])
      assert result.flags["count"] == 0
    end

    test "negative integers are accepted" do
      {:ok, result} = Parser.parse_string(@integer_spec, ["--count", "-100"])
      assert result.flags["count"] == -100
    end

    test "max int64 value is accepted" do
      {:ok, result} = Parser.parse_string(@integer_spec, ["--count", "9223372036854775807"])
      assert result.flags["count"] == 9_223_372_036_854_775_807
    end

    test "min int64 value is accepted" do
      {:ok, result} = Parser.parse_string(@integer_spec, ["--count", "-9223372036854775808"])
      assert result.flags["count"] == -9_223_372_036_854_775_808
    end

    test "value above max int64 is rejected" do
      {:error, errors} = Parser.parse_string(@integer_spec, ["--count", "9223372036854775808"])
      assert length(errors.errors) >= 1
      error = hd(errors.errors)
      assert error.error_type == "invalid_value"
      assert error.message =~ "outside int64 range"
    end

    test "value below min int64 is rejected" do
      {:error, errors} = Parser.parse_string(@integer_spec, ["--count", "-9223372036854775809"])
      assert length(errors.errors) >= 1
      error = hd(errors.errors)
      assert error.error_type == "invalid_value"
      assert error.message =~ "outside int64 range"
    end

    test "very large positive value is rejected" do
      {:error, errors} = Parser.parse_string(@integer_spec, ["--count", "99999999999999999999999"])
      error = hd(errors.errors)
      assert error.error_type == "invalid_value"
      assert error.message =~ "outside int64 range"
    end

    test "very large negative value is rejected" do
      {:error, errors} = Parser.parse_string(@integer_spec, ["--count", "-99999999999999999999999"])
      error = hd(errors.errors)
      assert error.error_type == "invalid_value"
      assert error.message =~ "outside int64 range"
    end

    test "int64 range check uses --count=VALUE form too" do
      {:error, errors} = Parser.parse_string(@integer_spec, ["--count=9223372036854775808"])
      error = hd(errors.errors)
      assert error.error_type == "invalid_value"
      assert error.message =~ "outside int64 range"
    end

    test "int64 range check uses -n short form" do
      {:error, errors} = Parser.parse_string(@integer_spec, ["-n", "9223372036854775808"])
      error = hd(errors.errors)
      assert error.error_type == "invalid_value"
      assert error.message =~ "outside int64 range"
    end
  end

  # ===========================================================================
  # Integration: multiple v1.1 features together
  # ===========================================================================

  describe "integration: multiple v1.1 features" do
    test "count + explicit_flags together" do
      {:ok, result} = Parser.parse_string(@count_spec, ["-vvv", "--quiet"])
      assert result.flags["verbose"] == 3
      assert result.flags["quiet"] == 1
      assert Enum.count(result.explicit_flags, &(&1 == "verbose")) == 3
      assert Enum.count(result.explicit_flags, &(&1 == "quiet")) == 1
      refute "debug" in result.explicit_flags
    end

    test "default_when_present + explicit_flags together" do
      {:ok, result} = Parser.parse_string(@enum_dwp_spec, ["--color", "--all"])
      assert result.flags["color"] == "always"
      assert result.flags["all"] == true
      assert "color" in result.explicit_flags
      assert "all" in result.explicit_flags
    end

    test "all features in one spec" do
      spec = """
      {
        "cli_builder_spec_version": "1.0",
        "name": "allfeatures",
        "description": "Test all v1.1 features",
        "version": "1.1",
        "flags": [
          {"id": "verbose", "short": "v", "long": "verbose", "description": "Verbosity", "type": "count"},
          {"id": "color", "long": "color", "description": "Color", "type": "enum",
           "enum_values": ["always", "auto", "never"], "default_when_present": "always"},
          {"id": "limit", "long": "limit", "short": "l", "description": "Limit", "type": "integer"}
        ],
        "arguments": []
      }
      """

      {:ok, result} =
        Parser.parse_string(spec, ["-vv", "--color", "--limit", "100"])

      assert result.flags["verbose"] == 2
      assert result.flags["color"] == "always"
      assert result.flags["limit"] == 100
      assert Enum.count(result.explicit_flags, &(&1 == "verbose")) == 2
      assert "color" in result.explicit_flags
      assert "limit" in result.explicit_flags
    end
  end
end
