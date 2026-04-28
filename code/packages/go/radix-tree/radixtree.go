package radixtree

import (
	"fmt"
	"sort"
)

type edge[V any] struct {
	label string
	child *node[V]
}

type node[V any] struct {
	children map[byte]*edge[V]
	terminal bool
	value    V
}

type RadixTree[V any] struct {
	root *node[V]
	size int
}

func New[V any]() *RadixTree[V] {
	return &RadixTree[V]{root: newNode[V]()}
}

func FromEntries[V any](entries map[string]V) *RadixTree[V] {
	tree := New[V]()
	for key, value := range entries {
		tree.Insert(key, value)
	}
	return tree
}

func (t *RadixTree[V]) Insert(key string, value V) {
	if insertRecursive(t.root, key, value) {
		t.size++
	}
}

func (t *RadixTree[V]) Search(key string) (V, bool) {
	current := t.root
	remaining := key
	for len(remaining) > 0 {
		edge, ok := current.children[remaining[0]]
		if !ok {
			var zero V
			return zero, false
		}
		common := commonPrefixLen(remaining, edge.label)
		if common < len(edge.label) {
			var zero V
			return zero, false
		}
		remaining = remaining[common:]
		current = edge.child
	}
	if !current.terminal {
		var zero V
		return zero, false
	}
	return current.value, true
}

func (t *RadixTree[V]) ContainsKey(key string) bool {
	return t.keyExists(key)
}

func (t *RadixTree[V]) Delete(key string) bool {
	deleted, _ := deleteRecursive(t.root, key)
	if deleted {
		t.size--
	}
	return deleted
}

func (t *RadixTree[V]) StartsWith(prefix string) bool {
	if prefix == "" {
		return t.size > 0
	}
	current := t.root
	remaining := prefix
	for len(remaining) > 0 {
		edge, ok := current.children[remaining[0]]
		if !ok {
			return false
		}
		common := commonPrefixLen(remaining, edge.label)
		if common == len(remaining) {
			return true
		}
		if common < len(edge.label) {
			return false
		}
		remaining = remaining[common:]
		current = edge.child
	}
	return current.terminal || len(current.children) > 0
}

func (t *RadixTree[V]) WordsWithPrefix(prefix string) []string {
	if prefix == "" {
		return t.Keys()
	}
	current := t.root
	remaining := prefix
	path := ""
	for len(remaining) > 0 {
		edge, ok := current.children[remaining[0]]
		if !ok {
			return []string{}
		}
		common := commonPrefixLen(remaining, edge.label)
		if common == len(remaining) {
			if common == len(edge.label) {
				path += edge.label
				current = edge.child
				remaining = ""
			} else {
				results := []string{}
				collectKeys(edge.child, path+edge.label, &results)
				return results
			}
		} else if common < len(edge.label) {
			return []string{}
		} else {
			path += edge.label
			remaining = remaining[common:]
			current = edge.child
		}
	}
	results := []string{}
	collectKeys(current, path, &results)
	return results
}

func (t *RadixTree[V]) LongestPrefixMatch(key string) (string, bool) {
	current := t.root
	remaining := key
	consumed := 0
	best := ""
	found := current.terminal
	for len(remaining) > 0 {
		edge, ok := current.children[remaining[0]]
		if !ok {
			break
		}
		common := commonPrefixLen(remaining, edge.label)
		if common < len(edge.label) {
			break
		}
		consumed += common
		remaining = remaining[common:]
		current = edge.child
		if current.terminal {
			best = key[:consumed]
			found = true
		}
	}
	return best, found
}

func (t *RadixTree[V]) Keys() []string {
	results := []string{}
	collectKeys(t.root, "", &results)
	return results
}

func (t *RadixTree[V]) ToMap() map[string]V {
	result := map[string]V{}
	collectValues(t.root, "", result)
	return result
}

func (t *RadixTree[V]) Len() int {
	return t.size
}

func (t *RadixTree[V]) IsEmpty() bool {
	return t.size == 0
}

func (t *RadixTree[V]) NodeCount() int {
	return countNodes(t.root)
}

func (t *RadixTree[V]) String() string {
	return fmt.Sprintf("RadixTree(%d keys: %v)", t.size, t.Keys())
}

func (t *RadixTree[V]) keyExists(key string) bool {
	current := t.root
	remaining := key
	for len(remaining) > 0 {
		edge, ok := current.children[remaining[0]]
		if !ok {
			return false
		}
		common := commonPrefixLen(remaining, edge.label)
		if common < len(edge.label) {
			return false
		}
		remaining = remaining[common:]
		current = edge.child
	}
	return current.terminal
}

func insertRecursive[V any](current *node[V], key string, value V) bool {
	if key == "" {
		added := !current.terminal
		current.terminal = true
		current.value = value
		return added
	}
	first := key[0]
	existing, ok := current.children[first]
	if !ok {
		current.children[first] = &edge[V]{label: key, child: leaf(value)}
		return true
	}
	common := commonPrefixLen(key, existing.label)
	if common == len(existing.label) {
		return insertRecursive(existing.child, key[common:], value)
	}
	commonLabel := existing.label[:common]
	labelRest := existing.label[common:]
	keyRest := key[common:]
	split := newNode[V]()
	split.children[labelRest[0]] = &edge[V]{label: labelRest, child: existing.child}
	if keyRest == "" {
		split.terminal = true
		split.value = value
	} else {
		split.children[keyRest[0]] = &edge[V]{label: keyRest, child: leaf(value)}
	}
	current.children[first] = &edge[V]{label: commonLabel, child: split}
	return true
}

func deleteRecursive[V any](current *node[V], key string) (bool, bool) {
	if key == "" {
		if !current.terminal {
			return false, false
		}
		current.terminal = false
		var zero V
		current.value = zero
		return true, len(current.children) == 1
	}
	first := key[0]
	existing, ok := current.children[first]
	if !ok {
		return false, false
	}
	common := commonPrefixLen(key, existing.label)
	if common < len(existing.label) {
		return false, false
	}
	deleted, childMergeable := deleteRecursive(existing.child, key[common:])
	if !deleted {
		return false, false
	}
	if childMergeable {
		for _, grandchild := range existing.child.children {
			current.children[first] = &edge[V]{label: existing.label + grandchild.label, child: grandchild.child}
			break
		}
	} else if !existing.child.terminal && len(existing.child.children) == 0 {
		delete(current.children, first)
	}
	return true, !current.terminal && len(current.children) == 1
}

func collectKeys[V any](current *node[V], path string, results *[]string) {
	if current.terminal {
		*results = append(*results, path)
	}
	for _, first := range sortedChildren(current) {
		edge := current.children[first]
		collectKeys(edge.child, path+edge.label, results)
	}
}

func collectValues[V any](current *node[V], path string, result map[string]V) {
	if current.terminal {
		result[path] = current.value
	}
	for _, first := range sortedChildren(current) {
		edge := current.children[first]
		collectValues(edge.child, path+edge.label, result)
	}
}

func countNodes[V any](current *node[V]) int {
	count := 1
	for _, edge := range current.children {
		count += countNodes(edge.child)
	}
	return count
}

func sortedChildren[V any](current *node[V]) []byte {
	keys := make([]byte, 0, len(current.children))
	for key := range current.children {
		keys = append(keys, key)
	}
	sort.Slice(keys, func(i, j int) bool { return keys[i] < keys[j] })
	return keys
}

func newNode[V any]() *node[V] {
	return &node[V]{children: map[byte]*edge[V]{}}
}

func leaf[V any](value V) *node[V] {
	n := newNode[V]()
	n.terminal = true
	n.value = value
	return n
}

func commonPrefixLen(left string, right string) int {
	index := 0
	for index < len(left) && index < len(right) && left[index] == right[index] {
		index++
	}
	return index
}
