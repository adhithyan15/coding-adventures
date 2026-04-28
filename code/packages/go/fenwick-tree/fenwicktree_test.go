package fenwicktree

import (
	"errors"
	"math"
	"strings"
	"testing"
)

func assertClose(t *testing.T, actual float64, expected float64) {
	t.Helper()
	if math.Abs(actual-expected) > 1e-9 {
		t.Fatalf("expected %v, got %v", expected, actual)
	}
}

func TestNewAndFromSlice(t *testing.T) {
	tree := FromSlice([]float64{3, 2, 1, 7, 4})
	if tree.Len() != 5 || tree.IsEmpty() {
		t.Fatalf("tree size state is wrong")
	}
	expected := []float64{3, 5, 6, 13, 17}
	for i, want := range expected {
		got, err := tree.PrefixSum(i + 1)
		if err != nil {
			t.Fatalf("PrefixSum returned error: %v", err)
		}
		assertClose(t, got, want)
	}

	empty := MustNew(0)
	if !empty.IsEmpty() {
		t.Fatalf("empty tree should report empty")
	}
	if _, err := New(-1); !errors.Is(err, ErrInvalidSize) {
		t.Fatalf("expected invalid size error, got %v", err)
	}
}

func TestPrefixRangePointAndUpdate(t *testing.T) {
	tree := FromValues(3, 2, 1, 7, 4)
	sum, err := tree.PrefixSum(0)
	if err != nil {
		t.Fatalf("PrefixSum(0) errored: %v", err)
	}
	assertClose(t, sum, 0)

	sum, err = tree.RangeSum(2, 4)
	if err != nil {
		t.Fatalf("RangeSum returned error: %v", err)
	}
	assertClose(t, sum, 10)
	sum, err = tree.RangeSum(1, 5)
	if err != nil {
		t.Fatalf("full RangeSum returned error: %v", err)
	}
	assertClose(t, sum, 17)

	point, err := tree.PointQuery(4)
	if err != nil {
		t.Fatalf("PointQuery returned error: %v", err)
	}
	assertClose(t, point, 7)

	if err := tree.Update(3, 5); err != nil {
		t.Fatalf("Update returned error: %v", err)
	}
	point, _ = tree.PointQuery(3)
	assertClose(t, point, 6)
	sum, _ = tree.PrefixSum(3)
	assertClose(t, sum, 11)
}

func TestFindKth(t *testing.T) {
	tree := FromValues(1, 2, 3, 4, 5)
	cases := map[float64]int{1: 1, 2: 2, 3: 2, 4: 3, 10: 4}
	for target, want := range cases {
		got, err := tree.FindKth(target)
		if err != nil {
			t.Fatalf("FindKth(%v) errored: %v", target, err)
		}
		if got != want {
			t.Fatalf("FindKth(%v) = %d", target, got)
		}
	}
}

func TestFindKthErrors(t *testing.T) {
	if _, err := MustNew(0).FindKth(1); !errors.Is(err, ErrEmptyTree) {
		t.Fatalf("expected empty tree error, got %v", err)
	}
	tree := FromValues(1, 2, 3)
	if _, err := tree.FindKth(0); !errors.Is(err, ErrNonPositiveTarget) {
		t.Fatalf("expected non-positive target error, got %v", err)
	}
	if _, err := tree.FindKth(100); !errors.Is(err, ErrTargetExceedsSum) {
		t.Fatalf("expected target-exceeds-total error, got %v", err)
	}
}

func TestInvalidIndicesAndRanges(t *testing.T) {
	tree := FromValues(1, 2, 3)
	if _, err := tree.PrefixSum(4); !errors.Is(err, ErrIndexOutOfRange) {
		t.Fatalf("expected prefix range error, got %v", err)
	}
	if err := tree.Update(0, 1); !errors.Is(err, ErrIndexOutOfRange) {
		t.Fatalf("expected update range error, got %v", err)
	}
	if _, err := tree.RangeSum(3, 1); !errors.Is(err, ErrInvalidRange) {
		t.Fatalf("expected invalid range error, got %v", err)
	}
	if _, err := tree.RangeSum(0, 3); !errors.Is(err, ErrIndexOutOfRange) {
		t.Fatalf("expected range index error, got %v", err)
	}
	if _, err := tree.RangeSum(1, 4); !errors.Is(err, ErrIndexOutOfRange) {
		t.Fatalf("expected right range index error, got %v", err)
	}
	if _, err := tree.PointQuery(0); !errors.Is(err, ErrIndexOutOfRange) {
		t.Fatalf("expected point query index error, got %v", err)
	}
}

func TestMustNewPanicsOnInvalidSize(t *testing.T) {
	defer func() {
		if recover() == nil {
			t.Fatalf("MustNew should panic for invalid size")
		}
	}()
	_ = MustNew(-1)
}

func TestBruteForceAndRendering(t *testing.T) {
	values := []float64{5, -2, 7, 1.5, 4.5}
	tree := FromSlice(values)
	for index := 1; index <= len(values); index++ {
		prefix := 0.0
		for i := 0; i < index; i++ {
			prefix += values[i]
		}
		got, _ := tree.PrefixSum(index)
		assertClose(t, got, prefix)
	}
	if got := tree.BitArray(); len(got) != len(values) {
		t.Fatalf("BitArray length = %d", len(got))
	}
	if !strings.Contains(tree.String(), "FenwickTree") {
		t.Fatalf("String should identify the tree")
	}
}
