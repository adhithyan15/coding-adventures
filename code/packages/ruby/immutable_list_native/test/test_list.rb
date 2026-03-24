# frozen_string_literal: true

# --------------------------------------------------------------------------
# test_list.rb -- Comprehensive tests for ImmutableList native extension
# --------------------------------------------------------------------------
#
# These tests verify that the Rust-backed ImmutableList behaves correctly
# from Ruby, including its critical immutability invariant: every mutation
# method returns a NEW list, leaving the original unchanged.
#
# Test categories:
# 1. Construction (new, from_array)
# 2. Push (returns new list, preserves original)
# 3. Get (index access, bounds checking)
# 4. Set (returns new list, preserves original, bounds checking)
# 5. Pop (returns [new_list, value], preserves original, empty check)
# 6. Size and empty?
# 7. to_a (conversion to Ruby Array)
# 8. each (block iteration)
# 9. inspect/to_s (string representation)
# 10. Equality (==)
# 11. Structural sharing (immutability invariants)

require_relative "test_helper"

class TestImmutableList < Minitest::Test
  # Convenience alias for the class under test
  ImmutableList = CodingAdventures::ImmutableListNative::ImmutableList

  # =========================================================================
  # Construction
  # =========================================================================

  def test_new_creates_empty_list
    list = ImmutableList.new
    assert_equal 0, list.size
    assert list.empty?
  end

  def test_from_array_creates_list_with_elements
    list = ImmutableList.from_array(["a", "b", "c"])
    assert_equal 3, list.size
    assert_equal "a", list.get(0)
    assert_equal "b", list.get(1)
    assert_equal "c", list.get(2)
  end

  def test_from_array_with_empty_array
    list = ImmutableList.from_array([])
    assert_equal 0, list.size
    assert list.empty?
  end

  def test_from_array_with_single_element
    list = ImmutableList.from_array(["solo"])
    assert_equal 1, list.size
    assert_equal "solo", list.get(0)
  end

  # =========================================================================
  # Push
  # =========================================================================

  def test_push_returns_new_list_with_element
    list = ImmutableList.new
    result = list.push("hello")
    assert_equal 1, result.size
    assert_equal "hello", result.get(0)
  end

  def test_push_does_not_modify_original
    original = ImmutableList.new
    _pushed = original.push("hello")
    assert_equal 0, original.size
    assert original.empty?
  end

  def test_push_multiple_elements
    list = ImmutableList.new
    list = list.push("a")
    list = list.push("b")
    list = list.push("c")
    assert_equal 3, list.size
    assert_equal "a", list.get(0)
    assert_equal "b", list.get(1)
    assert_equal "c", list.get(2)
  end

  def test_push_preserves_all_versions
    v0 = ImmutableList.new
    v1 = v0.push("first")
    v2 = v1.push("second")
    v3 = v2.push("third")

    # All versions are still valid and unchanged
    assert_equal 0, v0.size
    assert_equal 1, v1.size
    assert_equal 2, v2.size
    assert_equal 3, v3.size

    assert_equal "first", v1.get(0)
    assert_equal "first", v2.get(0)
    assert_equal "second", v2.get(1)
  end

  def test_push_many_elements_triggers_trie_promotion
    # Push more than 32 elements to trigger tail-to-trie promotion
    list = ImmutableList.new
    50.times { |i| list = list.push("item_#{i}") }
    assert_equal 50, list.size
    assert_equal "item_0", list.get(0)
    assert_equal "item_49", list.get(49)
  end

  # =========================================================================
  # Get
  # =========================================================================

  def test_get_returns_element_at_index
    list = ImmutableList.from_array(["x", "y", "z"])
    assert_equal "x", list.get(0)
    assert_equal "y", list.get(1)
    assert_equal "z", list.get(2)
  end

  def test_get_returns_nil_for_out_of_bounds
    list = ImmutableList.from_array(["a", "b"])
    assert_nil list.get(2)
    assert_nil list.get(100)
  end

  def test_get_returns_nil_for_empty_list
    list = ImmutableList.new
    assert_nil list.get(0)
  end

  # =========================================================================
  # Set
  # =========================================================================

  def test_set_returns_new_list_with_replaced_element
    list = ImmutableList.from_array(["a", "b", "c"])
    result = list.set(1, "B")
    assert_equal "B", result.get(1)
  end

  def test_set_does_not_modify_original
    original = ImmutableList.from_array(["a", "b", "c"])
    _modified = original.set(1, "B")
    assert_equal "b", original.get(1)
  end

  def test_set_first_element
    list = ImmutableList.from_array(["a", "b", "c"])
    result = list.set(0, "Z")
    assert_equal "Z", result.get(0)
    assert_equal "b", result.get(1)
    assert_equal "c", result.get(2)
  end

  def test_set_last_element
    list = ImmutableList.from_array(["a", "b", "c"])
    result = list.set(2, "Z")
    assert_equal "a", result.get(0)
    assert_equal "b", result.get(1)
    assert_equal "Z", result.get(2)
  end

  def test_set_raises_on_out_of_bounds
    list = ImmutableList.from_array(["a", "b"])
    assert_raises(ArgumentError) { list.set(5, "x") }
  end

  def test_set_raises_on_empty_list
    list = ImmutableList.new
    assert_raises(ArgumentError) { list.set(0, "x") }
  end

  def test_set_preserves_size
    list = ImmutableList.from_array(["a", "b", "c"])
    result = list.set(1, "B")
    assert_equal 3, result.size
  end

  # =========================================================================
  # Pop
  # =========================================================================

  def test_pop_returns_array_of_new_list_and_value
    list = ImmutableList.from_array(["a", "b", "c"])
    result = list.pop
    assert_instance_of Array, result
    assert_equal 2, result.length
  end

  def test_pop_removes_last_element
    list = ImmutableList.from_array(["a", "b", "c"])
    new_list, value = list.pop
    assert_equal "c", value
    assert_equal 2, new_list.size
    assert_equal "a", new_list.get(0)
    assert_equal "b", new_list.get(1)
  end

  def test_pop_does_not_modify_original
    original = ImmutableList.from_array(["a", "b", "c"])
    _new_list, _value = original.pop
    assert_equal 3, original.size
    assert_equal "c", original.get(2)
  end

  def test_pop_single_element_list
    list = ImmutableList.from_array(["solo"])
    new_list, value = list.pop
    assert_equal "solo", value
    assert_equal 0, new_list.size
    assert new_list.empty?
  end

  def test_pop_raises_on_empty_list
    list = ImmutableList.new
    assert_raises(ArgumentError) { list.pop }
  end

  def test_pop_multiple_times
    list = ImmutableList.from_array(["a", "b", "c"])
    list2, val1 = list.pop
    assert_equal "c", val1
    list3, val2 = list2.pop
    assert_equal "b", val2
    list4, val3 = list3.pop
    assert_equal "a", val3
    assert list4.empty?
  end

  # =========================================================================
  # Size and empty?
  # =========================================================================

  def test_size_of_empty_list
    assert_equal 0, ImmutableList.new.size
  end

  def test_size_grows_with_push
    list = ImmutableList.new
    assert_equal 0, list.size
    list = list.push("a")
    assert_equal 1, list.size
    list = list.push("b")
    assert_equal 2, list.size
  end

  def test_empty_on_empty_list
    assert ImmutableList.new.empty?
  end

  def test_empty_on_non_empty_list
    refute ImmutableList.from_array(["a"]).empty?
  end

  # =========================================================================
  # to_a
  # =========================================================================

  def test_to_a_returns_ruby_array
    list = ImmutableList.from_array(["a", "b", "c"])
    result = list.to_a
    assert_instance_of Array, result
    assert_equal ["a", "b", "c"], result
  end

  def test_to_a_on_empty_list
    list = ImmutableList.new
    assert_equal [], list.to_a
  end

  def test_to_a_preserves_order
    items = (0..9).map { |i| "item_#{i}" }
    list = ImmutableList.from_array(items)
    assert_equal items, list.to_a
  end

  # =========================================================================
  # each
  # =========================================================================

  def test_each_returns_array_of_elements
    list = ImmutableList.from_array(["a", "b", "c"])
    result = list.each
    assert_instance_of Array, result
    assert_equal ["a", "b", "c"], result
  end

  def test_each_with_block_via_array
    # each returns an Array, so Ruby-side block iteration works via Array#each
    list = ImmutableList.from_array(["a", "b", "c"])
    collected = []
    list.each.each { |e| collected << e }
    assert_equal ["a", "b", "c"], collected
  end

  def test_each_on_empty_list
    result = ImmutableList.new.each
    assert_equal [], result
  end

  # =========================================================================
  # inspect / to_s
  # =========================================================================

  def test_inspect_empty_list
    list = ImmutableList.new
    assert_equal "ImmutableList[]", list.inspect
  end

  def test_inspect_with_elements
    list = ImmutableList.from_array(["a", "b", "c"])
    assert_equal "ImmutableList[a, b, c]", list.inspect
  end

  def test_to_s_matches_inspect
    list = ImmutableList.from_array(["x", "y"])
    assert_equal list.inspect, list.to_s
  end

  # =========================================================================
  # Equality (==)
  # =========================================================================

  def test_equal_lists
    a = ImmutableList.from_array(["a", "b", "c"])
    b = ImmutableList.from_array(["a", "b", "c"])
    assert_equal a, b
  end

  def test_unequal_lists_different_elements
    a = ImmutableList.from_array(["a", "b"])
    b = ImmutableList.from_array(["a", "c"])
    refute_equal a, b
  end

  def test_unequal_lists_different_lengths
    a = ImmutableList.from_array(["a", "b"])
    b = ImmutableList.from_array(["a", "b", "c"])
    refute_equal a, b
  end

  def test_empty_lists_are_equal
    a = ImmutableList.new
    b = ImmutableList.new
    assert_equal a, b
  end

  def test_equality_after_same_operations
    # Two lists built differently but with same content should be equal
    a = ImmutableList.new.push("x").push("y")
    b = ImmutableList.from_array(["x", "y"])
    assert_equal a, b
  end

  # =========================================================================
  # Structural sharing / immutability invariants
  # =========================================================================

  def test_push_set_pop_chain_preserves_all_versions
    v1 = ImmutableList.from_array(["a", "b", "c"])
    v2 = v1.push("d")
    v3 = v2.set(0, "A")
    v4_list, v4_val = v3.pop

    # v1 is unchanged
    assert_equal ["a", "b", "c"], v1.to_a

    # v2 has the pushed element
    assert_equal ["a", "b", "c", "d"], v2.to_a

    # v3 has the set element
    assert_equal ["A", "b", "c", "d"], v3.to_a

    # v4 has the popped element
    assert_equal "d", v4_val
    assert_equal ["A", "b", "c"], v4_list.to_a
  end

  def test_large_list_operations
    # Build a list with 100 elements
    list = ImmutableList.new
    100.times { |i| list = list.push("elem_#{i}") }

    assert_equal 100, list.size
    assert_equal "elem_0", list.get(0)
    assert_equal "elem_50", list.get(50)
    assert_equal "elem_99", list.get(99)
    assert_nil list.get(100)

    # Set an element in the middle
    modified = list.set(50, "MODIFIED")
    assert_equal "MODIFIED", modified.get(50)
    assert_equal "elem_50", list.get(50) # original unchanged

    # Pop
    popped, val = list.pop
    assert_equal "elem_99", val
    assert_equal 99, popped.size
  end

  def test_from_array_round_trip
    items = ["hello", "world", "foo", "bar"]
    list = ImmutableList.from_array(items)
    assert_equal items, list.to_a
  end

  def test_unicode_strings
    list = ImmutableList.from_array(["hello", "world"])
    assert_equal "hello", list.get(0)
    assert_equal "world", list.get(1)
  end

  def test_empty_strings
    list = ImmutableList.from_array(["", "", ""])
    assert_equal 3, list.size
    assert_equal "", list.get(0)
    assert_equal ["", "", ""], list.to_a
  end
end
