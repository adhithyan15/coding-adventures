# frozen_string_literal: true

require "minitest/autorun"
require "coding_adventures_immutable_list"

IL = CodingAdventures::ImmutableList
PV = CodingAdventures::ImmutableList::PersistentVector

class TestImmutableListVersion < Minitest::Test
  def test_version_exists
    refute_nil IL::VERSION
  end
end

class TestImmutableListEmpty < Minitest::Test
  def test_empty_has_size_zero
    assert_equal 0, IL.empty.size
  end

  def test_empty_is_empty
    assert IL.empty.empty?
  end

  def test_non_empty_is_not_empty
    refute IL.empty.push("x").empty?
  end

  def test_empty_to_a
    assert_equal [], IL.empty.to_a
  end

  def test_empty_get_returns_nil
    assert_nil IL.empty.get(0)
    assert_nil IL.empty.get(100)
  end
end

class TestImmutableListPush < Minitest::Test
  def test_push_increases_size
    list = IL.empty.push("a")
    assert_equal 1, list.size
  end

  def test_push_preserves_original
    original = IL.empty
    _new_list = original.push("a")
    assert_equal 0, original.size
    assert original.empty?
  end

  def test_push_multiple_elements
    list = IL.empty.push("a").push("b").push("c")
    assert_equal 3, list.size
    assert_equal "a", list.get(0)
    assert_equal "b", list.get(1)
    assert_equal "c", list.get(2)
  end

  def test_push_chain_many
    list = IL.empty
    100.times { |i| list = list.push("item-#{i}") }
    assert_equal 100, list.size
    assert_equal "item-0", list.get(0)
    assert_equal "item-99", list.get(99)
  end

  def test_push_past_32_tail_threshold
    # The tail buffer holds 32 elements before trie promotion
    list = IL.empty
    35.times { |i| list = list.push(i) }
    assert_equal 35, list.size
    assert_equal 0, list.get(0)
    assert_equal 31, list.get(31)
    assert_equal 32, list.get(32)
    assert_equal 34, list.get(34)
  end

  def test_push_past_1024_second_level
    list = IL.empty
    1025.times { |i| list = list.push(i) }
    assert_equal 1025, list.size
    assert_equal 0, list.get(0)
    assert_equal 1024, list.get(1024)
  end
end

class TestImmutableListGet < Minitest::Test
  def test_get_existing_element
    list = IL.of("x", "y", "z")
    assert_equal "x", list.get(0)
    assert_equal "y", list.get(1)
    assert_equal "z", list.get(2)
  end

  def test_get_out_of_bounds_returns_nil
    list = IL.of("a")
    assert_nil list.get(1)
    assert_nil list.get(100)
  end

  def test_get_negative_returns_nil
    list = IL.of("a")
    assert_nil list.get(-1)
  end

  def test_subscript_alias
    list = IL.of("a", "b")
    assert_equal "a", list[0]
    assert_equal "b", list[1]
  end
end

class TestImmutableListSet < Minitest::Test
  def test_set_returns_new_list
    original = IL.of("a", "b", "c")
    updated = original.set(1, "B")
    assert_equal "B", updated.get(1)
    assert_equal "b", original.get(1)  # original unchanged
  end

  def test_set_preserves_other_elements
    list = IL.of("a", "b", "c", "d")
    updated = list.set(2, "C")
    assert_equal ["a", "b", "C", "d"], updated.to_a
    assert_equal ["a", "b", "c", "d"], list.to_a
  end

  def test_set_first_element
    list = IL.of("x", "y")
    updated = list.set(0, "X")
    assert_equal "X", updated.get(0)
    assert_equal "y", updated.get(1)
  end

  def test_set_last_element
    list = IL.of("x", "y")
    updated = list.set(1, "Y")
    assert_equal "x", updated.get(0)
    assert_equal "Y", updated.get(1)
  end

  def test_set_out_of_bounds_raises
    list = IL.of("a")
    assert_raises(IndexError) { list.set(5, "z") }
  end

  def test_set_on_empty_raises
    assert_raises(IndexError) { IL.empty.set(0, "z") }
  end

  def test_set_in_trie_portion
    # Build a list large enough to have trie nodes
    list = IL.empty
    64.times { |i| list = list.push(i) }
    updated = list.set(10, 999)
    assert_equal 999, updated.get(10)
    assert_equal 10, list.get(10)  # original unchanged
  end
end

class TestImmutableListPop < Minitest::Test
  def test_pop_returns_pair
    list = IL.of("a", "b", "c")
    shorter, val = list.pop
    assert_equal "c", val
    assert_equal 2, shorter.size
    assert_equal ["a", "b"], shorter.to_a
  end

  def test_pop_preserves_original
    list = IL.of("a", "b")
    _shorter, _val = list.pop
    assert_equal 2, list.size
    assert_equal "b", list.get(1)
  end

  def test_pop_to_empty
    list = IL.of("only")
    empty, val = list.pop
    assert_equal "only", val
    assert_equal 0, empty.size
    assert empty.empty?
  end

  def test_pop_chain
    list = IL.of("a", "b", "c", "d")
    l1, v1 = list.pop
    l2, v2 = l1.pop
    l3, v3 = l2.pop
    l4, v4 = l3.pop
    assert_equal "d", v1
    assert_equal "c", v2
    assert_equal "b", v3
    assert_equal "a", v4
    assert l4.empty?
  end

  def test_pop_from_empty_raises
    assert_raises(IndexError) { IL.empty.pop }
  end

  def test_pop_and_push_are_inverses
    list = IL.of("a", "b")
    pushed = list.push("c")
    popped, val = pushed.pop
    assert_equal "c", val
    assert_equal list.to_a, popped.to_a
  end
end

class TestImmutableListFromArray < Minitest::Test
  def test_from_array_roundtrip
    arr = ["alpha", "beta", "gamma", "delta"]
    list = IL.from_array(arr)
    assert_equal arr, list.to_a
  end

  def test_from_empty_array
    list = IL.from_array([])
    assert list.empty?
    assert_equal 0, list.size
  end

  def test_of_variadic
    list = IL.of("x", "y", "z")
    assert_equal 3, list.size
    assert_equal "x", list.get(0)
    assert_equal "z", list.get(2)
  end
end

class TestImmutableListPersistence < Minitest::Test
  def test_multiple_versions_coexist
    v0 = IL.empty
    v1 = v0.push("a")
    v2 = v1.push("b")
    v3 = v2.push("c")
    v4 = v2.set(0, "A")  # branch from v2

    assert_equal [], v0.to_a
    assert_equal ["a"], v1.to_a
    assert_equal ["a", "b"], v2.to_a
    assert_equal ["a", "b", "c"], v3.to_a
    assert_equal ["A", "b"], v4.to_a
  end

  def test_set_does_not_affect_original
    original = IL.of("a", "b", "c")
    modified = original.set(1, "B")
    assert_equal ["a", "b", "c"], original.to_a
    assert_equal ["a", "B", "c"], modified.to_a
  end
end

class TestImmutableListEquality < Minitest::Test
  def test_equal_lists
    a = IL.of("x", "y")
    b = IL.of("x", "y")
    assert_equal a, b
  end

  def test_unequal_lists
    a = IL.of("x", "y")
    b = IL.of("x", "z")
    refute_equal a, b
  end

  def test_different_sizes
    a = IL.of("x")
    b = IL.of("x", "y")
    refute_equal a, b
  end
end

class TestImmutableListLarge < Minitest::Test
  def test_large_push_and_get
    list = IL.empty
    1000.times { |i| list = list.push("item-#{i}") }
    assert_equal 1000, list.size
    assert_equal "item-0", list.get(0)
    assert_equal "item-500", list.get(500)
    assert_equal "item-999", list.get(999)
  end

  def test_large_to_a
    arr = (0...200).map { |i| "v#{i}" }
    list = IL.from_array(arr)
    assert_equal arr, list.to_a
  end
end
