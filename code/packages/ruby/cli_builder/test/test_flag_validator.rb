# frozen_string_literal: true

require_relative "test_helper"

# Tests for FlagValidator — validates parsed_flags against spec constraints:
#   1. Duplicate non-repeatable flags
#   2. conflicts_with violations
#   3. requires violations (transitive)
#   4. required flags (with required_unless exemption)
#   5. mutually exclusive groups (violation + required group missing)
class TestFlagValidator < Minitest::Test
  include CodingAdventures::CliBuilder

  # ---------------------------------------------------------------------------
  # Helpers
  # ---------------------------------------------------------------------------

  def validator(flags, groups = [])
    FlagValidator.new(flags, groups)
  end

  def validate(flags_def, parsed_flags, duplicate_flags: [], groups: [])
    validator(flags_def, groups).validate(parsed_flags, duplicate_flags)
  end

  def assert_error(errors, type)
    assert(
      errors.any? { |e| e.error_type == type },
      "Expected error #{type.inspect}, got: #{errors.map(&:error_type).inspect}"
    )
  end

  def refute_error(errors, type)
    refute(
      errors.any? { |e| e.error_type == type },
      "Expected no #{type.inspect} error, but got one"
    )
  end

  # ---------------------------------------------------------------------------
  # Rule 1: duplicate non-repeatable flags
  # ---------------------------------------------------------------------------

  def test_duplicate_non_repeatable_flag_raises_error
    flags = [
      {"id" => "verbose", "short" => "v", "type" => "boolean",
       "required" => false, "repeatable" => false,
       "conflicts_with" => [], "requires" => [], "required_unless" => []}
    ]
    errors = validate(flags, {"verbose" => true}, duplicate_flags: ["verbose"])
    assert_error(errors, "duplicate_flag")
    assert_match(/more than once/, errors.first.message)
  end

  def test_duplicate_repeatable_flag_no_error
    flags = [
      {"id" => "header", "short" => "H", "type" => "string",
       "required" => false, "repeatable" => true,
       "conflicts_with" => [], "requires" => [], "required_unless" => []}
    ]
    errors = validate(flags, {"header" => ["v1", "v2"]}, duplicate_flags: ["header"])
    refute_error(errors, "duplicate_flag")
  end

  def test_duplicate_unknown_flag_id_is_ignored
    # If the duplicate id doesn't correspond to a known flag, skip it
    flags = [
      {"id" => "verbose", "short" => "v", "type" => "boolean",
       "required" => false, "repeatable" => false,
       "conflicts_with" => [], "requires" => [], "required_unless" => []}
    ]
    errors = validate(flags, {"verbose" => true}, duplicate_flags: ["nonexistent"])
    assert_equal [], errors
  end

  # ---------------------------------------------------------------------------
  # Rule 2: conflicts_with
  # ---------------------------------------------------------------------------

  def test_conflicting_flags_raises_error
    flags = [
      {"id" => "foo", "short" => "f", "type" => "boolean",
       "required" => false, "repeatable" => false,
       "conflicts_with" => ["bar"], "requires" => [], "required_unless" => []},
      {"id" => "bar", "short" => "b", "type" => "boolean",
       "required" => false, "repeatable" => false,
       "conflicts_with" => ["foo"], "requires" => [], "required_unless" => []}
    ]
    errors = validate(flags, {"foo" => true, "bar" => true})
    assert_error(errors, "conflicting_flags")
  end

  def test_conflicting_flags_reported_only_once
    # Both foo and bar list each other in conflicts_with. The error should appear once.
    flags = [
      {"id" => "foo", "short" => "f", "type" => "boolean",
       "required" => false, "repeatable" => false,
       "conflicts_with" => ["bar"], "requires" => [], "required_unless" => []},
      {"id" => "bar", "short" => "b", "type" => "boolean",
       "required" => false, "repeatable" => false,
       "conflicts_with" => ["foo"], "requires" => [], "required_unless" => []}
    ]
    errors = validate(flags, {"foo" => true, "bar" => true})
    conflict_errors = errors.select { |e| e.error_type == "conflicting_flags" }
    assert_equal 1, conflict_errors.size
  end

  def test_no_conflict_when_only_one_present
    flags = [
      {"id" => "foo", "short" => "f", "type" => "boolean",
       "required" => false, "repeatable" => false,
       "conflicts_with" => ["bar"], "requires" => [], "required_unless" => []},
      {"id" => "bar", "short" => "b", "type" => "boolean",
       "required" => false, "repeatable" => false,
       "conflicts_with" => ["foo"], "requires" => [], "required_unless" => []}
    ]
    errors = validate(flags, {"foo" => true})
    refute_error(errors, "conflicting_flags")
  end

  def test_conflict_with_flag_not_in_flag_by_id_shows_unknown_name
    # conflicts_with references an id whose flag def is not in @flag_by_id.
    # The conflict is still reported (because parsed_flags has the key), but
    # the display name for the conflicting flag falls back to "???".
    flags = [
      {"id" => "foo", "short" => "f", "type" => "boolean",
       "required" => false, "repeatable" => false,
       "conflicts_with" => ["ghost"], "requires" => [], "required_unless" => []}
    ]
    errors = validate(flags, {"foo" => true, "ghost" => true})
    conflict_errors = errors.select { |e| e.error_type == "conflicting_flags" }
    assert_equal 1, conflict_errors.size
    assert_match(/\?\?\?/, conflict_errors.first.message)
  end

  # ---------------------------------------------------------------------------
  # Rule 3: requires violations (transitive)
  # ---------------------------------------------------------------------------

  def test_requires_direct_violation
    flags = [
      {"id" => "human", "short" => "h", "type" => "boolean",
       "required" => false, "repeatable" => false,
       "conflicts_with" => [], "requires" => ["long"], "required_unless" => []},
      {"id" => "long", "short" => "l", "type" => "boolean",
       "required" => false, "repeatable" => false,
       "conflicts_with" => [], "requires" => [], "required_unless" => []}
    ]
    errors = validate(flags, {"human" => true})
    assert_error(errors, "missing_dependency_flag")
    assert_match(/requires/, errors.first.message)
  end

  def test_requires_satisfied_no_error
    flags = [
      {"id" => "human", "short" => "h", "type" => "boolean",
       "required" => false, "repeatable" => false,
       "conflicts_with" => [], "requires" => ["long"], "required_unless" => []},
      {"id" => "long", "short" => "l", "type" => "boolean",
       "required" => false, "repeatable" => false,
       "conflicts_with" => [], "requires" => [], "required_unless" => []}
    ]
    errors = validate(flags, {"human" => true, "long" => true})
    refute_error(errors, "missing_dependency_flag")
  end

  def test_requires_transitive_chain
    # a requires b, b requires c; if only a is present, error for b and c
    flags = [
      {"id" => "a", "short" => "a", "type" => "boolean",
       "required" => false, "repeatable" => false,
       "conflicts_with" => [], "requires" => ["b"], "required_unless" => []},
      {"id" => "b", "short" => "b", "type" => "boolean",
       "required" => false, "repeatable" => false,
       "conflicts_with" => [], "requires" => ["c"], "required_unless" => []},
      {"id" => "c", "short" => "c", "type" => "boolean",
       "required" => false, "repeatable" => false,
       "conflicts_with" => [], "requires" => [], "required_unless" => []}
    ]
    errors = validate(flags, {"a" => true})
    dep_errors = errors.select { |e| e.error_type == "missing_dependency_flag" }
    # Should have errors for b and c (transitively required)
    assert dep_errors.size >= 1
  end

  # ---------------------------------------------------------------------------
  # Rule 4: required flags
  # ---------------------------------------------------------------------------

  def test_required_flag_missing_raises_error
    flags = [
      {"id" => "output", "long" => "output", "type" => "string",
       "required" => true, "repeatable" => false,
       "conflicts_with" => [], "requires" => [], "required_unless" => []}
    ]
    errors = validate(flags, {})
    assert_error(errors, "missing_required_flag")
    assert_match(/is required/, errors.first.message)
  end

  def test_required_flag_present_no_error
    flags = [
      {"id" => "output", "long" => "output", "type" => "string",
       "required" => true, "repeatable" => false,
       "conflicts_with" => [], "requires" => [], "required_unless" => []}
    ]
    errors = validate(flags, {"output" => "file.txt"})
    refute_error(errors, "missing_required_flag")
  end

  def test_required_unless_exemption_when_other_flag_present
    # --output is required unless --stdout is present
    flags = [
      {"id" => "output", "long" => "output", "type" => "string",
       "required" => true, "repeatable" => false,
       "conflicts_with" => [], "requires" => [],
       "required_unless" => ["stdout"]},
      {"id" => "stdout", "long" => "stdout", "type" => "boolean",
       "required" => false, "repeatable" => false,
       "conflicts_with" => [], "requires" => [], "required_unless" => []}
    ]
    errors = validate(flags, {"stdout" => true})
    refute_error(errors, "missing_required_flag")
  end

  def test_required_unless_no_exemption_when_other_flag_absent
    flags = [
      {"id" => "output", "long" => "output", "type" => "string",
       "required" => true, "repeatable" => false,
       "conflicts_with" => [], "requires" => [],
       "required_unless" => ["stdout"]},
      {"id" => "stdout", "long" => "stdout", "type" => "boolean",
       "required" => false, "repeatable" => false,
       "conflicts_with" => [], "requires" => [], "required_unless" => []}
    ]
    errors = validate(flags, {})
    assert_error(errors, "missing_required_flag")
  end

  # ---------------------------------------------------------------------------
  # Rule 5: mutually exclusive groups
  # ---------------------------------------------------------------------------

  def test_exclusive_group_violation
    flags = [
      {"id" => "json", "long" => "json", "type" => "boolean",
       "required" => false, "repeatable" => false,
       "conflicts_with" => [], "requires" => [], "required_unless" => []},
      {"id" => "csv", "long" => "csv", "type" => "boolean",
       "required" => false, "repeatable" => false,
       "conflicts_with" => [], "requires" => [], "required_unless" => []}
    ]
    groups = [{"id" => "format", "flag_ids" => ["json", "csv"], "required" => false}]
    errors = validate(flags, {"json" => true, "csv" => true}, groups: groups)
    assert_error(errors, "exclusive_group_violation")
    assert_match(/Only one of/, errors.first.message)
  end

  def test_exclusive_group_no_violation_when_one_present
    flags = [
      {"id" => "json", "long" => "json", "type" => "boolean",
       "required" => false, "repeatable" => false,
       "conflicts_with" => [], "requires" => [], "required_unless" => []},
      {"id" => "csv", "long" => "csv", "type" => "boolean",
       "required" => false, "repeatable" => false,
       "conflicts_with" => [], "requires" => [], "required_unless" => []}
    ]
    groups = [{"id" => "format", "flag_ids" => ["json", "csv"], "required" => false}]
    errors = validate(flags, {"json" => true}, groups: groups)
    refute_error(errors, "exclusive_group_violation")
  end

  def test_exclusive_group_no_violation_when_none_present
    flags = [
      {"id" => "json", "long" => "json", "type" => "boolean",
       "required" => false, "repeatable" => false,
       "conflicts_with" => [], "requires" => [], "required_unless" => []},
      {"id" => "csv", "long" => "csv", "type" => "boolean",
       "required" => false, "repeatable" => false,
       "conflicts_with" => [], "requires" => [], "required_unless" => []}
    ]
    groups = [{"id" => "format", "flag_ids" => ["json", "csv"], "required" => false}]
    errors = validate(flags, {}, groups: groups)
    assert_equal [], errors
  end

  def test_required_exclusive_group_missing_raises_error
    flags = [
      {"id" => "json", "long" => "json", "type" => "boolean",
       "required" => false, "repeatable" => false,
       "conflicts_with" => [], "requires" => [], "required_unless" => []},
      {"id" => "csv", "long" => "csv", "type" => "boolean",
       "required" => false, "repeatable" => false,
       "conflicts_with" => [], "requires" => [], "required_unless" => []}
    ]
    groups = [{"id" => "format", "flag_ids" => ["json", "csv"], "required" => true}]
    errors = validate(flags, {}, groups: groups)
    assert_error(errors, "missing_exclusive_group")
    assert_match(/One of/, errors.first.message)
  end

  def test_required_exclusive_group_satisfied_no_error
    flags = [
      {"id" => "json", "long" => "json", "type" => "boolean",
       "required" => false, "repeatable" => false,
       "conflicts_with" => [], "requires" => [], "required_unless" => []},
      {"id" => "csv", "long" => "csv", "type" => "boolean",
       "required" => false, "repeatable" => false,
       "conflicts_with" => [], "requires" => [], "required_unless" => []}
    ]
    groups = [{"id" => "format", "flag_ids" => ["json", "csv"], "required" => true}]
    errors = validate(flags, {"json" => true}, groups: groups)
    refute_error(errors, "missing_exclusive_group")
  end

  # ---------------------------------------------------------------------------
  # flag_display_name edge cases
  # ---------------------------------------------------------------------------

  def test_flag_with_only_short_displays_short
    flags = [
      {"id" => "verbose", "short" => "v", "type" => "boolean",
       "required" => true, "repeatable" => false,
       "conflicts_with" => [], "requires" => [], "required_unless" => []}
    ]
    errors = validate(flags, {})
    assert_match(/-v/, errors.first.message)
  end

  def test_flag_with_only_long_displays_long
    flags = [
      {"id" => "verbose", "long" => "verbose", "type" => "boolean",
       "required" => true, "repeatable" => false,
       "conflicts_with" => [], "requires" => [], "required_unless" => []}
    ]
    errors = validate(flags, {})
    assert_match(/--verbose/, errors.first.message)
  end

  def test_flag_with_only_single_dash_long_displays_sdl
    flags = [
      {"id" => "classpath", "single_dash_long" => "classpath", "type" => "string",
       "required" => true, "repeatable" => false,
       "conflicts_with" => [], "requires" => [], "required_unless" => []}
    ]
    errors = validate(flags, {})
    assert_match(/-classpath/, errors.first.message)
  end

  def test_flag_display_name_nil_returns_unknown
    # When @flag_by_id[id] is nil, flag_display_name returns "???"
    # This happens in validate_conflicts when other_id isn't in @flag_by_id
    flags = [
      {"id" => "foo", "short" => "f", "type" => "boolean",
       "required" => false, "repeatable" => false,
       "conflicts_with" => ["ghost"], "requires" => [], "required_unless" => []}
    ]
    errors = validate(flags, {"foo" => true, "ghost" => true})
    # "ghost" has no flag def → display name "???"
    conflict_errors = errors.select { |e| e.error_type == "conflicting_flags" }
    assert_equal 1, conflict_errors.size
    assert_match(/\?\?\?/, conflict_errors.first.message)
  end

  # ---------------------------------------------------------------------------
  # Multiple errors collected (not fail-fast)
  # ---------------------------------------------------------------------------

  def test_collects_multiple_errors
    flags = [
      {"id" => "output", "long" => "output", "type" => "string",
       "required" => true, "repeatable" => false,
       "conflicts_with" => [], "requires" => [], "required_unless" => []},
      {"id" => "verbose", "long" => "verbose", "type" => "boolean",
       "required" => true, "repeatable" => false,
       "conflicts_with" => [], "requires" => [], "required_unless" => []}
    ]
    errors = validate(flags, {})
    assert_equal 2, errors.select { |e| e.error_type == "missing_required_flag" }.size
  end

  def test_empty_parsed_flags_with_all_optional_no_errors
    flags = [
      {"id" => "verbose", "short" => "v", "type" => "boolean",
       "required" => false, "repeatable" => false,
       "conflicts_with" => [], "requires" => [], "required_unless" => []}
    ]
    errors = validate(flags, {})
    assert_equal [], errors
  end
end
