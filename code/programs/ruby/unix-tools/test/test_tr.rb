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
    result = parse_tr_argv(["-d", "abc"])
    assert result.flags["delete"]
  end

  def test_squeeze_flag
    result = parse_tr_argv(["-s", "a-z", "A-Z"])
    assert result.flags["squeeze_repeats"]
  end

  def test_complement_flag
    result = parse_tr_argv(["-c", "a-z", "X"])
    assert result.flags["complement"]
  end
end

class TestTrExpandEscapes < Minitest::Test
  def test_newline
    assert_equal "\n", tr_expand_escapes("\\n")
  end

  def test_tab
    assert_equal "\t", tr_expand_escapes("\\t")
  end

  def test_carriage_return
    assert_equal "\r", tr_expand_escapes("\\r")
  end

  def test_backslash
    assert_equal "\\", tr_expand_escapes("\\\\")
  end

  def test_no_escapes
    assert_equal "abc", tr_expand_escapes("abc")
  end

  def test_mixed
    assert_equal "a\nb", tr_expand_escapes("a\\nb")
  end

  def test_bell
    assert_equal "\a", tr_expand_escapes("\\a")
  end

  def test_backspace
    assert_equal "\b", tr_expand_escapes("\\b")
  end

  def test_form_feed
    assert_equal "\f", tr_expand_escapes("\\f")
  end

  def test_vertical_tab
    assert_equal "\v", tr_expand_escapes("\\v")
  end
end

class TestTrExpandSet < Minitest::Test
  def test_simple_chars
    assert_equal "abc", tr_expand_set("abc")
  end

  def test_range
    assert_equal "abcdef", tr_expand_set("a-f")
  end

  def test_upper_class
    result = tr_expand_set("[:upper:]")
    assert_equal ("A".."Z").to_a.join, result
  end

  def test_lower_class
    result = tr_expand_set("[:lower:]")
    assert_equal ("a".."z").to_a.join, result
  end

  def test_digit_class
    result = tr_expand_set("[:digit:]")
    assert_equal ("0".."9").to_a.join, result
  end

  def test_alpha_class
    result = tr_expand_set("[:alpha:]")
    assert_includes result, "A"
    assert_includes result, "z"
  end

  def test_alnum_class
    result = tr_expand_set("[:alnum:]")
    assert_includes result, "A"
    assert_includes result, "0"
  end

  def test_space_class
    result = tr_expand_set("[:space:]")
    assert_includes result, " "
    assert_includes result, "\t"
    assert_includes result, "\n"
  end

  def test_blank_class
    result = tr_expand_set("[:blank:]")
    assert_equal " \t", result
  end

  def test_xdigit_class
    result = tr_expand_set("[:xdigit:]")
    assert_includes result, "a"
    assert_includes result, "F"
    assert_includes result, "0"
  end

  def test_punct_class
    result = tr_expand_set("[:punct:]")
    assert_includes result, "!"
    assert_includes result, "."
  end

  def test_escape_in_set
    result = tr_expand_set("\\n")
    assert_equal "\n", result
  end
end

class TestTrTranslate < Minitest::Test
  def test_basic_translate
    assert_equal "HELLO", tr_translate("hello", "abcdefghijklmnopqrstuvwxyz",
                                       "ABCDEFGHIJKLMNOPQRSTUVWXYZ", squeeze: false)
  end

  def test_translate_partial
    assert_equal "HEllo", tr_translate("hello", "he", "HE", squeeze: false)
  end

  def test_translate_with_squeeze
    result = tr_translate("aabbcc", "abc", "xyz", squeeze: true)
    assert_equal "xyz", result
  end

  def test_translate_empty_set2_uses_set1
    result = tr_translate("abc", "abc", "", squeeze: false)
    assert_equal "abc", result
  end

  def test_set2_shorter_pads_with_last_char
    result = tr_translate("abc", "abc", "x", squeeze: false)
    assert_equal "xxx", result
  end

  def test_passthrough_unmatched
    # Lowercase letters map to uppercase, spaces and digits pass through unchanged
    assert_equal "HELLO 123", tr_translate("hello 123", "abcdefghijklmnopqrstuvwxyz",
                                           "ABCDEFGHIJKLMNOPQRSTUVWXYZ", squeeze: false)
  end
end

class TestTrDelete < Minitest::Test
  def test_basic_delete
    assert_equal "hll", tr_delete("hello", "eo", squeeze: false, squeeze_set_chars: "")
  end

  def test_delete_all
    assert_equal "", tr_delete("aaa", "a", squeeze: false, squeeze_set_chars: "")
  end

  def test_delete_none
    assert_equal "hello", tr_delete("hello", "xyz", squeeze: false, squeeze_set_chars: "")
  end

  def test_delete_with_squeeze
    result = tr_delete("aaabbbccc", "a", squeeze: true, squeeze_set_chars: "bc")
    assert_equal "bc", result
  end
end

class TestTrSqueezeOnly < Minitest::Test
  def test_squeeze_repeated
    assert_equal "abc", tr_squeeze_only("aabbcc", "abc")
  end

  def test_squeeze_selective
    assert_equal "abbc", tr_squeeze_only("aabbcc", "ac")
  end

  def test_no_repeats
    assert_equal "abc", tr_squeeze_only("abc", "abc")
  end

  def test_empty_input
    assert_equal "", tr_squeeze_only("", "abc")
  end
end

class TestTrComplement < Minitest::Test
  def test_complement
    result = tr_complement("abc")
    refute_includes result, "a"
    refute_includes result, "b"
    refute_includes result, "c"
    assert_includes result, "d"
    assert_includes result, "z"
  end
end

class TestTrMainIntegration < Minitest::Test
  def test_main_translate
    old_argv = ARGV.dup
    old_stdin = $stdin
    $stdin = StringIO.new("hello")
    ARGV.replace(["a-z", "A-Z"])
    out, _err = capture_io { tr_main }
    assert_equal "HELLO", out
  ensure
    ARGV.replace(old_argv)
    $stdin = old_stdin
  end

  def test_main_delete
    old_argv = ARGV.dup
    old_stdin = $stdin
    $stdin = StringIO.new("hello world")
    ARGV.replace(["-d", "lo"])
    out, _err = capture_io { tr_main }
    assert_equal "he wrd", out
  ensure
    ARGV.replace(old_argv)
    $stdin = old_stdin
  end

  def test_main_squeeze
    old_argv = ARGV.dup
    old_stdin = $stdin
    $stdin = StringIO.new("aabbcc")
    ARGV.replace(["-s", "abc"])
    out, _err = capture_io { tr_main }
    assert_equal "abc", out
  ensure
    ARGV.replace(old_argv)
    $stdin = old_stdin
  end

  def test_main_delete_and_squeeze
    old_argv = ARGV.dup
    old_stdin = $stdin
    $stdin = StringIO.new("aaabbbccc")
    ARGV.replace(["-d", "-s", "a", "bc"])
    out, _err = capture_io { tr_main }
    assert_equal "bc", out
  ensure
    ARGV.replace(old_argv)
    $stdin = old_stdin
  end

  def test_main_complement
    old_argv = ARGV.dup
    old_stdin = $stdin
    $stdin = StringIO.new("hello123")
    ARGV.replace(["-d", "-c", "a-z"])
    out, _err = capture_io { tr_main }
    assert_equal "hello", out
  ensure
    ARGV.replace(old_argv)
    $stdin = old_stdin
  end

  def test_main_translate_with_squeeze
    old_argv = ARGV.dup
    old_stdin = $stdin
    $stdin = StringIO.new("aabbcc")
    ARGV.replace(["-s", "abc", "xyz"])
    out, _err = capture_io { tr_main }
    assert_equal "xyz", out
  ensure
    ARGV.replace(old_argv)
    $stdin = old_stdin
  end

  def test_main_missing_set2
    old_argv = ARGV.dup
    old_stdin = $stdin
    $stdin = StringIO.new("hello")
    ARGV.replace(["abc"])
    _out, err = capture_io do
      e = assert_raises(SystemExit) { tr_main }
      assert_equal 1, e.status
    end
    assert_includes err, "missing operand"
  ensure
    ARGV.replace(old_argv)
    $stdin = old_stdin
  end

  def test_main_help
    old_argv = ARGV.dup
    ARGV.replace(["--help"])
    out, _err = capture_io do
      e = assert_raises(SystemExit) { tr_main }
      assert_equal 0, e.status
    end
    assert_includes out, "tr"
  ensure
    ARGV.replace(old_argv)
  end

  def test_main_version
    old_argv = ARGV.dup
    ARGV.replace(["--version"])
    out, _err = capture_io do
      e = assert_raises(SystemExit) { tr_main }
      assert_equal 0, e.status
    end
    assert_includes out, "1.0.0"
  ensure
    ARGV.replace(old_argv)
  end
end
