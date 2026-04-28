# frozen_string_literal: true

require_relative "test_helper"

class TrieTest < Minitest::Test
  Trie = CodingAdventures::Trie::Trie

  def make_trie(*words)
    Trie.new.tap do |trie|
      words.each { |word| trie.insert(word) }
    end
  end

  def test_empty_trie
    trie = Trie.new
    assert_equal 0, trie.size
    assert trie.empty?
    assert_nil trie.search("anything")
    refute trie.starts_with?("a")
    assert trie.valid?
  end

  def test_insert_search_and_update
    trie = Trie.new
    trie.insert("hello", 42)
    assert_equal 42, trie.search("hello")
    assert_nil trie.search("hell")
    assert_nil trie.search("hellos")

    trie.insert("hello", 99)
    assert_equal 99, trie.search("hello")
    assert_equal 1, trie.length
    assert trie.key?("hello")
    assert trie.contains?("hello")
  end

  def test_prefix_words_and_sorted_keys
    trie = make_trie("banana", "app", "apple", "apply", "apt")
    assert_equal %w[app apple apply], trie.words_with_prefix("app").map(&:first)
    assert_empty trie.words_with_prefix("xyz")
    assert_equal %w[app apple apply apt banana], trie.keys
    assert_equal 5, trie.entries.length
    assert_equal trie.entries, trie.to_a
  end

  def test_delete_leaf_and_shared_prefix
    trie = make_trie("app", "apple", "apt")
    assert trie.delete("app")
    refute trie.key?("app")
    assert trie.key?("apple")
    assert trie.key?("apt")
    assert_equal 2, trie.size
    refute trie.delete("missing")
    refute trie.delete("ap")
    assert trie.delete("apple")
    assert trie.delete("apt")
    assert trie.empty?
    assert trie.valid?
  end

  def test_longest_prefix_match
    trie = Trie.new([["a", 1], ["ab", 2], ["abc", 3], ["abcd", 4]])
    assert_equal ["abcd", 4], trie.longest_prefix_match("abcde")
    assert_nil trie.longest_prefix_match("xyz")
    assert_equal ["a", 1], trie.longest_prefix_match("a")
  end

  def test_unicode_and_empty_string_keys
    trie = Trie.new
    trie.insert("", "root")
    trie.insert("cafe", "plain")
    trie.insert("cafe\u0301", "accent-combining")
    trie.insert("caf\u00e9", "accent-single")

    assert_equal "root", trie.search("")
    assert trie.starts_with?("")
    assert trie.starts_with?("caf")
    assert_equal "accent-single", trie.search("caf\u00e9")
    assert_equal ["cafe\u0301", "accent-combining"], trie.longest_prefix_match("cafe\u0301-au-lait")
    assert trie.delete("")
    assert_nil trie.search("")
    assert_match(/3 keys/, trie.to_s)
  end

  def test_each_and_argument_validation
    trie = make_trie("b", "a")
    assert_equal [["a", true], ["b", true]], trie.each.to_a
    assert_raises(ArgumentError) { trie.insert(:symbol) }
  end
end
