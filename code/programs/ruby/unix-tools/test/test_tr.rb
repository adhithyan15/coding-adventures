# frozen_string_literal: true

# test_tr.rb -- Tests for the Ruby tr tool
# ==========================================

require "simplecov"
SimpleCov.start do
  add_filter "/test/"
  enable_coverage :branch
  minimum_coverage 80
end

require "minitest/autorun"
require "set"
require "coding_adventures_cli_builder"

require_relative "../tr_tool"

module TrTestHelper
  TR_TEST_SPEC = File.join(File.dirname(__FILE__), "..", "tr.json")

  def parse_tr_argv(argv)
    CodingAdventures::CliBuilder::Parser.new(TR_TEST_SPEC, ["tr"] + argv).parse
  end
end

class TestTrCliIntegration < Minitest::Test
  include TrTestHelper

  def test_basic_parse
    result = parse_tr_argv(["a-z", "A-Z"])
    assert_kind_of CodingAdventures::CliBuilder::ParseResult, result
  end

  def test_help
    result = parse_tr_argv(["--help"])
    assert_kind_of CodingAdventures::CliBuilder::HelpResult, result
  end

  def test_version
    result = parse_tr_argv(["--version"])
    assert_kind_of CodingAdventures::CliBuilder::VersionResult, result
    assert_equal "1.0.0", result.version
  end

  def test_delete_flag
    result = parse_tr_argv(["-d", "aeiou"])
    assert result.flags["delete"]
  end

  def test_squeeze_flag
    result = parse_tr_argv(["-s", " "])
    assert result.flags["squeeze_repeats"]
  end
end

class TestTrExpandSet < Minitest::Test
  def test_literal_chars
    assert_equal "abc", tr_expand_set("abc")
  end

  def test_range
    assert_equal "abcde", tr_expand_set("a-e")
  end

  def test_digit_range
    assert_equal "0123456789", tr_expand_set("0-9")
  end

  def test_upper_class
    assert_equal "ABCDEFGHIJKLMNOPQRSTUVWXYZ", tr_expand_set("[:upper:]")
  end
end

class TestTrTranslate < Minitest::Test
  def test_lowercase_to_uppercase
    set1 = ("a".."z").to_a.join
    set2 = ("A".."Z").to_a.join
    assert_equal "HELLO", tr_translate("hello", set1, set2, squeeze: false)
  end

  def test_partial_translation
    assert_equal "xyc", tr_translate("abc", "ab", "xy", squeeze: false)
  end

  def test_set2_shorter
    assert_equal "xxx", tr_translate("abc", "abc", "x", squeeze: false)
  end

  def test_squeeze_after_translate
    assert_equal "xyz", tr_translate("aabbcc", "abc", "xyz", squeeze: true)
  end
end

class TestTrDelete < Minitest::Test
  def test_delete_vowels
    assert_equal "hll wrld", tr_delete("hello world", "aeiou", squeeze: false, squeeze_set_chars: "")
  end

  def test_delete_nothing
    assert_equal "hello", tr_delete("hello", "", squeeze: false, squeeze_set_chars: "")
  end
end

class TestTrSqueezeOnly < Minitest::Test
  def test_squeeze_spaces
    assert_equal "hello world", tr_squeeze_only("hello   world", " ")
  end

  def test_squeeze_letters
    assert_equal "abc", tr_squeeze_only("aabbbcccc", "abc")
  end
end

class TestTrComplement < Minitest::Test
  def test_complement_excludes_given_chars
    comp = tr_complement("abc")
    refute_includes comp, "a"
    refute_includes comp, "b"
    assert_includes comp, "d"
    assert_includes comp, "z"
  end
end
