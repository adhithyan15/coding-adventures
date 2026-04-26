defmodule CodingAdventures.TrieTest do
  use ExUnit.Case, async: true

  alias CodingAdventures.Trie

  defp make_trie(words) do
    Enum.reduce(words, Trie.new(), fn word, trie -> Trie.insert(trie, word, true) end)
  end

  test "new trie starts empty" do
    trie = Trie.new()
    assert Trie.size(trie) == 0
    assert Trie.empty?(trie)
    assert Trie.search(trie, "anything") == :error
    refute Trie.starts_with?(trie, "a")
    assert Trie.valid?(trie)
  end

  test "insert search and update exact keys" do
    trie = Trie.new() |> Trie.insert("hello", 42)
    assert Trie.search(trie, "hello") == {:ok, 42}
    assert Trie.search(trie, "hell") == :error
    assert Trie.search(trie, "hellos") == :error

    trie = Trie.insert(trie, "hello", 99)
    assert Trie.get(trie, "hello") == 99
    assert Trie.get(trie, "missing", :fallback) == :fallback
    assert Trie.size(trie) == 1
    assert Trie.contains?(trie, "hello")
  end

  test "prefix words and keys are lexicographic" do
    trie = make_trie(["banana", "app", "apple", "apply", "apt"])

    assert Trie.words_with_prefix(trie, "app") |> Enum.map(&elem(&1, 0)) == [
             "app",
             "apple",
             "apply"
           ]

    assert Trie.words_with_prefix(trie, "xyz") == []
    assert Trie.keys(trie) == ["app", "apple", "apply", "apt", "banana"]
    assert length(Trie.all_words(trie)) == 5
  end

  test "delete leaf and shared-prefix keys" do
    trie = make_trie(["app", "apple", "apt"])
    {trie, deleted?} = Trie.delete(trie, "app")
    assert deleted?
    refute Trie.contains?(trie, "app")
    assert Trie.contains?(trie, "apple")
    assert Trie.contains?(trie, "apt")
    assert Trie.size(trie) == 2

    {trie, false} = Trie.delete(trie, "missing")
    {trie, false} = Trie.delete(trie, "ap")
    {trie, true} = Trie.delete(trie, "apple")
    {trie, true} = Trie.delete(trie, "apt")
    assert Trie.empty?(trie)
    assert Trie.valid?(trie)
  end

  test "finds the longest stored prefix" do
    trie = Trie.from_list([{"a", 1}, {"ab", 2}, {"abc", 3}, {"abcd", 4}])
    assert Trie.longest_prefix_match(trie, "abcde") == {:ok, {"abcd", 4}}
    assert Trie.longest_prefix_match(trie, "xyz") == :error
    assert Trie.longest_prefix_match(trie, "a") == {:ok, {"a", 1}}
  end

  test "supports unicode and empty string keys" do
    trie =
      Trie.new()
      |> Trie.insert("", "root")
      |> Trie.insert("cafe", "plain")
      |> Trie.insert("cafe\u0301", "accent-combining")
      |> Trie.insert("caf\u00E9", "accent-single")

    assert Trie.search(trie, "") == {:ok, "root"}
    assert Trie.starts_with?(trie, "")
    assert Trie.starts_with?(trie, "caf")
    assert Trie.search(trie, "caf\u00E9") == {:ok, "accent-single"}

    assert Trie.longest_prefix_match(trie, "cafe\u0301-au-lait") ==
             {:ok, {"cafe\u0301", "accent-combining"}}

    {trie, true} = Trie.delete(trie, "")
    assert Trie.search(trie, "") == :error
  end

  test "from_list rejects invalid entries" do
    assert_raise ArgumentError, fn -> Trie.from_list([:bad]) end
  end
end
