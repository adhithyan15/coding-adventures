package treeset

import (
	"cmp"

	"github.com/adhithyan15/coding-adventures/code/packages/go/skip-list"
)

type TreeSet[T cmp.Ordered] struct {
	backend *skiplist.SkipList[T]
}

func New[T cmp.Ordered]() *TreeSet[T] {
	return &TreeSet[T]{backend: skiplist.New(func(a, b T) bool { return cmp.Compare(a, b) < 0 })}
}

func FromList[T cmp.Ordered](values []T) *TreeSet[T] {
	set := New[T]()
	for _, value := range values {
		set.Insert(value)
	}
	return set
}

func (s *TreeSet[T]) Clone() *TreeSet[T] {
	if s == nil {
		return New[T]()
	}
	return &TreeSet[T]{backend: s.backend.Clone()}
}

func (s *TreeSet[T]) Len() int {
	return s.backend.Len()
}

func (s *TreeSet[T]) IsEmpty() bool {
	return s.Len() == 0
}

func (s *TreeSet[T]) Contains(value T) bool {
	return s.backend.Contains(value)
}

func (s *TreeSet[T]) Min() (T, bool) {
	return s.backend.Min()
}

func (s *TreeSet[T]) Max() (T, bool) {
	return s.backend.Max()
}

func (s *TreeSet[T]) Insert(value T) bool {
	return s.backend.Insert(value)
}

func (s *TreeSet[T]) Remove(value T) bool {
	return s.backend.Delete(value)
}

func (s *TreeSet[T]) Delete(value T) bool {
	return s.Remove(value)
}

func (s *TreeSet[T]) Rank(value T) int {
	return s.backend.Rank(value)
}

func (s *TreeSet[T]) KthSmallest(k int) (T, bool) {
	return s.backend.KthSmallest(k)
}

func (s *TreeSet[T]) ToSlice() []T {
	return s.backend.ToSlice()
}

func (s *TreeSet[T]) Range(min, max T, inclusive bool) []T {
	return s.backend.Range(min, max, inclusive)
}

func (s *TreeSet[T]) Predecessor(value T) (T, bool) {
	var zero T
	values := s.ToSlice()
	var found bool
	for _, item := range values {
		if cmp.Compare(item, value) < 0 {
			zero = item
			found = true
			continue
		}
		break
	}
	return zero, found
}

func (s *TreeSet[T]) Successor(value T) (T, bool) {
	var zero T
	for _, item := range s.ToSlice() {
		if cmp.Compare(item, value) > 0 {
			return item, true
		}
	}
	return zero, false
}

func (s *TreeSet[T]) Union(other *TreeSet[T]) *TreeSet[T] {
	result := New[T]()
	for _, value := range s.ToSlice() {
		result.Insert(value)
	}
	for _, value := range other.ToSlice() {
		result.Insert(value)
	}
	return result
}

func (s *TreeSet[T]) Intersection(other *TreeSet[T]) *TreeSet[T] {
	result := New[T]()
	for _, value := range s.ToSlice() {
		if other.Contains(value) {
			result.Insert(value)
		}
	}
	return result
}

func (s *TreeSet[T]) Difference(other *TreeSet[T]) *TreeSet[T] {
	result := New[T]()
	for _, value := range s.ToSlice() {
		if !other.Contains(value) {
			result.Insert(value)
		}
	}
	return result
}

func (s *TreeSet[T]) SymmetricDifference(other *TreeSet[T]) *TreeSet[T] {
	return s.Difference(other).Union(other.Difference(s))
}

func (s *TreeSet[T]) IsSubset(other *TreeSet[T]) bool {
	for _, value := range s.ToSlice() {
		if !other.Contains(value) {
			return false
		}
	}
	return true
}

func (s *TreeSet[T]) IsSuperset(other *TreeSet[T]) bool {
	return other.IsSubset(s)
}

func (s *TreeSet[T]) IsDisjoint(other *TreeSet[T]) bool {
	for _, value := range s.ToSlice() {
		if other.Contains(value) {
			return false
		}
	}
	return true
}

func (s *TreeSet[T]) Equals(other *TreeSet[T]) bool {
	left := s.ToSlice()
	right := other.ToSlice()
	if len(left) != len(right) {
		return false
	}
	for i := range left {
		if left[i] != right[i] {
			return false
		}
	}
	return true
}
