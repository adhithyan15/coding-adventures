package cache

import "fmt"

// Cache hierarchy — multi-level cache system (L1I + L1D + L2 + L3 + memory).
//
// A modern CPU doesn't have just one cache — it has a hierarchy of
// progressively larger and slower caches:
//
//	+---------+     +--------+     +--------+     +--------+     +--------+
//	|   CPU   | --> |  L1    | --> |   L2   | --> |   L3   | --> |  Main  |
//	|  core   |     | 1 cyc  |     | 10 cyc |     | 30 cyc |     | Memory |
//	|         |     | 64KB   |     | 256KB  |     | 8MB    |     | 100cyc |
//	+---------+     +--------+     +--------+     +--------+     +--------+
//
// Analogy:
//   - L1 = the books open on your desk (tiny, instant access)
//   - L2 = the bookshelf in your office (bigger, a few seconds)
//   - L3 = the library downstairs (huge, takes a minute)
//   - Main memory = the warehouse across town (enormous, takes an hour)

// HierarchyAccess records an access through the full hierarchy.
//
// Tracks which level served the data and the total latency accumulated
// across all levels that were consulted.
type HierarchyAccess struct {
	Address       int           // The memory address that was accessed
	ServedBy      string        // Name of the level that had the data
	TotalCycles   int           // Total clock cycles from start to data delivery
	HitAtLevel    int           // Which hierarchy level served the data (0=L1, 1=L2, etc.)
	LevelAccesses []CacheAccess // Detailed access records from each cache level consulted
}

// cacheLevel is an internal pair of (name, cache) for iteration.
type cacheLevel struct {
	Name  string
	Cache *Cache
}

// CacheHierarchy represents a multi-level cache hierarchy.
//
// Fully configurable: pass any combination of cache levels. You can
// simulate anything from a simple L1-only system to a full 3-level
// hierarchy with separate instruction and data L1 caches.
type CacheHierarchy struct {
	L1I                *Cache // L1 instruction cache (optional)
	L1D                *Cache // L1 data cache (optional but typical)
	L2                 *Cache // L2 cache (optional)
	L3                 *Cache // L3 cache (optional)
	MainMemoryLatency  int    // Clock cycles for main memory access
	dataLevels         []cacheLevel
	instrLevels        []cacheLevel
}

// NewCacheHierarchy creates a cache hierarchy.
//
// Pass nil for any level you don't want.
func NewCacheHierarchy(l1i, l1d, l2, l3 *Cache, mainMemoryLatency int) *CacheHierarchy {
	result, _ := StartNew[*CacheHierarchy]("cache.NewCacheHierarchy", nil,
		func(op *Operation[*CacheHierarchy], rf *ResultFactory[*CacheHierarchy]) *OperationResult[*CacheHierarchy] {
			op.AddProperty("mainMemoryLatency", mainMemoryLatency)
			h := &CacheHierarchy{
				L1I:               l1i,
				L1D:               l1d,
				L2:                l2,
				L3:                l3,
				MainMemoryLatency: mainMemoryLatency,
			}
			if l1d != nil {
				h.dataLevels = append(h.dataLevels, cacheLevel{"L1D", l1d})
			}
			if l2 != nil {
				h.dataLevels = append(h.dataLevels, cacheLevel{"L2", l2})
			}
			if l3 != nil {
				h.dataLevels = append(h.dataLevels, cacheLevel{"L3", l3})
			}
			if l1i != nil {
				h.instrLevels = append(h.instrLevels, cacheLevel{"L1I", l1i})
			}
			if l2 != nil {
				h.instrLevels = append(h.instrLevels, cacheLevel{"L2", l2})
			}
			if l3 != nil {
				h.instrLevels = append(h.instrLevels, cacheLevel{"L3", l3})
			}
			return rf.Generate(true, false, h)
		}).GetResult()
	return result
}

// Read reads through the hierarchy. Returns which level served the data.
//
// Walks the hierarchy top-down. At each level:
//   - If hit: stop, fill all higher levels, return.
//   - If miss: accumulate latency, continue to next level.
//   - If all miss: data comes from main memory.
//
// The inclusive fill policy is used: when L3 serves data, it
// also fills L2 and L1D so subsequent accesses hit at L1.
func (h *CacheHierarchy) Read(address int, isInstruction bool, cycle int) HierarchyAccess {
	result, _ := StartNew[HierarchyAccess]("cache.HierarchyRead", HierarchyAccess{},
		func(op *Operation[HierarchyAccess], rf *ResultFactory[HierarchyAccess]) *OperationResult[HierarchyAccess] {
			op.AddProperty("address", address)
			op.AddProperty("isInstruction", isInstruction)
			op.AddProperty("cycle", cycle)
			levels := h.dataLevels
			if isInstruction {
				levels = h.instrLevels
			}
			if len(levels) == 0 {
				return rf.Generate(true, false, HierarchyAccess{
					Address:       address,
					ServedBy:      "memory",
					TotalCycles:   h.MainMemoryLatency,
					HitAtLevel:    len(levels),
					LevelAccesses: nil,
				})
			}
			totalCycles := 0
			var accesses []CacheAccess
			servedBy := "memory"
			hitLevel := len(levels)
			for levelIdx, level := range levels {
				access := level.Cache.Read(address, cycle)
				totalCycles += level.Cache.Config.AccessLatency
				accesses = append(accesses, access)
				if access.Hit {
					servedBy = level.Name
					hitLevel = levelIdx
					break
				}
			}
			if servedBy == "memory" {
				totalCycles += h.MainMemoryLatency
			}
			lineSize := h.getLineSize(levels)
			dummyData := make([]int, lineSize)
			for fillIdx := hitLevel - 1; fillIdx >= 0; fillIdx-- {
				levels[fillIdx].Cache.FillLine(address, dummyData, cycle)
			}
			return rf.Generate(true, false, HierarchyAccess{
				Address:       address,
				ServedBy:      servedBy,
				TotalCycles:   totalCycles,
				HitAtLevel:    hitLevel,
				LevelAccesses: accesses,
			})
		}).GetResult()
	return result
}

// Write writes through the hierarchy.
//
// With write-allocate + write-back:
//  1. If L1D hit: write to L1D, mark dirty. Done.
//  2. If L1D miss: allocate in L1D, walk down to find data, fill back up.
func (h *CacheHierarchy) Write(address int, data []int, cycle int) HierarchyAccess {
	result, _ := StartNew[HierarchyAccess]("cache.HierarchyWrite", HierarchyAccess{},
		func(op *Operation[HierarchyAccess], rf *ResultFactory[HierarchyAccess]) *OperationResult[HierarchyAccess] {
			op.AddProperty("address", address)
			op.AddProperty("cycle", cycle)
			levels := h.dataLevels
			if len(levels) == 0 {
				return rf.Generate(true, false, HierarchyAccess{
					Address:       address,
					ServedBy:      "memory",
					TotalCycles:   h.MainMemoryLatency,
					HitAtLevel:    0,
					LevelAccesses: nil,
				})
			}
			firstLevel := levels[0]
			access := firstLevel.Cache.Write(address, data, cycle)
			if access.Hit {
				return rf.Generate(true, false, HierarchyAccess{
					Address:       address,
					ServedBy:      firstLevel.Name,
					TotalCycles:   firstLevel.Cache.Config.AccessLatency,
					HitAtLevel:    0,
					LevelAccesses: []CacheAccess{access},
				})
			}
			totalCycles := firstLevel.Cache.Config.AccessLatency
			accesses := []CacheAccess{access}
			servedBy := "memory"
			hitLevel := len(levels)
			for levelIdx := 1; levelIdx < len(levels); levelIdx++ {
				level := levels[levelIdx]
				levelAccess := level.Cache.Read(address, cycle)
				totalCycles += level.Cache.Config.AccessLatency
				accesses = append(accesses, levelAccess)
				if levelAccess.Hit {
					servedBy = level.Name
					hitLevel = levelIdx
					break
				}
			}
			if servedBy == "memory" {
				totalCycles += h.MainMemoryLatency
			}
			return rf.Generate(true, false, HierarchyAccess{
				Address:       address,
				ServedBy:      servedBy,
				TotalCycles:   totalCycles,
				HitAtLevel:    hitLevel,
				LevelAccesses: accesses,
			})
		}).GetResult()
	return result
}

// getLineSize returns the line size from the first level in the hierarchy.
func (h *CacheHierarchy) getLineSize(levels []cacheLevel) int {
	if len(levels) > 0 {
		return levels[0].Cache.Config.LineSize
	}
	return 64 // default
}

// InvalidateAll invalidates all caches in the hierarchy (full flush).
func (h *CacheHierarchy) InvalidateAll() {
	_, _ = StartNew[struct{}]("cache.InvalidateAll", struct{}{},
		func(_ *Operation[struct{}], rf *ResultFactory[struct{}]) *OperationResult[struct{}] {
			for _, c := range []*Cache{h.L1I, h.L1D, h.L2, h.L3} {
				if c != nil {
					c.Invalidate()
				}
			}
			return rf.Generate(true, false, struct{}{})
		}).GetResult()
}

// ResetStats resets statistics for all cache levels.
func (h *CacheHierarchy) ResetStats() {
	_, _ = StartNew[struct{}]("cache.ResetStats", struct{}{},
		func(_ *Operation[struct{}], rf *ResultFactory[struct{}]) *OperationResult[struct{}] {
			for _, c := range []*Cache{h.L1I, h.L1D, h.L2, h.L3} {
				if c != nil {
					c.Stats.Reset()
				}
			}
			return rf.Generate(true, false, struct{}{})
		}).GetResult()
}

// String returns a human-readable summary of the hierarchy.
func (h *CacheHierarchy) String() string {
	result, _ := StartNew[string]("cache.HierarchyString", "",
		func(_ *Operation[string], rf *ResultFactory[string]) *OperationResult[string] {
			var parts []string
			if h.L1I != nil {
				parts = append(parts, fmt.Sprintf("L1I=%dKB", h.L1I.Config.TotalSize/1024))
			}
			if h.L1D != nil {
				parts = append(parts, fmt.Sprintf("L1D=%dKB", h.L1D.Config.TotalSize/1024))
			}
			if h.L2 != nil {
				parts = append(parts, fmt.Sprintf("L2=%dKB", h.L2.Config.TotalSize/1024))
			}
			if h.L3 != nil {
				parts = append(parts, fmt.Sprintf("L3=%dKB", h.L3.Config.TotalSize/1024))
			}
			parts = append(parts, fmt.Sprintf("mem=%dcyc", h.MainMemoryLatency))
			s := "CacheHierarchy("
			for i, p := range parts {
				if i > 0 {
					s += ", "
				}
				s += p
			}
			s += ")"
			return rf.Generate(true, false, s)
		}).GetResult()
	return result
}
