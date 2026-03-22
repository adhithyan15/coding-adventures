"""TLB — Translation Lookaside Buffer.

The TLB is a small, fast cache that stores recent virtual-to-physical address
translations. Without a TLB, every memory access would require walking the
page table — which itself lives in memory. That means every single memory
access by a program would actually require 2-3 memory accesses just to find
the physical address. This would cut CPU performance by 2-3x.

The TLB fixes this by caching the most recent translations. Because programs
exhibit strong LOCALITY of reference (they tend to access the same pages over
and over), a small TLB of 32-256 entries can achieve hit rates above 95%.

How it works:
=============

    CPU wants to access virtual address 0x1ABC
        |
        v
    TLB: "Do I have VPN 0x1 cached?"
        |               |
        YES (hit!)      NO (miss)
        |               |
        v               v
    Return frame    Walk page table
    from cache      (2-3 memory accesses)
        |               |
        v               v
    Physical addr   Found frame -> cache in TLB
                        |
                        v
                    Physical addr

Why flush on context switch?
============================

When the OS switches from process A to process B, the TLB contains A's
translations. If B accesses virtual page 5, the TLB might return A's frame
for page 5 — letting B read A's private memory! Flushing the TLB on every
context switch prevents this security hole.

This is also why context switches are expensive: after a flush, the first
few memory accesses by the new process all miss the TLB and must walk the
page table. The TLB gradually "warms up" as the process runs.

Eviction Policy:
===============

When the TLB is full and a new entry needs to be inserted, we must evict
an existing entry. Our implementation uses LRU (Least Recently Used)
eviction: we track the order in which entries were accessed, and evict
the entry that hasn't been accessed for the longest time.
"""

from virtual_memory.page_table_entry import PageTableEntry


class TLB:
    """Translation Lookaside Buffer — caches VPN -> (frame, PTE) mappings.

    The TLB stores a fixed number of recent translations. On a lookup hit,
    it returns the cached physical frame number instantly. On a miss, the
    caller must walk the page table and then insert the result into the TLB.

    Attributes:
        hits: Number of successful lookups (translation was cached).
        misses: Number of failed lookups (required page table walk).

    Example:
        >>> tlb = TLB(capacity=4)
        >>> pte = PageTableEntry(frame_number=42, present=True)
        >>> tlb.insert(vpn=5, frame=42, pte=pte)
        >>> result = tlb.lookup(vpn=5)
        >>> result is not None  # hit!
        True
        >>> frame, pte = result
        >>> frame
        42
        >>> tlb.hits
        1
    """

    def __init__(self, capacity: int = 64) -> None:
        """Initialize a TLB with the given capacity.

        Args:
            capacity: Maximum number of entries the TLB can hold.
                Real TLBs have 32-256 entries. 64 is a reasonable default
                for simulation. More entries = higher hit rate but more
                expensive hardware.
        """
        # Maps VPN -> (frame_number, PTE copy).
        # We store a copy of the PTE so we can check permission bits
        # without walking the page table again.
        self._entries: dict[int, tuple[int, PageTableEntry]] = {}

        self._capacity: int = capacity

        # Track access order for LRU eviction. Most recently accessed
        # VPNs are at the END of the list. When we need to evict, we
        # remove from the FRONT (least recently used).
        self._access_order: list[int] = []

        # Statistics for measuring TLB effectiveness.
        self.hits: int = 0
        self.misses: int = 0

    def lookup(self, vpn: int) -> tuple[int, PageTableEntry] | None:
        """Look up a VPN in the TLB cache.

        If the VPN is found (a "hit"), this returns the cached frame number
        and PTE, and moves the entry to the most-recently-used position.

        If the VPN is not found (a "miss"), returns None. The caller must
        then walk the page table to find the mapping.

        Args:
            vpn: The virtual page number to look up.

        Returns:
            A tuple of (frame_number, PageTableEntry) on a hit, or None on a miss.
        """
        if vpn in self._entries:
            self.hits += 1
            # Move to end of access order (most recently used).
            # This prevents frequently-accessed pages from being evicted.
            if vpn in self._access_order:
                self._access_order.remove(vpn)
            self._access_order.append(vpn)
            return self._entries[vpn]

        self.misses += 1
        return None

    def insert(self, vpn: int, frame: int, pte: PageTableEntry) -> None:
        """Insert a translation into the TLB.

        Called after a TLB miss, when the page table walk has found the
        mapping. Caches the result so future accesses to the same VPN
        will hit the TLB instead of walking the page table.

        If the TLB is full, the least recently used entry is evicted
        to make room.

        Args:
            vpn: The virtual page number.
            frame: The physical frame number it maps to.
            pte: The page table entry (cached for permission checks).
        """
        # If already present, update in place.
        if vpn in self._entries:
            self._entries[vpn] = (frame, pte)
            if vpn in self._access_order:
                self._access_order.remove(vpn)
            self._access_order.append(vpn)
            return

        # If full, evict the least recently used entry.
        if len(self._entries) >= self._capacity:
            self._evict_lru()

        self._entries[vpn] = (frame, pte)
        self._access_order.append(vpn)

    def _evict_lru(self) -> None:
        """Evict the least recently used entry from the TLB.

        Removes the entry at the front of the access order list (the one
        that hasn't been accessed for the longest time).
        """
        if self._access_order:
            victim_vpn = self._access_order.pop(0)
            self._entries.pop(victim_vpn, None)

    def invalidate(self, vpn: int) -> None:
        """Remove a single entry from the TLB.

        Called when a specific mapping changes (e.g., after a page is
        unmapped or remapped). The stale cached translation must be removed
        so future lookups don't return the old (wrong) frame number.

        Args:
            vpn: The virtual page number to invalidate.
        """
        self._entries.pop(vpn, None)
        if vpn in self._access_order:
            self._access_order.remove(vpn)

    def flush(self) -> None:
        """Remove ALL entries from the TLB.

        Called on context switch. When the OS switches from process A to
        process B, all of A's translations become invalid because B has
        a completely different page table.

        This is expensive — the new process starts with a "cold" TLB and
        every memory access will miss until the TLB warms up. This is one
        reason why context switches are costly.
        """
        self._entries.clear()
        self._access_order.clear()

    def hit_rate(self) -> float:
        """Calculate the TLB hit rate.

        Hit rate = hits / (hits + misses)

        A good TLB has >95% hit rate. Programs exhibit temporal and spatial
        locality, meaning they repeatedly access the same few pages. A
        64-entry TLB capturing the "working set" of recent pages is enough
        for most programs.

        Returns:
            The hit rate as a float between 0.0 and 1.0, or 0.0 if no
            lookups have been performed.
        """
        total = self.hits + self.misses
        if total == 0:
            return 0.0
        return self.hits / total

    @property
    def size(self) -> int:
        """Return the current number of entries in the TLB."""
        return len(self._entries)

    @property
    def capacity(self) -> int:
        """Return the maximum capacity of the TLB."""
        return self._capacity
