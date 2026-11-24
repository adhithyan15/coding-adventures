package fuzz

import (
	"math"
	"math/bits"
	"slices"
)

// Coverage map geometry (VER06 section 4.1): 65,536 edges in 8,192 bytes.
const (
	CoverageMapSize = 65_536
	CoverageBytes   = CoverageMapSize / 8
)

// Bitmap is a 65,536-bit execution map; bit e set means edge e was executed.
// Edge ids are reduced modulo 65,536 (id % 65536).  Collisions are the known
// price of a fixed-size map; AFL pays the same one.
//
// The zero value is an empty, ready-to-use map.
type Bitmap struct {
	m [CoverageBytes]byte
	// edges lists the masked ids in the order first seen, beside the map, so
	// listing the edges of one execution costs O(edges hit), not O(8192).
	edges []uint16
}

func slot(edge uint64) (idx int, mask byte) {
	e := edge % CoverageMapSize
	return int(e / 8), 1 << (e % 8)
}

// Record marks an edge as executed.
func (b *Bitmap) Record(edge uint64) {
	idx, mask := slot(edge)
	if b.m[idx]&mask == 0 {
		b.m[idx] |= mask
		b.edges = append(b.edges, uint16(edge%CoverageMapSize))
	}
}

// Has reports whether an edge is set.
func (b *Bitmap) Has(edge uint64) bool {
	idx, mask := slot(edge)
	return b.m[idx]&mask != 0
}

// Admit ORs the trace into this (cumulative) map and returns whether any edge
// was new, and how many unique edges were.
func (b *Bitmap) Admit(trace []uint16) (hasNew bool, countNew int) {
	for _, e := range trace {
		if !b.Has(uint64(e)) {
			b.Record(uint64(e))
			countNew++
		}
	}
	return countNew > 0, countNew
}

// Edges returns every covered edge, ascending (a fresh slice).
func (b *Bitmap) Edges() []uint16 {
	out := slices.Clone(b.edges)
	slices.Sort(out)
	return out
}

// Count is the number of distinct covered edges (a population count).
func (b *Bitmap) Count() int {
	n := 0
	for _, x := range b.m {
		n += bits.OnesCount8(x)
	}
	return n
}

// Merge ORs another bitmap into this one.
func (b *Bitmap) Merge(o *Bitmap) {
	for _, e := range o.edges {
		b.Record(uint64(e))
	}
}

// Reset clears the map in O(edges recorded).
func (b *Bitmap) Reset() {
	for _, e := range b.edges {
		idx, mask := slot(uint64(e))
		b.m[idx] &^= mask
	}
	b.edges = b.edges[:0]
}

// Raw returns a copy of the 8,192 bytes of the map (LSB-first within a byte).
func (b *Bitmap) Raw() []byte { return slices.Clone(b.m[:]) }

// CorpusAdmit is the pure novelty check: does the trace hold an edge absent
// from existing?  It returns (admitted, newEdges) where newEdges counts UNIQUE
// novel edges (a trace that repeats one novel edge three times adds one).
//
//	CorpusAdmit([1 2 3], [2 4]) -> (true, 1)    (edge 4 is new)
//	CorpusAdmit([1 2 3], [1 2]) -> (false, 0)
func CorpusAdmit(existing, trace []uint64) (bool, int) {
	have := make(map[uint64]struct{}, len(existing))
	for _, e := range existing {
		have[e] = struct{}{}
	}
	novel := 0
	for _, e := range trace {
		if _, seen := have[e]; !seen {
			have[e] = struct{}{} // count each novel edge once
			novel++
		}
	}
	return novel > 0, novel
}

// CalculateEnergy is the mutation budget:
// max(1, round(base * (1 + new/total) / sqrt(len))).
//
// Short seeds that found a large share of the coverage get the most energy.
// The edge bonus is 1.0 while nothing is covered (total == 0).  Rounding is
// half AWAY from zero (floor(x + 0.5)), as in Rust, Dart and Python.
//
// Worked example: base 10, len 4, 3 new of 6 total edges: 10*1.5/2 = 7.5 rounds
// to 8.
func CalculateEnergy(baseEnergy, seedLen, newEdges, totalEdges int) int {
	base := float64(max(1, baseEnergy))
	bonus := 1.0
	if totalEdges > 0 {
		bonus = 1.0 + float64(newEdges)/float64(totalEdges)
	}
	factor := math.Sqrt(float64(max(1, seedLen)))
	return max(1, int(math.Floor(base*bonus/factor+0.5)))
}
