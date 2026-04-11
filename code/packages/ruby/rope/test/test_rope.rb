# frozen_string_literal: true

require_relative "test_helper"

class TestRope < Minitest::Test
  Rope = CodingAdventures::Rope::Rope

  def test_concat_split_and_index_work
    left = Rope.from_string("hello")
    right = Rope.from_string(" world")
    rope = CodingAdventures::Rope.concat(left, right)

    assert_equal "hello world", rope.to_string
    assert_equal 11, rope.len
    assert_equal "e", rope.index(1)

    split_left, split_right = CodingAdventures::Rope.split(rope, 5)
    assert_equal "hello", split_left.to_string
    assert_equal " world", split_right.to_string
  end

  def test_editing_and_rebalance_work
    rope = Rope.from_string("banana")

    assert_equal "banana", rope.to_string
    assert_equal "ana", CodingAdventures::Rope.substring(rope, 1, 4)
    assert_equal "bana", CodingAdventures::Rope.delete(rope, 4, 2).to_string
    assert_equal "bananax", CodingAdventures::Rope.insert(rope, 6, "x").to_string
    assert CodingAdventures::Rope.is_balanced(rope)
    assert_equal "banana", CodingAdventures::Rope.rebalance(rope).to_string
  end

  def test_node_helpers_and_module_convenience_methods_work
    leaf = CodingAdventures::Rope::RopeNode.new(CodingAdventures::Rope::LeafNode.new("x"))
    assert leaf.leaf?
    refute leaf.internal?
    assert_equal 0, leaf.depth
    assert leaf.balanced?
    assert_equal "x", leaf.to_s

    internal = CodingAdventures::Rope::RopeNode.new(
      CodingAdventures::Rope::InternalNode.new(
        1,
        CodingAdventures::Rope::LeafNode.new("a"),
        CodingAdventures::Rope::LeafNode.new("b")
      )
    )
    assert internal.internal?
    refute internal.leaf?
    assert_equal 1, internal.depth
    assert internal.balanced?
    assert_equal "ab", internal.to_s

    rope = Rope.empty
    assert rope.empty?
    assert_equal 0, rope.depth
    assert_equal "", rope.to_string
    assert_equal 0, CodingAdventures::Rope.length(rope)
    assert_equal rope.to_string, CodingAdventures::Rope.to_string(rope)
    assert CodingAdventures::Rope.is_balanced(rope)
    assert_equal "", CodingAdventures::Rope.rebalance(rope).to_string

    non_empty = Rope.from_string("hello")
    assert_equal 5, CodingAdventures::Rope.length(non_empty)
    assert_equal "e", CodingAdventures::Rope.index(non_empty, 1)
    assert_equal "l", CodingAdventures::Rope.rope_index(non_empty, 2)
    assert_equal "ell", CodingAdventures::Rope.substring(non_empty, 1, 4)
    assert_equal "hello", CodingAdventures::Rope.rope_from_string("hello").to_string
    assert_equal "", CodingAdventures::Rope.rope_empty.to_string
  end
end
