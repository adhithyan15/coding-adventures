// Package cache implements a configurable CPU cache hierarchy simulator.
//
// This package simulates multi-level cache hierarchies (L1I/L1D/L2/L3) like
// those found in modern CPUs. The same Cache struct serves as L1, L2, or L3
// by configuring size, associativity, and latency differently.
package cache

import "fmt"

// Cache statistics tracking — measuring how well the cache is performing.
//
// Every cache keeps a scorecard. Just like a baseball player tracks batting
// average (hits / at-bats), a cache tracks its hit rate (cache hits /
// total accesses). A high hit rate means the cache is doing its job well —
// most memory requests are being served quickly from the cache rather than
// going to slower main memory.
//
// Key metrics:
//   - Reads/Writes: How many times the CPU asked for data or stored data.
//   - Hits: How many times the requested data was already in the cache.
//   - Misses: How many times we had to go to a slower level to get the data.
//   - Evictions: How many times we had to kick out old data to make room.
//   - Writebacks: How many evictions involved dirty data that needed to be
//     written back to the next level (only relevant for write-back caches).
//
// Analogy: Think of a library desk (L1 cache). If you keep the right books
// on your desk, you rarely need to walk to the shelf (L2). Your "hit rate"
// is how often the book you need is already on your desk.

// CacheStats tracks performance statistics for a single cache level.
//
// Every read or write to the cache updates these counters. After running
// a simulation, you can inspect HitRate() and MissRate() to see how
// effective the cache configuration is for a given workload.
type CacheStats struct {
	Reads      int // Number of read operations
	Writes     int // Number of write operations
	Hits       int // Number of cache hits
	Misses     int // Number of cache misses
	Evictions  int // Number of evictions
	Writebacks int // Number of dirty evictions that needed writeback
}

// TotalAccesses returns the total number of read + write operations.
func (s *CacheStats) TotalAccesses() int {
	result, _ := StartNew[int]("cache.TotalAccesses", 0,
		func(_ *Operation[int], rf *ResultFactory[int]) *OperationResult[int] {
			return rf.Generate(true, false, s.Reads+s.Writes)
		}).GetResult()
	return result
}

// HitRate returns the fraction of accesses that were cache hits (0.0 to 1.0).
//
// Returns 0.0 if no accesses have been made (avoid division by zero).
// A hit rate of 0.95 means 95% of memory requests were served from
// this cache level — excellent for an L1 cache.
func (s *CacheStats) HitRate() float64 {
	result, _ := StartNew[float64]("cache.HitRate", 0,
		func(_ *Operation[float64], rf *ResultFactory[float64]) *OperationResult[float64] {
			total := s.TotalAccesses()
			if total == 0 {
				return rf.Generate(true, false, 0.0)
			}
			return rf.Generate(true, false, float64(s.Hits)/float64(total))
		}).GetResult()
	return result
}

// MissRate returns the fraction of accesses that were cache misses (0.0 to 1.0).
//
// Always equals 1.0 - HitRate(). Provided for convenience since
// miss rate is the more commonly discussed metric in architecture
// papers ("this workload has a 5% L1 miss rate").
func (s *CacheStats) MissRate() float64 {
	result, _ := StartNew[float64]("cache.MissRate", 0,
		func(_ *Operation[float64], rf *ResultFactory[float64]) *OperationResult[float64] {
			total := s.TotalAccesses()
			if total == 0 {
				return rf.Generate(true, false, 0.0)
			}
			return rf.Generate(true, false, float64(s.Misses)/float64(total))
		}).GetResult()
	return result
}

// RecordRead records a read access. Pass hit=true for a cache hit.
func (s *CacheStats) RecordRead(hit bool) {
	_, _ = StartNew[struct{}]("cache.RecordRead", struct{}{},
		func(op *Operation[struct{}], rf *ResultFactory[struct{}]) *OperationResult[struct{}] {
			op.AddProperty("hit", hit)
			s.Reads++
			if hit {
				s.Hits++
			} else {
				s.Misses++
			}
			return rf.Generate(true, false, struct{}{})
		}).GetResult()
}

// RecordWrite records a write access. Pass hit=true for a cache hit.
func (s *CacheStats) RecordWrite(hit bool) {
	_, _ = StartNew[struct{}]("cache.RecordWrite", struct{}{},
		func(op *Operation[struct{}], rf *ResultFactory[struct{}]) *OperationResult[struct{}] {
			op.AddProperty("hit", hit)
			s.Writes++
			if hit {
				s.Hits++
			} else {
				s.Misses++
			}
			return rf.Generate(true, false, struct{}{})
		}).GetResult()
}

// RecordEviction records an eviction. Pass dirty=true if the evicted line was dirty.
//
// A dirty eviction means the data was modified in the cache but not
// yet written to the next level. The cache controller must "write back"
// the dirty data before discarding it — this is the extra cost of a
// write-back policy.
func (s *CacheStats) RecordEviction(dirty bool) {
	_, _ = StartNew[struct{}]("cache.RecordEviction", struct{}{},
		func(op *Operation[struct{}], rf *ResultFactory[struct{}]) *OperationResult[struct{}] {
			op.AddProperty("dirty", dirty)
			s.Evictions++
			if dirty {
				s.Writebacks++
			}
			return rf.Generate(true, false, struct{}{})
		}).GetResult()
}

// Reset zeros all counters.
//
// Useful when you want to measure stats for a specific phase of
// execution (e.g., "what's the hit rate during matrix multiply?"
// without counting the initial data loading phase).
func (s *CacheStats) Reset() {
	_, _ = StartNew[struct{}]("cache.Reset", struct{}{},
		func(_ *Operation[struct{}], rf *ResultFactory[struct{}]) *OperationResult[struct{}] {
			s.Reads = 0
			s.Writes = 0
			s.Hits = 0
			s.Misses = 0
			s.Evictions = 0
			s.Writebacks = 0
			return rf.Generate(true, false, struct{}{})
		}).GetResult()
}

// String returns a human-readable summary of cache statistics.
func (s *CacheStats) String() string {
	result, _ := StartNew[string]("cache.CacheStatsString", "",
		func(_ *Operation[string], rf *ResultFactory[string]) *OperationResult[string] {
			return rf.Generate(true, false, fmt.Sprintf(
				"CacheStats(accesses=%d, hits=%d, misses=%d, hit_rate=%.1f%%, evictions=%d, writebacks=%d)",
				s.TotalAccesses(), s.Hits, s.Misses, s.HitRate()*100,
				s.Evictions, s.Writebacks,
			))
		}).GetResult()
	return result
}
