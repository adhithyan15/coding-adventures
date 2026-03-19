package cache

import (
	"fmt"
	"math"
)

// Cache set — a group of cache lines that share the same set index.
//
// A cache set is like a row of labeled boxes on a shelf. When the CPU
// accesses memory, the address tells us *which shelf* (set) to look at.
// Within that shelf, we check each box (way) to see if our data is there.
//
// In a 4-way set-associative cache, each set has 4 lines (ways).
// When all 4 are full and we need to bring in new data, we must evict
// one. The LRU (Least Recently Used) policy picks the line that hasn't
// been accessed for the longest time.
//
// Associativity is a key design tradeoff:
//   - Direct-mapped (1-way): Fast lookup, but high conflict misses.
//   - Fully associative (N-way = total lines): No conflicts, but expensive.
//   - Set-associative (2/4/8/16-way): The sweet spot.
//
//	Set 0: [ Way 0 ] [ Way 1 ] [ Way 2 ] [ Way 3 ]
//	Set 1: [ Way 0 ] [ Way 1 ] [ Way 2 ] [ Way 3 ]
//	...

// CacheConfig holds configuration for a cache level — the knobs you turn
// to get L1/L2/L3.
//
// By adjusting these parameters, the exact same Cache struct can simulate
// anything from a tiny 1KB direct-mapped L1 to a massive 32MB 16-way L3.
//
// Real-world examples:
//
//	ARM Cortex-A78: L1D = 64KB, 4-way, 64B lines, 1 cycle
//	Intel Alder Lake: L1D = 48KB, 12-way, 64B lines, 5 cycles
//	Apple M4: L1D = 128KB, 8-way, 64B lines, ~3 cycles
type CacheConfig struct {
	Name          string // Human-readable name ("L1D", "L2", etc.)
	TotalSize     int    // Total capacity in bytes (e.g., 65536 for 64KB)
	LineSize      int    // Bytes per cache line. Must be a power of 2.
	Associativity int    // Number of ways per set. 1 = direct-mapped.
	AccessLatency int    // Clock cycles to access this level on a hit.
	WritePolicy   string // "write-back" or "write-through"
}

// NewCacheConfig creates a validated CacheConfig.
//
// Cache sizes and line sizes must be powers of 2 — this is a hardware
// constraint because address bit-slicing only works cleanly with
// power-of-2 sizes.
func NewCacheConfig(name string, totalSize, lineSize, associativity, accessLatency int, writePolicy string) (CacheConfig, error) {
	if totalSize <= 0 {
		return CacheConfig{}, fmt.Errorf("total_size must be positive, got %d", totalSize)
	}
	if lineSize <= 0 || (lineSize&(lineSize-1)) != 0 {
		return CacheConfig{}, fmt.Errorf("line_size must be a positive power of 2, got %d", lineSize)
	}
	if associativity <= 0 {
		return CacheConfig{}, fmt.Errorf("associativity must be positive, got %d", associativity)
	}
	if totalSize%(lineSize*associativity) != 0 {
		return CacheConfig{}, fmt.Errorf(
			"total_size (%d) must be divisible by line_size * associativity (%d)",
			totalSize, lineSize*associativity,
		)
	}
	if writePolicy != "write-back" && writePolicy != "write-through" {
		return CacheConfig{}, fmt.Errorf("write_policy must be 'write-back' or 'write-through', got '%s'", writePolicy)
	}
	if accessLatency < 0 {
		return CacheConfig{}, fmt.Errorf("access_latency must be non-negative, got %d", accessLatency)
	}
	return CacheConfig{
		Name:          name,
		TotalSize:     totalSize,
		LineSize:      lineSize,
		Associativity: associativity,
		AccessLatency: accessLatency,
		WritePolicy:   writePolicy,
	}, nil
}

// NumLines returns the total number of cache lines = TotalSize / LineSize.
func (c CacheConfig) NumLines() int {
	return c.TotalSize / c.LineSize
}

// NumSets returns the number of sets = NumLines / Associativity.
func (c CacheConfig) NumSets() int {
	return c.NumLines() / c.Associativity
}

// CacheSet represents one set in the cache — contains N ways (lines).
//
// Implements LRU (Least Recently Used) replacement: when all ways are
// full and we need to bring in new data, evict the line that was
// accessed least recently.
//
// Think of it like a desk with N book slots. When all slots are full
// and you need a new book, you put away the one you haven't read in
// the longest time.
type CacheSet struct {
	Lines []*CacheLine // The ways (lines) in this set
}

// NewCacheSet creates a cache set with the given number of ways.
func NewCacheSet(associativity, lineSize int) *CacheSet {
	lines := make([]*CacheLine, associativity)
	for i := range lines {
		lines[i] = NewCacheLine(lineSize)
	}
	return &CacheSet{Lines: lines}
}

// Lookup checks if a tag is present in this set.
//
// Searches all ways for a valid line with a matching tag. This is
// what happens in hardware with a parallel tag comparator — all
// ways are checked simultaneously.
//
// Returns (hit, wayIndex). wayIndex is -1 on a miss.
func (cs *CacheSet) Lookup(tag int) (bool, int) {
	for i, line := range cs.Lines {
		if line.Valid && line.Tag == tag {
			return true, i
		}
	}
	return false, -1
}

// Access accesses this set for a given tag. Returns (hit, line).
//
// On a hit, updates the line's LRU timestamp so it becomes the
// most recently used. On a miss, returns the LRU victim line
// (the caller decides what to do — typically allocate new data).
func (cs *CacheSet) Access(tag, cycle int) (bool, *CacheLine) {
	hit, wayIndex := cs.Lookup(tag)
	if hit {
		line := cs.Lines[wayIndex]
		line.Touch(cycle)
		return true, line
	}
	// Miss — return the LRU line (candidate for eviction)
	lruIndex := cs.FindLRU()
	return false, cs.Lines[lruIndex]
}

// Allocate brings new data into this set after a cache miss.
//
// First tries to find an invalid (empty) way. If all ways are
// valid, evicts the LRU line. Returns the evicted line if it was
// dirty (the caller must write it back to the next level), or nil.
//
// Think of it like clearing a desk slot for a new book:
//  1. If there's an empty slot, use it (no eviction needed).
//  2. If all slots are full, pick the least-recently-read book.
//  3. If that book had notes scribbled in it (dirty), you need
//     to save those notes before putting the book away.
func (cs *CacheSet) Allocate(tag int, data []int, cycle int) *CacheLine {
	// Step 1: Look for an invalid (empty) way
	for _, line := range cs.Lines {
		if !line.Valid {
			line.Fill(tag, data, cycle)
			return nil // no eviction needed
		}
	}

	// Step 2: All ways full — evict the LRU line
	lruIndex := cs.FindLRU()
	victim := cs.Lines[lruIndex]

	// Step 3: Check if the victim is dirty (needs writeback)
	var evicted *CacheLine
	if victim.Dirty {
		// Create a copy of the evicted line for writeback
		evicted = NewCacheLine(len(victim.Data))
		evicted.Valid = true
		evicted.Dirty = true
		evicted.Tag = victim.Tag
		evicted.Data = make([]int, len(victim.Data))
		copy(evicted.Data, victim.Data)
		evicted.LastAccess = victim.LastAccess
	}

	// Step 4: Overwrite the victim with new data
	victim.Fill(tag, data, cycle)

	return evicted
}

// FindLRU finds the least recently used way index.
//
// LRU replacement is simple: each line records its last access
// time (cycle count). The line with the smallest timestamp is
// the one that hasn't been touched for the longest time.
//
// Special case: invalid lines are always preferred over valid ones
// (an empty slot is "older" than any real data).
func (cs *CacheSet) FindLRU() int {
	bestIndex := 0
	bestTime := math.MaxInt
	for i, line := range cs.Lines {
		// Invalid lines are always the best candidates
		if !line.Valid {
			return i
		}
		if line.LastAccess < bestTime {
			bestTime = line.LastAccess
			bestIndex = i
		}
	}
	return bestIndex
}
