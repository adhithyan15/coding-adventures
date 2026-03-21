# frozen_string_literal: true

require_relative "test_helper"

# Tests for PositionalResolver — the component that assigns positional tokens
# to named argument slots according to the spec definitions.
#
# Coverage goals:
#   - No-variadic path: one-to-one, too many, missing required, missing optional
#   - Variadic path: leading-wins, trailing-wins (last-wins algorithm), vmin, vmax
#   - required_unless_flag exemption
#   - All coerceable types: boolean, integer, float, path, file, directory, enum, string
class TestPositionalResolver < Minitest::Test
  include CodingAdventures::CliBuilder

  # ---------------------------------------------------------------------------
  # Helpers
  # ---------------------------------------------------------------------------

  def resolver(arg_defs)
    PositionalResolver.new(arg_defs)
  end

  def resolve(arg_defs, tokens, parsed_flags = {})
    resolver(arg_defs).resolve(tokens, parsed_flags)
  end

  def assert_resolve_error(arg_defs, tokens, error_type:, parsed_flags: {})
    err = assert_raises(ParseErrors) { resolve(arg_defs, tokens, parsed_flags) }
    assert(
      err.errors.any? { |e| e.error_type == error_type },
      "Expected error #{error_type.inspect}, got: #{err.errors.map(&:error_type).inspect}"
    )
    err
  end

  # ---------------------------------------------------------------------------
  # No-variadic: one-to-one assignment
  # ---------------------------------------------------------------------------

  def test_no_variadic_exact_match
    defs = [
      {"id" => "src", "name" => "SRC", "type" => "string", "required" => true},
      {"id" => "dst", "name" => "DST", "type" => "string", "required" => true}
    ]
    result = resolve(defs, ["a.txt", "b.txt"])
    assert_equal "a.txt", result["src"]
    assert_equal "b.txt", result["dst"]
  end

  def test_no_variadic_too_many_tokens
    defs = [
      {"id" => "src", "name" => "SRC", "type" => "string", "required" => true}
    ]
    assert_resolve_error(defs, ["a.txt", "b.txt", "c.txt"], error_type: "too_many_arguments")
  end

  def test_no_variadic_missing_required_arg
    defs = [
      {"id" => "src", "name" => "SRC", "type" => "string", "required" => true},
      {"id" => "dst", "name" => "DST", "type" => "string", "required" => true}
    ]
    assert_resolve_error(defs, ["a.txt"], error_type: "missing_required_argument")
  end

  def test_no_variadic_missing_optional_arg_uses_default
    defs = [
      {"id" => "src", "name" => "SRC", "type" => "string", "required" => true},
      {"id" => "mode", "name" => "MODE", "type" => "string", "required" => false,
       "default" => "copy", "required_unless_flag" => []}
    ]
    result = resolve(defs, ["a.txt"])
    assert_equal "a.txt", result["src"]
    assert_equal "copy", result["mode"]
  end

  def test_no_variadic_all_optional_empty_tokens
    defs = [
      {"id" => "opt", "name" => "OPT", "type" => "string", "required" => false,
       "default" => nil, "required_unless_flag" => []}
    ]
    result = resolve(defs, [])
    assert_nil result["opt"]
  end

  # ---------------------------------------------------------------------------
  # No-variadic: required_unless_flag exemption
  # ---------------------------------------------------------------------------

  def test_no_variadic_required_unless_flag_exempts_missing_arg
    defs = [
      {"id" => "pattern", "name" => "PATTERN", "type" => "string", "required" => true,
       "required_unless_flag" => ["pattern-option"]}
    ]
    # With pattern-option flag present, the required arg is optional
    result = resolve(defs, [], {"pattern-option" => "foo"})
    assert_nil result["pattern"]
  end

  def test_no_variadic_required_unless_flag_not_exempt_when_flag_absent
    defs = [
      {"id" => "pattern", "name" => "PATTERN", "type" => "string", "required" => true,
       "required_unless_flag" => ["pattern-option"]}
    ]
    assert_resolve_error(defs, [], error_type: "missing_required_argument")
  end

  # ---------------------------------------------------------------------------
  # Type coercion: integer
  # ---------------------------------------------------------------------------

  def test_coerce_integer_valid
    defs = [{"id" => "n", "name" => "N", "type" => "integer", "required" => true}]
    result = resolve(defs, ["42"])
    assert_equal 42, result["n"]
    assert_instance_of Integer, result["n"]
  end

  def test_coerce_integer_invalid
    defs = [{"id" => "n", "name" => "N", "type" => "integer", "required" => true}]
    assert_resolve_error(defs, ["abc"], error_type: "invalid_value")
  end

  def test_coerce_integer_negative
    defs = [{"id" => "n", "name" => "N", "type" => "integer", "required" => true}]
    result = resolve(defs, ["-7"])
    assert_equal(-7, result["n"])
  end

  # ---------------------------------------------------------------------------
  # Type coercion: float
  # ---------------------------------------------------------------------------

  def test_coerce_float_valid
    defs = [{"id" => "ratio", "name" => "RATIO", "type" => "float", "required" => true}]
    result = resolve(defs, ["3.14"])
    assert_in_delta 3.14, result["ratio"]
    assert_instance_of Float, result["ratio"]
  end

  def test_coerce_float_invalid
    defs = [{"id" => "ratio", "name" => "RATIO", "type" => "float", "required" => true}]
    assert_resolve_error(defs, ["abc"], error_type: "invalid_value")
  end

  def test_coerce_float_integer_value
    # Ruby Float("42") works fine
    defs = [{"id" => "ratio", "name" => "RATIO", "type" => "float", "required" => true}]
    result = resolve(defs, ["42"])
    assert_instance_of Float, result["ratio"]
    assert_in_delta 42.0, result["ratio"]
  end

  # ---------------------------------------------------------------------------
  # Type coercion: boolean
  # ---------------------------------------------------------------------------

  def test_coerce_boolean_true
    defs = [{"id" => "flag", "name" => "FLAG", "type" => "boolean", "required" => true}]
    result = resolve(defs, ["true"])
    assert_equal true, result["flag"]
  end

  def test_coerce_boolean_false_string
    defs = [{"id" => "flag", "name" => "FLAG", "type" => "boolean", "required" => true}]
    result = resolve(defs, ["false"])
    assert_equal false, result["flag"]
  end

  def test_coerce_boolean_other_string
    defs = [{"id" => "flag", "name" => "FLAG", "type" => "boolean", "required" => true}]
    result = resolve(defs, ["yes"])
    assert_equal false, result["flag"]
  end

  # ---------------------------------------------------------------------------
  # Type coercion: path (any string accepted)
  # ---------------------------------------------------------------------------

  def test_coerce_path_returns_string_unchanged
    defs = [{"id" => "p", "name" => "PATH", "type" => "path", "required" => true}]
    result = resolve(defs, ["/some/weird/../path"])
    assert_equal "/some/weird/../path", result["p"]
  end

  # ---------------------------------------------------------------------------
  # Type coercion: file (must exist and be readable)
  # ---------------------------------------------------------------------------

  def test_coerce_file_valid_readable_file
    require "tempfile"
    f = Tempfile.new("cli_test")
    f.write("hello")
    f.close
    defs = [{"id" => "inp", "name" => "INPUT", "type" => "file", "required" => true}]
    result = resolve(defs, [f.path])
    assert_equal f.path, result["inp"]
  ensure
    f.unlink
  end

  def test_coerce_file_nonexistent_raises_error
    defs = [{"id" => "inp", "name" => "INPUT", "type" => "file", "required" => true}]
    assert_resolve_error(defs, ["/nonexistent/path/file.txt"], error_type: "invalid_value")
  end

  def test_coerce_file_directory_instead_of_file_raises_error
    require "tmpdir"
    defs = [{"id" => "inp", "name" => "INPUT", "type" => "file", "required" => true}]
    # Dir.tmpdir is a directory, not a file
    assert_resolve_error(defs, [Dir.tmpdir], error_type: "invalid_value")
  end

  # ---------------------------------------------------------------------------
  # Type coercion: directory (must exist and be a directory)
  # ---------------------------------------------------------------------------

  def test_coerce_directory_valid
    require "tmpdir"
    defs = [{"id" => "d", "name" => "DIR", "type" => "directory", "required" => true}]
    result = resolve(defs, [Dir.tmpdir])
    assert_equal Dir.tmpdir, result["d"]
  end

  def test_coerce_directory_nonexistent_raises_error
    defs = [{"id" => "d", "name" => "DIR", "type" => "directory", "required" => true}]
    assert_resolve_error(defs, ["/nonexistent/dir"], error_type: "invalid_value")
  end

  def test_coerce_directory_file_instead_of_dir_raises_error
    require "tempfile"
    f = Tempfile.new("cli_test")
    f.write("hello")
    f.close
    defs = [{"id" => "d", "name" => "DIR", "type" => "directory", "required" => true}]
    assert_resolve_error(defs, [f.path], error_type: "invalid_value")
  ensure
    f.unlink
  end

  # ---------------------------------------------------------------------------
  # Type coercion: enum
  # ---------------------------------------------------------------------------

  def test_coerce_enum_valid_value
    defs = [{"id" => "fmt", "name" => "FORMAT", "type" => "enum",
             "enum_values" => ["json", "csv", "table"], "required" => true}]
    result = resolve(defs, ["csv"])
    assert_equal "csv", result["fmt"]
  end

  def test_coerce_enum_invalid_value
    defs = [{"id" => "fmt", "name" => "FORMAT", "type" => "enum",
             "enum_values" => ["json", "csv", "table"], "required" => true}]
    assert_resolve_error(defs, ["xml"], error_type: "invalid_enum_value")
  end

  # ---------------------------------------------------------------------------
  # Type coercion: string (default / unknown type fallthrough)
  # ---------------------------------------------------------------------------

  def test_coerce_string_returns_value
    defs = [{"id" => "name", "name" => "NAME", "type" => "string", "required" => true}]
    result = resolve(defs, ["hello world"])
    assert_equal "hello world", result["name"]
  end

  def test_coerce_unknown_type_fallback_to_string
    # A type we don't specifically handle falls through to the else branch
    defs = [{"id" => "x", "name" => "X", "type" => "custom_type", "required" => true}]
    result = resolve(defs, ["some_value"])
    assert_equal "some_value", result["x"]
  end

  # ---------------------------------------------------------------------------
  # Variadic path: basic
  # ---------------------------------------------------------------------------

  def test_variadic_only_single_token
    defs = [
      {"id" => "files", "name" => "FILE", "type" => "string",
       "required" => false, "variadic" => true, "variadic_min" => 0,
       "variadic_max" => nil}
    ]
    result = resolve(defs, ["a.txt"])
    assert_equal ["a.txt"], result["files"]
  end

  def test_variadic_only_multiple_tokens
    defs = [
      {"id" => "files", "name" => "FILE", "type" => "string",
       "required" => false, "variadic" => true, "variadic_min" => 0,
       "variadic_max" => nil}
    ]
    result = resolve(defs, ["a.txt", "b.txt", "c.txt"])
    assert_equal ["a.txt", "b.txt", "c.txt"], result["files"]
  end

  def test_variadic_only_zero_tokens_with_min_0
    defs = [
      {"id" => "files", "name" => "FILE", "type" => "string",
       "required" => false, "variadic" => true, "variadic_min" => 0,
       "variadic_max" => nil}
    ]
    result = resolve(defs, [])
    assert_equal [], result["files"]
  end

  def test_variadic_min_not_met_raises_error
    defs = [
      {"id" => "files", "name" => "FILE", "type" => "string",
       "required" => true, "variadic" => true, "variadic_min" => 2,
       "variadic_max" => nil}
    ]
    assert_resolve_error(defs, ["a.txt"], error_type: "too_few_arguments")
  end

  def test_variadic_max_exceeded_raises_error
    defs = [
      {"id" => "files", "name" => "FILE", "type" => "string",
       "required" => false, "variadic" => true, "variadic_min" => 0,
       "variadic_max" => 2}
    ]
    assert_resolve_error(defs, ["a.txt", "b.txt", "c.txt"], error_type: "too_many_arguments")
  end

  def test_variadic_max_exactly_met_is_valid
    defs = [
      {"id" => "files", "name" => "FILE", "type" => "string",
       "required" => false, "variadic" => true, "variadic_min" => 0,
       "variadic_max" => 3}
    ]
    result = resolve(defs, ["a.txt", "b.txt", "c.txt"])
    assert_equal ["a.txt", "b.txt", "c.txt"], result["files"]
  end

  # ---------------------------------------------------------------------------
  # Variadic path: leading + variadic (no trailing)
  # ---------------------------------------------------------------------------

  def test_variadic_with_leading_fixed_arg
    defs = [
      {"id" => "prefix", "name" => "PREFIX", "type" => "string", "required" => true},
      {"id" => "files", "name" => "FILE", "type" => "string",
       "required" => false, "variadic" => true, "variadic_min" => 0,
       "variadic_max" => nil}
    ]
    result = resolve(defs, ["myprefix", "a.txt", "b.txt"])
    assert_equal "myprefix", result["prefix"]
    assert_equal ["a.txt", "b.txt"], result["files"]
  end

  # ---------------------------------------------------------------------------
  # Variadic path: variadic + trailing (last-wins algorithm)
  # ---------------------------------------------------------------------------

  def test_variadic_with_trailing_required_arg
    defs = [
      {"id" => "sources", "name" => "SOURCE", "type" => "string",
       "required" => true, "variadic" => true, "variadic_min" => 1,
       "variadic_max" => nil},
      {"id" => "dest", "name" => "DEST", "type" => "string", "required" => true}
    ]
    result = resolve(defs, ["a.txt", "b.txt", "c.txt", "/dest/"])
    assert_equal ["a.txt", "b.txt", "c.txt"], result["sources"]
    assert_equal "/dest/", result["dest"]
  end

  def test_variadic_with_trailing_missing_required_trailing
    defs = [
      {"id" => "sources", "name" => "SOURCE", "type" => "string",
       "required" => true, "variadic" => true, "variadic_min" => 1,
       "variadic_max" => nil},
      {"id" => "dest", "name" => "DEST", "type" => "string", "required" => true}
    ]
    # Only one token — not enough for both variadic_min=1 and trailing required=1
    assert_resolve_error(defs, ["a.txt"], error_type: "too_few_arguments")
  end

  def test_variadic_with_trailing_optional_arg_absent
    defs = [
      {"id" => "sources", "name" => "SOURCE", "type" => "string",
       "required" => false, "variadic" => true, "variadic_min" => 0,
       "variadic_max" => nil},
      {"id" => "dest", "name" => "DEST", "type" => "string",
       "required" => false, "default" => nil, "required_unless_flag" => []}
    ]
    result = resolve(defs, [])
    assert_equal [], result["sources"]
    assert_nil result["dest"]
  end

  # ---------------------------------------------------------------------------
  # Variadic path: leading not satisfied triggers error
  # ---------------------------------------------------------------------------

  def test_variadic_leading_required_missing_when_zero_tokens
    defs = [
      {"id" => "pattern", "name" => "PATTERN", "type" => "string",
       "required" => true, "required_unless_flag" => []},
      {"id" => "files", "name" => "FILE", "type" => "string",
       "required" => false, "variadic" => true, "variadic_min" => 0,
       "variadic_max" => nil}
    ]
    assert_resolve_error(defs, [], error_type: "missing_required_argument")
  end

  # ---------------------------------------------------------------------------
  # Variadic path: n == n_leading + n_trailing with required_unless_flag
  # ---------------------------------------------------------------------------

  def test_variadic_leading_required_unless_flag_not_satisfied_triggers_error
    # This tests the n == n_leading + n_trailing code path where required_unless_flag
    # is present but the flag is absent — we should get a missing_required_argument
    defs = [
      {"id" => "pattern", "name" => "PATTERN", "type" => "string",
       "required" => true, "required_unless_flag" => ["pattern-option"]},
      {"id" => "files", "name" => "FILE", "type" => "string",
       "required" => false, "variadic" => true, "variadic_min" => 0,
       "variadic_max" => nil}
    ]
    # n=1, n_leading=1, n_trailing=0 → n == n_leading + n_trailing
    # pattern-option is not in parsed_flags → required_unless not satisfied → error
    assert_resolve_error(defs, ["file.txt"], error_type: "missing_required_argument",
      parsed_flags: {})
  end

  def test_variadic_leading_required_unless_flag_satisfied_no_error
    # Same scenario but with pattern-option present → no error
    defs = [
      {"id" => "pattern", "name" => "PATTERN", "type" => "string",
       "required" => true, "required_unless_flag" => ["pattern-option"]},
      {"id" => "files", "name" => "FILE", "type" => "string",
       "required" => false, "variadic" => true, "variadic_min" => 0,
       "variadic_max" => nil}
    ]
    result = resolve(defs, ["file.txt"], {"pattern-option" => "foo"})
    # When flag is present: n == n_leading + n_trailing and required_unless satisfied
    # → no error, pattern gets "file.txt", files gets []
    assert_equal "file.txt", result["pattern"]
    assert_equal [], result["files"]
  end

  # ---------------------------------------------------------------------------
  # Variadic path: not enough tokens for leading + trailing minimums
  # ---------------------------------------------------------------------------

  def test_variadic_not_enough_tokens_for_fixed_args
    # Tests the n < n_leading + n_trailing branch
    defs = [
      {"id" => "a", "name" => "A", "type" => "string", "required" => true},
      {"id" => "files", "name" => "FILE", "type" => "string",
       "required" => false, "variadic" => true, "variadic_min" => 0,
       "variadic_max" => nil},
      {"id" => "b", "name" => "B", "type" => "string", "required" => true}
    ]
    # n=1 < n_leading(1) + n_trailing(1) = 2 → both required args missing
    assert_resolve_error(defs, ["only_one"], error_type: "missing_required_argument")
  end

  def test_variadic_not_enough_tokens_leading_and_trailing_both_required
    defs = [
      {"id" => "lead", "name" => "LEAD", "type" => "string", "required" => true},
      {"id" => "mid", "name" => "MID", "type" => "string",
       "required" => false, "variadic" => true, "variadic_min" => 0,
       "variadic_max" => nil},
      {"id" => "trail", "name" => "TRAIL", "type" => "string", "required" => true}
    ]
    # Zero tokens — both lead and trail are missing
    assert_resolve_error(defs, [], error_type: "missing_required_argument")
  end
end
