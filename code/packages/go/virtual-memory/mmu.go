package virtualmemory

import (
	"errors"
	"fmt"
)

// =============================================================================
// MMU — Memory Management Unit
// =============================================================================
//
// The MMU sits between the CPU and physical memory, intercepting every memory
// access and translating virtual addresses to physical addresses.
//
//     CPU                    MMU                   Physical Memory
//     +---------+          +-----------+          +----------------+
//     | Program |--vaddr-->| Translate |--paddr-->| RAM            |
//     | (PID 1) |          |           |          |                |
//     +---------+          | TLB cache |          | Frame 0: [...] |
//                          | Page table|          | Frame 1: [...] |
//                          | walk      |          | Frame 2: [...] |
//                          +-----------+          +----------------+
//
// Translation process (for every memory access):
//   1. Split virtual address into VPN and offset
//   2. Check TLB (fast path — ~1 cycle)
//   3. On TLB miss, walk page table (slow path — ~10 cycles)
//   4. If page not present, handle page fault (very slow)
//   5. Compute physical address = (frame << 12) | offset
//   6. Cache in TLB for next time

// Common errors returned by MMU operations.
var (
	ErrAddressSpaceExists = errors.New("address space already exists")
	ErrNoAddressSpace     = errors.New("no address space for PID")
	ErrOutOfMemory        = errors.New("out of physical memory")
)

// MMU manages per-process address spaces and translates virtual addresses.
type MMU struct {
	pageTables     map[int]*TwoLevelPageTable // pid -> page table
	tlb            *TLB
	frameAllocator *PhysicalFrameAllocator
	policy         ReplacementPolicy
	frameToPIDVPN  map[int][2]int // frame -> [pid, vpn]
	frameRefcounts map[int]int    // frame -> refcount (for COW)
	activePID      int            // currently active process (-1 = none)
}

// NewMMU creates an MMU with the given number of physical frames.
// If policy is nil, FIFO is used as the default replacement policy.
func NewMMU(totalFrames int, policy ReplacementPolicy) *MMU {
	result, _ := StartNew[*MMU]("virtual-memory.NewMMU", nil,
		func(op *Operation[*MMU], rf *ResultFactory[*MMU]) *OperationResult[*MMU] {
			op.AddProperty("totalFrames", totalFrames)
			if policy == nil {
				policy = NewFIFOPolicy()
			}
			return rf.Generate(true, false, &MMU{
				pageTables:     make(map[int]*TwoLevelPageTable),
				tlb:            NewTLB(64),
				frameAllocator: NewPhysicalFrameAllocator(totalFrames),
				policy:         policy,
				frameToPIDVPN:  make(map[int][2]int),
				frameRefcounts: make(map[int]int),
				activePID:      -1,
			})
		}).GetResult()
	return result
}

// =============================================================================
// Address Space Management
// =============================================================================

// CreateAddressSpace creates a new, empty address space for a process.
func (m *MMU) CreateAddressSpace(pid int) error {
	_, err := StartNew[struct{}]("virtual-memory.MMU.CreateAddressSpace", struct{}{},
		func(op *Operation[struct{}], rf *ResultFactory[struct{}]) *OperationResult[struct{}] {
			op.AddProperty("pid", pid)
			if _, ok := m.pageTables[pid]; ok {
				return rf.Fail(struct{}{}, fmt.Errorf("%w: PID %d", ErrAddressSpaceExists, pid))
			}
			m.pageTables[pid] = NewTwoLevelPageTable()
			return rf.Generate(true, false, struct{}{})
		}).GetResult()
	return err
}

// DestroyAddressSpace frees all frames owned by a process and removes
// its page table.
func (m *MMU) DestroyAddressSpace(pid int) error {
	_, err := StartNew[struct{}]("virtual-memory.MMU.DestroyAddressSpace", struct{}{},
		func(op *Operation[struct{}], rf *ResultFactory[struct{}]) *OperationResult[struct{}] {
			op.AddProperty("pid", pid)
			if _, ok := m.pageTables[pid]; !ok {
				return rf.Fail(struct{}{}, fmt.Errorf("%w: PID %d", ErrNoAddressSpace, pid))
			}

			// Find and release all frames owned by this process.
			var framesToFree []int
			for frame, info := range m.frameToPIDVPN {
				if info[0] == pid {
					framesToFree = append(framesToFree, frame)
				}
			}

			for _, frame := range framesToFree {
				m.releaseFrame(frame)
			}

			delete(m.pageTables, pid)
			return rf.Generate(true, false, struct{}{})
		}).GetResult()
	return err
}

// releaseFrame handles COW reference counting when freeing a frame.
func (m *MMU) releaseFrame(frame int) {
	refcount := m.frameRefcounts[frame]
	if refcount == 0 {
		refcount = 1
	}

	if refcount > 1 {
		m.frameRefcounts[frame] = refcount - 1
	} else {
		delete(m.frameRefcounts, frame)
		delete(m.frameToPIDVPN, frame)
		m.policy.RemoveFrame(frame)
		if m.frameAllocator.IsAllocated(frame) {
			_ = m.frameAllocator.Free(frame)
		}
	}
}

// =============================================================================
// Page Mapping
// =============================================================================

// MapPage maps a virtual address to a newly allocated physical frame.
// Returns the allocated frame number, or an error.
func (m *MMU) MapPage(pid, virtualAddr int, writable, executable bool) (int, error) {
	return StartNew[int]("virtual-memory.MMU.MapPage", -1,
		func(op *Operation[int], rf *ResultFactory[int]) *OperationResult[int] {
			op.AddProperty("pid", pid)
			op.AddProperty("virtualAddr", virtualAddr)
			if _, ok := m.pageTables[pid]; !ok {
				return rf.Fail(-1, fmt.Errorf("%w: PID %d", ErrNoAddressSpace, pid))
			}

			frame := m.frameAllocator.Allocate()
			if frame < 0 {
				// Memory full — evict a page.
				evicted := m.evictPage()
				if evicted < 0 {
					return rf.Fail(-1, fmt.Errorf("%w: cannot allocate frame", ErrOutOfMemory))
				}
				frame = evicted
			}

			vpn := virtualAddr >> PageOffsetBits
			pt := m.pageTables[pid]
			pt.Map(virtualAddr, frame, writable, executable, true)

			m.frameToPIDVPN[frame] = [2]int{pid, vpn}
			m.frameRefcounts[frame] = 1
			m.policy.AddFrame(frame)
			m.tlb.Invalidate(vpn)

			return rf.Generate(true, false, frame)
		}).GetResult()
}

// =============================================================================
// Address Translation
// =============================================================================

// Translate converts a virtual address to a physical address.
// This is the core MMU operation — every memory access goes through here.
func (m *MMU) Translate(pid, virtualAddr int, write bool) (int, error) {
	return StartNew[int]("virtual-memory.MMU.Translate", 0,
		func(op *Operation[int], rf *ResultFactory[int]) *OperationResult[int] {
			op.AddProperty("pid", pid)
			op.AddProperty("virtualAddr", virtualAddr)
			if _, ok := m.pageTables[pid]; !ok {
				return rf.Fail(0, fmt.Errorf("%w: PID %d", ErrNoAddressSpace, pid))
			}

			// Auto-flush TLB when switching between PIDs.
			if m.activePID >= 0 && m.activePID != pid {
				m.tlb.Flush()
			}
			m.activePID = pid

			vpn := virtualAddr >> PageOffsetBits
			offset := virtualAddr & OffsetMask

			// Step 1: Check TLB (fast path).
			if frame, pte, ok := m.tlb.Lookup(vpn); ok {
				if write && !pte.Writable {
					m.handleCOWFault(pid, vpn)
					physAddr, err := m.Translate(pid, virtualAddr, write)
					if err != nil {
						return rf.Fail(0, err)
					}
					return rf.Generate(true, false, physAddr)
				}
				pte.Accessed = true
				if write {
					pte.Dirty = true
				}
				m.policy.RecordAccess(frame)
				return rf.Generate(true, false, (frame<<PageOffsetBits)|offset)
			}

			// Step 2: TLB miss — walk page table.
			pt := m.pageTables[pid]
			result := pt.Translate(virtualAddr)

			if result == nil || !result.PTE.Present {
				// Page fault — allocate on demand.
				physAddr, err := m.HandlePageFault(pid, virtualAddr)
				if err != nil {
					return rf.Fail(0, err)
				}
				if write {
					result2 := pt.Translate(virtualAddr)
					if result2 != nil {
						result2.PTE.Dirty = true
					}
				}
				return rf.Generate(true, false, physAddr)
			}

			pte := result.PTE

			// Handle COW fault on write to read-only shared page.
			if write && !pte.Writable {
				m.handleCOWFault(pid, vpn)
				physAddr, err := m.Translate(pid, virtualAddr, write)
				if err != nil {
					return rf.Fail(0, err)
				}
				return rf.Generate(true, false, physAddr)
			}

			pte.Accessed = true
			if write {
				pte.Dirty = true
			}

			m.tlb.Insert(vpn, pte.FrameNumber, pte)
			m.policy.RecordAccess(pte.FrameNumber)

			return rf.Generate(true, false, result.PhysicalAddr)
		}).GetResult()
}

// =============================================================================
// Page Fault Handling
// =============================================================================

// HandlePageFault allocates a frame for a faulting page and maps it.
func (m *MMU) HandlePageFault(pid, virtualAddr int) (int, error) {
	return StartNew[int]("virtual-memory.MMU.HandlePageFault", 0,
		func(op *Operation[int], rf *ResultFactory[int]) *OperationResult[int] {
			op.AddProperty("pid", pid)
			op.AddProperty("virtualAddr", virtualAddr)
			vpn := virtualAddr >> PageOffsetBits
			offset := virtualAddr & OffsetMask

			frame := m.frameAllocator.Allocate()
			if frame < 0 {
				evicted := m.evictPage()
				if evicted < 0 {
					return rf.Fail(0, fmt.Errorf("%w: during page fault", ErrOutOfMemory))
				}
				frame = evicted
			}

			pt := m.pageTables[pid]
			pt.Map(virtualAddr, frame, true, false, true)

			m.frameToPIDVPN[frame] = [2]int{pid, vpn}
			m.frameRefcounts[frame] = 1
			m.policy.AddFrame(frame)
			m.tlb.Invalidate(vpn)

			pte, _ := pt.LookupVPN(vpn)
			if pte != nil {
				m.tlb.Insert(vpn, frame, pte)
			}

			return rf.Generate(true, false, (frame<<PageOffsetBits)|offset)
		}).GetResult()
}

// =============================================================================
// Page Eviction
// =============================================================================

// evictPage uses the replacement policy to evict a page.
// Returns the freed frame number (still marked allocated), or -1.
func (m *MMU) evictPage() int {
	victimFrame := m.policy.SelectVictim()
	if victimFrame < 0 {
		return -1
	}

	info, ok := m.frameToPIDVPN[victimFrame]
	if ok {
		ownerPID, ownerVPN := info[0], info[1]
		if pt, exists := m.pageTables[ownerPID]; exists {
			pt.Unmap(ownerVPN << PageOffsetBits)
		}
		m.tlb.Invalidate(ownerVPN)
		delete(m.frameToPIDVPN, victimFrame)
		delete(m.frameRefcounts, victimFrame)
	}

	// Do NOT free the frame — caller will reuse it directly.
	return victimFrame
}

// =============================================================================
// Copy-on-Write (COW)
// =============================================================================

// CloneAddressSpace creates a COW copy of a process's address space.
// All shared pages are marked read-only in both parent and child.
func (m *MMU) CloneAddressSpace(fromPID, toPID int) error {
	_, err := StartNew[struct{}]("virtual-memory.MMU.CloneAddressSpace", struct{}{},
		func(op *Operation[struct{}], rf *ResultFactory[struct{}]) *OperationResult[struct{}] {
			op.AddProperty("fromPID", fromPID)
			op.AddProperty("toPID", toPID)
			sourcePT, ok := m.pageTables[fromPID]
			if !ok {
				return rf.Fail(struct{}{}, fmt.Errorf("%w: PID %d", ErrNoAddressSpace, fromPID))
			}
			if _, ok := m.pageTables[toPID]; ok {
				return rf.Fail(struct{}{}, fmt.Errorf("%w: PID %d", ErrAddressSpaceExists, toPID))
			}

			destPT := NewTwoLevelPageTable()
			dir := sourcePT.Directory()

			for l1Idx := 0; l1Idx < L1Entries; l1Idx++ {
				l2Table := dir[l1Idx]
				if l2Table == nil {
					continue
				}

				for l2VPN, pte := range l2Table.Entries() {
					if !pte.Present {
						continue
					}

					fullVPN := (l1Idx << 10) | l2VPN

					// Mark parent as read-only (COW).
					pte.Writable = false

					// Map in child with same frame, read-only.
					destPT.MapVPN(fullVPN, pte.FrameNumber, false, pte.Executable, pte.UserAccessible)

					// Increment reference count.
					refcount := m.frameRefcounts[pte.FrameNumber]
					if refcount == 0 {
						refcount = 1
					}
					m.frameRefcounts[pte.FrameNumber] = refcount + 1
				}
			}

			m.pageTables[toPID] = destPT
			m.tlb.Flush()
			return rf.Generate(true, false, struct{}{})
		}).GetResult()
	return err
}

// handleCOWFault resolves a copy-on-write fault by making a private copy.
func (m *MMU) handleCOWFault(pid, vpn int) {
	pt := m.pageTables[pid]
	pte, ok := pt.LookupVPN(vpn)
	if !ok || pte == nil {
		return
	}

	oldFrame := pte.FrameNumber
	refcount := m.frameRefcounts[oldFrame]
	if refcount == 0 {
		refcount = 1
	}

	if refcount > 1 {
		// Shared frame — allocate a private copy.
		newFrame := m.frameAllocator.Allocate()
		if newFrame < 0 {
			newFrame = m.evictPage()
			if newFrame < 0 {
				return // Out of memory during COW
			}
		}

		pte.FrameNumber = newFrame
		pte.Writable = true
		pte.Dirty = false

		m.frameToPIDVPN[newFrame] = [2]int{pid, vpn}
		m.frameRefcounts[newFrame] = 1
		m.policy.AddFrame(newFrame)

		m.frameRefcounts[oldFrame] = refcount - 1

		// If sole owner remains, restore write access.
		if m.frameRefcounts[oldFrame] == 1 {
			if info, exists := m.frameToPIDVPN[oldFrame]; exists {
				otherPID, otherVPN := info[0], info[1]
				if otherPT, ptExists := m.pageTables[otherPID]; ptExists {
					if otherPTE, found := otherPT.LookupVPN(otherVPN); found {
						otherPTE.Writable = true
					}
				}
			}
		}
	} else {
		// Sole owner — just make writable.
		pte.Writable = true
	}

	m.tlb.Invalidate(vpn)
}

// =============================================================================
// Context Switching
// =============================================================================

// ContextSwitch switches to a different process's address space.
// Flushes the TLB to prevent security violations.
func (m *MMU) ContextSwitch(newPID int) error {
	_, err := StartNew[struct{}]("virtual-memory.MMU.ContextSwitch", struct{}{},
		func(op *Operation[struct{}], rf *ResultFactory[struct{}]) *OperationResult[struct{}] {
			op.AddProperty("newPID", newPID)
			if _, ok := m.pageTables[newPID]; !ok {
				return rf.Fail(struct{}{}, fmt.Errorf("%w: PID %d", ErrNoAddressSpace, newPID))
			}
			m.activePID = newPID
			m.tlb.Flush()
			return rf.Generate(true, false, struct{}{})
		}).GetResult()
	return err
}

// =============================================================================
// Accessors
// =============================================================================

// TLB returns the TLB instance for inspection.
func (m *MMU) TLB() *TLB {
	result, _ := StartNew[*TLB]("virtual-memory.MMU.TLB", nil,
		func(op *Operation[*TLB], rf *ResultFactory[*TLB]) *OperationResult[*TLB] {
			return rf.Generate(true, false, m.tlb)
		}).GetResult()
	return result
}

// FrameAllocator returns the frame allocator for inspection.
func (m *MMU) FrameAllocator() *PhysicalFrameAllocator {
	result, _ := StartNew[*PhysicalFrameAllocator]("virtual-memory.MMU.FrameAllocator", nil,
		func(op *Operation[*PhysicalFrameAllocator], rf *ResultFactory[*PhysicalFrameAllocator]) *OperationResult[*PhysicalFrameAllocator] {
			return rf.Generate(true, false, m.frameAllocator)
		}).GetResult()
	return result
}

// ActivePID returns the currently active process ID (-1 if none).
func (m *MMU) ActivePID() int {
	result, _ := StartNew[int]("virtual-memory.MMU.ActivePID", -1,
		func(op *Operation[int], rf *ResultFactory[int]) *OperationResult[int] {
			return rf.Generate(true, false, m.activePID)
		}).GetResult()
	return result
}
