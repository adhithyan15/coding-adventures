from __future__ import annotations

from radix_tree_native import RadixTree


def test_insert_search_and_delete() -> None:
    tree = RadixTree()
    tree.insert("app", 1)
    tree.insert("apple", 2)

    assert tree.search("app") == 1
    assert tree.search("apple") == 2
    assert tree.delete("app") is True
    assert tree.search("app") is None
    assert tree.search("apple") == 2


def test_prefix_queries_and_iteration() -> None:
    tree = RadixTree()
    tree.insert("search", "base")
    tree.insert("searcher", "person")
    tree.insert("searching", "progressive")

    assert tree.starts_with("sear")
    assert tree.words_with_prefix("search") == ["search", "searcher", "searching"]
    assert tree.longest_prefix_match("searching-party") == "searching"
    assert list(tree) == ["search", "searcher", "searching"]


def test_to_dict_and_empty_key() -> None:
    tree = RadixTree()
    tree.insert("", "root")
    tree.insert("alpha", 1)
    tree.insert("beta", None)

    assert tree.search("") == "root"
    assert tree.to_dict() == {"": "root", "alpha": 1, "beta": None}
    assert "" in tree
