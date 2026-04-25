package trie

import (
	"fmt"
	"testing"
)

func makeTrie(words ...string) *Trie[bool] {
	trie := New[bool]()
	for _, word := range words {
		trie.Insert(word, true)
	}
	return trie
}

func TestEmptyTrie(t *testing.T) {
	trie := New[int]()
	if trie.Len() != 0 || !trie.IsEmpty() {
		t.Fatalf("new trie should be empty")
	}
	if _, ok := trie.Search("anything"); ok {
		t.Fatalf("empty trie unexpectedly found a key")
	}
	if trie.StartsWith("a") {
		t.Fatalf("empty trie should not have prefixes")
	}
	if !trie.IsValid() {
		t.Fatalf("empty trie should be valid")
	}
}

func TestInsertSearchAndUpdate(t *testing.T) {
	trie := New[int]()
	trie.Insert("hello", 42)
	if value, ok := trie.Search("hello"); !ok || value != 42 {
		t.Fatalf("Search(hello) = %v, %v", value, ok)
	}
	if _, ok := trie.Search("hell"); ok {
		t.Fatalf("prefix should not be an exact key")
	}
	if _, ok := trie.Search("hellos"); ok {
		t.Fatalf("superset should not be found")
	}

	trie.Insert("hello", 99)
	if value, _ := trie.Search("hello"); value != 99 {
		t.Fatalf("updated value = %d", value)
	}
	if trie.Len() != 1 || !trie.ContainsKey("hello") || !trie.Contains("hello") {
		t.Fatalf("updated trie has wrong membership")
	}
}

func TestPrefixWordsAreLexicographic(t *testing.T) {
	trie := makeTrie("banana", "app", "apple", "apply", "apt")
	if got := entryKeys(trie.WordsWithPrefix("app")); fmt.Sprint(got) != "[app apple apply]" {
		t.Fatalf("WordsWithPrefix(app) = %v", got)
	}
	if got := trie.WordsWithPrefix("xyz"); len(got) != 0 {
		t.Fatalf("WordsWithPrefix(xyz) = %v", got)
	}
	if got := fmt.Sprint(trie.Keys()); got != "[app apple apply apt banana]" {
		t.Fatalf("Keys() = %s", got)
	}
	if len(trie.AllWords()) != 5 {
		t.Fatalf("AllWords length = %d", len(trie.AllWords()))
	}
}

func TestDeleteLeafAndSharedPrefix(t *testing.T) {
	trie := makeTrie("app", "apple", "apt")
	if !trie.Delete("app") {
		t.Fatalf("expected app to be deleted")
	}
	if trie.ContainsKey("app") || !trie.ContainsKey("apple") || !trie.ContainsKey("apt") {
		t.Fatalf("delete disturbed shared-prefix keys")
	}
	if trie.Len() != 2 {
		t.Fatalf("size after delete = %d", trie.Len())
	}
	if trie.Delete("missing") || trie.Delete("ap") {
		t.Fatalf("delete should reject absent keys")
	}
	if !trie.Delete("apple") || !trie.Delete("apt") {
		t.Fatalf("expected remaining keys to delete")
	}
	if !trie.IsEmpty() || !trie.IsValid() {
		t.Fatalf("deleted trie should be empty and valid")
	}
}

func TestLongestPrefixMatch(t *testing.T) {
	trie := FromEntries([]Entry[int]{
		{Key: "a", Value: 1},
		{Key: "ab", Value: 2},
		{Key: "abc", Value: 3},
		{Key: "abcd", Value: 4},
	})

	if match, ok := trie.LongestPrefixMatch("abcde"); !ok || match.Key != "abcd" || match.Value != 4 {
		t.Fatalf("LongestPrefixMatch(abcde) = %v, %v", match, ok)
	}
	if _, ok := trie.LongestPrefixMatch("xyz"); ok {
		t.Fatalf("unexpected longest-prefix match")
	}
	if match, ok := trie.LongestPrefixMatch("a"); !ok || match.Key != "a" || match.Value != 1 {
		t.Fatalf("LongestPrefixMatch(a) = %v, %v", match, ok)
	}
}

func TestUnicodeAndEmptyStringKeys(t *testing.T) {
	trie := New[string]()
	trie.Insert("", "root")
	trie.Insert("cafe", "plain")
	trie.Insert("cafe\u0301", "accent-combining")
	trie.Insert("caf\u00e9", "accent-single")

	if value, ok := trie.Search(""); !ok || value != "root" {
		t.Fatalf("empty key = %q, %v", value, ok)
	}
	if !trie.StartsWith("") || !trie.StartsWith("caf") {
		t.Fatalf("expected empty and caf prefixes")
	}
	if value, ok := trie.Search("caf\u00e9"); !ok || value != "accent-single" {
		t.Fatalf("single-codepoint accent = %q, %v", value, ok)
	}
	if match, ok := trie.LongestPrefixMatch("cafe\u0301-au-lait"); !ok || match.Key != "cafe\u0301" {
		t.Fatalf("combining accent match = %v, %v", match, ok)
	}
	if !trie.Delete("") {
		t.Fatalf("expected empty key deletion")
	}
	if _, ok := trie.Search(""); ok {
		t.Fatalf("empty key should be deleted")
	}
	if fmt.Sprint(trie) == "" {
		t.Fatalf("String should render a non-empty summary")
	}
}

func entryKeys[V any](entries []Entry[V]) []string {
	keys := make([]string, 0, len(entries))
	for _, entry := range entries {
		keys = append(keys, entry.Key)
	}
	return keys
}
