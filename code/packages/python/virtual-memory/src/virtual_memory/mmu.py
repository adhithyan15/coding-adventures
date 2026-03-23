"""MMU — Memory Management Unit.

The MMU is the central component of the virtual memory system. It sits
between the CPU and physical memory, intercepting every memory access and
translating virtual addresses to physical addresses.

    CPU                    MMU                   Physical Memory
    +---------+          +-----------+          +----------------+
    | Program |--vaddr-->| Translate |--paddr-->| RAM            |
    | (PID 1) |          |           |          |                |
    +---------+          | TLB cache |          | Frame 0: [...] |
                         | Page table|          | Frame 1: [...] |
                         | walk      |          | Frame 2: [...] |
                         +-----------+          +----------------+

The translation process (for every single memory access):

    1. Split virtual address into VPN and offset.
    2. Check TLB for cached translation (fast path — ~1 cycle).
    3. If TLB miss, walk the page table (slow path — ~10 cycles).
    4. If page not present, handle page fault (very slow — ~1M cycles if disk).
    5. Check permissions (read/write/execute).
    6. Compute physical address = (frame << 12) | offset.
    7. Cache the translation in TLB for next time.

The MMU also manages:
    - Per-process page tables (each process has its own address space)
    - Physical frame allocation (which frames are free/used)
    - Page fault handling (allocate frame on demand)
    - Copy-on-write for efficient fork()
    - Context switching (flush TLB when switching processes)

Copy-on-Write (COW)
====================

When a process calls fork(), the child gets a copy of the parent's entire
address space. Naively copying every page would be extremely expensive.

COW defers the copy: both parent and child share the same physical frames,
but all shared pages are marked read-only. When either process tries to
write to a shared page, a page fault occurs. The fault handler then makes
a private copy of just that one page.

    Before fork():
        Parent: VPN 0 -> Frame 5 (RW)

    After fork() with COW:
        Parent: VPN 0 -> Frame 5 (RO)  ]  shared, refcount=2
        Child:  VPN 0 -> Frame 5 (RO)  ]

    Parent writes to VPN 0 -> page fault!
        1. Allocate new Frame 12
        2. Copy Frame 5 -> Frame 12
        3. Parent: VPN 0 -> Frame 12 (RW)
        4. Child: VPN 0 -> Frame 5 (RW, sole owner)
"""

from virtual_memory.frame_allocator import PhysicalFrameAllocator
from virtual_memory.multi_level_page_table import TwoLevelPageTable
from virtual_memory.page_table_entry import PAGE_OFFSET_BITS
from virtual_memory.replacement import FIFOPolicy, ReplacementPolicy
from virtual_memory.tlb import TLB


class MMU:
    """Memory Management Unit — the core of virtual memory.

    Manages per-process address spaces, translates virtual addresses to
    physical addresses, handles page faults, and supports copy-on-write
    for efficient process forking.

    Example:
        >>> mmu = MMU(total_frames=256)
        >>> mmu.create_address_space(pid=1)
        >>> frame = mmu.map_page(pid=1, virtual_addr=0x1000)
        >>> physical = mmu.translate(pid=1, virtual_addr=0x1ABC)
    """

    def __init__(
        self,
        total_frames: int,
        replacement_policy: ReplacementPolicy | None = None,
    ) -> None:
        """Initialize the MMU.

        Args:
            total_frames: Number of physical frames available. For a machine
                with N bytes of RAM: total_frames = N / 4096.
            replacement_policy: Which algorithm to use when evicting pages.
                Defaults to FIFO (simplest). Can be LRU or Clock.
        """
        # Per-process page tables. Each process ID maps to its own
        # TwoLevelPageTable, providing complete isolation between processes.
        self._page_tables: dict[int, TwoLevelPageTable] = {}

        # The TLB — shared translation cache. Flushed on context switch
        # to prevent one process from accessing another's memory.
        self._tlb: TLB = TLB()

        # Physical frame allocator — tracks which frames are free/used.
        self._frame_allocator: PhysicalFrameAllocator = PhysicalFrameAllocator(
            total_frames
        )

        # Page replacement policy — decides which page to evict when
        # memory is full.
        self._policy: ReplacementPolicy = replacement_policy or FIFOPolicy()

        # Reverse mapping: frame -> (pid, vpn).
        # When we need to evict a frame, we need to know which process
        # and virtual page it belongs to, so we can update that process's
        # page table. Without this reverse mapping, we would have to scan
        # every page table of every process — extremely expensive.
        self._frame_to_pid_vpn: dict[int, tuple[int, int]] = {}

        # Reference counts for COW (copy-on-write) shared frames.
        # frame -> count of processes sharing that frame.
        # When refcount drops to 1, the sole owner can write directly.
        # When refcount drops to 0, the frame can be freed.
        self._frame_refcounts: dict[int, int] = {}

        # Track which process is currently active (for context switching).
        self._active_pid: int | None = None

    # =========================================================================
    # Address Space Management
    # =========================================================================

    def create_address_space(self, pid: int) -> None:
        """Create a new, empty address space for a process.

        Called when a new process is created (e.g., via exec() or spawn()).
        The new process starts with no mapped pages — the kernel will map
        pages as needed (code, stack, heap).

        Args:
            pid: Process ID for the new address space.

        Raises:
            ValueError: If an address space already exists for this PID.
        """
        if pid in self._page_tables:
            msg = f"Address space already exists for PID {pid}"
            raise ValueError(msg)
        self._page_tables[pid] = TwoLevelPageTable()

    def destroy_address_space(self, pid: int) -> None:
        """Destroy a process's address space and free all its frames.

        Called when a process exits. Frees every physical frame owned by
        the process and removes its page table.

        For COW-shared frames, the reference count is decremented. The
        frame is only actually freed when the last process releases it.

        Args:
            pid: Process ID whose address space to destroy.

        Raises:
            KeyError: If no address space exists for this PID.
        """
        if pid not in self._page_tables:
            msg = f"No address space for PID {pid}"
            raise KeyError(msg)

        # Find all frames owned by this process and free them.
        frames_to_free: list[int] = []
        for frame, (owner_pid, _vpn) in list(self._frame_to_pid_vpn.items()):
            if owner_pid == pid:
                frames_to_free.append(frame)

        for frame in frames_to_free:
            self._release_frame(frame)

        del self._page_tables[pid]

    def _release_frame(self, frame: int) -> None:
        """Release a frame, handling COW reference counting.

        If the frame is shared (refcount > 1), just decrement the count.
        If it is the last reference (refcount <= 1), actually free the frame.

        Args:
            frame: The physical frame to release.
        """
        refcount = self._frame_refcounts.get(frame, 1)

        if refcount > 1:
            # Frame is shared — just decrement the count.
            self._frame_refcounts[frame] = refcount - 1
        else:
            # Last reference — actually free the frame.
            self._frame_refcounts.pop(frame, None)
            self._frame_to_pid_vpn.pop(frame, None)
            self._policy.remove_frame(frame)
            if self._frame_allocator.is_allocated(frame):
                self._frame_allocator.free(frame)

    # =========================================================================
    # Page Mapping
    # =========================================================================

    def map_page(
        self,
        pid: int,
        virtual_addr: int,
        writable: bool = True,
        executable: bool = False,
    ) -> int:
        """Map a virtual address to a newly allocated physical frame.

        Allocates a physical frame and creates a mapping in the process's
        page table. If no frames are available, triggers page replacement
        (eviction) to free a frame.

        Args:
            pid: Process ID.
            virtual_addr: The virtual address to map. The entire 4 KB page
                containing this address is mapped.
            writable: Whether the page is writable.
            executable: Whether the page is executable.

        Returns:
            The physical frame number that was allocated.

        Raises:
            KeyError: If no address space exists for this PID.
            MemoryError: If no frames are available and eviction fails.
        """
        if pid not in self._page_tables:
            msg = f"No address space for PID {pid}"
            raise KeyError(msg)

        # Allocate a physical frame.
        frame = self._frame_allocator.allocate()

        if frame is None:
            # Memory is full — must evict a page to free a frame.
            frame = self._evict_page()
            if frame is None:
                msg = "Out of physical memory and eviction failed"
                raise MemoryError(msg)

        # Extract the VPN from the virtual address.
        vpn = virtual_addr >> PAGE_OFFSET_BITS

        # Create the mapping in the page table.
        page_table = self._page_tables[pid]
        page_table.map(virtual_addr, frame, writable=writable, executable=executable)

        # Record the reverse mapping and replacement policy tracking.
        self._frame_to_pid_vpn[frame] = (pid, vpn)
        self._frame_refcounts[frame] = 1
        self._policy.add_frame(frame)

        # Invalidate any stale TLB entry for this VPN.
        self._tlb.invalidate(vpn)

        return frame

    # =========================================================================
    # Address Translation
    # =========================================================================

    def translate(
        self, pid: int, virtual_addr: int, write: bool = False
    ) -> int:
        """Translate a virtual address to a physical address.

        This is the core operation of the MMU. Every memory access by every
        process goes through this method.

        Translation steps:
            1. Split virtual address into VPN and offset.
            2. Check TLB (fast path).
            3. On TLB miss, walk the page table (slow path).
            4. If page not present, handle page fault.
            5. Update accessed/dirty bits.
            6. Cache in TLB.
            7. Return physical address.

        Args:
            pid: Process ID making the access.
            virtual_addr: The virtual address to translate.
            write: Whether this is a write access (for dirty bit and COW).

        Returns:
            The physical address.

        Raises:
            KeyError: If no address space exists for this PID.
            MemoryError: If page fault handling fails (no memory).
        """
        if pid not in self._page_tables:
            msg = f"No address space for PID {pid}"
            raise KeyError(msg)

        # If translating for a different PID than last time, flush the TLB.
        # The TLB is keyed by VPN alone, so entries from one process would
        # return wrong frames for another process using the same VPN.
        # Real CPUs either key the TLB by (ASID, VPN) or flush on switch.
        if self._active_pid is not None and self._active_pid != pid:
            self._tlb.flush()
        self._active_pid = pid

        # Step 1: Split the virtual address.
        vpn = virtual_addr >> PAGE_OFFSET_BITS
        offset = virtual_addr & 0xFFF

        # Step 2: Check the TLB (fast path).
        tlb_result = self._tlb.lookup(vpn)
        if tlb_result is not None:
            frame, pte = tlb_result

            # Handle write to read-only (possibly COW) page.
            if write and not pte.writable:
                self._handle_cow_fault(pid, vpn)
                # Re-translate after COW.
                return self.translate(pid, virtual_addr, write=write)

            # Update accessed/dirty bits.
            pte.accessed = True
            if write:
                pte.dirty = True

            # Record access for replacement policy.
            self._policy.record_access(frame)

            return (frame << PAGE_OFFSET_BITS) | offset

        # Step 3: TLB miss — walk the page table.
        page_table = self._page_tables[pid]
        result = page_table.translate(virtual_addr)

        if result is None or not result[1].present:
            # Step 4: Page not present — handle page fault.
            physical_addr = self.handle_page_fault(pid, virtual_addr)
            # After fault handling, the page is now present.
            # The fault handler returns the physical address directly.
            if write:
                # Re-lookup to set dirty bit and handle COW.
                result = page_table.translate(virtual_addr)
                if result is not None:
                    result[1].dirty = True
            return physical_addr

        physical_addr, pte = result

        # Handle write to read-only (possibly COW) page.
        if write and not pte.writable:
            self._handle_cow_fault(pid, vpn)
            return self.translate(pid, virtual_addr, write=write)

        # Step 5: Update accessed/dirty bits.
        pte.accessed = True
        if write:
            pte.dirty = True

        # Step 6: Cache in TLB.
        self._tlb.insert(vpn, pte.frame_number, pte)

        # Record access for replacement policy.
        self._policy.record_access(pte.frame_number)

        # Step 7: Return physical address.
        return physical_addr

    # =========================================================================
    # Page Fault Handling
    # =========================================================================

    def handle_page_fault(self, pid: int, virtual_addr: int) -> int:
        """Handle a page fault by allocating a frame and mapping the page.

        A page fault occurs when a process accesses a virtual page that
        is not currently mapped to a physical frame. This is interrupt 14
        in x86 and RISC-V.

        Page faults are not necessarily errors. They are a normal part of
        DEMAND PAGING: pages are allocated lazily, only when first accessed.
        This means a process can have a large virtual address space but
        only use physical frames for pages it actually touches.

        Args:
            pid: Process ID that caused the fault.
            virtual_addr: The virtual address that was accessed.

        Returns:
            The physical address after the fault is resolved.

        Raises:
            MemoryError: If no frames are available and eviction fails.
        """
        vpn = virtual_addr >> PAGE_OFFSET_BITS
        offset = virtual_addr & 0xFFF

        # Allocate a physical frame.
        frame = self._frame_allocator.allocate()

        if frame is None:
            # No free frames — must evict a page.
            frame = self._evict_page()
            if frame is None:
                msg = "Out of physical memory during page fault"
                raise MemoryError(msg)

        # Map the page in the process's page table.
        page_table = self._page_tables[pid]
        page_table.map(virtual_addr, frame)

        # Record reverse mapping and policy tracking.
        self._frame_to_pid_vpn[frame] = (pid, vpn)
        self._frame_refcounts[frame] = 1
        self._policy.add_frame(frame)

        # Invalidate any stale TLB entry.
        self._tlb.invalidate(vpn)

        # Cache the new translation in TLB.
        pte = page_table.lookup_vpn(vpn)
        if pte is not None:
            self._tlb.insert(vpn, frame, pte)

        return (frame << PAGE_OFFSET_BITS) | offset

    # =========================================================================
    # Page Eviction
    # =========================================================================

    def _evict_page(self) -> int | None:
        """Evict a page using the configured replacement policy.

        Asks the replacement policy to select a victim frame, then unmaps
        the page from its owner's page table. The frame remains allocated
        in the bitmap — the caller will reuse it directly for a new mapping.

        Returns:
            The frame number (still marked allocated), or None if eviction failed.
        """
        victim_frame = self._policy.select_victim()
        if victim_frame is None:
            return None

        # Look up which process and VPN own this frame.
        mapping = self._frame_to_pid_vpn.get(victim_frame)
        if mapping is not None:
            owner_pid, owner_vpn = mapping
            # Unmap the page from the owner's page table.
            if owner_pid in self._page_tables:
                owner_pt = self._page_tables[owner_pid]
                owner_pt.unmap(owner_vpn << PAGE_OFFSET_BITS)

            # Invalidate the TLB entry.
            self._tlb.invalidate(owner_vpn)

            # Clean up tracking.
            del self._frame_to_pid_vpn[victim_frame]
            self._frame_refcounts.pop(victim_frame, None)

        # NOTE: We do NOT free the frame in the allocator. The caller will
        # reuse this frame directly for a new mapping. If we freed it and
        # the caller then allocated again, we would get the same frame back
        # anyway — but the bitmap would briefly show it as free, which
        # could cause confusion in concurrent scenarios.

        return victim_frame

    # =========================================================================
    # Copy-on-Write
    # =========================================================================

    def clone_address_space(self, from_pid: int, to_pid: int) -> None:
        """Clone an address space using copy-on-write (COW).

        Creates a new address space for to_pid that shares all physical
        frames with from_pid. All shared pages are marked read-only in
        BOTH processes. When either process writes to a shared page, a
        page fault triggers a private copy (see _handle_cow_fault).

        This is how fork() works:
            1. Parent calls fork().
            2. Kernel calls clone_address_space(parent_pid, child_pid).
            3. Child gets an identical address space (same data) instantly.
            4. Writes by either process trigger COW faults.

        Args:
            from_pid: Source process ID (parent).
            to_pid: Destination process ID (child).

        Raises:
            KeyError: If from_pid has no address space.
            ValueError: If to_pid already has an address space.
        """
        if from_pid not in self._page_tables:
            msg = f"No address space for PID {from_pid}"
            raise KeyError(msg)

        if to_pid in self._page_tables:
            msg = f"Address space already exists for PID {to_pid}"
            raise ValueError(msg)

        source_pt = self._page_tables[from_pid]
        dest_pt = TwoLevelPageTable()

        # Walk the source page table and copy all mappings.
        for l1_idx, l2_table in enumerate(source_pt.directory):
            if l2_table is None:
                continue

            for l2_vpn, pte in l2_table.entries().items():
                if not pte.present:
                    continue

                # Reconstruct the full VPN from L1 and L2 indices.
                full_vpn = (l1_idx << 10) | l2_vpn

                # Create a COW copy: both parent and child point to the
                # same physical frame, but both are marked read-only.
                #
                # Mark the source PTE as read-only (even if it was writable).
                pte.writable = False

                # Create a copy of the PTE for the child.
                child_pte = pte.copy()
                child_pte.writable = False

                # Map in the child's page table.
                dest_pt.map_vpn(
                    full_vpn,
                    pte.frame_number,
                    writable=False,
                    executable=pte.executable,
                    user=pte.user_accessible,
                )

                # Increment the reference count for the shared frame.
                refcount = self._frame_refcounts.get(pte.frame_number, 1)
                self._frame_refcounts[pte.frame_number] = refcount + 1

        self._page_tables[to_pid] = dest_pt

        # Flush TLB because source PTEs changed (now read-only).
        self._tlb.flush()

    def _handle_cow_fault(self, pid: int, vpn: int) -> None:
        """Handle a copy-on-write fault.

        Called when a process tries to write to a read-only page that is
        shared via COW. Allocates a new frame, copies the data (conceptually),
        and gives the writing process a private writable copy.

        Args:
            pid: Process that tried to write.
            vpn: Virtual page number that was written to.
        """
        page_table = self._page_tables[pid]
        pte = page_table.lookup_vpn(vpn)

        if pte is None:
            return

        old_frame = pte.frame_number
        refcount = self._frame_refcounts.get(old_frame, 1)

        if refcount > 1:
            # Frame is shared — make a private copy.
            new_frame = self._frame_allocator.allocate()
            if new_frame is None:
                new_frame = self._evict_page()
                if new_frame is None:
                    msg = "Out of memory during COW fault"
                    raise MemoryError(msg)

            # Update the PTE to point to the new frame.
            pte.frame_number = new_frame
            pte.writable = True
            pte.dirty = False

            # Track the new frame.
            self._frame_to_pid_vpn[new_frame] = (pid, vpn)
            self._frame_refcounts[new_frame] = 1
            self._policy.add_frame(new_frame)

            # Decrement refcount on the old frame.
            self._frame_refcounts[old_frame] = refcount - 1

            # If the old frame now has only one owner, restore write access.
            if self._frame_refcounts[old_frame] == 1:
                old_mapping = self._frame_to_pid_vpn.get(old_frame)
                if old_mapping is not None:
                    other_pid, other_vpn = old_mapping
                    if other_pid in self._page_tables:
                        other_pte = self._page_tables[other_pid].lookup_vpn(other_vpn)
                        if other_pte is not None:
                            other_pte.writable = True
        else:
            # Sole owner — just make it writable.
            pte.writable = True

        # Invalidate TLB entry for this VPN.
        self._tlb.invalidate(vpn)

    # =========================================================================
    # Context Switching
    # =========================================================================

    def context_switch(self, new_pid: int) -> None:
        """Switch to a different process's address space.

        Flushes the TLB because the new process has a completely different
        set of virtual-to-physical mappings. Without flushing, the TLB
        would return stale translations from the old process — a security
        vulnerability.

        This is one reason context switches are expensive: the new process
        starts with a cold TLB, and every memory access misses until the
        TLB warms up.

        Args:
            new_pid: The process ID to switch to.

        Raises:
            KeyError: If no address space exists for the new PID.
        """
        if new_pid not in self._page_tables:
            msg = f"No address space for PID {new_pid}"
            raise KeyError(msg)

        self._active_pid = new_pid
        self._tlb.flush()

    # =========================================================================
    # Properties
    # =========================================================================

    @property
    def tlb(self) -> TLB:
        """Access the TLB for inspection or statistics."""
        return self._tlb

    @property
    def frame_allocator(self) -> PhysicalFrameAllocator:
        """Access the frame allocator for inspection."""
        return self._frame_allocator

    @property
    def active_pid(self) -> int | None:
        """Return the currently active process ID, or None."""
        return self._active_pid
