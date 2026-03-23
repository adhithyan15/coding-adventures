# frozen_string_literal: true

require_relative "test_helper"

# Tests for TokenClassifier — the component that classifies individual argv
# tokens into typed events for the parsing state machine.
#
# Test matrix:
#   "--"             → end_of_flags
#   "--verbose"      → long_flag (boolean)
#   "--output=f.txt" → long_flag_with_value
#   "-classpath"     → single_dash_long (longest match wins over short 'c')
#   "-c"             → short_flag (when no SDL 'c' exists)
#   "-l"             → short_flag (boolean)
#   "-ffile.txt"     → short_flag_with_value (f is non-boolean)
#   "-lah"           → stacked_flags [l, a, h]
#   "-"              → positional (bare dash = stdin)
#   "file.txt"       → positional
#   "--unknown"      → unknown_flag
class TestTokenClassifier < Minitest::Test
  include CodingAdventures::CliBuilder

  # ---------------------------------------------------------------------------
  # Fixtures — flag sets used in tests
  # ---------------------------------------------------------------------------

  # Basic boolean flags: -l, -a, -h
  LS_FLAGS = [
    {"id" => "long", "short" => "l", "type" => "boolean", "description" => "long listing"},
    {"id" => "all", "short" => "a", "type" => "boolean", "description" => "show all"},
    {"id" => "human", "short" => "h", "long" => "human-readable", "type" => "boolean", "description" => "human-readable sizes"},
    {"id" => "verbose", "long" => "verbose", "type" => "boolean", "description" => "verbose"},
    {"id" => "output", "long" => "output", "type" => "string", "description" => "output file", "value_name" => "FILE"}
  ].freeze

  # Java-style flags: -classpath (SDL), -cp (SDL), -c (short boolean)
  JAVA_FLAGS = [
    {"id" => "classpath", "single_dash_long" => "classpath", "type" => "string", "description" => "classpath"},
    {"id" => "cp", "single_dash_long" => "cp", "type" => "string", "description" => "classpath alias"},
    {"id" => "jar", "single_dash_long" => "jar", "type" => "boolean", "description" => "jar mode"},
    {"id" => "check", "short" => "c", "type" => "boolean", "description" => "check"}
  ].freeze

  # Non-boolean short flag for value tests
  OUTPUT_FLAGS = [
    {"id" => "output", "short" => "o", "type" => "string", "description" => "output"},
    {"id" => "file", "short" => "f", "type" => "string", "description" => "file"},
    {"id" => "long", "short" => "l", "type" => "boolean", "description" => "long"},
    {"id" => "all", "short" => "a", "type" => "boolean", "description" => "all"},
    {"id" => "human", "short" => "h", "type" => "boolean", "description" => "human"}
  ].freeze

  def classifier(flags = LS_FLAGS)
    TokenClassifier.new(flags)
  end

  # ---------------------------------------------------------------------------
  # End of flags
  # ---------------------------------------------------------------------------

  def test_double_dash_is_end_of_flags
    result = classifier.classify("--")
    assert_equal :end_of_flags, result[:type]
  end

  # ---------------------------------------------------------------------------
  # Long flags
  # ---------------------------------------------------------------------------

  def test_long_boolean_flag
    result = classifier.classify("--verbose")
    assert_equal :long_flag, result[:type]
    assert_equal "verbose", result[:flag]["id"]
  end

  def test_long_flag_with_inline_value
    result = classifier.classify("--output=file.txt")
    assert_equal :long_flag_with_value, result[:type]
    assert_equal "output", result[:flag]["id"]
    assert_equal "file.txt", result[:value]
  end

  def test_long_flag_with_value_containing_equals
    result = classifier.classify("--output=a=b")
    assert_equal :long_flag_with_value, result[:type]
    assert_equal "output", result[:flag]["id"]
    assert_equal "a=b", result[:value]
  end

  def test_unknown_long_flag
    result = classifier.classify("--nonexistent")
    assert_equal :unknown_flag, result[:type]
    assert_equal "--nonexistent", result[:token]
  end

  def test_unknown_long_flag_with_value
    result = classifier.classify("--nonexistent=value")
    assert_equal :unknown_flag, result[:type]
  end

  # ---------------------------------------------------------------------------
  # Single-dash-long flags — longest match wins
  # ---------------------------------------------------------------------------

  def test_single_dash_long_exact_match
    c = TokenClassifier.new(JAVA_FLAGS)
    result = c.classify("-classpath")
    assert_equal :single_dash_long, result[:type]
    assert_equal "classpath", result[:flag]["id"]
  end

  def test_single_dash_long_wins_over_short_stacking
    # "-classpath" should match SDL "classpath", NOT "-c" + "lasspath"
    c = TokenClassifier.new(JAVA_FLAGS)
    result = c.classify("-classpath")
    assert_equal :single_dash_long, result[:type]
    assert_equal "classpath", result[:flag]["id"]
  end

  def test_single_dash_long_shorter_name
    c = TokenClassifier.new(JAVA_FLAGS)
    result = c.classify("-cp")
    assert_equal :single_dash_long, result[:type]
    assert_equal "cp", result[:flag]["id"]
  end

  def test_single_dash_long_boolean
    c = TokenClassifier.new(JAVA_FLAGS)
    result = c.classify("-jar")
    assert_equal :single_dash_long, result[:type]
    assert_equal "jar", result[:flag]["id"]
  end

  # When there is no SDL match, fall back to short flag
  def test_short_flag_when_no_sdl_match
    c = TokenClassifier.new(JAVA_FLAGS)
    result = c.classify("-c")
    assert_equal :short_flag, result[:type]
    assert_equal "check", result[:flag]["id"]
  end

  # ---------------------------------------------------------------------------
  # Short flags — boolean
  # ---------------------------------------------------------------------------

  def test_short_boolean_flag
    result = classifier.classify("-l")
    assert_equal :short_flag, result[:type]
    assert_equal "long", result[:flag]["id"]
  end

  def test_short_boolean_flag_h
    result = classifier.classify("-h")
    assert_equal :short_flag, result[:type]
    assert_equal "human", result[:flag]["id"]
  end

  # ---------------------------------------------------------------------------
  # Short flags — non-boolean with inline value
  # ---------------------------------------------------------------------------

  def test_short_flag_with_inline_value
    c = TokenClassifier.new(OUTPUT_FLAGS)
    result = c.classify("-ffile.txt")
    assert_equal :short_flag_with_value, result[:type]
    assert_equal "file", result[:flag]["id"]
    assert_equal "file.txt", result[:value]
  end

  def test_short_flag_without_inline_value
    c = TokenClassifier.new(OUTPUT_FLAGS)
    result = c.classify("-f")
    assert_equal :short_flag, result[:type]
    assert_equal "file", result[:flag]["id"]
  end

  def test_short_flag_output_with_inline_value
    c = TokenClassifier.new(OUTPUT_FLAGS)
    result = c.classify("-o/dev/null")
    assert_equal :short_flag_with_value, result[:type]
    assert_equal "output", result[:flag]["id"]
    assert_equal "/dev/null", result[:value]
  end

  # ---------------------------------------------------------------------------
  # Stacked flags
  # ---------------------------------------------------------------------------

  def test_stacked_boolean_flags
    result = classifier.classify("-lah")
    assert_equal :stacked_flags, result[:type]
    ids = result[:flags].map { |f| f["id"] }
    assert_equal ["long", "all", "human"], ids
    assert_nil result[:last_value]
  end

  def test_stacked_two_boolean_flags
    result = classifier.classify("-la")
    assert_equal :stacked_flags, result[:type]
    assert_equal 2, result[:flags].length
    assert_nil result[:last_value]
  end

  def test_stacked_boolean_plus_non_boolean_with_value
    # "-lf" with l=boolean, f=string — last flag has inline value "file.txt" in "-lffile.txt"
    c = TokenClassifier.new(OUTPUT_FLAGS)
    result = c.classify("-lffile.txt")
    assert_equal :stacked_flags, result[:type]
    ids = result[:flags].map { |f| f["id"] }
    assert_equal ["long", "file"], ids
    assert_equal "file.txt", result[:last_value]
  end

  def test_stacked_boolean_plus_non_boolean_no_inline_value
    # "-lf" with l=boolean, f=string — f has no inline value, next token is value
    c = TokenClassifier.new(OUTPUT_FLAGS)
    result = c.classify("-lf")
    assert_equal :stacked_flags, result[:type]
    ids = result[:flags].map { |f| f["id"] }
    assert_equal ["long", "file"], ids
    assert_nil result[:last_value]
  end

  def test_stacked_with_unknown_character_is_unknown_flag
    result = classifier.classify("-lXa")
    # 'X' is not a known short flag → unknown
    assert_equal :unknown_flag, result[:type]
  end

  # ---------------------------------------------------------------------------
  # Positional tokens
  # ---------------------------------------------------------------------------

  def test_bare_dash_is_positional
    result = classifier.classify("-")
    assert_equal :positional, result[:type]
    assert_equal "-", result[:value]
  end

  def test_plain_word_is_positional
    result = classifier.classify("file.txt")
    assert_equal :positional, result[:type]
    assert_equal "file.txt", result[:value]
  end

  def test_path_is_positional
    result = classifier.classify("/usr/bin/env")
    assert_equal :positional, result[:type]
    assert_equal "/usr/bin/env", result[:value]
  end

  def test_number_is_positional
    result = classifier.classify("42")
    assert_equal :positional, result[:type]
    assert_equal "42", result[:value]
  end

  # ---------------------------------------------------------------------------
  # Unknown flags
  # ---------------------------------------------------------------------------

  def test_unknown_short_flag
    result = classifier.classify("-z")
    assert_equal :unknown_flag, result[:type]
    assert_equal "-z", result[:token]
  end

  def test_unknown_multi_char_single_dash
    # "-xyz" with no SDL match and unknown chars
    result = classifier.classify("-xyz")
    assert_equal :unknown_flag, result[:type]
  end

  # ---------------------------------------------------------------------------
  # Edge cases: stacked flags where first char is non-boolean (inline value)
  # ---------------------------------------------------------------------------

  def test_stacked_flags_first_char_non_boolean_rest_is_value
    # "-fout.txt" with f=non-boolean → short_flag_with_value (not stacked)
    c = TokenClassifier.new(OUTPUT_FLAGS)
    result = c.classify("-fout.txt")
    assert_equal :short_flag_with_value, result[:type]
    assert_equal "file", result[:flag]["id"]
    assert_equal "out.txt", result[:value]
  end

  # ---------------------------------------------------------------------------
  # Long flag: name only after "--" (empty string edge case)
  # ---------------------------------------------------------------------------

  def test_long_flag_unknown_with_value
    result = classifier.classify("--nonexistent=foo")
    assert_equal :unknown_flag, result[:type]
    assert_equal "--nonexistent=foo", result[:token]
  end

  # ---------------------------------------------------------------------------
  # Single_dash_long: no match at all → falls to unknown
  # ---------------------------------------------------------------------------

  def test_single_dash_multi_char_no_match_is_unknown
    c = TokenClassifier.new(JAVA_FLAGS)
    # "-junk" doesn't match any SDL and 'j' is not a known short flag
    result = c.classify("-junk")
    assert_equal :unknown_flag, result[:type]
  end

  # ---------------------------------------------------------------------------
  # Stacked flags: all boolean, returns stacked with last_value nil
  # ---------------------------------------------------------------------------

  def test_stacked_all_boolean_returns_nil_last_value
    c = TokenClassifier.new(OUTPUT_FLAGS)
    result = c.classify("-lah")
    assert_equal :stacked_flags, result[:type]
    assert_nil result[:last_value]
    ids = result[:flags].map { |f| f["id"] }
    assert_equal ["long", "all", "human"], ids
  end

  # ---------------------------------------------------------------------------
  # Stacked flags: non-boolean last with empty remainder → last_value nil
  # ---------------------------------------------------------------------------

  def test_stacked_non_boolean_last_empty_remainder_nil_last_value
    # "-lf" → last flag 'f' is non-boolean, remainder after 'f' is empty → last_value nil
    c = TokenClassifier.new(OUTPUT_FLAGS)
    result = c.classify("-lf")
    assert_equal :stacked_flags, result[:type]
    assert_nil result[:last_value]
  end

  # ---------------------------------------------------------------------------
  # Classify with no flags at all — everything is unknown or positional
  # ---------------------------------------------------------------------------

  def test_empty_flag_set_long_flag_is_unknown
    c = TokenClassifier.new([])
    result = c.classify("--verbose")
    assert_equal :unknown_flag, result[:type]
  end

  def test_empty_flag_set_positional_still_works
    c = TokenClassifier.new([])
    result = c.classify("file.txt")
    assert_equal :positional, result[:type]
  end

  # ---------------------------------------------------------------------------
  # Short flag: non-boolean without remainder (bare -f)
  # ---------------------------------------------------------------------------

  def test_non_boolean_short_flag_alone_is_short_flag
    c = TokenClassifier.new(OUTPUT_FLAGS)
    result = c.classify("-o")
    assert_equal :short_flag, result[:type]
    assert_equal "output", result[:flag]["id"]
  end
end
