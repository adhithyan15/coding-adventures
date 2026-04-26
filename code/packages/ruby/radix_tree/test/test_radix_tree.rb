# frozen_string_literal: true

require_relative "test_helper"

class RadixTreeTest < Minitest::Test
  RadixTree = CodingAdventures::RadixTree::RadixTree

  def tree_with(*keys)
    RadixTree.new.tap do |tree|
      keys.each_with_index { |key, index| tree.insert(key, index + 1) }
    end
  end

  def test_insert_search_and_duplicates
    tree = RadixTree.new
    tree.insert("application", 1)
    tree.insert("apple", 2)
    tree.insert("app", 3)
    tree.insert("apt", 4)
    assert_equal 1, tree.search("application")
    assert_equal 2, tree.get("apple")
    assert_equal 3, tree.search("app")
    assert_equal 4, tree.search("apt")
    assert_nil tree.search("appl")
    tree.put("app", 99)
    assert_equal 99, tree.search("app")
    assert_equal 4, tree.size
    assert tree.contains?("app")
  end

  def test_delete_merges_compressed_edges
    tree = tree_with("app", "apple")
    assert_equal 3, tree.node_count
    assert tree.delete("app")
    assert_nil tree.search("app")
    assert_equal 2, tree.search("apple")
    assert_equal 2, tree.node_count
    refute tree.delete("missing")
  end

  def test_prefix_queries_and_keys
    tree = tree_with("search", "searcher", "searching", "banana")
    assert tree.starts_with?("sear")
    refute tree.starts_with?("seek")
    assert_equal %w[search searcher searching], tree.words_with_prefix("search")
    assert_equal %w[banana search searcher searching], tree.keys
  end

  def test_longest_prefix_match_and_empty_key
    tree = tree_with("a", "ab", "abc", "application")
    assert_equal "abc", tree.longest_prefix_match("abcdef")
    assert_equal "application", tree.longest_prefix_match("application/json")
    assert_nil tree.longest_prefix_match("xyz")

    empty = RadixTree.new
    empty.insert("", 1)
    empty.insert("a", 2)
    assert_equal 1, empty.search("")
    assert_equal "", empty.longest_prefix_match("xyz")
    assert empty.delete("")
  end

  def test_maps_values_and_rendering
    tree = RadixTree.new([["foo", 1], ["bar", 2], ["baz", 3]])
    assert_equal({ "bar" => 2, "baz" => 3, "foo" => 1 }, tree.to_h)
    assert_equal [2, 3, 1], tree.values
    assert_match(/3 keys/, tree.to_s)
    refute tree.empty?
  end

  def test_absent_paths_and_empty_prefixes
    empty = RadixTree.new
    refute empty.starts_with?("")
    assert_empty empty.words_with_prefix("x")
    refute empty.contains?("x")
    assert empty.empty?

    tree = tree_with("apple")
    assert tree.starts_with?("")
    refute tree.starts_with?("apz")
    assert_empty tree.words_with_prefix("apz")
    assert_empty tree.words_with_prefix("applez")
    assert_nil tree.search("apz")
    refute tree.contains?("app")
  end

  def test_delete_leaf_prefix_and_empty_key_cases
    tree = tree_with("apple", "banana")
    assert tree.delete("apple")
    assert_nil tree.search("apple")
    assert_equal 1, tree.size
    refute tree.delete("app")

    empty_key = RadixTree.new
    empty_key.insert("", "root")
    empty_key.insert("alpha", "letter")
    assert_equal "root", empty_key.search("")
    assert empty_key.delete("")
    assert_nil empty_key.search("")
    assert_equal "letter", empty_key.search("alpha")
  end
end
