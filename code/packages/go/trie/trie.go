package trie

import (
	"fmt"
	"sort"
)

type node[V any] struct {
	children map[rune]*node[V]
	terminal bool
	value    V
}

type Entry[V any] struct {
	Key   string
	Value V
}

type Trie[V any] struct {
	root *node[V]
	size int
}

func New[V any]() *Trie[V] {
	return &Trie[V]{root: newNode[V]()}
}

func FromEntries[V any](entries []Entry[V]) *Trie[V] {
	trie := New[V]()
	for _, entry := range entries {
		trie.Insert(entry.Key, entry.Value)
	}
	return trie
}

func (t *Trie[V]) Insert(key string, value V) {
	t.ensureRoot()
	current := t.root
	for _, ch := range key {
		child, ok := current.children[ch]
		if !ok {
			child = newNode[V]()
			current.children[ch] = child
		}
		current = child
	}

	if !current.terminal {
		t.size++
	}
	current.terminal = true
	current.value = value
}

func (t *Trie[V]) Search(key string) (V, bool) {
	node := t.findNode(key)
	if node == nil || !node.terminal {
		var zero V
		return zero, false
	}
	return node.value, true
}

func (t *Trie[V]) ContainsKey(key string) bool {
	return t.keyExists(key)
}

func (t *Trie[V]) Contains(key string) bool {
	return t.ContainsKey(key)
}

func (t *Trie[V]) Delete(key string) bool {
	if !t.keyExists(key) {
		return false
	}

	deleteRecursive(t.root, []rune(key), 0)
	t.size--
	return true
}

func (t *Trie[V]) StartsWith(prefix string) bool {
	if prefix == "" {
		return t.size > 0
	}
	return t.findNode(prefix) != nil
}

func (t *Trie[V]) WordsWithPrefix(prefix string) []Entry[V] {
	node := t.findNode(prefix)
	if node == nil {
		return []Entry[V]{}
	}

	results := make([]Entry[V], 0)
	collect(node, []rune(prefix), &results)
	return results
}

func (t *Trie[V]) AllWords() []Entry[V] {
	t.ensureRoot()
	results := make([]Entry[V], 0, t.size)
	collect(t.root, []rune{}, &results)
	return results
}

func (t *Trie[V]) Keys() []string {
	entries := t.AllWords()
	keys := make([]string, 0, len(entries))
	for _, entry := range entries {
		keys = append(keys, entry.Key)
	}
	return keys
}

func (t *Trie[V]) LongestPrefixMatch(input string) (Entry[V], bool) {
	t.ensureRoot()
	current := t.root
	prefix := []rune{}
	var best Entry[V]
	found := false

	if current.terminal {
		best = Entry[V]{Key: "", Value: current.value}
		found = true
	}

	for _, ch := range input {
		child, ok := current.children[ch]
		if !ok {
			break
		}
		prefix = append(prefix, ch)
		current = child
		if current.terminal {
			best = Entry[V]{Key: string(prefix), Value: current.value}
			found = true
		}
	}

	return best, found
}

func (t *Trie[V]) Len() int {
	return t.size
}

func (t *Trie[V]) IsEmpty() bool {
	return t.size == 0
}

func (t *Trie[V]) IsValid() bool {
	t.ensureRoot()
	return countEndpoints(t.root) == t.size
}

func (t *Trie[V]) String() string {
	words := t.AllWords()
	if len(words) > 5 {
		words = words[:5]
	}
	return fmt.Sprintf("Trie(%d keys: %v)", t.size, words)
}

func (t *Trie[V]) ensureRoot() {
	if t.root == nil {
		t.root = newNode[V]()
	}
}

func (t *Trie[V]) findNode(key string) *node[V] {
	t.ensureRoot()
	current := t.root
	for _, ch := range key {
		child, ok := current.children[ch]
		if !ok {
			return nil
		}
		current = child
	}
	return current
}

func (t *Trie[V]) keyExists(key string) bool {
	node := t.findNode(key)
	return node != nil && node.terminal
}

func newNode[V any]() *node[V] {
	return &node[V]{children: map[rune]*node[V]{}}
}

func collect[V any](current *node[V], prefix []rune, results *[]Entry[V]) {
	if current.terminal {
		*results = append(*results, Entry[V]{Key: string(prefix), Value: current.value})
	}

	keys := sortedChildren(current)
	for _, ch := range keys {
		next := make([]rune, len(prefix)+1)
		copy(next, prefix)
		next[len(prefix)] = ch
		collect(current.children[ch], next, results)
	}
}

func deleteRecursive[V any](current *node[V], chars []rune, depth int) bool {
	if depth == len(chars) {
		current.terminal = false
		var zero V
		current.value = zero
		return len(current.children) == 0
	}

	ch := chars[depth]
	child, ok := current.children[ch]
	if !ok {
		return false
	}

	if deleteRecursive(child, chars, depth+1) {
		delete(current.children, ch)
	}

	return len(current.children) == 0 && !current.terminal
}

func countEndpoints[V any](current *node[V]) int {
	count := 0
	if current.terminal {
		count = 1
	}
	for _, child := range current.children {
		count += countEndpoints(child)
	}
	return count
}

func sortedChildren[V any](current *node[V]) []rune {
	keys := make([]rune, 0, len(current.children))
	for ch := range current.children {
		keys = append(keys, ch)
	}
	sort.Slice(keys, func(i, j int) bool { return keys[i] < keys[j] })
	return keys
}
