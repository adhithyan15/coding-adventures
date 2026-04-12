package skiplist

import (
	"math/rand"
)

const (
	defaultMaxLevel = 16
	defaultSeed     = 1
)

type node[T any] struct {
	value   T
	forward []*node[T]
}

type SkipList[T any] struct {
	head   *node[T]
	level  int
	length int
	less   func(a, b T) bool
	rng    *rand.Rand
}

func New[T any](less func(a, b T) bool) *SkipList[T] {
	head := &node[T]{forward: make([]*node[T], defaultMaxLevel)}
	return &SkipList[T]{
		head:  head,
		level: 1,
		less:  less,
		rng:   rand.New(rand.NewSource(defaultSeed)),
	}
}

func (s *SkipList[T]) Clone() *SkipList[T] {
	if s == nil {
		return nil
	}
	clone := New(s.less)
	for _, value := range s.ToSlice() {
		clone.Insert(value)
	}
	return clone
}

func (s *SkipList[T]) Len() int {
	if s == nil {
		return 0
	}
	return s.length
}

func (s *SkipList[T]) Size() int {
	return s.Len()
}

func (s *SkipList[T]) IsEmpty() bool {
	return s.Len() == 0
}

func (s *SkipList[T]) Contains(value T) bool {
	_, found := s.find(value)
	return found
}

func (s *SkipList[T]) Insert(value T) bool {
	update := make([]*node[T], defaultMaxLevel)
	current := s.head

	for level := s.level - 1; level >= 0; level-- {
		for current.forward[level] != nil && s.less(current.forward[level].value, value) {
			current = current.forward[level]
		}
		update[level] = current
	}

	next := current.forward[0]
	if next != nil && s.equal(next.value, value) {
		next.value = value
		return false
	}

	newLevel := s.randomLevel()
	if newLevel > s.level {
		for i := s.level; i < newLevel; i++ {
			update[i] = s.head
		}
		s.level = newLevel
	}

	n := &node[T]{value: value, forward: make([]*node[T], newLevel)}
	for i := 0; i < newLevel; i++ {
		n.forward[i] = update[i].forward[i]
		update[i].forward[i] = n
	}
	s.length++
	return true
}

func (s *SkipList[T]) Delete(value T) bool {
	update := make([]*node[T], defaultMaxLevel)
	current := s.head

	for level := s.level - 1; level >= 0; level-- {
		for current.forward[level] != nil && s.less(current.forward[level].value, value) {
			current = current.forward[level]
		}
		update[level] = current
	}

	target := current.forward[0]
	if target == nil || !s.equal(target.value, value) {
		return false
	}

	for i := 0; i < s.level; i++ {
		if update[i].forward[i] != target {
			break
		}
		update[i].forward[i] = target.forward[i]
	}

	for s.level > 1 && s.head.forward[s.level-1] == nil {
		s.level--
	}
	s.length--
	return true
}

func (s *SkipList[T]) Rank(value T) int {
	rank := 0
	for _, item := range s.ToSlice() {
		if s.less(item, value) {
			rank++
			continue
		}
		break
	}
	return rank
}

func (s *SkipList[T]) KthSmallest(k int) (T, bool) {
	var zero T
	if k <= 0 || k > s.Len() {
		return zero, false
	}
	items := s.ToSlice()
	return items[k-1], true
}

func (s *SkipList[T]) Min() (T, bool) {
	var zero T
	if s == nil || s.head.forward[0] == nil {
		return zero, false
	}
	return s.head.forward[0].value, true
}

func (s *SkipList[T]) Max() (T, bool) {
	var zero T
	if s == nil || s.head.forward[0] == nil {
		return zero, false
	}
	current := s.head.forward[0]
	for current.forward[0] != nil {
		current = current.forward[0]
	}
	return current.value, true
}

func (s *SkipList[T]) ToSlice() []T {
	if s == nil {
		return nil
	}
	values := make([]T, 0, s.length)
	for current := s.head.forward[0]; current != nil; current = current.forward[0] {
		values = append(values, current.value)
	}
	return values
}

func (s *SkipList[T]) Range(min, max T, inclusive bool) []T {
	values := s.ToSlice()
	result := make([]T, 0)
	for _, value := range values {
		if inclusive {
			if !s.less(value, min) && !s.less(max, value) {
				result = append(result, value)
			}
			continue
		}
		if s.less(min, value) && s.less(value, max) {
			result = append(result, value)
		}
	}
	return result
}

func (s *SkipList[T]) FirstGreaterOrEqual(value T) (T, bool) {
	var zero T
	for _, item := range s.ToSlice() {
		if !s.less(item, value) {
			return item, true
		}
	}
	return zero, false
}

func (s *SkipList[T]) Equal(a, b T) bool {
	return s.equal(a, b)
}

func (s *SkipList[T]) equal(a, b T) bool {
	return !s.less(a, b) && !s.less(b, a)
}

func (s *SkipList[T]) find(value T) (*node[T], bool) {
	current := s.head
	for level := s.level - 1; level >= 0; level-- {
		for current.forward[level] != nil && s.less(current.forward[level].value, value) {
			current = current.forward[level]
		}
	}
	current = current.forward[0]
	if current != nil && s.equal(current.value, value) {
		return current, true
	}
	return nil, false
}

func (s *SkipList[T]) randomLevel() int {
	level := 1
	for level < defaultMaxLevel && s.rng.Intn(2) == 0 {
		level++
	}
	return level
}
