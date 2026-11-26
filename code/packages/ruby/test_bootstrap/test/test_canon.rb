# frozen_string_literal: true

require_relative "test_helper"

class TestCanon < HookedTest
  C = CorpusJson::Canon

  def test_scalars
    assert_equal "nil", C.render(nil)
    assert_equal "true", C.render(true)
    assert_equal "false", C.render(false)
    assert_equal "-12", C.render(-12)
    assert_equal "1.5", C.render(1.5)
    assert_equal ":sym", C.render(:sym)
  end

  def test_strings_bytes_and_invalid_text
    assert_equal '"a\\"b\\\\c"', C.render('a"b\\c')
    assert_equal "0x00ff", C.render("\x00\xff".b)
    assert_equal "0x", C.render("".b)
    assert_equal "0xff", C.render("\xff".dup.force_encoding("UTF-8")) # invalid UTF-8 text
  end

  def test_control_characters_are_escaped
    assert_equal '"a\\nb\\rc\\td"', C.render("a\nb\rc\td")
    assert_equal '"\\u0000\\u001b\\u007f\\u0085\\u2028\\u2029"', C.render("\u0000\e\u007f\u0085  ")
    refute_includes C.render(Object.new.tap { |o| def o.inspect = "x\ny" }), "\n"
  end

  def test_collections
    assert_equal "[1,[2,3],\"a\"]", C.render([1, [2, 3], "a"])
    assert_equal "[]", C.render([])
    assert_equal "{1:2,\"k\":nil}", C.render({1 => 2, "k" => nil})
    assert_equal "{}", C.render({})
  end

  def test_depth_cap_renders_an_ellipsis
    deep = 1
    20.times { deep = [deep] }
    out = C.render(deep)
    assert_includes out, "..."
    assert_equal C::MAX_DEPTH, out.count("[")
  end

  def test_a_huge_integer_is_not_expanded
    assert_equal "<bigint #{(2**200_000).bit_length} bits>", C.render(2**200_000)
    assert_equal (2**99_999).to_s, C.render(2**99_999)
  end

  def test_node_cap_truncates
    out = C.render(Array.new(50_000, 1))
    assert out.end_with?("...")
    assert_operator out.size, :<, 25_000
  end

  def test_output_cap_truncates
    out = C.render(Array.new(5) { "x" * 400_000 })
    assert_operator out.bytesize, :<=, C::MAX_OUTPUT + 3
    assert out.end_with?("...")
  end

  def test_user_inspect_errors_make_the_whole_witness_unrenderable
    bomb = Object.new
    def bomb.inspect = raise("boom")
    assert_equal "<unrenderable>", C.render([1, bomb])
    assert_equal "<unrenderable>", C.render({a: bomb})
    deep = Object.new
    def deep.inspect = inspect
    assert_equal "<unrenderable>", C.render(deep)
    hostile = Class.new(Array) { def each = raise("each boom") }.new([1])
    assert_equal "<unrenderable>", C.render(hostile)
  end

  def test_render_does_not_swallow_process_level_exceptions
    bomb = Object.new
    def bomb.inspect = raise(Interrupt)
    assert_raises(Interrupt) { C.render(bomb) }
  end

  def test_clip
    assert_equal "abc", CorpusJson::Clip.clip("abc", 5)
    assert_equal "ab...", CorpusJson::Clip.clip("abcdef", 2)
    assert_equal "7", CorpusJson::Clip.clip(7, 5)
  end
end
