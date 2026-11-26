# frozen_string_literal: true

require_relative "test_helper"

class TestValues < HookedTest
  V = CorpusJson::Values

  def int(text) = {"kind" => "int", "value" => text}

  def bytes(hex) = {"kind" => "bytes", "hex" => hex}

  def test_decode_scalars
    assert_equal 12, V.decode(int("12"))
    assert_equal(-12, V.decode(int("-12")))
    assert_equal 0, V.decode(int("0"))
    assert_equal 10**30, V.decode(int("1#{"0" * 30}"))
    assert_equal true, V.decode(true)
    assert_equal "text", V.decode("text")
    assert_equal "\x00\xff".b, V.decode(bytes("00ff"))
    assert_equal Encoding::BINARY, V.decode(bytes("")).encoding
    assert_equal [1, [true]], V.decode([int("1"), [true]])
  end

  def test_decode_refuses_non_canonical_values
    ["007", "-0", "+1", "1_0", " 1", "1 ", "", "-", "1.0", "１"].each do |bad|
      assert_raises(V::CaseError, bad.inspect) { V.decode(int(bad)) }
    end
    assert_raises(V::CaseError) { V.decode(int(5)) }
    assert_raises(V::CaseError) { V.decode(int("9" * 1001)) }
    ["FF", "abc", "0g", " 00", "00\n"].each { |bad| assert_raises(V::CaseError, bad.inspect) { V.decode(bytes(bad)) } }
    assert_raises(V::CaseError) { V.decode(bytes(5)) }
    assert_raises(V::CaseError) { V.decode(bytes("00" * 10_001)) }
  end

  def test_decode_refuses_unknown_shapes_and_oversize_or_deep_input
    assert_raises(V::CaseError) { V.decode(nil) }
    assert_raises(V::CaseError) { V.decode(CorpusJson::StrictJson::Number.new("1")) }
    assert_raises(V::CaseError) { V.decode({"kind" => "int", "value" => "1", "extra" => 1}) }
    assert_raises(V::CaseError) { V.decode({"kind" => "bytes", "hex" => "00", "value" => "1"}) }
    assert_raises(V::CaseError) { V.decode({"kind" => "float"}) }
    assert_raises(V::CaseError) { V.decode({}) }
    assert_raises(V::CaseError) { V.decode(Array.new(10_001, true)) }
    deep = true
    70.times { deep = [deep] }
    assert_raises(V::CaseError) { V.decode(deep) }
  end

  def test_same_is_kind_sensitive
    assert V.same?(1, 1)
    refute V.same?(1, 2)
    refute V.same?(true, 1)
    refute V.same?(1, true)
    refute V.same?(false, nil)
    assert V.same?(true, true)
    refute V.same?([1], "\x01".b)
    refute V.same?("a", "a".b) # text vs bytes
    assert V.same?("a".b, "a".b)
    assert V.same?("a", "a")
    refute V.same?("a".b, "b".b)
    assert V.same?([1, [2, "x".b]], [1, [2, "x".b]])
    refute V.same?([1, 2], [1, 2, 3])
    refute V.same?([1, 2], [1, 3])
    refute V.same?([], {})
    assert V.same?(2.5, 2.5)
    refute V.same?(1, 1.0)
  end

  def test_same_is_depth_bounded
    a = b = []
    100.times do
      a = [a]
      b = [b]
    end
    refute V.same?(a, b)
  end

  def test_show_is_bounded
    assert_operator V.show("x" * 10_000).size, :<=, V::MAX_FRAGMENT_LEN + 3
  end
end
