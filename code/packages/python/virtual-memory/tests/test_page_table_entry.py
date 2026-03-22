"""Tests for PageTableEntry — the metadata for a single page mapping."""

from virtual_memory.page_table_entry import (
    PAGE_OFFSET_BITS,
    PAGE_SIZE,
    VPN_BITS,
    PageTableEntry,
)


class TestConstants:
    """Verify the fundamental page geometry constants."""

    def test_page_size_is_4kb(self) -> None:
        """4 KB pages are the standard since the Intel 386 (1985)."""
        assert PAGE_SIZE == 4096

    def test_page_offset_bits_is_12(self) -> None:
        """12 bits address every byte within a 4 KB page (2^12 = 4096)."""
        assert PAGE_OFFSET_BITS == 12

    def test_vpn_bits_is_20(self) -> None:
        """20 bits for VPN: 2^20 pages * 4 KB = 4 GB address space."""
        assert VPN_BITS == 20

    def test_page_size_matches_offset_bits(self) -> None:
        """The page size must be 2^offset_bits."""
        assert PAGE_SIZE == (1 << PAGE_OFFSET_BITS)

    def test_total_address_bits(self) -> None:
        """VPN bits + offset bits = 32 (full 32-bit address space)."""
        assert VPN_BITS + PAGE_OFFSET_BITS == 32


class TestPageTableEntry:
    """Test PTE creation and flag behavior."""

    def test_default_values(self) -> None:
        """A fresh PTE has sensible defaults: not present, frame 0."""
        pte = PageTableEntry()
        assert pte.frame_number == 0
        assert pte.present is False
        assert pte.dirty is False
        assert pte.accessed is False
        assert pte.writable is True
        assert pte.executable is False
        assert pte.user_accessible is True

    def test_custom_frame_number(self) -> None:
        """Frame number can be set at creation."""
        pte = PageTableEntry(frame_number=42)
        assert pte.frame_number == 42

    def test_present_flag(self) -> None:
        """The present flag indicates the page is in physical memory."""
        pte = PageTableEntry(present=True)
        assert pte.present is True

    def test_dirty_flag(self) -> None:
        """The dirty flag indicates the page has been written to."""
        pte = PageTableEntry(dirty=True)
        assert pte.dirty is True

    def test_accessed_flag(self) -> None:
        """The accessed flag indicates the page was recently used."""
        pte = PageTableEntry(accessed=True)
        assert pte.accessed is True

    def test_writable_flag(self) -> None:
        """Pages can be made read-only (e.g., code pages)."""
        pte = PageTableEntry(writable=False)
        assert pte.writable is False

    def test_executable_flag(self) -> None:
        """The executable flag controls the NX (no-execute) bit."""
        pte = PageTableEntry(executable=True)
        assert pte.executable is True

    def test_user_accessible_flag(self) -> None:
        """Kernel pages are not user-accessible."""
        pte = PageTableEntry(user_accessible=False)
        assert pte.user_accessible is False

    def test_all_flags_set(self) -> None:
        """All flags can be set simultaneously."""
        pte = PageTableEntry(
            frame_number=100,
            present=True,
            dirty=True,
            accessed=True,
            writable=True,
            executable=True,
            user_accessible=True,
        )
        assert pte.frame_number == 100
        assert pte.present is True
        assert pte.dirty is True
        assert pte.accessed is True
        assert pte.writable is True
        assert pte.executable is True
        assert pte.user_accessible is True

    def test_flags_are_mutable(self) -> None:
        """PTE flags can be changed after creation (hardware does this)."""
        pte = PageTableEntry()
        pte.accessed = True
        pte.dirty = True
        assert pte.accessed is True
        assert pte.dirty is True

    def test_copy_creates_independent_instance(self) -> None:
        """copy() creates a deep copy — modifying one doesn't affect the other."""
        original = PageTableEntry(frame_number=5, present=True, writable=True)
        copy = original.copy()

        # Same values initially.
        assert copy.frame_number == 5
        assert copy.present is True
        assert copy.writable is True

        # Modify the copy — original should be unaffected.
        copy.frame_number = 99
        copy.writable = False
        assert original.frame_number == 5
        assert original.writable is True

    def test_copy_preserves_all_flags(self) -> None:
        """copy() preserves every single flag."""
        original = PageTableEntry(
            frame_number=42,
            present=True,
            dirty=True,
            accessed=True,
            writable=False,
            executable=True,
            user_accessible=False,
        )
        copy = original.copy()
        assert copy.frame_number == 42
        assert copy.present is True
        assert copy.dirty is True
        assert copy.accessed is True
        assert copy.writable is False
        assert copy.executable is True
        assert copy.user_accessible is False
