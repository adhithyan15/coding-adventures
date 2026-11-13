package recmerge

// Path arithmetic and the index the merge asks its overlap questions of.
//
// # The two relations
//
// Every rule of KILN04 section 7 is phrased with two relations on
// repository-relative paths:
//
//	anc(p, q)     p == q, or p is a PATH-prefix ancestor of q
//	overlap(p, q) anc(p, q) or anc(q, p)
//
// "Path-prefix" is the whole point.  The string "a/b" is a byte prefix of
// "a/bc", but "a/bc" is NOT inside the directory "a/b", so a naive
// strings.HasPrefix would wrongly report an overlap and, worse, the mirrored
// mistake elsewhere would hide a real one.  A path is inside another only
// when the next byte after the prefix is a '/':
//
//	anc("a/b", "a/b")     true      equal
//	anc("a/b", "a/b/c")   true      a real descendant
//	anc("a/b", "a/bc")    FALSE     a sibling that shares a spelling
//	anc("a/b", "a")       false     a is the PARENT
//
// The one special path is ".", which only appears as the argument of a
// "list:." manifest entry and means the repository root: it is an ancestor of
// every path and a descendant of none (but itself).
//
// # The index
//
// A pathIndex maps a path to the actions that hold it and answers
//
//	ancestors(q)    every indexed p with anc(p, q)         walk q's prefixes
//	descendants(q)  every indexed p with anc(q, p)         binary search
//	overlapping(q)  both
//
// in O(depth + hits) instead of comparing q with every path.  The fuzz
// target in this package checks the index against a brute-force oracle that
// uses neither the index nor this file's helpers.

import (
	"math"
	"sort"
	"strings"
)

// anc is the relation above.
func anc(p, q string) bool {
	return p == q || p == "." || (len(q) > len(p) && q[len(p)] == '/' && q[:len(p)] == p)
}

// overlap is anc in either direction.
func overlap(p, q string) bool { return anc(p, q) || anc(q, p) }

// meter is the global work budget (KILN04 7.1, L-3).  Every node an overlap
// walk visits, and every list element a rule looks at, spends from one shared
// counter.  Once it is exhausted every walk stops at its next step and the
// merge reports run-void "size", so a crafted record (1,500 nested probes
// over 60,000 writes) costs a bounded amount of time.  A nil meter is
// unlimited, for tests of the index alone.
type meter struct{ spent, max int64 }

func (m *meter) spend(n int64) bool {
	if m == nil {
		return true
	}
	// Saturate: a wrapped counter would read as "almost nothing spent".
	if n < 0 || m.spent > math.MaxInt64-n {
		m.spent = math.MaxInt64
	} else {
		m.spent += n
	}
	return m.spent <= m.max
}

func (m *meter) over() bool { return m != nil && m.spent > m.max }

// pathIndex maps a path to the ids of the actions that hold it.
type pathIndex struct {
	owners map[string][]int
	sorted []string
	built  bool
	mt     *meter
}

func newPathIndex(mt *meter) *pathIndex { return &pathIndex{owners: map[string][]int{}, mt: mt} }

// add records that action id holds p.  Ids must be added action by action (all
// of one id's paths before the next id's), which keeps each owner list sorted
// and duplicate-free with one comparison.
func (x *pathIndex) add(p string, id int) {
	o := x.owners[p]
	if n := len(o); n > 0 && o[n-1] == id {
		return
	}
	x.owners[p] = append(o, id)
	x.built = false
}

func (x *pathIndex) freeze() {
	if x.built {
		return
	}
	x.sorted = x.sorted[:0]
	for p := range x.owners {
		x.sorted = append(x.sorted, p)
	}
	sort.Strings(x.sorted)
	x.built = true
}

// walk is the callback of every query: it receives an indexed path and its
// owners, and returns false to stop (a caller that only needs the first few
// hits asks for no more).
type walk func(p string, owners []int) bool

// ancestors visits every indexed p with anc(p, q), nearest-to-root first.
func (x *pathIndex) ancestors(q string, fn walk) bool {
	for i := 0; i < len(q); i++ {
		if q[i] != '/' {
			continue
		}
		// A probe hashes the prefix, so its cost is its LENGTH, not 1: a long
		// path probed at every level is quadratic in its length.
		if !x.mt.spend(int64(1 + i/32)) {
			return false
		}
		if o, ok := x.owners[q[:i]]; ok && (!x.mt.spend(int64(len(o))) || !fn(q[:i], o)) {
			return false
		}
	}
	if !x.mt.spend(int64(1 + len(q)/32)) {
		return false
	}
	if o, ok := x.owners[q]; ok && (!x.mt.spend(int64(len(o))) || !fn(q, o)) {
		return false
	}
	return true
}

// descendants visits, in bytewise order, every indexed p strictly beneath q
// (and q itself unless strict).  For q == "." that is everything.
func (x *pathIndex) descendants(q string, strict bool, fn walk) bool {
	x.freeze()
	if q == "." {
		for _, p := range x.sorted {
			if !x.mt.spend(1+int64(len(x.owners[p]))) || !fn(p, x.owners[p]) {
				return false
			}
		}
		return true
	}
	if !strict {
		if o, ok := x.owners[q]; ok && !fn(q, o) {
			return false
		}
	}
	prefix := q + "/"
	for i := sort.SearchStrings(x.sorted, prefix); i < len(x.sorted) && strings.HasPrefix(x.sorted[i], prefix); i++ {
		// One unit per node, one per owner (the callback does work for each),
		// and the HasPrefix comparison costs the length of the query.
		if !x.mt.spend(1+int64(len(x.owners[x.sorted[i]]))+int64(len(q)/32)) || !fn(x.sorted[i], x.owners[x.sorted[i]]) {
			return false
		}
	}
	return true
}

// overlapping visits every indexed p with overlap(p, q), each once.
func (x *pathIndex) overlapping(q string, fn walk) bool {
	if !x.ancestors(q, fn) {
		return false
	}
	return x.descendants(q, true, fn)
}

// anyOther reports whether owners holds an id other than self.
func anyOther(owners []int, self int) bool {
	for _, o := range owners {
		if o != self {
			return true
		}
	}
	return false
}

// pathSet is a pathIndex whose owners do not matter (the inventory, the
// tainted set).
type pathSet struct{ x *pathIndex }

func newPathSet(paths []string, mt *meter) pathSet {
	x := newPathIndex(mt)
	for _, p := range paths {
		x.add(p, 0)
	}
	x.freeze()
	return pathSet{x}
}

// each visits the members satisfying the query until fn returns false.
func (s pathSet) overlapping(q string, fn func(p string) bool) bool {
	return s.x.overlapping(q, func(p string, _ []int) bool { return fn(p) })
}
func (s pathSet) ancestors(q string, fn func(p string) bool) bool {
	return s.x.ancestors(q, func(p string, _ []int) bool { return fn(p) })
}
func (s pathSet) under(q string, fn func(p string) bool) bool {
	return s.x.descendants(q, false, func(p string, _ []int) bool { return fn(p) })
}
func (s pathSet) all(fn func(p string) bool) bool {
	return s.x.descendants(".", false, func(p string, _ []int) bool { return fn(p) })
}
