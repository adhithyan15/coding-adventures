"""
Tests for trie.py — Prefix Tree.

Test strategy follows the spec's test cases:
1. Empty trie
2. Single insert + search
3. Prefix sharing
4. Delete leaf
5. Delete non-leaf
6. Delete only shared prefix
7. Delete all words
8. Longest prefix match
9. All words sorted
10. Unicode keys
11. Empty string key
12. Case sensitivity
13. Large scale
14. IP routing simulation
15. Autocomplete simulation

Plus: dict-like interface, dunder methods, is_valid, error conditions.
"""

from __future__ import annotations

import random
import string

import pytest

from trie import Trie, TrieError
from trie.trie import KeyNotFoundError


# ─── Helpers ─────────────────────────────────────────────────────────────────


def make_trie(*words: str) -> Trie[bool]:
    """Create a Trie[bool] with all given words inserted as True."""
    t: Trie[bool] = Trie()
    for w in words:
        t.insert(w)
    return t


# ─── Construction and Empty Trie ─────────────────────────────────────────────


class TestEmpty:
    def test_empty_len(self) -> None:
        t: Trie[str] = Trie()
        assert len(t) == 0

    def test_empty_bool(self) -> None:
        t: Trie[str] = Trie()
        assert not t

    def test_empty_search(self) -> None:
        t: Trie[str] = Trie()
        assert t.search("any") is None

    def test_empty_contains(self) -> None:
        t: Trie[str] = Trie()
        assert "any" not in t

    def test_empty_starts_with(self) -> None:
        t: Trie[str] = Trie()
        assert not t.starts_with("a")

    def test_empty_starts_with_empty_prefix(self) -> None:
        t: Trie[str] = Trie()
        # empty trie: starts_with("") is False because no words exist
        assert not t.starts_with("")

    def test_empty_all_words(self) -> None:
        t: Trie[str] = Trie()
        assert t.all_words() == []

    def test_empty_words_with_prefix(self) -> None:
        t: Trie[str] = Trie()
        assert t.words_with_prefix("a") == []

    def test_empty_longest_prefix_match(self) -> None:
        t: Trie[str] = Trie()
        assert t.longest_prefix_match("abc") is None

    def test_empty_iter(self) -> None:
        t: Trie[str] = Trie()
        assert list(t) == []

    def test_empty_is_valid(self) -> None:
        t: Trie[str] = Trie()
        assert t.is_valid()


# ─── Single Insert + Search (Spec case 2) ────────────────────────────────────


class TestSingleInsert:
    def test_search_exact(self) -> None:
        t: Trie[int] = Trie()
        t.insert("hello", 42)
        assert t.search("hello") == 42

    def test_search_prefix_only(self) -> None:
        t: Trie[int] = Trie()
        t.insert("hello", 42)
        assert t.search("hell") is None

    def test_search_superset(self) -> None:
        t: Trie[int] = Trie()
        t.insert("hello", 42)
        assert t.search("hellos") is None

    def test_len_after_insert(self) -> None:
        t: Trie[int] = Trie()
        t.insert("hello", 42)
        assert len(t) == 1
        assert bool(t)

    def test_contains_after_insert(self) -> None:
        t: Trie[int] = Trie()
        t.insert("hello", 42)
        assert "hello" in t
        assert "hell" not in t

    def test_update_existing_key(self) -> None:
        t: Trie[int] = Trie()
        t.insert("hello", 1)
        t.insert("hello", 99)
        assert t.search("hello") == 99
        assert len(t) == 1  # count does not increase for update

    def test_starts_with_after_insert(self) -> None:
        t: Trie[int] = Trie()
        t.insert("hello", 1)
        assert t.starts_with("h")
        assert t.starts_with("hel")
        assert t.starts_with("hello")
        assert not t.starts_with("world")

    def test_starts_with_empty_prefix_nonempty_trie(self) -> None:
        t: Trie[int] = Trie()
        t.insert("hello", 1)
        assert t.starts_with("")  # empty prefix matches everything


# ─── Prefix Sharing (Spec case 3) ────────────────────────────────────────────


class TestPrefixSharing:
    def test_shared_prefix_words(self) -> None:
        t = make_trie("app", "apple", "apply")
        assert len(t) == 3
        assert t.search("app") is not None
        assert t.search("apple") is not None
        assert t.search("apply") is not None

    def test_starts_with_shared_prefix(self) -> None:
        t = make_trie("app", "apple", "apply")
        assert t.starts_with("app")
        assert t.starts_with("appl")
        assert not t.starts_with("apz")

    def test_words_with_prefix_sorted(self) -> None:
        t = make_trie("app", "apple", "apply")
        result = [w for w, _ in t.words_with_prefix("app")]
        assert result == ["app", "apple", "apply"]  # sorted

    def test_words_with_prefix_no_match(self) -> None:
        t = make_trie("app", "apple")
        assert t.words_with_prefix("xyz") == []

    def test_words_with_prefix_exact_word(self) -> None:
        t = make_trie("app", "apple")
        result = t.words_with_prefix("apple")
        assert [w for w, _ in result] == ["apple"]

    def test_words_with_prefix_single_char(self) -> None:
        t = make_trie("app", "apple", "banana")
        result = [w for w, _ in t.words_with_prefix("a")]
        assert result == ["app", "apple"]

    def test_all_words_sorted(self) -> None:
        words = ["banana", "app", "apple", "apply", "apt"]
        t = make_trie(*words)
        result = [w for w, _ in t.all_words()]
        assert result == sorted(words)


# ─── Delete (Spec cases 4–7) ──────────────────────────────────────────────────


class TestDelete:
    def test_delete_leaf(self) -> None:
        """Spec case 4: delete leaf should clean up the entire branch."""
        t = make_trie("apple")
        assert t.delete("apple")
        assert t.search("apple") is None
        assert len(t) == 0
        # The root should have no children after cleanup
        assert not t._root.children

    def test_delete_non_leaf(self) -> None:
        """Spec case 5: deleting 'app' should not affect 'apple'."""
        t = make_trie("app", "apple")
        assert t.delete("app")
        assert t.search("app") is None
        assert t.search("apple") is not None
        assert len(t) == 1

    def test_delete_shared_prefix_word(self) -> None:
        """Spec case 6: words_with_prefix should reflect deletion."""
        t = make_trie("app", "apple")
        t.delete("app")
        result = [w for w, _ in t.words_with_prefix("app")]
        assert result == ["apple"]

    def test_delete_all_words(self) -> None:
        """Spec case 7: after deleting all words, root should be empty."""
        words = ["app", "apple", "apt", "banana"]
        t = make_trie(*words)
        for w in words:
            assert t.delete(w)
        assert len(t) == 0
        assert not t._root.children
        assert t.is_valid()

    def test_delete_nonexistent_returns_false(self) -> None:
        t = make_trie("apple")
        assert not t.delete("xyz")

    def test_delete_prefix_not_word(self) -> None:
        """Deleting 'ap' when only 'app' is stored should return False."""
        t = make_trie("app")
        assert not t.delete("ap")

    def test_delete_preserves_sibling(self) -> None:
        """Deleting 'apt' should not affect 'app'."""
        t = make_trie("app", "apt")
        t.delete("apt")
        assert "app" in t
        assert "apt" not in t

    def test_delete_order_independence(self) -> None:
        """Inserting and deleting in different orders should be consistent."""
        words = ["a", "ab", "abc", "abcd"]
        for _ in range(4):
            t = make_trie(*words)
            random.shuffle(words)
            for w in words:
                t.delete(w)
            assert len(t) == 0
            assert t.is_valid()


# ─── Longest Prefix Match (Spec case 8) ──────────────────────────────────────


class TestLongestPrefixMatch:
    def test_spec_example(self) -> None:
        t: Trie[int] = Trie()
        for i, w in enumerate(["a", "ab", "abc", "abcd"]):
            t.insert(w, i)
        assert t.longest_prefix_match("abcde") == ("abcd", 3)
        assert t.longest_prefix_match("xyz") is None
        assert t.longest_prefix_match("a") == ("a", 0)

    def test_no_prefix_match(self) -> None:
        t = make_trie("app", "apple")
        assert t.longest_prefix_match("xyz") is None

    def test_exact_match(self) -> None:
        t = make_trie("abc")
        result = t.longest_prefix_match("abc")
        assert result is not None
        assert result[0] == "abc"

    def test_partial_path_no_word(self) -> None:
        """Path exists but no word ends early enough."""
        t = make_trie("abcde")
        # "abcde" IS a prefix of "abcdefgh", so it matches
        result = t.longest_prefix_match("abcdefgh")
        assert result is not None
        assert result[0] == "abcde"
        # A string where the path breaks before any word endpoint → None
        assert t.longest_prefix_match("abcdz") is None

    def test_ip_routing_simulation(self) -> None:
        """Spec case 14: IP routing table simulation."""
        t: Trie[str] = Trie()
        routes = [
            ("192", "iface0"),
            ("192.168", "iface1"),
            ("192.168.1", "iface2"),
            ("10", "iface3"),
            ("10.0", "iface4"),
        ]
        for prefix, iface in routes:
            t.insert(prefix, iface)

        assert t.longest_prefix_match("192.168.1.5") == ("192.168.1", "iface2")
        assert t.longest_prefix_match("192.168.2.1") == ("192.168", "iface1")
        assert t.longest_prefix_match("172.16.0.1") is None

    def test_multiple_candidates(self) -> None:
        t: Trie[int] = Trie()
        t.insert("cat", 1)
        t.insert("ca", 2)
        t.insert("c", 3)
        assert t.longest_prefix_match("catalog") == ("cat", 1)
        assert t.longest_prefix_match("ca") == ("ca", 2)
        assert t.longest_prefix_match("cobalt") == ("c", 3)


# ─── All Words Sorted (Spec case 9) ──────────────────────────────────────────


class TestAllWordsSorted:
    def test_all_words_random_insert_order(self) -> None:
        words = ["banana", "app", "apple", "apply", "apt", "cat"]
        rng = random.Random(1)
        shuffled = words[:]
        rng.shuffle(shuffled)
        t = make_trie(*shuffled)
        result = [w for w, _ in t.all_words()]
        assert result == sorted(words)

    def test_iter_gives_sorted_keys(self) -> None:
        words = ["z", "a", "m", "aa", "ab"]
        t = make_trie(*words)
        assert list(t) == sorted(words)

    def test_items_gives_sorted_pairs(self) -> None:
        t: Trie[int] = Trie()
        pairs = [("b", 2), ("a", 1), ("c", 3)]
        for k, v in pairs:
            t.insert(k, v)
        result = list(t.items())
        assert [k for k, _ in result] == ["a", "b", "c"]


# ─── Unicode Keys (Spec case 10) ─────────────────────────────────────────────


class TestUnicodeKeys:
    def test_unicode_insert_search(self) -> None:
        t: Trie[int] = Trie()
        t.insert("café", 1)
        t.insert("cafe", 2)
        t.insert("caf", 3)
        assert t.search("café") == 1
        assert t.search("cafe") == 2
        assert t.search("caf") == 3
        assert len(t) == 3

    def test_emoji_keys(self) -> None:
        t: Trie[str] = Trie()
        t.insert("hello", "greeting")
        t.insert("hell", "bad place")
        assert t.search("hello") == "greeting"
        assert t.search("hell") == "bad place"

    def test_unicode_prefix(self) -> None:
        t: Trie[int] = Trie()
        t.insert("café", 1)
        t.insert("cafeteria", 2)
        assert t.starts_with("caf")
        result = [w for w, _ in t.words_with_prefix("caf")]
        assert "café" in result
        assert "cafeteria" in result


# ─── Empty String Key (Spec case 11) ─────────────────────────────────────────


class TestEmptyStringKey:
    def test_insert_empty_key(self) -> None:
        t: Trie[str] = Trie()
        t.insert("", "root value")
        assert t.search("") == "root value"
        assert len(t) == 1

    def test_starts_with_empty_prefix_after_insert(self) -> None:
        t: Trie[str] = Trie()
        t.insert("", "root")
        assert t.starts_with("")

    def test_words_with_empty_prefix_includes_empty_key(self) -> None:
        t: Trie[str] = Trie()
        t.insert("", "root")
        t.insert("a", "letter")
        result = [w for w, _ in t.words_with_prefix("")]
        assert "" in result
        assert "a" in result

    def test_delete_empty_key(self) -> None:
        t: Trie[str] = Trie()
        t.insert("", "root")
        t.insert("a", "letter")
        assert t.delete("")
        assert t.search("") is None
        assert t.search("a") == "letter"
        assert len(t) == 1


# ─── Case Sensitivity (Spec case 12) ─────────────────────────────────────────


class TestCaseSensitivity:
    def test_case_sensitive_search(self) -> None:
        t = make_trie("Hello")
        assert t.search("Hello") is not None
        assert t.search("hello") is None

    def test_case_sensitive_distinct_keys(self) -> None:
        t: Trie[int] = Trie()
        t.insert("Hello", 1)
        t.insert("hello", 2)
        assert len(t) == 2
        assert t.search("Hello") == 1
        assert t.search("hello") == 2


# ─── Large Scale (Spec case 13) ───────────────────────────────────────────────


class TestLargeScale:
    def test_insert_and_search_1000_words(self) -> None:
        rng = random.Random(42)
        words = list({
            "".join(rng.choices(string.ascii_lowercase, k=rng.randint(3, 10)))
            for _ in range(1000)
        })
        t = make_trie(*words)
        assert len(t) == len(words)
        for w in words:
            assert t.search(w) is not None

    def test_all_words_1000_sorted(self) -> None:
        rng = random.Random(99)
        words = list({
            "".join(rng.choices(string.ascii_lowercase, k=rng.randint(3, 8)))
            for _ in range(500)
        })
        t = make_trie(*words)
        result = [w for w, _ in t.all_words()]
        assert result == sorted(set(words))


# ─── Autocomplete Simulation (Spec case 15) ───────────────────────────────────


class TestAutocomplete:
    def test_autocomplete_matches_linear_scan(self) -> None:
        rng = random.Random(7)
        words = list({
            "".join(rng.choices(string.ascii_lowercase, k=rng.randint(3, 10)))
            for _ in range(500)
        })
        t = make_trie(*words)

        prefix = "pre"
        trie_result = [w for w, _ in t.words_with_prefix(prefix)]
        brute_result = sorted(w for w in words if w.startswith(prefix))

        assert trie_result == brute_result

    def test_words_with_empty_prefix_returns_all(self) -> None:
        words = ["cat", "car", "bat", "bar"]
        t = make_trie(*words)
        result = [w for w, _ in t.words_with_prefix("")]
        assert result == sorted(words)


# ─── Dict-like Interface ─────────────────────────────────────────────────────


class TestDictLikeInterface:
    def test_setitem_getitem(self) -> None:
        t: Trie[int] = Trie()
        t["apple"] = 42
        assert t["apple"] == 42

    def test_getitem_not_found_raises(self) -> None:
        t: Trie[int] = Trie()
        with pytest.raises(KeyNotFoundError):
            _ = t["missing"]

    def test_delitem(self) -> None:
        t: Trie[int] = Trie()
        t["apple"] = 1
        del t["apple"]
        assert "apple" not in t

    def test_delitem_not_found_raises(self) -> None:
        t: Trie[int] = Trie()
        with pytest.raises(KeyNotFoundError):
            del t["missing"]

    def test_contains_non_string(self) -> None:
        t: Trie[int] = Trie()
        t.insert("hello", 1)
        assert 42 not in t  # type: ignore[operator]

    def test_setitem_update(self) -> None:
        t: Trie[int] = Trie()
        t["key"] = 1
        t["key"] = 2
        assert t["key"] == 2
        assert len(t) == 1


# ─── is_valid ─────────────────────────────────────────────────────────────────


class TestIsValid:
    def test_valid_after_inserts(self) -> None:
        t = make_trie("app", "apple", "apt")
        assert t.is_valid()

    def test_valid_after_deletes(self) -> None:
        t = make_trie("app", "apple", "apt")
        t.delete("apple")
        assert t.is_valid()

    def test_valid_after_all_deleted(self) -> None:
        t = make_trie("a", "b", "c")
        for w in ["a", "b", "c"]:
            t.delete(w)
        assert t.is_valid()


# ─── Dunder Methods ───────────────────────────────────────────────────────────


class TestDunderMethods:
    def test_bool_empty(self) -> None:
        t: Trie[int] = Trie()
        assert not t

    def test_bool_nonempty(self) -> None:
        t = make_trie("hello")
        assert t

    def test_len(self) -> None:
        t = make_trie("a", "b", "c")
        assert len(t) == 3

    def test_iter_empty(self) -> None:
        t: Trie[int] = Trie()
        assert list(t) == []

    def test_iter_sorted(self) -> None:
        t = make_trie("c", "a", "b")
        assert list(t) == ["a", "b", "c"]

    def test_items_sorted(self) -> None:
        t: Trie[int] = Trie()
        t.insert("c", 3)
        t.insert("a", 1)
        t.insert("b", 2)
        result = [(k, v) for k, v in t.items()]
        assert result == [("a", 1), ("b", 2), ("c", 3)]

    def test_repr_small(self) -> None:
        t = make_trie("apple", "app")
        r = repr(t)
        assert "Trie(" in r
        assert "2 keys" in r

    def test_repr_large_truncated(self) -> None:
        # More than 5 words triggers the "...+N" truncation path in __repr__
        t = make_trie("a", "b", "c", "d", "e", "f", "g")
        r = repr(t)
        assert "...+" in r

    def test_getitem_key_with_default_true_value(self) -> None:
        # Exercises _key_exists via __getitem__ when search returns True (not None)
        t: Trie[bool] = Trie()
        t.insert("key")  # value defaults to True
        assert t["key"] is True


# ─── Stress Test ──────────────────────────────────────────────────────────────


class TestStress:
    def test_interleaved_insert_delete(self) -> None:
        """Insert and delete words in random order, verifying correctness."""
        rng = random.Random(13)
        vocab = [
            "".join(rng.choices(string.ascii_lowercase, k=rng.randint(2, 8)))
            for _ in range(200)
        ]
        vocab = list(set(vocab))

        t: Trie[bool] = Trie()
        alive: set[str] = set()

        for _ in range(500):
            op = rng.choice(["insert", "delete", "search"])
            if op == "insert":
                w = rng.choice(vocab)
                t.insert(w)
                alive.add(w)
            elif op == "delete":
                if alive:
                    w = rng.choice(list(alive))
                    t.delete(w)
                    alive.discard(w)
            else:
                w = rng.choice(vocab)
                found = t.search(w) is not None
                assert found == (w in alive), (
                    f"search({w!r}) returned {found}, expected {w in alive}"
                )

        assert len(t) == len(alive)
        assert t.is_valid()
