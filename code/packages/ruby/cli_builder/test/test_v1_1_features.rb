# frozen_string_literal: true

require_relative "test_helper"

# ---------------------------------------------------------------------------
# Tests for CLI Builder v1.1 features
# ---------------------------------------------------------------------------
#
# This file tests the four backwards-compatible features added in v1.1:
#
#   1. Count type        — flags like -vvv that count occurrences
#   2. default_when_present — enum flags that work as boolean toggles
#   3. explicit_flags    — tracking which flags the user explicitly set
#   4. int64 range       — rejecting integers outside 64-bit signed range
#
# Each feature has its own test group with multiple test cases covering
# normal operation, edge cases, and error conditions.
# ---------------------------------------------------------------------------

class TestV11Features < Minitest::Test
  include CodingAdventures::CliBuilder

  # Helper: parse argv against an inline spec hash
  def parse(spec_hash, argv)
    Parser.new(nil, argv, spec_hash: spec_hash).parse
  end

  def assert_parse_error(spec_hash, argv, error_type: nil, message_match: nil)
    err = assert_raises(ParseErrors) { parse(spec_hash, argv) }
    if error_type
      assert(err.errors.any? { |e| e.error_type == error_type },
        "Expected error type #{error_type.inspect} but got: #{err.errors.map(&:error_type).inspect}")
    end
    if message_match
      assert(err.errors.any? { |e| e.message =~ message_match },
        "Expected message matching #{message_match.inspect} but got: #{err.errors.map(&:message).inspect}")
    end
    err
  end

  # ===========================================================================
  # Shared spec for count + explicit_flags tests
  # ===========================================================================

  COUNT_SPEC = {
    "cli_builder_spec_version" => "1.0",
    "name" => "curl",
    "description" => "Transfer data from or to a server",
    "version" => "8.0",
    "flags" => [
      {
        "id" => "verbose",
        "short" => "v",
        "long" => "verbose",
        "description" => "Increase verbosity (can be repeated: -v, -vv, -vvv)",
        "type" => "count",
        "default" => 0
      },
      {
        "id" => "silent",
        "short" => "s",
        "long" => "silent",
        "description" => "Silent mode",
        "type" => "boolean"
      },
      {
        "id" => "output",
        "short" => "o",
        "long" => "output",
        "description" => "Output file",
        "type" => "string"
      }
    ],
    "arguments" => [
      {
        "id" => "url",
        "name" => "URL",
        "description" => "The URL to request",
        "type" => "string",
        "required" => false
      }
    ]
  }.freeze

  # ===========================================================================
  # Feature 4: Count Type
  # ===========================================================================

  def test_count_single_short
    # -v → count is 1
    result = parse(COUNT_SPEC, ["curl", "-v"])
    assert_equal 1, result.flags["verbose"]
  end

  def test_count_stacked_short
    # -vvv → count is 3 (each 'v' in the stack increments)
    result = parse(COUNT_SPEC, ["curl", "-vvv"])
    assert_equal 3, result.flags["verbose"]
  end

  def test_count_repeated_long
    # --verbose --verbose → count is 2
    result = parse(COUNT_SPEC, ["curl", "--verbose", "--verbose"])
    assert_equal 2, result.flags["verbose"]
  end

  def test_count_mixed_short_and_long
    # -vv --verbose → count is 3
    result = parse(COUNT_SPEC, ["curl", "-vv", "--verbose"])
    assert_equal 3, result.flags["verbose"]
  end

  def test_count_absent_defaults_to_zero
    # No -v flag → defaults to 0
    result = parse(COUNT_SPEC, ["curl"])
    assert_equal 0, result.flags["verbose"]
  end

  def test_count_does_not_consume_value
    # --verbose 5 → verbose=1, "5" is positional (not consumed as value)
    result = parse(COUNT_SPEC, ["curl", "--verbose", "http://example.com"])
    assert_equal 1, result.flags["verbose"]
    assert_equal "http://example.com", result.arguments["url"]
  end

  def test_count_stacked_with_boolean
    # -vsv → v=count, s=boolean, v=count → verbose=2, silent=true
    result = parse(COUNT_SPEC, ["curl", "-vsv"])
    assert_equal 2, result.flags["verbose"]
    assert_equal true, result.flags["silent"]
  end

  def test_count_stacked_with_value_flag_at_end
    # -vvo file → v=count, v=count, o=string, value="file"
    result = parse(COUNT_SPEC, ["curl", "-vvo", "file.txt"])
    assert_equal 2, result.flags["verbose"]
    assert_equal "file.txt", result.flags["output"]
  end

  def test_count_five_repeats
    result = parse(COUNT_SPEC, ["curl", "-v", "-v", "-v", "-v", "-v"])
    assert_equal 5, result.flags["verbose"]
  end

  # ===========================================================================
  # Feature 1: Enum Optional Values (default_when_present)
  # ===========================================================================

  ENUM_SPEC = {
    "cli_builder_spec_version" => "1.0",
    "name" => "ls",
    "description" => "List directory contents",
    "version" => "9.0",
    "flags" => [
      {
        "id" => "color",
        "long" => "color",
        "short" => "c",
        "description" => "Colorize output",
        "type" => "enum",
        "enum_values" => ["always", "auto", "never"],
        "default" => "auto",
        "default_when_present" => "always"
      },
      {
        "id" => "long-listing",
        "short" => "l",
        "long" => "long",
        "description" => "Use long listing format",
        "type" => "boolean"
      },
      {
        "id" => "all",
        "short" => "a",
        "long" => "all",
        "description" => "Show hidden files",
        "type" => "boolean"
      }
    ],
    "arguments" => [
      {
        "id" => "path",
        "name" => "PATH",
        "description" => "Directory to list",
        "type" => "string",
        "required" => false
      }
    ]
  }.freeze

  def test_enum_with_equals_value
    # --color=always → standard enum parsing
    result = parse(ENUM_SPEC, ["ls", "--color=always"])
    assert_equal "always", result.flags["color"]
  end

  def test_enum_with_equals_never
    result = parse(ENUM_SPEC, ["ls", "--color=never"])
    assert_equal "never", result.flags["color"]
  end

  def test_enum_at_end_of_argv_uses_default_when_present
    # --color at end of argv → uses default_when_present ("always")
    result = parse(ENUM_SPEC, ["ls", "--color"])
    assert_equal "always", result.flags["color"]
  end

  def test_enum_followed_by_flag_uses_default_when_present
    # --color --long → color gets default_when_present, --long is separate
    result = parse(ENUM_SPEC, ["ls", "--color", "--long"])
    assert_equal "always", result.flags["color"]
    assert_equal true, result.flags["long-listing"]
  end

  def test_enum_followed_by_valid_enum_value_consumes_it
    # --color auto → "auto" is a valid enum value, consume it
    result = parse(ENUM_SPEC, ["ls", "--color", "auto"])
    assert_equal "auto", result.flags["color"]
  end

  def test_enum_followed_by_never_consumes_it
    result = parse(ENUM_SPEC, ["ls", "--color", "never"])
    assert_equal "never", result.flags["color"]
  end

  def test_enum_followed_by_non_enum_value_uses_default_positional
    # --color somedir → "somedir" is NOT an enum value,
    # use default_when_present, "somedir" becomes positional
    result = parse(ENUM_SPEC, ["ls", "--color", "/tmp"])
    assert_equal "always", result.flags["color"]
    assert_equal "/tmp", result.arguments["path"]
  end

  def test_enum_absent_uses_default
    # No --color flag → uses default ("auto")
    result = parse(ENUM_SPEC, ["ls"])
    assert_equal "auto", result.flags["color"]
  end

  def test_enum_short_flag_at_end
    # -c at end → uses default_when_present
    result = parse(ENUM_SPEC, ["ls", "-c"])
    assert_equal "always", result.flags["color"]
  end

  def test_enum_short_flag_followed_by_valid_value
    # -c auto → consumes "auto"
    result = parse(ENUM_SPEC, ["ls", "-c", "auto"])
    assert_equal "auto", result.flags["color"]
  end

  def test_enum_short_flag_followed_by_flag
    # -c -l → color gets default_when_present, -l is separate
    result = parse(ENUM_SPEC, ["ls", "-c", "-l"])
    assert_equal "always", result.flags["color"]
    assert_equal true, result.flags["long-listing"]
  end

  def test_enum_short_stacked_with_boolean
    # -lc → l=boolean, c=enum with default_when_present
    # c is last in stack with no inline value → uses default_when_present
    result = parse(ENUM_SPEC, ["ls", "-lc"])
    assert_equal true, result.flags["long-listing"]
    assert_equal "always", result.flags["color"]
  end

  def test_enum_short_followed_by_non_enum_value
    # -c somefile → "somefile" is not an enum value, use default_when_present
    result = parse(ENUM_SPEC, ["ls", "-c", "somefile.txt"])
    assert_equal "always", result.flags["color"]
    assert_equal "somefile.txt", result.arguments["path"]
  end

  # ===========================================================================
  # Feature 1: Spec validation for default_when_present
  # ===========================================================================

  def test_validation_default_when_present_not_in_enum_values
    spec = {
      "cli_builder_spec_version" => "1.0",
      "name" => "test",
      "description" => "Test",
      "flags" => [
        {
          "id" => "color",
          "long" => "color",
          "type" => "enum",
          "enum_values" => ["always", "auto", "never"],
          "default_when_present" => "maybe"
        }
      ]
    }
    # Must use with_spec_file because spec_hash: bypasses SpecLoader validation
    with_spec_file(spec) do |path|
      err = assert_raises(SpecError) { Parser.new(path, ["test"]).parse }
      assert_match(/default_when_present/, err.message)
      assert_match(/not in enum_values/, err.message)
    end
  end

  def test_validation_default_when_present_on_non_enum
    spec = {
      "cli_builder_spec_version" => "1.0",
      "name" => "test",
      "description" => "Test",
      "flags" => [
        {
          "id" => "output",
          "long" => "output",
          "type" => "string",
          "default_when_present" => "stdout"
        }
      ]
    }
    with_spec_file(spec) do |path|
      err = assert_raises(SpecError) { Parser.new(path, ["test"]).parse }
      assert_match(/default_when_present/, err.message)
      assert_match(/must be "enum"/, err.message)
    end
  end

  def test_validation_default_when_present_with_empty_enum_values
    spec = {
      "cli_builder_spec_version" => "1.0",
      "name" => "test",
      "description" => "Test",
      "flags" => [
        {
          "id" => "color",
          "long" => "color",
          "type" => "enum",
          "enum_values" => [],
          "default_when_present" => "always"
        }
      ]
    }
    # Caught by Rule 7 (enum with no values) or Rule 10
    with_spec_file(spec) do |path|
      assert_raises(SpecError) { Parser.new(path, ["test"]).parse }
    end
  end

  # ===========================================================================
  # Feature 3: Flag Presence Detection (explicit_flags)
  # ===========================================================================

  def test_explicit_flags_present_for_boolean
    result = parse(COUNT_SPEC, ["curl", "-s"])
    assert_includes result.explicit_flags, "silent"
  end

  def test_explicit_flags_absent_for_default
    result = parse(COUNT_SPEC, ["curl"])
    refute_includes result.explicit_flags, "silent"
    refute_includes result.explicit_flags, "verbose"
    refute_includes result.explicit_flags, "output"
  end

  def test_explicit_flags_for_string_flag
    result = parse(COUNT_SPEC, ["curl", "-o", "file.txt"])
    assert_includes result.explicit_flags, "output"
  end

  def test_explicit_flags_for_count_when_present
    result = parse(COUNT_SPEC, ["curl", "-vvv"])
    assert_includes result.explicit_flags, "verbose"
  end

  def test_explicit_flags_for_count_when_absent
    result = parse(COUNT_SPEC, ["curl"])
    refute_includes result.explicit_flags, "verbose"
  end

  def test_explicit_flags_multiple
    result = parse(COUNT_SPEC, ["curl", "-vs", "-o", "out.txt"])
    assert_includes result.explicit_flags, "verbose"
    assert_includes result.explicit_flags, "silent"
    assert_includes result.explicit_flags, "output"
  end

  def test_explicit_flags_not_duplicated_for_count
    # -v -v -v → verbose appears once in explicit_flags (it's a Set)
    result = parse(COUNT_SPEC, ["curl", "-v", "-v", "-v"])
    count = result.explicit_flags.count { |f| f == "verbose" }
    assert_equal 1, count
  end

  def test_explicit_flags_for_enum_with_equals
    result = parse(ENUM_SPEC, ["ls", "--color=always"])
    assert_includes result.explicit_flags, "color"
  end

  def test_explicit_flags_for_enum_with_default_when_present
    result = parse(ENUM_SPEC, ["ls", "--color"])
    assert_includes result.explicit_flags, "color"
  end

  def test_explicit_flags_enum_absent
    result = parse(ENUM_SPEC, ["ls"])
    refute_includes result.explicit_flags, "color"
  end

  def test_explicit_flags_for_long_flag_with_value
    result = parse(COUNT_SPEC, ["curl", "--output", "file.txt"])
    assert_includes result.explicit_flags, "output"
  end

  def test_explicit_flags_for_long_flag_with_equals
    result = parse(COUNT_SPEC, ["curl", "--output=file.txt"])
    assert_includes result.explicit_flags, "output"
  end

  # ===========================================================================
  # Feature 2: int64 Range Validation
  # ===========================================================================

  INT_SPEC = {
    "cli_builder_spec_version" => "1.0",
    "name" => "test",
    "description" => "Test integer ranges",
    "flags" => [
      {
        "id" => "count",
        "long" => "count",
        "short" => "n",
        "description" => "Number of items",
        "type" => "integer"
      }
    ]
  }.freeze

  def test_int64_normal_value
    result = parse(INT_SPEC, ["test", "--count", "42"])
    assert_equal 42, result.flags["count"]
  end

  def test_int64_negative_value
    result = parse(INT_SPEC, ["test", "--count", "-100"])
    assert_equal(-100, result.flags["count"])
  end

  def test_int64_max_value
    result = parse(INT_SPEC, ["test", "--count", "9223372036854775807"])
    assert_equal 9223372036854775807, result.flags["count"]
  end

  def test_int64_min_value
    result = parse(INT_SPEC, ["test", "--count", "-9223372036854775808"])
    assert_equal(-9223372036854775808, result.flags["count"])
  end

  def test_int64_overflow
    assert_parse_error(INT_SPEC, ["test", "--count", "9223372036854775808"],
      error_type: "invalid_value",
      message_match: /out of range/)
  end

  def test_int64_underflow
    assert_parse_error(INT_SPEC, ["test", "--count", "-9223372036854775809"],
      error_type: "invalid_value",
      message_match: /out of range/)
  end

  def test_int64_way_out_of_range
    assert_parse_error(INT_SPEC, ["test", "--count", "99999999999999999999"],
      error_type: "invalid_value",
      message_match: /out of range/)
  end

  def test_int64_zero
    result = parse(INT_SPEC, ["test", "--count", "0"])
    assert_equal 0, result.flags["count"]
  end

  # ===========================================================================
  # Integration: multiple v1.1 features together
  # ===========================================================================

  COMBINED_SPEC = {
    "cli_builder_spec_version" => "1.0",
    "name" => "tool",
    "description" => "A tool with v1.1 features",
    "version" => "1.1.0",
    "flags" => [
      {
        "id" => "verbose",
        "short" => "v",
        "long" => "verbose",
        "description" => "Verbosity",
        "type" => "count",
        "default" => 0
      },
      {
        "id" => "color",
        "long" => "color",
        "short" => "c",
        "description" => "Colorize output",
        "type" => "enum",
        "enum_values" => ["always", "auto", "never"],
        "default" => "auto",
        "default_when_present" => "always"
      },
      {
        "id" => "limit",
        "long" => "limit",
        "short" => "n",
        "description" => "Max items",
        "type" => "integer"
      }
    ],
    "arguments" => [
      {
        "id" => "path",
        "name" => "PATH",
        "description" => "Path",
        "type" => "string",
        "required" => false
      }
    ]
  }.freeze

  def test_combined_count_and_enum_and_explicit
    result = parse(COMBINED_SPEC, ["tool", "-vvv", "--color", "--limit", "10", "/tmp"])
    assert_equal 3, result.flags["verbose"]
    assert_equal "always", result.flags["color"]
    assert_equal 10, result.flags["limit"]
    assert_equal "/tmp", result.arguments["path"]
    assert_includes result.explicit_flags, "verbose"
    assert_includes result.explicit_flags, "color"
    assert_includes result.explicit_flags, "limit"
  end

  def test_combined_defaults_and_explicit_flags
    result = parse(COMBINED_SPEC, ["tool"])
    assert_equal 0, result.flags["verbose"]
    assert_equal "auto", result.flags["color"]
    assert_nil result.flags["limit"]
    assert_empty result.explicit_flags
  end

  def test_combined_enum_with_following_positional
    # --color /tmp → "auto" is not a valid enum, so color=always, /tmp=positional
    result = parse(COMBINED_SPEC, ["tool", "--color", "/tmp"])
    assert_equal "always", result.flags["color"]
    assert_equal "/tmp", result.arguments["path"]
  end

  def test_combined_int64_overflow_with_valid_flags
    assert_parse_error(COMBINED_SPEC,
      ["tool", "-vv", "--limit", "99999999999999999999"],
      error_type: "invalid_value")
  end

  # ===========================================================================
  # Help generator: count and default_when_present display
  # ===========================================================================

  def test_help_count_flag_no_value_shown
    result = parse(COMBINED_SPEC, ["tool", "--help"])
    assert_instance_of HelpResult, result
    # Count flags should NOT show a value placeholder (like boolean)
    refute_match(/<COUNT>/, result.text)
    assert_match(/--verbose/, result.text)
  end

  def test_help_enum_with_default_when_present_shows_optional_value
    result = parse(COMBINED_SPEC, ["tool", "--help"])
    assert_instance_of HelpResult, result
    # Enum with default_when_present should show [=VALUE]
    assert_match(/\[=ENUM\]/, result.text)
  end
end
