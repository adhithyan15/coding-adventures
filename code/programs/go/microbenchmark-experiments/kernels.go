package main

import (
	"fmt"
	"math/rand"
	"unsafe"

	mb "github.com/adhithyan15/coding-adventures/code/packages/go/microbenchmark"
)

type node struct {
	value int64
	next  int
}
type workCounts struct {
	Allocations int `json:"allocations"`
	Copied      int `json:"copied_elements"`
}
type comparison struct {
	ID, Workload, BaselineLabel, CandidateLabel    string
	RequestedBytes, ActualBytes, Visits, NodeBytes int
	BaselineCounts, CandidateCounts                *workCounts
	Expected                                       int64
	Baseline, Candidate                            mb.Batch
}

// All arrays are constructed here, with at most 2^22 values in [0,250].
// A batch has at most 2^20 operations, so even the largest checksum is below
// 2^50. These bounds make hot-loop addition safe without hiding overflow by wrap.
func values(n int) ([]int64, int64) {
	a := make([]int64, n)
	var sum int64
	for i := range a {
		a[i] = int64((17*i + 3) % 251)
		sum += a[i]
	}
	return a, sum
}
func validBatch(n int) error {
	if n < 0 || n > 1048576 {
		return fmt.Errorf("batch out of experiment bounds")
	}
	return nil
}

//go:noinline
func indexed(a []int64, n int) (int64, error) {
	if err := validBatch(n); err != nil {
		return 0, err
	}
	var sum int64
	for k := 0; k < n; k++ {
		for i := 0; i < len(a); i++ {
			sum += a[i]
		}
	}
	return sum, nil
}

//go:noinline
func ranged(a []int64, n int) (int64, error) {
	if err := validBatch(n); err != nil {
		return 0, err
	}
	var sum int64
	for k := 0; k < n; k++ {
		for _, v := range a {
			sum += v
		}
	}
	return sum, nil
}

// Both access-order variants call this identical function. Each next index
// depends on the preceding load, while the number of logical visits stays n.
//
//go:noinline
func traverse(a []node, n int) (int64, error) {
	if err := validBatch(n); err != nil {
		return 0, err
	}
	var sum int64
	for k := 0; k < n; k++ {
		index := 0
		for visit := 0; visit < len(a); visit++ {
			sum += a[index].value
			index = a[index].next
		}
	}
	return sum, nil
}

func cycleValid(a []node, expected int64) error {
	if len(a) == 0 {
		return fmt.Errorf("empty cycle")
	}
	seen := make([]bool, len(a))
	index := 0
	var sum int64
	for range a {
		if index < 0 || index >= len(a) || seen[index] {
			return fmt.Errorf("invalid successor cycle")
		}
		seen[index] = true
		sum += a[index].value
		index = a[index].next
	}
	if index != 0 || sum != expected {
		return fmt.Errorf("incomplete cycle or wrong values")
	}
	return nil
}

// growth performs the allocations and copies, rather than predicting their
// cost. Counts come from the same policy but are collected outside timing.
func growth(length int, doubling bool) int64 {
	var a []int64
	for i := 0; i < length; i++ {
		if len(a) == cap(a) {
			capacity := cap(a) + 32
			if doubling {
				capacity = cap(a) * 2
				if capacity == 0 {
					capacity = 1
				}
			}
			next := make([]int64, len(a), capacity)
			copy(next, a)
			a = next
		}
		a = append(a, int64((17*i+3)%251))
	}
	var sum int64
	for _, v := range a {
		sum += v
	}
	return sum
}
func growthCounts(length int, doubling bool) workCounts {
	var counts workCounts
	for capacity := 0; capacity < length; {
		counts.Copied += capacity
		counts.Allocations++
		if doubling {
			if capacity == 0 {
				capacity = 1
			} else {
				capacity *= 2
			}
		} else {
			capacity += 32
		}
	}
	return counts
}
func growBatch(length, n int, doubling bool) (int64, error) {
	if err := validBatch(n); err != nil {
		return 0, err
	}
	var total int64
	for k := 0; k < n; k++ {
		total += growth(length, doubling)
	}
	return total, nil
}

func prepare(kind string, size int, seed int64) (comparison, error) {
	c := comparison{ID: fmt.Sprintf("%s-%d-seed-%d", kind, size, seed)}
	switch kind {
	case "indexed":
		if size < 16 || size > 32<<20 {
			return c, fmt.Errorf("memory size out of bounds")
		}
		a, sum := values(size / 8)
		c.Workload = "indexed-versus-range-sum"
		c.BaselineLabel = "indexed"
		c.CandidateLabel = "range"
		c.RequestedBytes = size
		c.ActualBytes = len(a) * 8
		c.Visits = len(a)
		c.Expected = sum
		c.Baseline = func(n int) (int64, error) { return indexed(a, n) }
		c.Candidate = func(n int) (int64, error) { return ranged(a, n) }
	case "links":
		if size < 16 || size > 32<<20 {
			return c, fmt.Errorf("memory size out of bounds")
		}
		nodeBytes := int(unsafe.Sizeof(node{}))
		n := size / nodeBytes
		v, sum := values(n)
		ordered, permuted := make([]node, n), make([]node, n)
		for i := range ordered {
			ordered[i] = node{v[i], (i + 1) % n}
			permuted[i].value = v[i]
		}
		order := rand.New(rand.NewSource(seed)).Perm(n)
		for i, index := range order {
			permuted[index].next = order[(i+1)%n]
		}
		if err := cycleValid(ordered, sum); err != nil {
			return c, err
		}
		if err := cycleValid(permuted, sum); err != nil {
			return c, err
		}
		c.Workload = "dependent-link-order"
		c.BaselineLabel = "sequential successors"
		c.CandidateLabel = "permuted successors"
		c.RequestedBytes = size
		c.ActualBytes = n * nodeBytes
		c.NodeBytes = nodeBytes
		c.Visits = n
		c.Expected = sum
		c.Baseline = func(n int) (int64, error) { return traverse(ordered, n) }
		c.Candidate = func(n int) (int64, error) { return traverse(permuted, n) }
	case "growth":
		if size < 1 || size > 32768 {
			return c, fmt.Errorf("growth length out of bounds")
		}
		_, sum := values(size)
		a, ac := growth(size, false), growthCounts(size, false)
		b, bc := growth(size, true), growthCounts(size, true)
		if a != sum || b != sum {
			return c, fmt.Errorf("growth checksum mismatch")
		}
		c.Workload = "fixed-versus-doubling-growth"
		c.BaselineLabel = "capacity +32"
		c.CandidateLabel = "capacity doubles"
		c.Visits = size
		c.Expected = sum
		c.BaselineCounts = &ac
		c.CandidateCounts = &bc
		c.Baseline = func(n int) (int64, error) { return growBatch(size, n, false) }
		c.Candidate = func(n int) (int64, error) { return growBatch(size, n, true) }
	default:
		return c, fmt.Errorf("unknown comparison kind")
	}
	return c, nil
}
