package cache

import "fmt"

// Cache line — the smallest unit of data in a cache.
//
// In a real CPU, data is not moved one byte at a time between memory and the
// cache. Instead, it moves in fixed-size chunks called cache lines (also
// called cache blocks). A typical cache line is 64 bytes.
//
// Analogy: Think of a warehouse that ships goods in standard containers.
// You can't order a single screw — you get the whole container (cache line)
// that includes the screw you need plus 63 other bytes of nearby data.
// This works well because of spatial locality: if you accessed byte N,
// you'll likely access bytes N+1, N+2, ... soon.
//
// Each cache line stores:
//
//	+-------+-------+-----+------+---------------------------+
//	| valid | dirty | tag | LRU  |     data (64 bytes)       |
//	+-------+-------+-----+------+---------------------------+
//
//   - valid: Is this line holding real data?
//   - dirty: Has the data been modified since loaded from memory?
//   - tag: The high bits of the memory address.
//   - data: The actual bytes — a slice of integers, each 0-255.
//   - LastAccess: A timestamp (cycle count) for LRU replacement.

// CacheLine represents a single cache line — one slot in the cache.
type CacheLine struct {
	Valid      bool  // Is this line holding real data?
	Dirty      bool  // Has this line been modified? (write-back policy tracking)
	Tag        int   // High bits of the address — identifies which memory block is cached
	LastAccess int   // Cycle count of last access — used for LRU replacement
	Data       []int // The actual bytes stored in this cache line (each 0-255)
}

// NewCacheLine creates a new invalid cache line with the given size.
//
// After creation, the line is invalid (empty box). It becomes valid when
// data is loaded into it via Fill().
func NewCacheLine(lineSize int) *CacheLine {
	data := make([]int, lineSize)
	return &CacheLine{
		Valid:      false,
		Dirty:      false,
		Tag:        0,
		LastAccess: 0,
		Data:       data,
	}
}

// Fill loads data into this cache line, marking it valid.
//
// This is called when a cache miss brings data from a lower level
// (L2, L3, or main memory) into this line.
func (cl *CacheLine) Fill(tag int, data []int, cycle int) {
	cl.Valid = true
	cl.Dirty = false // freshly loaded data is clean
	cl.Tag = tag
	cl.Data = make([]int, len(data)) // defensive copy
	copy(cl.Data, data)
	cl.LastAccess = cycle
}

// Touch updates the last access time — called on every hit.
//
// This is the heartbeat of LRU: the most recently used line
// gets the highest timestamp, so it's the *last* to be evicted.
func (cl *CacheLine) Touch(cycle int) {
	cl.LastAccess = cycle
}

// Invalidate marks this line as invalid (empty).
//
// Used during cache flushes or coherence protocol invalidations.
// The data is not zeroed — it's just marked as not-present.
func (cl *CacheLine) Invalidate() {
	cl.Valid = false
	cl.Dirty = false
}

// LineSize returns the number of bytes in this cache line.
func (cl *CacheLine) LineSize() int {
	return len(cl.Data)
}

// String returns a compact representation for debugging.
func (cl *CacheLine) String() string {
	state := "-"
	if cl.Valid {
		state = "V"
	}
	if cl.Dirty {
		state += "D"
	} else {
		state += "-"
	}
	return fmt.Sprintf("CacheLine(%s, tag=0x%X, lru=%d)", state, cl.Tag, cl.LastAccess)
}
