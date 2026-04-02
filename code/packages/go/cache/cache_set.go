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
	type ccResult struct {
		cfg CacheConfig
		err error
	}
	res, _ := StartNew[ccResult]("cache.NewCacheConfig", ccResult{},
		func(op *Operation[ccResult], rf *ResultFactory[ccResult]) *OperationResult[ccResult] {
			op.AddProperty("name", name)
			op.AddProperty("totalSize", totalSize)
			op.AddProperty("lineSize", lineSize)
			op.AddProperty("associativity", associativity)
			op.AddProperty("accessLatency", accessLatency)
			if totalSize <= 0 {
				return rf.Generate(true, false, ccResult{err: fmt.Errorf("total_size must be positive, got %d", totalSize)})
			}
			if lineSize <= 0 || (lineSize&(lineSize-1)) != 0 {
				return rf.Generate(true, false, ccResult{err: fmt.Errorf("line_size must be a positive power of 2, got %d", lineSize)})
			}
			if associativity <= 0 {
				return rf.Generate(true, false, ccResult{err: fmt.Errorf("associativity must be positive, got %d", associativity)})
			}
			if totalSize%(lineSize*associativity) != 0 {
				return rf.Generate(true, false, ccResult{err: fmt.Errorf(
					"total_size (%d) must be divisible by line_size * associativity (%d)",
					totalSize, lineSize*associativity,
				)})
			}
			if writePolicy != "write-back" && writePolicy != "write-through" {
				return rf.Generate(true, false, ccResult{err: fmt.Errorf("write_policy must be 'write-back' or 'write-through', got '%s'", writePolicy)})
			}
			if accessLatency < 0 {
				return rf.Generate(true, false, ccResult{err: fmt.Errorf("access_latency must be non-negative, got %d", accessLatency)})
			}
			return rf.Generate(true, false, ccResult{cfg: CacheConfig{
				Name:          name,
				TotalSize:     totalSize,
				LineSize:      lineSize,
				Associativity: associativity,
				AccessLatency: accessLatency,
				WritePolicy:   writePolicy,
			}})
		}).GetResult()
	return res.cfg, res.err
}

// NumLines returns the total number of cache lines = TotalSize / LineSize.
func (c CacheConfig) NumLines() int {
	result, _ := StartNew[int]("cache.NumLines", 0,
		func(_ *Operation[int], rf *ResultFactory[int]) *OperationResult[int] {
			return rf.Generate(true, false, c.TotalSize/c.LineSize)
		}).GetResult()
	return result
}

// NumSets returns the number of sets = NumLines / Associativity.
func (c CacheConfig) NumSets() int {
	result, _ := StartNew[int]("cache.NumSets", 0,
		func(_ *Operation[int], rf *ResultFactory[int]) *OperationResult[int] {
			return rf.Generate(true, false, c.NumLines()/c.Associativity)
		}).GetResult()
	return result
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
	result, _ := StartNew[*CacheSet]("cache.NewCacheSet", nil,
		func(op *Operation[*CacheSet], rf *ResultFactory[*CacheSet]) *OperationResult[*CacheSet] {
			op.AddProperty("associativity", associativity)
			op.AddProperty("lineSize", lineSize)
			lines := make([]*CacheLine, associativity)
			for i := range lines {
				lines[i] = NewCacheLine(lineSize)
			}
			return rf.Generate(true, false, &CacheSet{Lines: lines})
		}).GetResult()
	return result
}

// Lookup checks if a tag is present in this set.
//
// Searches all ways for a valid line with a matching tag. This is
// what happens in hardware with a parallel tag comparator — all
// ways are checked simultaneously.
//
// Returns (hit, wayIndex). wayIndex is -1 on a miss.
func (cs *CacheSet) Lookup(tag int) (bool, int) {
	type lookupResult struct {
		hit bool
		idx int
	}
	res, _ := StartNew[lookupResult]("cache.Lookup", lookupResult{idx: -1},
		func(op *Operation[lookupResult], rf *ResultFactory[lookupResult]) *OperationResult[lookupResult] {
			op.AddProperty("tag", tag)
			for i, line := range cs.Lines {
				if line.Valid && line.Tag == tag {
					return rf.Generate(true, false, lookupResult{hit: true, idx: i})
				}
			}
			return rf.Generate(true, false, lookupResult{hit: false, idx: -1})
		}).GetResult()
	return res.hit, res.idx
}

// Access accesses this set for a given tag. Returns (hit, line).
//
// On a hit, updates the line's LRU timestamp so it becomes the
// most recently used. On a miss, returns the LRU victim line
// (the caller decides what to do — typically allocate new data).
func (cs *CacheSet) Access(tag, cycle int) (bool, *CacheLine) {
	type accessResult struct {
		hit  bool
		line *CacheLine
	}
	res, _ := StartNew[accessResult]("cache.Access", accessResult{},
		func(op *Operation[accessResult], rf *ResultFactory[accessResult]) *OperationResult[accessResult] {
			op.AddProperty("tag", tag)
			op.AddProperty("cycle", cycle)
			hit, wayIndex := cs.Lookup(tag)
			if hit {
				line := cs.Lines[wayIndex]
				line.Touch(cycle)
				return rf.Generate(true, false, accessResult{hit: true, line: line})
			}
			lruIndex := cs.FindLRU()
			return rf.Generate(true, false, accessResult{hit: false, line: cs.Lines[lruIndex]})
		}).GetResult()
	return res.hit, res.line
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
	result, _ := StartNew[*CacheLine]("cache.Allocate", nil,
		func(op *Operation[*CacheLine], rf *ResultFactory[*CacheLine]) *OperationResult[*CacheLine] {
			op.AddProperty("tag", tag)
			op.AddProperty("cycle", cycle)
			for _, line := range cs.Lines {
				if !line.Valid {
					line.Fill(tag, data, cycle)
					return rf.Generate(true, false, (*CacheLine)(nil))
				}
			}

			lruIndex := cs.FindLRU()
			victim := cs.Lines[lruIndex]

			var evicted *CacheLine
			if victim.Dirty {
				evicted = NewCacheLine(len(victim.Data))
				evicted.Valid = true
				evicted.Dirty = true
				evicted.Tag = victim.Tag
				evicted.Data = make([]int, len(victim.Data))
				copy(evicted.Data, victim.Data)
				evicted.LastAccess = victim.LastAccess
			}

			victim.Fill(tag, data, cycle)
			return rf.Generate(true, false, evicted)
		}).GetResult()
	return result
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
	result, _ := StartNew[int]("cache.FindLRU", 0,
		func(_ *Operation[int], rf *ResultFactory[int]) *OperationResult[int] {
			bestIndex := 0
			bestTime := math.MaxInt
			for i, line := range cs.Lines {
				if !line.Valid {
					return rf.Generate(true, false, i)
				}
				if line.LastAccess < bestTime {
					bestTime = line.LastAccess
					bestIndex = i
				}
			}
			return rf.Generate(true, false, bestIndex)
		}).GetResult()
	return result
}
