package hashmap

import (
	"bytes"
	"sort"
)

type Entry[V any] struct {
	Key   []byte
	Value V
}

type HashMap[V any] struct {
	entries map[string]Entry[V]
}

func New[V any]() *HashMap[V] {
	return &HashMap[V]{entries: make(map[string]Entry[V])}
}

func (m *HashMap[V]) Clone() *HashMap[V] {
	if m == nil {
		return New[V]()
	}
	clone := New[V]()
	for _, entry := range m.entries {
		clone.entries[string(entry.Key)] = Entry[V]{
			Key:   cloneBytes(entry.Key),
			Value: entry.Value,
		}
	}
	return clone
}

func (m *HashMap[V]) Set(key []byte, value V) {
	if m.entries == nil {
		m.entries = make(map[string]Entry[V])
	}
	keyCopy := cloneBytes(key)
	m.entries[string(keyCopy)] = Entry[V]{Key: keyCopy, Value: value}
}

func (m *HashMap[V]) Get(key []byte) (V, bool) {
	if m == nil || m.entries == nil {
		var zero V
		return zero, false
	}
	entry, ok := m.entries[string(key)]
	if !ok {
		var zero V
		return zero, false
	}
	return entry.Value, true
}

func (m *HashMap[V]) Delete(key []byte) bool {
	if m == nil || m.entries == nil {
		return false
	}
	keyStr := string(key)
	if _, ok := m.entries[keyStr]; !ok {
		return false
	}
	delete(m.entries, keyStr)
	return true
}

func (m *HashMap[V]) Has(key []byte) bool {
	if m == nil || m.entries == nil {
		return false
	}
	_, ok := m.entries[string(key)]
	return ok
}

func (m *HashMap[V]) Size() int {
	if m == nil || m.entries == nil {
		return 0
	}
	return len(m.entries)
}

func (m *HashMap[V]) Keys() [][]byte {
	entries := m.Entries()
	keys := make([][]byte, 0, len(entries))
	for _, entry := range entries {
		keys = append(keys, cloneBytes(entry.Key))
	}
	return keys
}

func (m *HashMap[V]) Entries() []Entry[V] {
	if m == nil || m.entries == nil {
		return nil
	}
	entries := make([]Entry[V], 0, len(m.entries))
	for _, entry := range m.entries {
		entries = append(entries, Entry[V]{
			Key:   cloneBytes(entry.Key),
			Value: entry.Value,
		})
	}
	sort.Slice(entries, func(i, j int) bool {
		return bytes.Compare(entries[i].Key, entries[j].Key) < 0
	})
	return entries
}

func (m *HashMap[V]) Clear() {
	m.entries = make(map[string]Entry[V])
}

func cloneBytes(data []byte) []byte {
	if data == nil {
		return nil
	}
	return append([]byte(nil), data...)
}
