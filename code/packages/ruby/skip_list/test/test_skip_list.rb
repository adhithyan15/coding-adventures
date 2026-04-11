# frozen_string_literal: true

require "minitest/autorun"
require "coding_adventures_skip_list"

class TestSkipList < Minitest::Test
  include CodingAdventures::SkipList

  def test_insert_search_delete
    list = SkipList.new
    list.insert(3, "c")
    list.insert(1, "a")
    list.insert(2, "b")

    assert_equal "b", list.search(2)
    assert_equal 1, list.rank(2)
    assert_equal [1, "a"], list.first
    assert_equal [3, "c"], list.last
    assert_equal "b", list.delete(2)
    refute list.include?(2)
  end

  def test_range_returns_sorted_entries
    list = SkipList.new
    [10, 5, 7, 1, 9].each { |value| list.insert(value, value) }

    assert_equal [[5, 5], [7, 7], [9, 9]], list.range(5, 9)
  end
end
