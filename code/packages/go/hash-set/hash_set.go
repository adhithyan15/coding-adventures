package hashset

import (
	"bytes"
	"sort"
)

type HashSet struct {
	items map[string][]byte
}

func New() *HashSet {
	return &HashSet{items: make(map[string][]byte)}
}

func (s *HashSet) Clone() *HashSet {
	if s == nil {
		return New()
	}
	clone := New()
	for _, item := range s.items {
		clone.items[string(item)] = cloneBytes(item)
	}
	return clone
}

func (s *HashSet) Add(item []byte) bool {
	if s.items == nil {
		s.items = make(map[string][]byte)
	}
	key := string(item)
	_, exists := s.items[key]
	if !exists {
		s.items[key] = cloneBytes(item)
	}
	return !exists
}

func (s *HashSet) Remove(item []byte) bool {
	if s == nil || s.items == nil {
		return false
	}
	key := string(item)
	if _, exists := s.items[key]; !exists {
		return false
	}
	delete(s.items, key)
	return true
}

func (s *HashSet) Contains(item []byte) bool {
	if s == nil || s.items == nil {
		return false
	}
	_, ok := s.items[string(item)]
	return ok
}

func (s *HashSet) Size() int {
	if s == nil || s.items == nil {
		return 0
	}
	return len(s.items)
}

func (s *HashSet) IsEmpty() bool {
	return s.Size() == 0
}

func (s *HashSet) ToSlice() [][]byte {
	if s == nil || s.items == nil {
		return nil
	}
	values := make([][]byte, 0, len(s.items))
	for _, item := range s.items {
		values = append(values, cloneBytes(item))
	}
	sort.Slice(values, func(i, j int) bool {
		return bytes.Compare(values[i], values[j]) < 0
	})
	return values
}

func (s *HashSet) Union(other *HashSet) *HashSet {
	result := New()
	for _, item := range s.ToSlice() {
		result.Add(item)
	}
	for _, item := range other.ToSlice() {
		result.Add(item)
	}
	return result
}

func (s *HashSet) Intersection(other *HashSet) *HashSet {
	result := New()
	for _, item := range s.ToSlice() {
		if other.Contains(item) {
			result.Add(item)
		}
	}
	return result
}

func (s *HashSet) Difference(other *HashSet) *HashSet {
	result := New()
	for _, item := range s.ToSlice() {
		if !other.Contains(item) {
			result.Add(item)
		}
	}
	return result
}

func (s *HashSet) SymmetricDifference(other *HashSet) *HashSet {
	left := s.Difference(other)
	right := other.Difference(s)
	return left.Union(right)
}

func (s *HashSet) IsSubset(other *HashSet) bool {
	for _, item := range s.ToSlice() {
		if !other.Contains(item) {
			return false
		}
	}
	return true
}

func (s *HashSet) IsSuperset(other *HashSet) bool {
	return other.IsSubset(s)
}

func (s *HashSet) IsDisjoint(other *HashSet) bool {
	for _, item := range s.ToSlice() {
		if other.Contains(item) {
			return false
		}
	}
	return true
}

func cloneBytes(data []byte) []byte {
	if data == nil {
		return nil
	}
	return append([]byte(nil), data...)
}
