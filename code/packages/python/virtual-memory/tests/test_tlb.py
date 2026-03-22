"""Tests for TLB — Translation Lookaside Buffer."""

from virtual_memory.page_table_entry import PageTableEntry
from virtual_memory.tlb import TLB


def _make_pte(frame: int) -> PageTableEntry:
    """Helper to create a present PTE with the given frame number."""
    return PageTableEntry(frame_number=frame, present=True)


class TestTLBBasics:
    """Test basic TLB operations."""

    def test_empty_tlb_lookup_misses(self) -> None:
        """Looking up anything in an empty TLB is a miss."""
        tlb = TLB(capacity=4)
        result = tlb.lookup(vpn=5)
        assert result is None
        assert tlb.misses == 1
        assert tlb.hits == 0

    def test_insert_and_lookup_hit(self) -> None:
        """Inserting an entry and looking it up should be a hit."""
        tlb = TLB(capacity=4)
        tlb.insert(vpn=5, frame=42, pte=_make_pte(42))

        result = tlb.lookup(vpn=5)
        assert result is not None
        frame, pte = result
        assert frame == 42
        assert pte.frame_number == 42
        assert tlb.hits == 1
        assert tlb.misses == 0

    def test_lookup_different_vpn_misses(self) -> None:
        """Looking up a VPN that wasn't inserted is a miss."""
        tlb = TLB(capacity=4)
        tlb.insert(vpn=5, frame=42, pte=_make_pte(42))

        result = tlb.lookup(vpn=10)
        assert result is None
        assert tlb.misses == 1

    def test_multiple_entries(self) -> None:
        """Multiple entries can coexist in the TLB."""
        tlb = TLB(capacity=4)
        tlb.insert(vpn=1, frame=10, pte=_make_pte(10))
        tlb.insert(vpn=2, frame=20, pte=_make_pte(20))
        tlb.insert(vpn=3, frame=30, pte=_make_pte(30))

        for vpn, frame in [(1, 10), (2, 20), (3, 30)]:
            result = tlb.lookup(vpn)
            assert result is not None
            assert result[0] == frame

    def test_update_existing_entry(self) -> None:
        """Inserting the same VPN again updates the cached frame."""
        tlb = TLB(capacity=4)
        tlb.insert(vpn=5, frame=10, pte=_make_pte(10))
        tlb.insert(vpn=5, frame=99, pte=_make_pte(99))

        result = tlb.lookup(vpn=5)
        assert result is not None
        assert result[0] == 99


class TestTLBEviction:
    """Test LRU eviction when the TLB is full."""

    def test_eviction_when_full(self) -> None:
        """When the TLB is full, the LRU entry is evicted."""
        tlb = TLB(capacity=3)
        tlb.insert(vpn=1, frame=10, pte=_make_pte(10))
        tlb.insert(vpn=2, frame=20, pte=_make_pte(20))
        tlb.insert(vpn=3, frame=30, pte=_make_pte(30))

        # TLB is full. Insert VPN 4 -> should evict VPN 1 (LRU).
        tlb.insert(vpn=4, frame=40, pte=_make_pte(40))

        assert tlb.lookup(vpn=1) is None  # evicted
        assert tlb.lookup(vpn=4) is not None  # present

    def test_access_prevents_eviction(self) -> None:
        """Accessing an entry moves it to MRU, preventing eviction."""
        tlb = TLB(capacity=3)
        tlb.insert(vpn=1, frame=10, pte=_make_pte(10))
        tlb.insert(vpn=2, frame=20, pte=_make_pte(20))
        tlb.insert(vpn=3, frame=30, pte=_make_pte(30))

        # Access VPN 1, making it most recently used.
        tlb.lookup(vpn=1)

        # Insert VPN 4 -> should evict VPN 2 (now LRU), not VPN 1.
        tlb.insert(vpn=4, frame=40, pte=_make_pte(40))

        assert tlb.lookup(vpn=1) is not None  # protected by access
        assert tlb.lookup(vpn=2) is None  # evicted

    def test_size_property(self) -> None:
        """size tracks current number of entries."""
        tlb = TLB(capacity=4)
        assert tlb.size == 0
        tlb.insert(vpn=1, frame=10, pte=_make_pte(10))
        assert tlb.size == 1
        tlb.insert(vpn=2, frame=20, pte=_make_pte(20))
        assert tlb.size == 2

    def test_capacity_property(self) -> None:
        """capacity returns the maximum size."""
        tlb = TLB(capacity=128)
        assert tlb.capacity == 128


class TestTLBFlush:
    """Test TLB flush operations."""

    def test_flush_clears_all_entries(self) -> None:
        """flush() removes all entries from the TLB."""
        tlb = TLB(capacity=4)
        tlb.insert(vpn=1, frame=10, pte=_make_pte(10))
        tlb.insert(vpn=2, frame=20, pte=_make_pte(20))

        tlb.flush()

        assert tlb.lookup(vpn=1) is None
        assert tlb.lookup(vpn=2) is None
        assert tlb.size == 0

    def test_invalidate_single_entry(self) -> None:
        """invalidate() removes only the specified entry."""
        tlb = TLB(capacity=4)
        tlb.insert(vpn=1, frame=10, pte=_make_pte(10))
        tlb.insert(vpn=2, frame=20, pte=_make_pte(20))

        tlb.invalidate(vpn=1)

        assert tlb.lookup(vpn=1) is None
        assert tlb.lookup(vpn=2) is not None

    def test_invalidate_nonexistent_is_safe(self) -> None:
        """Invalidating a VPN that doesn't exist is a no-op."""
        tlb = TLB(capacity=4)
        tlb.invalidate(vpn=999)  # should not raise


class TestTLBStatistics:
    """Test hit/miss counters and hit rate calculation."""

    def test_hit_rate_no_lookups(self) -> None:
        """Hit rate is 0.0 when no lookups have been performed."""
        tlb = TLB(capacity=4)
        assert tlb.hit_rate() == 0.0

    def test_hit_rate_all_misses(self) -> None:
        """Hit rate is 0.0 when every lookup misses."""
        tlb = TLB(capacity=4)
        tlb.lookup(vpn=1)
        tlb.lookup(vpn=2)
        assert tlb.hit_rate() == 0.0

    def test_hit_rate_all_hits(self) -> None:
        """Hit rate is 1.0 when every lookup hits."""
        tlb = TLB(capacity=4)
        tlb.insert(vpn=1, frame=10, pte=_make_pte(10))
        tlb.lookup(vpn=1)
        tlb.lookup(vpn=1)
        assert tlb.hit_rate() == 1.0

    def test_hit_rate_mixed(self) -> None:
        """Hit rate reflects the ratio of hits to total lookups."""
        tlb = TLB(capacity=4)
        tlb.insert(vpn=1, frame=10, pte=_make_pte(10))

        tlb.lookup(vpn=1)   # hit
        tlb.lookup(vpn=99)  # miss
        tlb.lookup(vpn=1)   # hit
        tlb.lookup(vpn=98)  # miss

        # 2 hits, 2 misses = 50% hit rate
        assert tlb.hit_rate() == 0.5

    def test_counters_persist_across_flush(self) -> None:
        """Hit and miss counters are NOT reset by flush()."""
        tlb = TLB(capacity=4)
        tlb.insert(vpn=1, frame=10, pte=_make_pte(10))
        tlb.lookup(vpn=1)   # hit
        tlb.lookup(vpn=99)  # miss

        tlb.flush()

        assert tlb.hits == 1
        assert tlb.misses == 1
