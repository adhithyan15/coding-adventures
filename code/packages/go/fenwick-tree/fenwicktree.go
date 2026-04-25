package fenwicktree

import (
	"errors"
	"fmt"
)

var (
	ErrInvalidSize       = errors.New("size must be a non-negative integer")
	ErrIndexOutOfRange   = errors.New("index out of range")
	ErrInvalidRange      = errors.New("left must be <= right")
	ErrEmptyTree         = errors.New("find kth called on empty tree")
	ErrNonPositiveTarget = errors.New("target must be positive")
	ErrTargetExceedsSum  = errors.New("target exceeds total sum")
)

type FenwickTree struct {
	n   int
	bit []float64
}

func New(n int) (*FenwickTree, error) {
	if n < 0 {
		return nil, fmt.Errorf("%w: %d", ErrInvalidSize, n)
	}
	return &FenwickTree{n: n, bit: make([]float64, n+1)}, nil
}

func MustNew(n int) *FenwickTree {
	tree, err := New(n)
	if err != nil {
		panic(err)
	}
	return tree
}

func FromSlice(values []float64) *FenwickTree {
	tree := MustNew(len(values))
	for index := 1; index <= tree.n; index++ {
		tree.bit[index] += values[index-1]
		parent := index + lowbit(index)
		if parent <= tree.n {
			tree.bit[parent] += tree.bit[index]
		}
	}
	return tree
}

func FromValues(values ...float64) *FenwickTree {
	return FromSlice(values)
}

func (t *FenwickTree) Update(index int, delta float64) error {
	if err := t.checkIndex(index); err != nil {
		return err
	}

	for current := index; current <= t.n; current += lowbit(current) {
		t.bit[current] += delta
	}
	return nil
}

func (t *FenwickTree) PrefixSum(index int) (float64, error) {
	if index < 0 || index > t.n {
		return 0, fmt.Errorf("%w: prefix index %d out of range [0, %d]", ErrIndexOutOfRange, index, t.n)
	}

	total := 0.0
	for current := index; current > 0; current -= lowbit(current) {
		total += t.bit[current]
	}
	return total, nil
}

func (t *FenwickTree) RangeSum(left int, right int) (float64, error) {
	if left > right {
		return 0, fmt.Errorf("%w: left=%d right=%d", ErrInvalidRange, left, right)
	}
	if err := t.checkIndex(left); err != nil {
		return 0, err
	}
	if err := t.checkIndex(right); err != nil {
		return 0, err
	}
	if left == 1 {
		return t.PrefixSum(right)
	}

	rightSum, err := t.PrefixSum(right)
	if err != nil {
		return 0, err
	}
	beforeLeft, err := t.PrefixSum(left - 1)
	if err != nil {
		return 0, err
	}
	return rightSum - beforeLeft, nil
}

func (t *FenwickTree) PointQuery(index int) (float64, error) {
	if err := t.checkIndex(index); err != nil {
		return 0, err
	}
	return t.RangeSum(index, index)
}

func (t *FenwickTree) FindKth(target float64) (int, error) {
	if t.n == 0 {
		return 0, ErrEmptyTree
	}
	if target <= 0 {
		return 0, fmt.Errorf("%w: %v", ErrNonPositiveTarget, target)
	}

	total, err := t.PrefixSum(t.n)
	if err != nil {
		return 0, err
	}
	if target > total {
		return 0, fmt.Errorf("%w: target=%v total=%v", ErrTargetExceedsSum, target, total)
	}

	index := 0
	step := highestPowerOfTwoAtMost(t.n)
	for step > 0 {
		next := index + step
		if next <= t.n && t.bit[next] < target {
			index = next
			target -= t.bit[index]
		}
		step >>= 1
	}
	return index + 1, nil
}

func (t *FenwickTree) Len() int {
	return t.n
}

func (t *FenwickTree) IsEmpty() bool {
	return t.n == 0
}

func (t *FenwickTree) BitArray() []float64 {
	out := make([]float64, t.n)
	copy(out, t.bit[1:])
	return out
}

func (t *FenwickTree) String() string {
	return fmt.Sprintf("FenwickTree(n=%d, bit=%v)", t.n, t.BitArray())
}

func (t *FenwickTree) checkIndex(index int) error {
	if index >= 1 && index <= t.n {
		return nil
	}
	return fmt.Errorf("%w: index %d out of range [1, %d]", ErrIndexOutOfRange, index, t.n)
}

func lowbit(index int) int {
	return index & -index
}

func highestPowerOfTwoAtMost(n int) int {
	if n == 0 {
		return 0
	}
	power := 1
	for power<<1 <= n {
		power <<= 1
	}
	return power
}
