# frozen_string_literal: true

require "minitest/autorun"
require "coding_adventures_hash_set"

class TestHashSet < Minitest::Test
  include CodingAdventures::HashSet

  def test_membership_and_duplicates
    set = HashSet.new
    set.add("red").add("green").add("red")

    assert_equal 2, set.size
    assert set.include?("green")
    refute set.include?("blue")
  end

  def test_set_algebra
    left = HashSet.new(%w[a b c])
    right = HashSet.new(%w[b c d])

    assert_equal %w[a b c d], left.union(right).to_a.sort
    assert_equal %w[b c], left.intersection(right).to_a.sort
    assert_equal %w[a], left.difference(right).to_a.sort
    assert left.superset?(HashSet.new(%w[a b]))
    assert right.subset?(HashSet.new(%w[a b c d e]))
  end

  def test_delete
    set = HashSet.new(%w[a b])
    assert set.delete("a")
    refute set.include?("a")
    refute set.delete("missing")
  end
end
