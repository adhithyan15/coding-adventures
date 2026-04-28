package bloomfilter

import (
	"errors"
	"strings"
	"testing"
)

func TestDefaultStartsEmpty(t *testing.T) {
	filter := Default()
	if filter.BitsSet() != 0 || filter.FillRatio() != 0 || filter.EstimatedFalsePositiveRate() != 0 {
		t.Fatalf("default filter should start empty")
	}
	if filter.IsOverCapacity() {
		t.Fatalf("new filter should not be over capacity")
	}
	if filter.Contains("anything") {
		t.Fatalf("empty filter should not contain a value")
	}
}

func TestNoFalseNegatives(t *testing.T) {
	filter := MustNew(1000, 0.01)
	for i := 0; i < 250; i++ {
		filter.Add("item-" + string(rune(i)))
	}
	for i := 0; i < 250; i++ {
		if !filter.Contains("item-" + string(rune(i))) {
			t.Fatalf("false negative for %d", i)
		}
	}
	if filter.BitsSet() == 0 {
		t.Fatalf("adding values should set bits")
	}
}

func TestFromParams(t *testing.T) {
	filter := MustFromParams(10_000, 7)
	if filter.BitCount() != 10_000 || filter.HashCount() != 7 || filter.SizeBytes() != 1250 {
		t.Fatalf("explicit params not applied")
	}
	filter.Add("hello")
	if !filter.Contains("hello") {
		t.Fatalf("explicit-params filter missed inserted value")
	}
	if filter.IsOverCapacity() {
		t.Fatalf("explicit params should not track capacity")
	}
}

func TestDuplicateAddsDoNotDoubleCountBits(t *testing.T) {
	filter := Default()
	filter.Add("dup")
	afterFirst := filter.BitsSet()
	filter.Add("dup")
	if filter.BitsSet() != afterFirst {
		t.Fatalf("duplicate add changed set bit count")
	}
}

func TestSizingHelpers(t *testing.T) {
	m := OptimalM(1_000_000, 0.01)
	k := OptimalK(m, 1_000_000)
	if m <= 9_000_000 {
		t.Fatalf("m too small: %d", m)
	}
	if k != 7 {
		t.Fatalf("k = %d", k)
	}
	if CapacityForMemory(1_000_000, 0.01) <= 0 {
		t.Fatalf("capacity should be positive")
	}
}

func TestOverCapacityAndRendering(t *testing.T) {
	filter := MustNew(3, 0.01)
	filter.Add("a")
	filter.Add("b")
	filter.Add("c")
	if filter.IsOverCapacity() {
		t.Fatalf("at capacity should not be over")
	}
	filter.Add("d")
	if !filter.IsOverCapacity() || filter.EstimatedFalsePositiveRate() <= 0 {
		t.Fatalf("over-capacity stats not updated")
	}
	if !strings.Contains(filter.String(), "BloomFilter") {
		t.Fatalf("String() should identify the filter")
	}
}

func TestVariousElementTypes(t *testing.T) {
	filter := MustNew(100, 0.01)
	values := []any{42, 3.14, true, nil, []int{1, 2}, "cafe\u0301"}
	for _, value := range values {
		filter.Add(value)
		if !filter.Contains(value) {
			t.Fatalf("false negative for %#v", value)
		}
	}
}

func TestInvalidParameters(t *testing.T) {
	if _, err := New(0, 0.01); !errors.Is(err, ErrInvalidExpectedItems) {
		t.Fatalf("expected invalid expectedItems error, got %v", err)
	}
	if _, err := New(1, 0); !errors.Is(err, ErrInvalidFalsePositiveRate) {
		t.Fatalf("expected invalid fpr error, got %v", err)
	}
	if _, err := FromParams(0, 1); !errors.Is(err, ErrInvalidBitCount) {
		t.Fatalf("expected invalid bitCount error, got %v", err)
	}
	if _, err := FromParams(1, 0); !errors.Is(err, ErrInvalidHashCount) {
		t.Fatalf("expected invalid hashCount error, got %v", err)
	}
}
