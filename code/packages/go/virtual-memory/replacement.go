package virtualmemory

// =============================================================================
// Page Replacement Policies — FIFO, LRU, Clock
// =============================================================================
//
// When physical memory is full and a new page needs to be loaded, the OS must
// choose which existing page to EVICT. The choice matters enormously:
//
//     - Evict a page needed soon -> immediate page fault (bad!)
//     - Evict a page not needed for a long time -> smooth sailing
//
// The optimal algorithm (Belady's MIN) would evict the page that won't be
// used for the longest time in the future. Since we can't see the future,
// we use heuristics based on past access patterns.
//
//     +--------+------------+---------------------------------------------+
//     | Policy | Complexity | Strategy                                    |
//     +--------+------------+---------------------------------------------+
//     | FIFO   | O(1)       | Evict the oldest page (first loaded)        |
//     | LRU    | O(n)       | Evict the least recently accessed page      |
//     | Clock  | O(n) worst | LRU approximation using a "use bit"         |
//     +--------+------------+---------------------------------------------+

// ReplacementPolicy is the interface for page replacement algorithms.
// Any struct implementing these four methods can serve as a replacement
// policy for the MMU.
type ReplacementPolicy interface {
	// RecordAccess notes that a frame was accessed (read or write).
	RecordAccess(frame int)
	// SelectVictim chooses a frame to evict. Returns -1 if none available.
	SelectVictim() int
	// AddFrame registers a newly allocated frame for tracking.
	AddFrame(frame int)
	// RemoveFrame stops tracking a frame (freed externally).
	RemoveFrame(frame int)
}

// =============================================================================
// FIFO — First-In, First-Out
// =============================================================================

// FIFOPolicy evicts the page that has been in memory the longest.
// Like a queue at a grocery store: first in, first out.
//
// Pros: Simple, O(1) eviction, no access tracking overhead.
// Cons: Can evict frequently-used pages; Belady's anomaly possible.
type FIFOPolicy struct {
	queue []int // Front = oldest (first to evict), back = newest.
}

// NewFIFOPolicy creates a FIFO replacement policy.
func NewFIFOPolicy() *FIFOPolicy {
	result, _ := StartNew[*FIFOPolicy]("virtual-memory.NewFIFOPolicy", nil,
		func(op *Operation[*FIFOPolicy], rf *ResultFactory[*FIFOPolicy]) *OperationResult[*FIFOPolicy] {
			return rf.Generate(true, false, &FIFOPolicy{queue: make([]int, 0)})
		}).GetResult()
	return result
}

// RecordAccess is a no-op for FIFO — it only cares about insertion order.
func (f *FIFOPolicy) RecordAccess(frame int) {
	_, _ = StartNew[struct{}]("virtual-memory.FIFOPolicy.RecordAccess", struct{}{},
		func(op *Operation[struct{}], rf *ResultFactory[struct{}]) *OperationResult[struct{}] {
			op.AddProperty("frame", frame)
			// Intentionally empty. FIFO ignores access events.
			return rf.Generate(true, false, struct{}{})
		}).GetResult()
}

// SelectVictim evicts the oldest frame (front of queue).
func (f *FIFOPolicy) SelectVictim() int {
	result, _ := StartNew[int]("virtual-memory.FIFOPolicy.SelectVictim", -1,
		func(op *Operation[int], rf *ResultFactory[int]) *OperationResult[int] {
			if len(f.queue) == 0 {
				return rf.Generate(true, false, -1)
			}
			victim := f.queue[0]
			f.queue = f.queue[1:]
			return rf.Generate(true, false, victim)
		}).GetResult()
	return result
}

// AddFrame adds a newly loaded frame to the back of the queue.
func (f *FIFOPolicy) AddFrame(frame int) {
	_, _ = StartNew[struct{}]("virtual-memory.FIFOPolicy.AddFrame", struct{}{},
		func(op *Operation[struct{}], rf *ResultFactory[struct{}]) *OperationResult[struct{}] {
			op.AddProperty("frame", frame)
			f.queue = append(f.queue, frame)
			return rf.Generate(true, false, struct{}{})
		}).GetResult()
}

// RemoveFrame removes a frame from the queue.
func (f *FIFOPolicy) RemoveFrame(frame int) {
	_, _ = StartNew[struct{}]("virtual-memory.FIFOPolicy.RemoveFrame", struct{}{},
		func(op *Operation[struct{}], rf *ResultFactory[struct{}]) *OperationResult[struct{}] {
			op.AddProperty("frame", frame)
			for i, v := range f.queue {
				if v == frame {
					f.queue = append(f.queue[:i], f.queue[i+1:]...)
					return rf.Generate(true, false, struct{}{})
				}
			}
			return rf.Generate(true, false, struct{}{})
		}).GetResult()
}

// =============================================================================
// LRU — Least Recently Used
// =============================================================================

// LRUPolicy evicts the page that hasn't been accessed for the longest time.
// Based on temporal locality: recently used pages are likely to be used again.
//
// Implementation: a logical clock increments on every access. Each frame
// records its last access time. SelectVictim finds the minimum timestamp.
//
// Pros: Generally best-performing. No Belady's anomaly.
// Cons: Every access updates the clock. SelectVictim is O(n).
type LRUPolicy struct {
	accessTimes map[int]int // frame -> last access timestamp
	clock       int         // monotonically increasing logical clock
}

// NewLRUPolicy creates an LRU replacement policy.
func NewLRUPolicy() *LRUPolicy {
	result, _ := StartNew[*LRUPolicy]("virtual-memory.NewLRUPolicy", nil,
		func(op *Operation[*LRUPolicy], rf *ResultFactory[*LRUPolicy]) *OperationResult[*LRUPolicy] {
			return rf.Generate(true, false, &LRUPolicy{
				accessTimes: make(map[int]int),
			})
		}).GetResult()
	return result
}

// RecordAccess updates a frame's timestamp to the current clock value.
func (l *LRUPolicy) RecordAccess(frame int) {
	_, _ = StartNew[struct{}]("virtual-memory.LRUPolicy.RecordAccess", struct{}{},
		func(op *Operation[struct{}], rf *ResultFactory[struct{}]) *OperationResult[struct{}] {
			op.AddProperty("frame", frame)
			l.accessTimes[frame] = l.clock
			l.clock++
			return rf.Generate(true, false, struct{}{})
		}).GetResult()
}

// SelectVictim evicts the frame with the oldest (smallest) timestamp.
func (l *LRUPolicy) SelectVictim() int {
	result, _ := StartNew[int]("virtual-memory.LRUPolicy.SelectVictim", -1,
		func(op *Operation[int], rf *ResultFactory[int]) *OperationResult[int] {
			if len(l.accessTimes) == 0 {
				return rf.Generate(true, false, -1)
			}

			minTime := l.clock + 1 // larger than any existing timestamp
			victim := -1
			for frame, t := range l.accessTimes {
				if t < minTime {
					minTime = t
					victim = frame
				}
			}

			if victim >= 0 {
				delete(l.accessTimes, victim)
			}
			return rf.Generate(true, false, victim)
		}).GetResult()
	return result
}

// AddFrame registers a new frame with the current timestamp.
func (l *LRUPolicy) AddFrame(frame int) {
	_, _ = StartNew[struct{}]("virtual-memory.LRUPolicy.AddFrame", struct{}{},
		func(op *Operation[struct{}], rf *ResultFactory[struct{}]) *OperationResult[struct{}] {
			op.AddProperty("frame", frame)
			l.accessTimes[frame] = l.clock
			l.clock++
			return rf.Generate(true, false, struct{}{})
		}).GetResult()
}

// RemoveFrame stops tracking a frame.
func (l *LRUPolicy) RemoveFrame(frame int) {
	_, _ = StartNew[struct{}]("virtual-memory.LRUPolicy.RemoveFrame", struct{}{},
		func(op *Operation[struct{}], rf *ResultFactory[struct{}]) *OperationResult[struct{}] {
			op.AddProperty("frame", frame)
			delete(l.accessTimes, frame)
			return rf.Generate(true, false, struct{}{})
		}).GetResult()
}

// =============================================================================
// Clock — Second Chance
// =============================================================================

// ClockPolicy is a practical approximation of LRU used by real operating systems.
// Pages are arranged in a circular buffer with a "clock hand" sweeping around.
//
// Each page has a USE BIT:
//   - true:  recently accessed -> give it a second chance, clear the bit
//   - false: not recently accessed -> evict it
//
// The hand sweeps clockwise. When it finds a page with use=false, it evicts it.
// When it finds use=true, it clears the bit and moves on.
//
// Visualization:
//
//	       +---+
//	   +---| A |<-- use=1 -> clear, move on
//	   |   +---+
//	   |     |
//	 +-+-+   |  +---+
//	 | D |   +--| B |<-- use=0 -> EVICT
//	 +---+      +---+
//	   |          |
//	   |   +---+  |
//	   +---| C |--+
//	       +---+
type ClockPolicy struct {
	frames  []int        // Circular buffer of frame numbers.
	useBits map[int]bool // Frame -> use bit.
	hand    int          // Current hand position.
}

// NewClockPolicy creates a Clock replacement policy.
func NewClockPolicy() *ClockPolicy {
	result, _ := StartNew[*ClockPolicy]("virtual-memory.NewClockPolicy", nil,
		func(op *Operation[*ClockPolicy], rf *ResultFactory[*ClockPolicy]) *OperationResult[*ClockPolicy] {
			return rf.Generate(true, false, &ClockPolicy{
				frames:  make([]int, 0),
				useBits: make(map[int]bool),
			})
		}).GetResult()
	return result
}

// RecordAccess sets the use bit for a frame, protecting it from eviction.
func (c *ClockPolicy) RecordAccess(frame int) {
	_, _ = StartNew[struct{}]("virtual-memory.ClockPolicy.RecordAccess", struct{}{},
		func(op *Operation[struct{}], rf *ResultFactory[struct{}]) *OperationResult[struct{}] {
			op.AddProperty("frame", frame)
			if _, ok := c.useBits[frame]; ok {
				c.useBits[frame] = true
			}
			return rf.Generate(true, false, struct{}{})
		}).GetResult()
}

// SelectVictim finds a victim using the clock (second chance) algorithm.
func (c *ClockPolicy) SelectVictim() int {
	result, _ := StartNew[int]("virtual-memory.ClockPolicy.SelectVictim", -1,
		func(op *Operation[int], rf *ResultFactory[int]) *OperationResult[int] {
			if len(c.frames) == 0 {
				return rf.Generate(true, false, -1)
			}

			maxIter := len(c.frames) * 2 // worst case: clear all bits, then evict

			for i := 0; i < maxIter; i++ {
				if c.hand >= len(c.frames) {
					c.hand = 0
				}

				frame := c.frames[c.hand]

				if !c.useBits[frame] {
					// Use bit clear -> evict.
					c.frames = append(c.frames[:c.hand], c.frames[c.hand+1:]...)
					delete(c.useBits, frame)
					if c.hand >= len(c.frames) && len(c.frames) > 0 {
						c.hand = 0
					}
					return rf.Generate(true, false, frame)
				}

				// Use bit set -> give second chance, clear it.
				c.useBits[frame] = false
				c.hand++
			}

			return rf.Generate(true, false, -1)
		}).GetResult()
	return result
}

// AddFrame adds a newly loaded frame. New frames start with use=true.
func (c *ClockPolicy) AddFrame(frame int) {
	_, _ = StartNew[struct{}]("virtual-memory.ClockPolicy.AddFrame", struct{}{},
		func(op *Operation[struct{}], rf *ResultFactory[struct{}]) *OperationResult[struct{}] {
			op.AddProperty("frame", frame)
			c.frames = append(c.frames, frame)
			c.useBits[frame] = true
			return rf.Generate(true, false, struct{}{})
		}).GetResult()
}

// RemoveFrame removes a frame from the clock buffer.
func (c *ClockPolicy) RemoveFrame(frame int) {
	_, _ = StartNew[struct{}]("virtual-memory.ClockPolicy.RemoveFrame", struct{}{},
		func(op *Operation[struct{}], rf *ResultFactory[struct{}]) *OperationResult[struct{}] {
			op.AddProperty("frame", frame)
			delete(c.useBits, frame)
			for i, v := range c.frames {
				if v == frame {
					c.frames = append(c.frames[:i], c.frames[i+1:]...)
					if i < c.hand {
						c.hand--
					}
					if c.hand >= len(c.frames) && len(c.frames) > 0 {
						c.hand = 0
					}
					return rf.Generate(true, false, struct{}{})
				}
			}
			return rf.Generate(true, false, struct{}{})
		}).GetResult()
}
