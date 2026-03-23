# D13 — Virtual Memory

## Overview

Virtual memory is one of the most important abstractions in computer science.
It gives every process the illusion that it has the entire memory space to
itself — starting at address 0, stretching to some large upper limit — even
though the physical machine has limited RAM shared among many processes.

Without virtual memory, every program would need to know exactly where in
physical RAM it was loaded. If program A uses addresses 0x1000–0x2000 and
program B also wants 0x1000–0x2000, they would overwrite each other. The
programmer would need to manually relocate addresses — a nightmare.

**Analogy:** Imagine an apartment building. Each tenant thinks their apartment
number starts at "Room 1" — they have Room 1 (bedroom), Room 2 (kitchen),
Room 3 (bathroom). But the building manager knows the truth: Tenant A's
"Room 1" is actually physical room 401, Tenant B's "Room 1" is physical room
712. The tenants never need to know their real room numbers. They just say
"go to my Room 1" and the building manager (the MMU) translates.

Virtual memory provides three critical services:

1. **Isolation:** Process A cannot see or modify process B's memory, even if
   they use the same virtual addresses. A buggy program cannot crash other
   programs.

2. **Abstraction:** Every program starts at the same virtual address. The
   linker and loader do not need to worry about where in physical RAM the
   program ends up.

3. **Overcommitment:** The system can promise more memory than physically
   exists. Pages not currently in use can be swapped to disk and brought back
   when needed. (We will not implement swapping in this spec, but the data
   structures support it.)

## Where It Fits

```
Process Manager (D14)
│
│  fork() calls mmu.clone_address_space()
│  exec() calls mmu.create_address_space()
│
▼
Virtual Memory (D13) ← YOU ARE HERE
│
│  ┌──────────────────────────────────────────────────┐
│  │  MMU                                              │
│  │  ├── Page Tables (per process)                    │
│  │  ├── TLB (translation cache)                      │
│  │  ├── Physical Frame Allocator                     │
│  │  └── Page Fault Handler (interrupt 14)            │
│  └──────────────────────────────────────────────────┘
│
│  Replaces the region-based MemoryManager from S04
│
▼
Physical Memory
│
│  Divided into 4 KB frames
│  Frame 0: [0x0000–0x0FFF]
│  Frame 1: [0x1000–0x1FFF]
│  Frame 2: [0x2000–0x2FFF]
│  ...
▼
Hardware / CPU Core (D05)
```

**Depends on:** S03 Interrupt Handler (page faults are interrupt 14), D05 Core
(provides the address bus that the MMU intercepts)

**Used by:** D14 Process Manager (fork/exec use address space operations), S04
Kernel (replaces MemoryManager)

## Key Concepts

### Pages and Frames

Virtual memory divides both virtual and physical address spaces into fixed-size
chunks. A chunk of virtual memory is called a **page**. A chunk of physical
memory is called a **frame**. Pages and frames are the same size: **4 KB
(4096 bytes)**.

Why 4 KB? It is a compromise. Smaller pages waste less memory when a program
uses only part of a page (internal fragmentation), but require larger page
tables. Larger pages mean smaller tables but more waste. 4 KB has been the
standard since the Intel 386 in 1985, and RISC-V uses it too.

```
Virtual Address Space (per process):      Physical Memory (shared):
┌──────────────────┐ Page 0               ┌──────────────────┐ Frame 0
│ 0x0000 – 0x0FFF  │──────────┐           │ 0x0000 – 0x0FFF  │
├──────────────────┤ Page 1   │           ├──────────────────┤ Frame 1
│ 0x1000 – 0x1FFF  │────┐     │           │ 0x1000 – 0x1FFF  │
├──────────────────┤    │     │           ├──────────────────┤ Frame 2
│ 0x2000 – 0x2FFF  │─┐  │     │           │ 0x2000 – 0x2FFF  │
├──────────────────┤ │  │     │           ├──────────────────┤ Frame 3
│      ...         │ │  │     │           │ 0x3000 – 0x3FFF  │
└──────────────────┘ │  │     │           ├──────────────────┤ Frame 4
                     │  │     │           │ 0x4000 – 0x4FFF  │
                     │  │     │           ├──────────────────┤ ...
    Page Table:      │  │     │           │      ...         │
    VPN → Frame#     │  │     │           └──────────────────┘
    ┌────┬────────┐  │  │     │
    │ 0  │ Frame 4│──┼──┼─────┘   Page 0 → Frame 4
    │ 1  │ Frame 2│──┼──┘         Page 1 → Frame 2
    │ 2  │ Frame 0│──┘            Page 2 → Frame 0
    └────┴────────┘
    (Any virtual page can map to any physical frame!)
```

### Extracting Page Number and Offset

Given a virtual address, we need to split it into two parts:

```
32-bit virtual address:
┌──────────────────────────┬────────────────┐
│ Virtual Page Number (VPN)│ Page Offset    │
│ bits 31–12 (20 bits)     │ bits 11–0      │
│                          │ (12 bits)      │
└──────────────────────────┴────────────────┘

VPN    = address >> 12           (shift right by 12 to drop the offset)
offset = address & 0xFFF         (mask the lower 12 bits)

Example: address = 0x00012ABC
  VPN    = 0x00012ABC >> 12 = 0x00012 = 18 (decimal)
  offset = 0x00012ABC & 0xFFF = 0xABC = 2748 (decimal)

  Meaning: this address is at byte 2748 within virtual page 18.

Physical address = (frame_number << 12) | offset
  If page 18 maps to frame 7:
  physical = (7 << 12) | 0xABC = 0x7000 | 0xABC = 0x7ABC
```

### Why 12 Bits for the Offset?

Because 2^12 = 4096 = 4 KB, which is exactly the page size. The offset
addresses every byte within a single page. The remaining upper bits identify
which page.

## Data Structures

### PageTableEntry (PTE)

Each entry in the page table describes the mapping for one virtual page:

```
PageTableEntry:
┌──────────────────────────────────────────────────────────────────┐
│ frame_number: int         # Which physical frame this page maps │
│                           # to. Only meaningful if present=true.│
│                                                                  │
│ present: bool             # Is this page currently in physical  │
│                           # memory? If false, accessing it      │
│                           # triggers a page fault (interrupt 14).│
│                           #                                      │
│                           # A page might not be present because: │
│                           # - It was never allocated             │
│                           # - It was swapped to disk             │
│                           # - It is a lazy allocation (allocated │
│                           #   on first access)                   │
│                                                                  │
│ dirty: bool               # Has this page been written to since │
│                           # it was loaded? If true, it must be  │
│                           # written back to disk before the     │
│                           # frame can be reused.                │
│                                                                  │
│ accessed: bool            # Has this page been read or written  │
│                           # recently? Used by page replacement  │
│                           # algorithms (Clock/LRU) to decide    │
│                           # which page to evict.                │
│                                                                  │
│ writable: bool            # Can this page be written to?        │
│                           # Code pages are read-only.           │
│                           # Stack/heap pages are writable.      │
│                           # Copy-on-write pages start read-only.│
│                                                                  │
│ executable: bool          # Can this page contain executable    │
│                           # code? Data pages should not be      │
│                           # executable (NX bit — prevents code  │
│                           # injection attacks).                 │
│                                                                  │
│ user_accessible: bool     # Can user-mode code access this      │
│                           # page? Kernel pages are not user-    │
│                           # accessible. This prevents user      │
│                           # programs from reading/writing       │
│                           # kernel memory.                      │
└──────────────────────────────────────────────────────────────────┘

Bit layout (matching RISC-V Sv32 PTE format):
┌────────────────────┬───┬───┬───┬───┬───┬───┬───┬───┐
│ PPN (frame number) │ D │ A │ G │ U │ X │ W │ R │ V │
│ bits 31–10         │ 7 │ 6 │ 5 │ 4 │ 3 │ 2 │ 1 │ 0 │
│ (22 bits)          │   │   │   │   │   │   │   │   │
└────────────────────┴───┴───┴───┴───┴───┴───┴───┴───┘
V = Valid (our "present")    R = Readable
W = Writable                 X = Executable
U = User-accessible          G = Global (ignore for now)
A = Accessed                 D = Dirty
```

### Single-Level Page Table

The simplest implementation: a hash map from virtual page number to PTE.

```
SingleLevelPageTable:
  entries: map[int → PageTableEntry]
    # Key: virtual page number (VPN)
    # Value: the PTE for that page

  lookup(vpn: int) → PageTableEntry | None
    return entries.get(vpn, None)

  insert(vpn: int, pte: PageTableEntry) → None
    entries[vpn] = pte

  remove(vpn: int) → None
    del entries[vpn]
```

This is simple but does not match how real hardware works. Real CPUs walk
multi-level page tables in hardware — they do not have hash map circuits.

### Two-Level Page Table (Sv32)

RISC-V's Sv32 scheme splits the 20-bit VPN into two 10-bit indices:

```
32-bit virtual address for Sv32:
┌────────────┬────────────┬────────────────┐
│ VPN[1]     │ VPN[0]     │ Page Offset    │
│ bits 31–22 │ bits 21–12 │ bits 11–0      │
│ (10 bits)  │ (10 bits)  │ (12 bits)      │
└────────────┴────────────┴────────────────┘

VPN[1] indexes into the PAGE DIRECTORY (1024 entries)
VPN[0] indexes into a PAGE TABLE   (1024 entries)

Translation walk:
                            ┌─────────────────────┐
  satp register ──────────► │   Page Directory     │
  (base address             │   (1024 entries)     │
   of page dir)             │                      │
                            │   entry[VPN[1]] ─────┼──► points to a Page Table
                            └─────────────────────┘
                                                        ┌─────────────────────┐
                                                   ──► │   Page Table         │
                                                        │   (1024 entries)     │
                                                        │                      │
                                                        │   entry[VPN[0]] ─────┼──► PTE
                                                        └─────────────────────┘
                                                                                    │
                                                     frame_number = PTE.frame_number│
                                                     physical_addr = (frame << 12) | offset
```

Why two levels? A single flat page table for a 32-bit address space would need
2^20 = 1,048,576 entries. Even if each entry is 4 bytes, that is 4 MB per
process — wasteful if the process only uses a small portion of the address
space. With two levels, we only allocate second-level tables for regions that
are actually in use. Most processes only need a handful of second-level tables.

```
TwoLevelPageTable:
  directory: array[1024] of (PageTablePointer | None)
    # Each entry either points to a second-level page table
    # or is None (meaning that 4 MB region is unmapped).
    # 1024 entries × 4 MB each = 4 GB total addressable.

  translate(vpn: int) → PageTableEntry | None:
    vpn1 = (vpn >> 10) & 0x3FF     # upper 10 bits
    vpn0 = vpn & 0x3FF             # lower 10 bits
    table = directory[vpn1]
    if table is None:
      return None                   # no mapping exists
    return table.entries[vpn0]      # may also be None/invalid
```

### TLB (Translation Lookaside Buffer)

Page table lookups are slow — every memory access would require 2–3 additional
memory accesses just to walk the page table. The TLB is a small, fast cache
that remembers recent translations.

```
TLB:
┌──────────────────────────────────────────────────────────────────┐
│ capacity: int = 64            # Number of entries. Real TLBs    │
│                               # have 32–256 entries. 64 is a    │
│                               # reasonable simulation value.    │
│                                                                  │
│ entries: map[(pid, vpn) → frame_number]                          │
│   # Keyed by (process ID, virtual page number) so that          │
│   # different processes can have the same VPN without conflict. │
│                                                                  │
│ hits: int = 0                 # Number of successful lookups.   │
│ misses: int = 0               # Number of failed lookups        │
│                               # (required a page table walk).   │
│                               #                                  │
│   Hit rate = hits / (hits + misses)                              │
│   A good TLB has >95% hit rate. Programs tend to access the     │
│   same pages repeatedly (temporal locality), so a small TLB     │
│   captures most translations.                                   │
└──────────────────────────────────────────────────────────────────┘

Methods:

  lookup(pid: int, vpn: int) → int | None
    # Check if we have a cached translation.
    # If found: increment hits, return frame number.
    # If not found: increment misses, return None.

  insert(pid: int, vpn: int, frame_number: int) → None
    # Add a translation to the cache.
    # If the TLB is full, evict the oldest entry (FIFO).
    # Real TLBs use more sophisticated eviction, but FIFO
    # is sufficient for simulation.

  flush() → None
    # Remove ALL entries. Called on context switch because
    # the new process has a different page table.
    # This is why context switches are expensive!

  flush_entry(pid: int, vpn: int) → None
    # Remove a single entry. Called when a specific mapping
    # changes (e.g., after a page fault resolves).
```

**Why flush on context switch?** When the kernel switches from process A to
process B, the TLB contains A's translations. If B accesses virtual page 5,
the TLB might return A's frame for page 5 — which would let B read A's memory!
Flushing prevents this security hole.

### PhysicalFrameAllocator

Manages which physical frames are free and which are in use. Uses a bitmap
where each bit represents one frame: 0 = free, 1 = in use.

```
PhysicalFrameAllocator:
┌──────────────────────────────────────────────────────────────────┐
│ total_frames: int             # Total physical frames available.│
│                               # For 16 MB RAM with 4 KB frames: │
│                               # 16 MB / 4 KB = 4096 frames.    │
│                                                                  │
│ bitmap: array[total_frames] of bit                               │
│   # bitmap[i] = 0 means frame i is free.                        │
│   # bitmap[i] = 1 means frame i is allocated.                   │
│   #                                                              │
│   # Example for 16 frames:                                      │
│   # [1,1,1,0,0,1,0,0,0,1,1,0,0,0,0,0]                         │
│   #  ^ ^ ^         ^ ^                                          │
│   #  kernel frames  process frames                               │
│                                                                  │
│ free_count: int               # How many frames are free.       │
│                               # Maintained for O(1) queries.    │
└──────────────────────────────────────────────────────────────────┘

Methods:

  allocate() → int | None
    # Find the first free frame, mark it as used, return its number.
    # Returns None if no frames are available (out of memory!).
    # Scans the bitmap linearly — simple but O(n). Real allocators
    # use free lists or buddy systems for O(1) allocation.

  free(frame_number: int) → None
    # Mark a frame as free. Raises error if already free
    # (double-free is a bug).

  is_allocated(frame_number: int) → bool
    # Check if a frame is currently in use.

  available() → int
    # Return free_count — how many frames are available.
```

## Algorithms

### Address Translation

The core operation of virtual memory. Every memory access by every process goes
through translation.

```
translate(pid: int, virtual_address: int) → int:
  # Step 1: Split the virtual address
  vpn    = virtual_address >> 12
  offset = virtual_address & 0xFFF

  # Step 2: Check the TLB (fast path)
  frame = tlb.lookup(pid, vpn)
  if frame is not None:
    return (frame << 12) | offset    # TLB hit! Done.

  # Step 3: TLB miss — walk the page table (slow path)
  page_table = get_page_table(pid)
  pte = page_table.lookup(vpn)

  # Step 4: Is the page present in memory?
  if pte is None or not pte.present:
    handle_page_fault(pid, virtual_address)
    # After handling, the page is now present.
    # Retry the translation.
    pte = page_table.lookup(vpn)

  # Step 5: Check permissions
  # (Is this a write to a read-only page? Execute on non-executable?)
  check_permissions(pte, access_type)

  # Step 6: Update accessed/dirty bits
  pte.accessed = true
  if access_type == WRITE:
    pte.dirty = true

  # Step 7: Cache in TLB for next time
  tlb.insert(pid, vpn, pte.frame_number)

  # Step 8: Compute physical address
  return (pte.frame_number << 12) | offset
```

### Page Fault Handling

A page fault occurs when a process accesses a virtual page that is not currently
mapped to a physical frame. This is interrupt 14.

```
handle_page_fault(pid: int, faulting_address: int) → None:
  vpn = faulting_address >> 12
  page_table = get_page_table(pid)
  pte = page_table.lookup(vpn)

  if pte is None:
    # Case 1: Page was never allocated.
    # This is a segmentation fault — the process accessed
    # memory it does not own. Kill the process.
    kill_process(pid, SIGSEGV)
    return

  if not pte.present:
    # Case 2: Page exists but is not in memory.
    # Allocate a physical frame and map it.
    frame = frame_allocator.allocate()

    if frame is None:
      # No free frames! Must evict a page.
      victim_frame = choose_victim()         # page replacement
      evict_page(victim_frame)               # write to disk if dirty
      frame = victim_frame

    pte.frame_number = frame
    pte.present = true
    pte.accessed = true

    # Invalidate any stale TLB entry
    tlb.flush_entry(pid, vpn)

  if not pte.writable and access_was_write:
    # Case 3: Copy-on-write fault.
    # This page is shared with another process (from fork).
    # Make a private copy.
    handle_cow_fault(pid, vpn, pte)
```

### Page Replacement Policies

When physical memory is full and a new frame is needed, the system must choose
a page to evict. The goal is to evict the page least likely to be used soon.

#### FIFO (First-In, First-Out)

The simplest policy: evict the oldest page — the one that has been in memory
the longest.

```
FIFO:
  queue: list of (pid, vpn)    # ordered by arrival time

  on_page_load(pid, vpn):
    queue.append((pid, vpn))

  choose_victim() → (pid, vpn):
    return queue.pop_front()    # remove and return the oldest

  Example:
  Queue: [A, B, C, D]  (A is oldest)
  Need to evict → evict A
  Queue becomes: [B, C, D, E]  (E is newly loaded)
```

FIFO is simple but can be pathological: it might evict a frequently used page
just because it was loaded a long time ago.

#### LRU (Least Recently Used)

Evict the page that has not been accessed for the longest time. Based on the
principle of temporal locality: if a page was used recently, it will probably
be used again soon.

```
LRU:
  access_order: ordered list of (pid, vpn)
    # Most recently accessed at the END.
    # Least recently accessed at the FRONT.

  on_page_access(pid, vpn):
    # Move this page to the end (most recent)
    access_order.remove((pid, vpn))
    access_order.append((pid, vpn))

  choose_victim() → (pid, vpn):
    return access_order.pop_front()   # least recently used

  Example:
  Access order: [C, A, D, B]  (C is least recently used)
  Process accesses A → [C, D, B, A]
  Need to evict → evict C
```

LRU is optimal in practice but expensive: every memory access must update the
access order. Hardware approximates it with the accessed bit.

#### Clock (Second-Chance)

A practical approximation of LRU that uses the accessed bit. Pages are arranged
in a circular buffer. A "clock hand" sweeps around:

```
Clock:
  pages: circular list of (pid, vpn)
  hand: pointer into the circular list

  choose_victim() → (pid, vpn):
    loop:
      page = pages[hand]
      pte = get_pte(page.pid, page.vpn)

      if not pte.accessed:
        # Not recently accessed → evict this one
        remove pages[hand]
        return page

      # Recently accessed → give it a second chance
      pte.accessed = false     # clear the bit
      hand = (hand + 1) % len(pages)   # move to next

  Visualization:
        ┌───┐
    ┌───┤ A │◄── accessed=1 → clear, move on
    │   │   │
    │   └───┘
    │     │
  ┌─┴─┐  │  ┌───┐
  │ D │  └──┤ B │◄── accessed=0 → EVICT THIS ONE
  │   │     │   │
  └───┘     └───┘
    │         │
    │   ┌───┐ │
    └───┤ C ├─┘
        │   │
        └───┘
```

The "second chance" name comes from the fact that a page with its accessed bit
set gets one more pass before eviction. If the page is accessed again before
the hand comes back around, its bit will be set again and it survives another
round.

## MMU (Memory Management Unit)

The MMU is the central component that ties everything together.

```
MMU:
┌──────────────────────────────────────────────────────────────────┐
│ page_tables: map[int → PageTable]                                │
│   # One page table per process. Key is PID.                     │
│                                                                  │
│ tlb: TLB                                                         │
│   # Shared translation cache.                                   │
│                                                                  │
│ frame_allocator: PhysicalFrameAllocator                          │
│   # Manages physical memory frames.                             │
│                                                                  │
│ replacement_policy: FIFO | LRU | Clock                           │
│   # Which algorithm to use when evicting pages.                 │
│   # Configurable at initialization.                             │
└──────────────────────────────────────────────────────────────────┘

Methods:

  translate(pid: int, vaddr: int) → int
    # The core operation. Described in the algorithm section above.

  create_address_space(pid: int) → None
    # Create a new, empty page table for a process.
    # Called by exec() or when creating a new process from scratch.

  clone_address_space(src_pid: int, dst_pid: int) → None
    # Copy src's page table into dst's page table.
    # Used by fork(). Does NOT copy physical frames — instead,
    # marks all pages as read-only in BOTH processes (COW).
    # When either process writes, a page fault triggers a copy.

  destroy_address_space(pid: int) → None
    # Free all frames owned by this process and delete its page table.
    # Called when a process exits.

  map_page(pid: int, vpn: int, frame: int, permissions: Permissions) → None
    # Create a mapping from virtual page vpn to physical frame.
    # Sets present=true and the permission flags.

  unmap_page(pid: int, vpn: int) → None
    # Remove a mapping. Frees the physical frame if this is the
    # last reference to it (no other process has a COW copy).
```

### Copy-on-Write (COW) for fork()

When a process calls fork(), the child gets a copy of the parent's entire
address space. Naively, this means copying every single page — potentially
megabytes of data. Most of it will never be modified (especially if the child
immediately calls exec() to load a new program).

Copy-on-write defers the copy: both parent and child share the same physical
frames, but all shared pages are marked read-only. When either process tries
to write, a page fault occurs. The fault handler then (and only then) makes a
private copy of that single page.

```
Before fork():
  Parent's page table:          Physical frames:
  VPN 0 → Frame 5 (RW)         Frame 5: [data...]
  VPN 1 → Frame 8 (RW)         Frame 8: [data...]
  VPN 2 → Frame 3 (RW)         Frame 3: [data...]

After fork() with COW:
  Parent's page table:          Physical frames:
  VPN 0 → Frame 5 (R-only)     Frame 5: [data...]  ← shared, refcount=2
  VPN 1 → Frame 8 (R-only)     Frame 8: [data...]  ← shared, refcount=2
  VPN 2 → Frame 3 (R-only)     Frame 3: [data...]  ← shared, refcount=2

  Child's page table:
  VPN 0 → Frame 5 (R-only)     (same physical frames!)
  VPN 1 → Frame 8 (R-only)
  VPN 2 → Frame 3 (R-only)

Parent writes to VPN 1 → page fault!
  1. Allocate new Frame 12
  2. Copy Frame 8 → Frame 12
  3. Parent: VPN 1 → Frame 12 (RW)
  4. Decrement Frame 8 refcount to 1
  5. Child still has: VPN 1 → Frame 8 (now RW, sole owner)

After the COW fault:
  Parent's page table:          Physical frames:
  VPN 0 → Frame 5 (R-only)     Frame 5: [data...]  ← still shared
  VPN 1 → Frame 12 (RW)        Frame 8: [data...]  ← child's private copy
  VPN 2 → Frame 3 (R-only)     Frame 12: [modified data...]  ← parent's copy
                                Frame 3: [data...]  ← still shared
  Child's page table:
  VPN 0 → Frame 5 (R-only)
  VPN 1 → Frame 8 (RW)         ← sole owner, restored to RW
  VPN 2 → Frame 3 (R-only)
```

This optimization is critical. Without COW, fork() in a process using 100 MB
of memory would require copying all 100 MB. With COW, fork() is nearly free —
it only copies page table entries (a few kilobytes). The actual data is copied
lazily, only when modified, and only the pages that are modified.

## How It Replaces the Existing MemoryManager

The S04 kernel currently uses a region-based MemoryManager:

```
Current (S04):                      New (D13):
─────────────────                   ─────────────────
Region { base, size, perms }        PageTable + PTE per page
Allocate contiguous region          Allocate individual frames
No isolation between processes      Full isolation via page tables
No demand paging                    Page faults allocate on demand
No COW                              COW for efficient fork()
```

The transition is:
1. Kernel creates an MMU with the PhysicalFrameAllocator sized to total memory.
2. Instead of `memory_manager.allocate_region(size)`, the kernel calls
   `mmu.map_page()` for each page the process needs.
3. Memory protection happens automatically via PTE permission bits.
4. The page fault handler (interrupt 14) is registered with the interrupt
   handler (S03) during kernel initialization.

## Syscalls

### sys_brk (number 12)

Adjusts the program break — the end of the data segment (heap).

```
sys_brk(new_break: address) → address
  # If new_break == 0: return current break (query mode).
  # If new_break > current_break: allocate pages to cover the gap.
  # If new_break < current_break: deallocate pages.
  # Returns the new break address on success, -1 on failure.
  #
  # This is how malloc() gets memory from the kernel. When your
  # C program calls malloc(1000), the C library checks if there
  # is room in the existing heap. If not, it calls brk() to ask
  # the kernel for more pages.

  Example:
  Current break = 0x10000 (page boundary)
  sys_brk(0x13000)
    → Need to map pages at VPN 0x10, 0x11, 0x12
    → Allocate 3 frames, create 3 PTEs
    → New break = 0x13000
```

### sys_mmap (number 9)

Maps a region of virtual memory. More flexible than brk — can map at any
address, with any permissions.

```
sys_mmap(addr: address, length: int, prot: int, flags: int) → address
  # addr: suggested address (0 = let kernel choose)
  # length: how many bytes to map
  # prot: protection flags (PROT_READ=1, PROT_WRITE=2, PROT_EXEC=4)
  # flags: MAP_ANONYMOUS=0x20 (not backed by a file)
  #
  # Returns the address of the mapped region, or -1 on failure.
  #
  # Pages are allocated lazily: the PTEs are created with present=false.
  # The first access to each page triggers a page fault, which allocates
  # a physical frame and zeros it.
```

### sys_munmap (number 11)

Unmaps a previously mapped region.

```
sys_munmap(addr: address, length: int) → int
  # Removes the mapping for the given range.
  # Frees the physical frames (unless shared via COW).
  # Flushes affected TLB entries.
  # Returns 0 on success, -1 on failure.
```

### New Interrupt

```
Interrupt 14: Page Fault
  Trigger: CPU accesses a virtual address whose PTE has present=false,
           or violates the page's permission bits (write to read-only,
           execute non-executable, user access to kernel page).
  Handler: The MMU's page fault handler.
  Action:  Allocate a frame and map it (demand paging),
           or perform a COW copy (write to shared page),
           or kill the process (segfault — invalid access).
```

## Dependencies

```
D13 Virtual Memory
│
├── depends on: S03 Interrupt Handler
│   # Page faults are delivered as interrupt 14.
│   # The MMU registers its page fault handler with S03.
│
├── depends on: D05 Core
│   # The CPU's memory access path is intercepted by the MMU.
│   # Every load/store instruction goes through translate().
│
├── used by: D14 Process Manager
│   # fork() uses clone_address_space() (COW)
│   # exec() uses create_address_space() + destroy old
│   # exit() uses destroy_address_space()
│
└── replaces: S04 MemoryManager
    # The region-based allocator is replaced by page-based
    # virtual memory.
```

## Testing Strategy

### Unit Tests

1. **Page/offset splitting:** Verify that address 0x12ABC splits into VPN=0x12,
   offset=0xABC. Test edge cases: address 0x0, address 0xFFFFFFFF.

2. **PageTableEntry flags:** Create a PTE, set each flag individually, verify
   they are stored and retrievable.

3. **SingleLevelPageTable:** Insert a mapping, look it up, verify the PTE.
   Look up a nonexistent VPN, verify None. Remove a mapping, verify it is gone.

4. **TwoLevelPageTable:** Same tests as single-level, but verify that
   second-level tables are created on demand and that the two-level indexing
   math is correct.

5. **TLB:**
   - Insert + lookup returns the cached frame number.
   - Lookup for missing entry returns None and increments misses.
   - flush() clears all entries.
   - flush_entry() removes only the specified entry.
   - When TLB is full (64 entries), inserting evicts the oldest.
   - Hit/miss counters are accurate.

6. **PhysicalFrameAllocator:**
   - Fresh allocator has all frames free.
   - allocate() returns sequential frame numbers (0, 1, 2...).
   - free(frame) makes it available for re-allocation.
   - allocate() when all frames are used returns None.
   - Double-free raises an error.

7. **Page replacement (FIFO):** Load pages A, B, C, D into a 3-frame system.
   D triggers eviction — verify A is evicted (oldest).

8. **Page replacement (LRU):** Load A, B, C. Access A. Load D — verify B is
   evicted (least recently used, since A was re-accessed).

9. **Page replacement (Clock):** Load A, B, C with accessed bits. Clear A's
   bit. Run clock — verify A is evicted. If all bits set, verify the hand
   clears bits and wraps around.

### Integration Tests

10. **Full translation:** Create an MMU, map VPN 5 → frame 10 for PID 1.
    Translate virtual address 0x5ABC → verify physical address 0xAABC.

11. **Page fault → allocation:** Access an unmapped page, verify page fault
    handler allocates a frame and retries successfully.

12. **COW fork:** Create address space for PID 1, write data. Clone to PID 2.
    Verify both read the same data. Write in PID 2 — verify PID 1's data is
    unchanged (COW copy occurred).

13. **TLB integration:** Translate the same address twice. First translation
    is a TLB miss (miss count = 1). Second is a TLB hit (hit count = 1).
    Flush TLB, translate again — miss count = 2.

14. **sys_brk:** Call sys_brk to extend heap, verify new pages are accessible.
    Call sys_brk to shrink heap, verify old pages are freed.

### Coverage Target

Target: 95%+ line coverage. The MMU, page table, and TLB are critical
infrastructure — bugs here corrupt all memory access. Exhaustive testing is
essential.
