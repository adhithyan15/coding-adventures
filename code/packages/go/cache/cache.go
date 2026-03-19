package cache

import (
	"fmt"
	"math"
)

// Cache — a single configurable level of the cache hierarchy.
//
// This file implements the core cache logic. The same struct is used for
// L1, L2, and L3 — the only difference is the configuration (size,
// associativity, latency). This reflects real hardware: an L1 and an L3
// use the same SRAM cell design, just at different scales.
//
// Address Decomposition
//
// When the CPU accesses memory address 0x1A2B3C4D, the cache must figure
// out three things:
//
//  1. Offset (lowest bits): Which byte within the cache line?
//  2. Set Index (middle bits): Which set should we look in?
//  3. Tag (highest bits): Which memory block is this?
//
// Visual for a 64KB, 4-way, 64B-line cache (256 sets):
//
//	Address: | tag (18 bits) | set index (8 bits) | offset (6 bits) |
//
// This bit-slicing is why cache sizes must be powers of 2.

// CacheAccess records a single cache access — for debugging and performance analysis.
//
// Every Read() or Write() call returns one of these, telling you exactly
// what happened: was it a hit? Which set? Was anything evicted? How many
// cycles did it cost?
type CacheAccess struct {
	Address  int        // The full memory address that was accessed
	Hit      bool       // True if the data was found in the cache
	Tag      int        // The tag bits extracted from the address
	SetIndex int        // The set index bits
	Offset   int        // The offset bits — byte position within the cache line
	Cycles   int        // Clock cycles this access took
	Evicted  *CacheLine // Evicted dirty CacheLine or nil
}

// Cache represents a single level of cache — configurable to be L1, L2, or L3.
//
// This is the workhorse of the cache simulator. Give it a CacheConfig
// and it handles address decomposition, set lookup, LRU replacement,
// and statistics tracking.
type Cache struct {
	Config     CacheConfig
	Stats      CacheStats
	Sets       []*CacheSet
	offsetBits int
	setBits    int
	setMask    int
}

// NewCache creates a new cache with the given configuration.
//
// Creates all sets, precomputes bit positions for address decomposition,
// and initializes statistics.
func NewCache(config CacheConfig) *Cache {
	numSets := config.NumSets()
	sets := make([]*CacheSet, numSets)
	for i := range sets {
		sets[i] = NewCacheSet(config.Associativity, config.LineSize)
	}

	// Precompute bit positions for address decomposition.
	//   offset_bits = log2(line_size)    e.g., log2(64) = 6
	//   set_bits    = log2(num_sets)     e.g., log2(256) = 8
	offsetBits := int(math.Log2(float64(config.LineSize)))
	setBits := 0
	if numSets > 1 {
		setBits = int(math.Log2(float64(numSets)))
	}
	setMask := numSets - 1 // e.g., 0xFF for 256 sets

	return &Cache{
		Config:     config,
		Sets:       sets,
		offsetBits: offsetBits,
		setBits:    setBits,
		setMask:    setMask,
	}
}

// DecomposeAddress splits a memory address into (tag, setIndex, offset).
//
// This is pure bit manipulation — no division needed because all
// sizes are powers of 2.
func (c *Cache) DecomposeAddress(address int) (tag, setIndex, offset int) {
	offset = address & ((1 << c.offsetBits) - 1)
	setIndex = (address >> c.offsetBits) & c.setMask
	tag = address >> (c.offsetBits + c.setBits)
	return
}

// Read reads data from the cache.
//
// On a hit, the data is returned immediately with the cache's
// access latency. On a miss, dummy data is allocated (the caller
// — typically the hierarchy — is responsible for actually fetching
// from the next level).
func (c *Cache) Read(address, cycle int) CacheAccess {
	tag, setIndex, offset := c.DecomposeAddress(address)
	cacheSet := c.Sets[setIndex]

	hit, line := cacheSet.Access(tag, cycle)
	_ = line

	if hit {
		c.Stats.RecordRead(true)
		return CacheAccess{
			Address:  address,
			Hit:      true,
			Tag:      tag,
			SetIndex: setIndex,
			Offset:   offset,
			Cycles:   c.Config.AccessLatency,
		}
	}

	// Miss — allocate the line with dummy data.
	c.Stats.RecordRead(false)
	dummyData := make([]int, c.Config.LineSize)
	evicted := cacheSet.Allocate(tag, dummyData, cycle)
	if evicted != nil {
		c.Stats.RecordEviction(true)
	} else if allWaysValid(cacheSet) {
		c.Stats.RecordEviction(false)
	}

	return CacheAccess{
		Address:  address,
		Hit:      false,
		Tag:      tag,
		SetIndex: setIndex,
		Offset:   offset,
		Cycles:   c.Config.AccessLatency,
		Evicted:  evicted,
	}
}

// Write writes data to the cache.
//
// Write-back policy: Write only to the cache. Mark the line as dirty.
// Write-through policy: Write to both cache and next level. Line stays clean.
//
// On a write miss, we use write-allocate: first bring the line into
// the cache (like a read miss), then perform the write.
func (c *Cache) Write(address int, data []int, cycle int) CacheAccess {
	tag, setIndex, offset := c.DecomposeAddress(address)
	cacheSet := c.Sets[setIndex]

	hit, line := cacheSet.Access(tag, cycle)

	if hit {
		c.Stats.RecordWrite(true)
		// Write the data into the line
		if data != nil {
			for i, b := range data {
				if offset+i < len(line.Data) {
					line.Data[offset+i] = b
				}
			}
		}
		// Mark dirty for write-back; write-through stays clean
		if c.Config.WritePolicy == "write-back" {
			line.Dirty = true
		}
		return CacheAccess{
			Address:  address,
			Hit:      true,
			Tag:      tag,
			SetIndex: setIndex,
			Offset:   offset,
			Cycles:   c.Config.AccessLatency,
		}
	}

	// Write miss — allocate (write-allocate policy), then write
	c.Stats.RecordWrite(false)
	fillData := make([]int, c.Config.LineSize)
	if data != nil {
		for i, b := range data {
			if offset+i < len(fillData) {
				fillData[offset+i] = b
			}
		}
	}

	evicted := cacheSet.Allocate(tag, fillData, cycle)
	if evicted != nil {
		c.Stats.RecordEviction(true)
	} else if allWaysValid(cacheSet) {
		c.Stats.RecordEviction(false)
	}

	// For write-back, mark the newly allocated line as dirty
	newHit, newLine := cacheSet.Access(tag, cycle)
	if newHit && c.Config.WritePolicy == "write-back" {
		newLine.Dirty = true
	}

	return CacheAccess{
		Address:  address,
		Hit:      false,
		Tag:      tag,
		SetIndex: setIndex,
		Offset:   offset,
		Cycles:   c.Config.AccessLatency,
		Evicted:  evicted,
	}
}

// allWaysValid checks if all ways in a set are valid (meaning an eviction occurred).
func allWaysValid(cacheSet *CacheSet) bool {
	for _, line := range cacheSet.Lines {
		if !line.Valid {
			return false
		}
	}
	return true
}

// Invalidate invalidates all lines in the cache (cache flush).
//
// This is equivalent to a cold start — after invalidation, every
// access will be a compulsory miss.
func (c *Cache) Invalidate() {
	for _, cacheSet := range c.Sets {
		for _, line := range cacheSet.Lines {
			line.Invalidate()
		}
	}
}

// FillLine directly fills a cache line with data (used by hierarchy on miss).
//
// This bypasses the normal read/write path — it's used when the
// hierarchy fetches data from a lower level and wants to install
// it in this cache.
func (c *Cache) FillLine(address int, data []int, cycle int) *CacheLine {
	tag, setIndex, _ := c.DecomposeAddress(address)
	cacheSet := c.Sets[setIndex]
	return cacheSet.Allocate(tag, data, cycle)
}

// String returns a human-readable summary of the cache configuration.
func (c *Cache) String() string {
	return fmt.Sprintf(
		"Cache(%s: %dKB, %d-way, %dB lines, %d sets)",
		c.Config.Name,
		c.Config.TotalSize/1024,
		c.Config.Associativity,
		c.Config.LineSize,
		c.Config.NumSets(),
	)
}
