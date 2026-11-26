# frozen_string_literal: true

require_relative "test_helper"

class TestStrictJson < HookedTest
  J = CorpusJson::StrictJson

  def parse(text) = J.parse(text)

  def refuses(text, message = nil)
    err = assert_raises(J::JsonError, text.to_s[0, 40]) { parse(text) }
    assert_match(message, err.message) if message
  end

  def test_values
    assert_equal({"a" => [1, true, false, nil, "x"].map { |v| v.is_a?(Integer) ? J::Number.new(v.to_s) : v }}, parse('{"a": [1, true, false, null, "x"]}'))
    assert_equal J::Number.new("-0.5e+10"), parse("-0.5e+10")
    assert_equal J::Number.new("123456789012345678901234567890"), parse("123456789012345678901234567890")
    assert_equal({}, parse(" {} "))
    assert_equal [], parse("\n[\t]\r")
    assert_equal J::Number.new("0"), parse("0")
  end

  def test_string_escapes
    assert_equal "a\"b\\c/d\b\f\n\r\t", parse('"a\\"b\\\\c\\/d\\b\\f\\n\\r\\t"')
    assert_equal "é", parse('"\\u00e9"')
    assert_equal "é", parse('"é"')
    assert_equal "\u{1F600}", parse('"\\ud83d\\ude00"')
    assert_equal Encoding::UTF_8, parse('"x"').encoding
  end

  def test_refuses_duplicate_keys
    refuses('{"a":1,"a":2}', /duplicate key/)
    refuses('{"a":{"b":1,"b":2}}', /duplicate key/)
  end

  def test_refuses_trailing_data_and_empty_input
    refuses("1 2", /trailing/)
    refuses("{}x", /trailing/)
    refuses("", /unexpected end/)
    refuses("   ", /unexpected end/)
  end

  def test_refuses_malformed_numbers
    %w[01 -01 +1 1. .5 1e 1e+ - NaN Infinity 0x10].each { |t| refuses(t) }
    refuses("-", /invalid number/)
    refuses("00", /leading zero/)
  end

  def test_refuses_malformed_literals_and_structure
    refuses("tru")
    refuses("nul")
    refuses("[1,]")
    refuses("[1 2]")
    refuses("{,}")
    refuses('{"a" 1}')
    refuses('{"a":1,}')
    refuses("{1:2}")
    refuses("[")
    refuses('{"a":')
    refuses("// c\n1")
    refuses("'x'")
  end

  def test_refuses_malformed_strings
    refuses("\"a\nb\"", /control/)
    refuses("\"a\tb\"", /control/)
    refuses('"abc')
    refuses('"\\x"', /invalid escape/)
    refuses('"\\')
    refuses('"\\u12"', /\\u/)
    refuses('"\\u12G4"', /\\u/)
    refuses('"\\ud800"', /surrogate/)
    refuses('"\\ud800x"', /surrogate/)
    refuses('"\\ud800\\u0041"', /surrogate/)
    refuses('"\\udc00"', /surrogate/)
  end

  def test_refuses_invalid_utf8_and_a_bom
    refuses("\"\xff\"".b, /UTF-8/)
    refuses("\"\xc0\xaf\"".b, /UTF-8/) # overlong
    refuses("\"\xed\xa0\x80\"".b, /UTF-8/) # a surrogate encoded directly
    refuses("﻿{}".b)
    refuses(5, /must be a String/)
  end

  def test_depth_is_capped_at_64_containers
    assert_equal 1, nest(64)
    assert_raises(J::JsonError) { nest(65) }
    err = assert_raises(J::JsonError) { parse(("[" * 1_000_000) + ("]" * 1_000_000)) }
    assert_match(/nesting deeper than 64/, err.message)
    assert_raises(J::JsonError) { parse('{"a":' * 65) }
  end

  def nest(depth)
    parse(("[" * depth) + ("]" * depth))
    1
  end

  def test_size_caps
    refuses("[" + "0," * 200_000 + "0]", /more than 200000 values/)
    parse("[" + "0," * 199_990 + "0]") # under the node cap
    refuses("[\"" + ("a" * J::MAX_BYTES) + "\"]", /exceeds/)
  end
end
