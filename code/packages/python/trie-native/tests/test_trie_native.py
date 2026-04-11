from __future__ import annotations

import pytest

from trie_native import KeyNotFoundError, Trie


def test_insert_search_and_prefix_queries() -> None:
    trie = Trie()
    trie.insert("app", 1)
    trie.insert("apple", 2)
    trie.insert("apply")

    assert trie.search("app") == 1
    assert trie.search("apple") == 2
    assert trie.search("apply") is True
    assert trie.starts_with("app")
    assert trie.words_with_prefix("app") == [("app", 1), ("apple", 2), ("apply", True)]


def test_longest_prefix_match_and_all_words() -> None:
    trie = Trie()
    trie.insert("", "root")
    trie.insert("cat", "animal")
    trie.insert("cater", "verb")

    assert trie.longest_prefix_match("caterpillar") == ("cater", "verb")
    assert trie.longest_prefix_match("zzz") == ("", "root")
    assert trie.all_words() == [("", "root"), ("cat", "animal"), ("cater", "verb")]


def test_dict_like_item_access_and_delete() -> None:
    trie = Trie()
    trie["banana"] = None
    assert trie["banana"] is None
    assert "banana" in trie
    assert len(trie) == 1

    del trie["banana"]
    assert trie.search("banana") is None

    with pytest.raises(KeyNotFoundError):
        _ = trie["banana"]


def test_iteration_and_validation() -> None:
    trie = Trie()
    trie.insert("b", 2)
    trie.insert("a", 1)
    trie.insert("c", 3)

    assert list(trie) == ["a", "b", "c"]
    assert trie.items() == [("a", 1), ("b", 2), ("c", 3)]
    assert trie.is_valid()


def test_delete_nonexistent_returns_false() -> None:
    trie = Trie()
    trie.insert("apple", 1)
    assert trie.delete("missing") is False
