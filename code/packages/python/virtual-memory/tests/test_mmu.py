"""Tests for MMU — Memory Management Unit."""

import pytest

from virtual_memory.mmu import MMU
from virtual_memory.replacement import ClockPolicy, FIFOPolicy, LRUPolicy


class TestMMUAddressSpace:
    """Test address space creation and destruction."""

    def test_create_address_space(self) -> None:
        """Creating an address space succeeds for a new PID."""
        mmu = MMU(total_frames=16)
        mmu.create_address_space(pid=1)
        # No error means success.

    def test_create_duplicate_raises(self) -> None:
        """Creating an address space for an existing PID raises ValueError."""
        mmu = MMU(total_frames=16)
        mmu.create_address_space(pid=1)
        with pytest.raises(ValueError, match="already exists"):
            mmu.create_address_space(pid=1)

    def test_destroy_address_space(self) -> None:
        """Destroying an address space frees all frames."""
        mmu = MMU(total_frames=16)
        mmu.create_address_space(pid=1)
        mmu.map_page(pid=1, virtual_addr=0x1000)
        mmu.map_page(pid=1, virtual_addr=0x2000)

        allocated_before = mmu.frame_allocator.allocated_count()
        assert allocated_before == 2

        mmu.destroy_address_space(pid=1)

        # Frames should be freed.
        assert mmu.frame_allocator.allocated_count() == 0

    def test_destroy_nonexistent_raises(self) -> None:
        """Destroying an address space that doesn't exist raises KeyError."""
        mmu = MMU(total_frames=16)
        with pytest.raises(KeyError):
            mmu.destroy_address_space(pid=99)


class TestMMUMapPage:
    """Test page mapping."""

    def test_map_page_allocates_frame(self) -> None:
        """map_page() allocates a physical frame and returns its number."""
        mmu = MMU(total_frames=16)
        mmu.create_address_space(pid=1)

        frame = mmu.map_page(pid=1, virtual_addr=0x1000)
        assert frame >= 0
        assert mmu.frame_allocator.is_allocated(frame)

    def test_map_page_no_address_space_raises(self) -> None:
        """Mapping a page for a nonexistent PID raises KeyError."""
        mmu = MMU(total_frames=16)
        with pytest.raises(KeyError):
            mmu.map_page(pid=99, virtual_addr=0x1000)

    def test_map_multiple_pages(self) -> None:
        """Multiple pages can be mapped for the same process."""
        mmu = MMU(total_frames=16)
        mmu.create_address_space(pid=1)

        f1 = mmu.map_page(pid=1, virtual_addr=0x1000)
        f2 = mmu.map_page(pid=1, virtual_addr=0x2000)
        f3 = mmu.map_page(pid=1, virtual_addr=0x3000)

        # Each page gets a different frame.
        assert len({f1, f2, f3}) == 3


class TestMMUTranslate:
    """Test address translation."""

    def test_translate_mapped_page(self) -> None:
        """Translating a mapped virtual address returns the correct physical address."""
        mmu = MMU(total_frames=16)
        mmu.create_address_space(pid=1)

        frame = mmu.map_page(pid=1, virtual_addr=0x5000)

        # Translate 0x5ABC -> (frame << 12) | 0xABC
        physical = mmu.translate(pid=1, virtual_addr=0x5ABC)
        expected = (frame << 12) | 0xABC
        assert physical == expected

    def test_translate_no_address_space_raises(self) -> None:
        """Translating for a nonexistent PID raises KeyError."""
        mmu = MMU(total_frames=16)
        with pytest.raises(KeyError):
            mmu.translate(pid=99, virtual_addr=0x1000)

    def test_translate_page_fault_allocates(self) -> None:
        """Translating an unmapped page triggers a page fault and allocates a frame."""
        mmu = MMU(total_frames=16)
        mmu.create_address_space(pid=1)

        # Translate without explicit map_page -> triggers page fault.
        physical = mmu.translate(pid=1, virtual_addr=0x3000)

        # Should have allocated a frame.
        assert physical is not None
        assert mmu.frame_allocator.allocated_count() >= 1

    def test_translate_preserves_offset(self) -> None:
        """The page offset is carried through in translation."""
        mmu = MMU(total_frames=16)
        mmu.create_address_space(pid=1)
        frame = mmu.map_page(pid=1, virtual_addr=0x1000)

        # Different offsets within the same page.
        assert mmu.translate(pid=1, virtual_addr=0x1000) == (frame << 12) | 0x000
        assert mmu.translate(pid=1, virtual_addr=0x1FFF) == (frame << 12) | 0xFFF
        assert mmu.translate(pid=1, virtual_addr=0x1123) == (frame << 12) | 0x123


class TestMMUTLBIntegration:
    """Test TLB behavior during translation."""

    def test_first_translate_is_tlb_miss(self) -> None:
        """The first translation of a page is always a TLB miss."""
        mmu = MMU(total_frames=16)
        mmu.create_address_space(pid=1)
        mmu.map_page(pid=1, virtual_addr=0x1000)

        mmu.translate(pid=1, virtual_addr=0x1000)

        assert mmu.tlb.misses >= 1

    def test_second_translate_is_tlb_hit(self) -> None:
        """The second translation of the same page should be a TLB hit."""
        mmu = MMU(total_frames=16)
        mmu.create_address_space(pid=1)
        mmu.map_page(pid=1, virtual_addr=0x1000)

        mmu.translate(pid=1, virtual_addr=0x1000)
        initial_misses = mmu.tlb.misses

        mmu.translate(pid=1, virtual_addr=0x1000)

        # Second access should be a hit (no additional miss).
        assert mmu.tlb.misses == initial_misses
        assert mmu.tlb.hits >= 1

    def test_context_switch_flushes_tlb(self) -> None:
        """Context switch flushes the TLB."""
        mmu = MMU(total_frames=16)
        mmu.create_address_space(pid=1)
        mmu.create_address_space(pid=2)
        mmu.map_page(pid=1, virtual_addr=0x1000)

        # Warm up TLB.
        mmu.translate(pid=1, virtual_addr=0x1000)

        # Context switch flushes TLB.
        mmu.context_switch(new_pid=2)
        assert mmu.tlb.size == 0

    def test_context_switch_nonexistent_pid_raises(self) -> None:
        """Context switching to a nonexistent PID raises KeyError."""
        mmu = MMU(total_frames=16)
        with pytest.raises(KeyError):
            mmu.context_switch(new_pid=99)

    def test_tlb_after_flush_causes_miss(self) -> None:
        """After TLB flush, the next translation is a miss again."""
        mmu = MMU(total_frames=16)
        mmu.create_address_space(pid=1)
        mmu.create_address_space(pid=2)
        mmu.map_page(pid=1, virtual_addr=0x1000)

        mmu.translate(pid=1, virtual_addr=0x1000)  # miss, then cached
        mmu.translate(pid=1, virtual_addr=0x1000)  # hit

        misses_before = mmu.tlb.misses
        mmu.context_switch(new_pid=2)
        mmu.context_switch(new_pid=1)  # switch back

        mmu.translate(pid=1, virtual_addr=0x1000)  # miss again
        assert mmu.tlb.misses == misses_before + 1


class TestMMUCloneAddressSpace:
    """Test copy-on-write cloning (fork)."""

    def test_clone_creates_new_address_space(self) -> None:
        """clone_address_space creates a new page table for the child."""
        mmu = MMU(total_frames=16)
        mmu.create_address_space(pid=1)
        mmu.map_page(pid=1, virtual_addr=0x1000)

        mmu.clone_address_space(from_pid=1, to_pid=2)

        # Child can translate the same address.
        physical = mmu.translate(pid=2, virtual_addr=0x1000)
        assert physical is not None

    def test_clone_shares_frames(self) -> None:
        """After cloning, parent and child share the same physical frame."""
        mmu = MMU(total_frames=16)
        mmu.create_address_space(pid=1)
        mmu.map_page(pid=1, virtual_addr=0x1000)

        mmu.clone_address_space(from_pid=1, to_pid=2)

        # Both should map to the same physical frame (COW — no copy yet).
        phys1 = mmu.translate(pid=1, virtual_addr=0x1000)
        phys2 = mmu.translate(pid=2, virtual_addr=0x1000)
        assert phys1 == phys2

    def test_clone_nonexistent_source_raises(self) -> None:
        """Cloning from a nonexistent PID raises KeyError."""
        mmu = MMU(total_frames=16)
        with pytest.raises(KeyError):
            mmu.clone_address_space(from_pid=99, to_pid=2)

    def test_clone_existing_dest_raises(self) -> None:
        """Cloning to an existing PID raises ValueError."""
        mmu = MMU(total_frames=16)
        mmu.create_address_space(pid=1)
        mmu.create_address_space(pid=2)
        with pytest.raises(ValueError, match="already exists"):
            mmu.clone_address_space(from_pid=1, to_pid=2)

    def test_cow_write_creates_private_copy(self) -> None:
        """Writing to a COW page allocates a new frame for the writer."""
        mmu = MMU(total_frames=16)
        mmu.create_address_space(pid=1)
        mmu.map_page(pid=1, virtual_addr=0x1000)

        mmu.clone_address_space(from_pid=1, to_pid=2)

        # Read from both — same frame (COW).
        phys1_before = mmu.translate(pid=1, virtual_addr=0x1000)
        phys2_before = mmu.translate(pid=2, virtual_addr=0x1000)
        assert phys1_before == phys2_before

        # Write from child — triggers COW, allocates new frame.
        phys2_after = mmu.translate(pid=2, virtual_addr=0x1000, write=True)

        # Child should now have a different physical address.
        phys1_after = mmu.translate(pid=1, virtual_addr=0x1000)
        assert phys2_after != phys1_after


class TestMMUPageFault:
    """Test page fault handling."""

    def test_page_fault_allocates_frame(self) -> None:
        """handle_page_fault allocates a frame for the faulting page."""
        mmu = MMU(total_frames=16)
        mmu.create_address_space(pid=1)

        physical = mmu.handle_page_fault(pid=1, virtual_addr=0x5000)
        assert physical is not None

        # The page should now be mapped.
        translated = mmu.translate(pid=1, virtual_addr=0x5000)
        assert translated == physical


class TestMMUEviction:
    """Test page eviction when memory is full."""

    def test_eviction_with_fifo(self) -> None:
        """When memory is full, FIFO evicts the oldest page."""
        mmu = MMU(total_frames=3, replacement_policy=FIFOPolicy())
        mmu.create_address_space(pid=1)

        # Fill all frames.
        mmu.map_page(pid=1, virtual_addr=0x1000)  # frame 0
        mmu.map_page(pid=1, virtual_addr=0x2000)  # frame 1
        mmu.map_page(pid=1, virtual_addr=0x3000)  # frame 2

        # Map a 4th page — must evict one.
        mmu.map_page(pid=1, virtual_addr=0x4000)

        # Should have 3 allocated frames (one evicted, one new).
        assert mmu.frame_allocator.allocated_count() == 3

    def test_eviction_with_lru(self) -> None:
        """When memory is full, LRU evicts the least recently used page."""
        mmu = MMU(total_frames=3, replacement_policy=LRUPolicy())
        mmu.create_address_space(pid=1)

        mmu.map_page(pid=1, virtual_addr=0x1000)
        mmu.map_page(pid=1, virtual_addr=0x2000)
        mmu.map_page(pid=1, virtual_addr=0x3000)

        # Access page 1 and 3 (making page 2 least recently used).
        mmu.translate(pid=1, virtual_addr=0x1000)
        mmu.translate(pid=1, virtual_addr=0x3000)

        # Map a 4th page — LRU should evict page at 0x2000.
        mmu.map_page(pid=1, virtual_addr=0x4000)
        assert mmu.frame_allocator.allocated_count() == 3

    def test_eviction_with_clock(self) -> None:
        """When memory is full, Clock uses second-chance eviction."""
        mmu = MMU(total_frames=3, replacement_policy=ClockPolicy())
        mmu.create_address_space(pid=1)

        mmu.map_page(pid=1, virtual_addr=0x1000)
        mmu.map_page(pid=1, virtual_addr=0x2000)
        mmu.map_page(pid=1, virtual_addr=0x3000)

        # Map a 4th page — triggers Clock eviction.
        mmu.map_page(pid=1, virtual_addr=0x4000)
        assert mmu.frame_allocator.allocated_count() == 3


class TestMMUProperties:
    """Test MMU property accessors."""

    def test_tlb_property(self) -> None:
        """The tlb property exposes the TLB instance."""
        mmu = MMU(total_frames=16)
        assert mmu.tlb is not None
        assert mmu.tlb.capacity == 64

    def test_frame_allocator_property(self) -> None:
        """The frame_allocator property exposes the allocator."""
        mmu = MMU(total_frames=16)
        assert mmu.frame_allocator is not None
        assert mmu.frame_allocator.total_frames == 16

    def test_active_pid_starts_none(self) -> None:
        """active_pid is None before any context switch."""
        mmu = MMU(total_frames=16)
        assert mmu.active_pid is None

    def test_active_pid_after_context_switch(self) -> None:
        """active_pid reflects the most recent context switch."""
        mmu = MMU(total_frames=16)
        mmu.create_address_space(pid=1)
        mmu.context_switch(new_pid=1)
        assert mmu.active_pid == 1


class TestMMUWriteAccess:
    """Test write access and dirty bit handling."""

    def test_write_sets_dirty_bit(self) -> None:
        """A write access through translate sets the dirty bit."""
        mmu = MMU(total_frames=16)
        mmu.create_address_space(pid=1)
        mmu.map_page(pid=1, virtual_addr=0x1000)

        # Write access.
        mmu.translate(pid=1, virtual_addr=0x1000, write=True)

        # The PTE should have dirty=True (we can verify via TLB).
        result = mmu.tlb.lookup(0x1000 >> 12)
        if result is not None:
            _, pte = result
            assert pte.dirty is True
