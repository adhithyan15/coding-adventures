# frozen_string_literal: true

require_relative "test_helper"
require "securerandom"

# ---------------------------------------------------------------------------
# Tests for the standalone validate_spec and validate_spec_string methods
# ---------------------------------------------------------------------------
#
# The validator wraps SpecLoader's validation in a non-raising interface.
# Instead of raising SpecError, it returns a ValidationResult with:
#
#   valid?  — boolean
#   errors  — array of error message strings (empty when valid)
#
# These tests mirror the SpecLoader error cases but verify the result-based
# API rather than exception-based behavior. They also test the
# validate_spec_string convenience method for in-memory JSON validation.
# ---------------------------------------------------------------------------
class TestValidator < Minitest::Test
  include CodingAdventures::CliBuilder

  # -------------------------------------------------------------------------
  # Helpers
  # -------------------------------------------------------------------------

  # A minimal valid spec — the smallest JSON that passes all 9 rules.
  #
  # It declares version "1.0", has a name and description, and contains
  # no flags, arguments, or commands (all optional). This is the baseline
  # for "valid spec" tests.
  VALID_SPEC = {
    "cli_builder_spec_version" => "1.0",
    "name" => "my-tool",
    "description" => "A simple tool"
  }.freeze

  # Write a spec hash to a tempfile and validate it via validate_spec.
  def validate_hash(hash)
    json = JSON.generate(hash)
    CodingAdventures::CliBuilder.validate_spec_string(json)
  end

  # -------------------------------------------------------------------------
  # Test: Valid spec returns valid?=true and empty errors
  # -------------------------------------------------------------------------
  #
  # The simplest positive case: a well-formed spec should produce a
  # ValidationResult where valid? is true and errors is an empty array.

  def test_valid_spec_returns_valid
    result = validate_hash(VALID_SPEC)

    assert result.valid?, "Expected valid? to be true for a valid spec"
    assert_empty result.errors, "Expected no errors for a valid spec"
  end

  # -------------------------------------------------------------------------
  # Test: A more complete valid spec (with flags and arguments)
  # -------------------------------------------------------------------------
  #
  # Make sure validation passes for specs with actual content, not just
  # the bare minimum.

  def test_valid_spec_with_flags_and_args
    spec = VALID_SPEC.merge(
      "version" => "1.0.0",
      "flags" => [
        {
          "id" => "verbose",
          "short" => "v",
          "description" => "Enable verbose output",
          "type" => "boolean"
        }
      ],
      "arguments" => [
        {
          "id" => "file",
          "display_name" => "FILE",
          "description" => "Input file",
          "type" => "string"
        }
      ]
    )

    result = validate_hash(spec)
    assert result.valid?
    assert_empty result.errors
  end

  # -------------------------------------------------------------------------
  # Test: Missing version field returns valid?=false
  # -------------------------------------------------------------------------
  #
  # Rule 1 of spec validation: cli_builder_spec_version must be present.
  # Omitting it entirely should produce a clear error.

  def test_missing_version_returns_invalid
    spec = VALID_SPEC.dup
    spec.delete("cli_builder_spec_version")

    result = validate_hash(spec)

    refute result.valid?, "Expected valid? to be false when version is missing"
    assert_equal 1, result.errors.length
    assert_match(/cli_builder_spec_version/, result.errors.first)
  end

  # -------------------------------------------------------------------------
  # Test: Unsupported version returns valid?=false
  # -------------------------------------------------------------------------
  #
  # Rule 1 continued: if the version is present but not "1.0", the spec
  # is for a future (or past) format we cannot handle. This catches typos
  # like "1.1" or "2.0" early.

  def test_unsupported_version_returns_invalid
    spec = VALID_SPEC.merge("cli_builder_spec_version" => "2.0")

    result = validate_hash(spec)

    refute result.valid?, "Expected valid? to be false for unsupported version"
    assert_equal 1, result.errors.length
    assert_match(/Unsupported spec version/, result.errors.first)
  end

  # -------------------------------------------------------------------------
  # Test: Missing required fields returns valid?=false
  # -------------------------------------------------------------------------
  #
  # Rule 2: name and description are required top-level fields. Testing
  # with name missing (description would behave identically).

  def test_missing_name_returns_invalid
    spec = VALID_SPEC.dup
    spec.delete("name")

    result = validate_hash(spec)

    refute result.valid?, "Expected valid? to be false when name is missing"
    assert_equal 1, result.errors.length
    assert_match(/name/, result.errors.first)
  end

  def test_missing_description_returns_invalid
    spec = VALID_SPEC.dup
    spec.delete("description")

    result = validate_hash(spec)

    refute result.valid?, "Expected valid? to be false when description is missing"
    assert_equal 1, result.errors.length
    assert_match(/description/, result.errors.first)
  end

  # -------------------------------------------------------------------------
  # Test: Invalid JSON returns valid?=false
  # -------------------------------------------------------------------------
  #
  # Before any semantic validation, the JSON must actually parse. Broken
  # JSON (missing braces, trailing commas, etc.) should produce a clear
  # error mentioning "Invalid JSON" rather than a raw parser exception.

  def test_invalid_json_returns_invalid
    result = CodingAdventures::CliBuilder.validate_spec_string("not valid json {{{")

    refute result.valid?, "Expected valid? to be false for invalid JSON"
    assert_equal 1, result.errors.length
    assert_match(/Invalid JSON/, result.errors.first)
  end

  # -------------------------------------------------------------------------
  # Test: Nonexistent file returns valid?=false
  # -------------------------------------------------------------------------
  #
  # If the spec file path does not exist on disk, we should get a clean
  # error rather than an unhandled Errno::ENOENT exception.

  def test_nonexistent_file_returns_invalid
    result = CodingAdventures::CliBuilder.validate_spec("/tmp/this_file_does_not_exist_#{SecureRandom.hex(8)}.json")

    refute result.valid?, "Expected valid? to be false for nonexistent file"
    assert_equal 1, result.errors.length
    assert_match(/not found/, result.errors.first)
  end

  # -------------------------------------------------------------------------
  # Test: Flag with no short/long returns valid?=false
  # -------------------------------------------------------------------------
  #
  # Rule 4: every flag must have at least one of short, long, or
  # single_dash_long. A flag with none of these is unreachable — the user
  # could never type it. This is always a spec authoring mistake.

  def test_flag_with_no_name_returns_invalid
    spec = VALID_SPEC.merge(
      "flags" => [
        {
          "id" => "ghost-flag",
          "description" => "This flag has no short or long name",
          "type" => "boolean"
        }
      ]
    )

    result = validate_hash(spec)

    refute result.valid?, "Expected valid? to be false when a flag has no short/long"
    assert_equal 1, result.errors.length
    assert_match(/ghost-flag/, result.errors.first)
    assert_match(/no short, long, or single_dash_long/, result.errors.first)
  end

  # -------------------------------------------------------------------------
  # Test: validate_spec with a real file (file-based API)
  # -------------------------------------------------------------------------
  #
  # The validate_spec_string method is tested above. Here we verify the
  # file-based validate_spec works with an actual tempfile on disk.

  def test_validate_spec_with_valid_file
    with_spec_file(VALID_SPEC) do |path|
      result = CodingAdventures::CliBuilder.validate_spec(path)
      assert result.valid?
      assert_empty result.errors
    end
  end

  def test_validate_spec_with_invalid_file
    invalid_spec = VALID_SPEC.dup
    invalid_spec.delete("cli_builder_spec_version")

    with_spec_file(invalid_spec) do |path|
      result = CodingAdventures::CliBuilder.validate_spec(path)
      refute result.valid?
      assert_match(/cli_builder_spec_version/, result.errors.first)
    end
  end

  # -------------------------------------------------------------------------
  # Test: ValidationResult structure
  # -------------------------------------------------------------------------
  #
  # Verify that ValidationResult behaves as expected: valid? is an alias
  # for valid, and errors is frozen (immutable after creation).

  def test_validation_result_errors_are_frozen
    result = ValidationResult.new(valid: false, errors: ["something broke"])
    assert result.errors.frozen?, "Expected errors array to be frozen"
  end

  def test_validation_result_valid_predicate_matches_valid
    valid_result = ValidationResult.new(valid: true)
    invalid_result = ValidationResult.new(valid: false, errors: ["oops"])

    assert_equal valid_result.valid, valid_result.valid?
    assert_equal invalid_result.valid, invalid_result.valid?
  end
end
