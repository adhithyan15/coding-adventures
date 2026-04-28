package radixtree

import (
	"fmt"
	"strings"
	"testing"
)

func treeWith(keys ...string) *RadixTree[int] {
	tree := New[int]()
	for index, key := range keys {
		tree.Insert(key, index+1)
	}
	return tree
}

func TestInsertSearchAndDuplicates(t *testing.T) {
	tree := New[int]()
	tree.Insert("application", 1)
	tree.Insert("apple", 2)
	tree.Insert("app", 3)
	tree.Insert("apt", 4)
	for key, want := range map[string]int{"application": 1, "apple": 2, "app": 3, "apt": 4} {
		if got, ok := tree.Search(key); !ok || got != want {
			t.Fatalf("Search(%s) = %v, %v", key, got, ok)
		}
	}
	if _, ok := tree.Search("appl"); ok {
		t.Fatalf("prefix-only key should be absent")
	}
	tree.Insert("app", 99)
	if got, _ := tree.Search("app"); got != 99 || tree.Len() != 4 {
		t.Fatalf("duplicate update failed")
	}
	if !tree.ContainsKey("app") || tree.IsEmpty() {
		t.Fatalf("membership state is wrong")
	}
}

func TestDeleteMergesCompressedEdges(t *testing.T) {
	tree := treeWith("app", "apple")
	if tree.NodeCount() != 3 {
		t.Fatalf("node count before delete = %d", tree.NodeCount())
	}
	if !tree.Delete("app") {
		t.Fatalf("delete should succeed")
	}
	if _, ok := tree.Search("app"); ok {
		t.Fatalf("deleted key still found")
	}
	if got, ok := tree.Search("apple"); !ok || got != 2 {
		t.Fatalf("apple should remain")
	}
	if tree.NodeCount() != 2 {
		t.Fatalf("node count after merge = %d", tree.NodeCount())
	}
	if tree.Delete("missing") {
		t.Fatalf("missing delete should return false")
	}
}

func TestPrefixQueriesAndKeys(t *testing.T) {
	tree := treeWith("search", "searcher", "searching", "banana")
	if !tree.StartsWith("sear") || tree.StartsWith("seek") {
		t.Fatalf("StartsWith result is wrong")
	}
	if New[int]().StartsWith("") {
		t.Fatalf("empty tree should not match empty prefix")
	}
	if !tree.StartsWith("") {
		t.Fatalf("non-empty tree should match empty prefix")
	}
	if got := fmt.Sprint(tree.WordsWithPrefix("search")); got != "[search searcher searching]" {
		t.Fatalf("WordsWithPrefix = %s", got)
	}
	if got := tree.WordsWithPrefix("sear"); fmt.Sprint(got) != "[search searcher searching]" {
		t.Fatalf("mid-edge WordsWithPrefix = %v", got)
	}
	if got := tree.WordsWithPrefix("seek"); len(got) != 0 {
		t.Fatalf("missing prefix returned %v", got)
	}
	if got := fmt.Sprint(tree.Keys()); got != "[banana search searcher searching]" {
		t.Fatalf("Keys = %s", got)
	}
}

func TestLongestPrefixMatchAndEmptyKey(t *testing.T) {
	tree := treeWith("a", "ab", "abc", "application")
	if got, ok := tree.LongestPrefixMatch("abcdef"); !ok || got != "abc" {
		t.Fatalf("LongestPrefixMatch abcdef = %s, %v", got, ok)
	}
	if got, ok := tree.LongestPrefixMatch("application/json"); !ok || got != "application" {
		t.Fatalf("LongestPrefixMatch application/json = %s, %v", got, ok)
	}
	if _, ok := tree.LongestPrefixMatch("xyz"); ok {
		t.Fatalf("unexpected prefix match")
	}

	empty := New[int]()
	empty.Insert("", 1)
	empty.Insert("a", 2)
	if got, ok := empty.Search(""); !ok || got != 1 {
		t.Fatalf("empty key lookup failed")
	}
	if got, ok := empty.LongestPrefixMatch("xyz"); !ok || got != "" {
		t.Fatalf("empty key should be longest prefix")
	}
	if !empty.Delete("") {
		t.Fatalf("empty key delete should succeed")
	}
}

func TestMapAndString(t *testing.T) {
	tree := FromEntries(map[string]int{"foo": 1, "bar": 2, "baz": 3})
	values := tree.ToMap()
	if values["foo"] != 1 || values["bar"] != 2 || values["baz"] != 3 {
		t.Fatalf("ToMap mismatch: %v", values)
	}
	if !strings.Contains(tree.String(), "3 keys") {
		t.Fatalf("String should mention size")
	}
	if tree.ContainsKey("ba") {
		t.Fatalf("prefix-only key should not count as membership")
	}
	if !tree.Delete("foo") {
		t.Fatalf("leaf delete should succeed")
	}
	if tree.Delete("fo") {
		t.Fatalf("prefix-only delete should fail")
	}
}
