# frozen_string_literal: true

require_relative "test_helper"

class TestSuffixTree < Minitest::Test
  SuffixTree = CodingAdventures::SuffixTree::SuffixTree

  def test_search_and_count_work
    tree = SuffixTree.build("banana")

    assert_equal [1, 3], tree.search("ana")
    assert_equal 2, tree.count_occurrences("ana")
    assert_equal 7, tree.node_count
  end

  def test_longest_substring_helpers_work
    tree = SuffixTree.build("banana")

    assert_equal "ana", tree.longest_repeated_substring
    assert_equal %w[banana anana nana ana na a], tree.all_suffixes
    assert_equal "abxa", SuffixTree.longest_common_substring("xabxac", "abcabxabcd")
  end
end
