"""Tests for PageTable — single-level VPN -> PTE dictionary."""

from virtual_memory.page_table import PageTable


class TestPageTable:
    """Test the single-level page table operations."""

    def test_empty_page_table(self) -> None:
        """A fresh page table has no mappings."""
        pt = PageTable()
        assert pt.mapped_count() == 0
        assert pt.lookup(0) is None

    def test_map_and_lookup(self) -> None:
        """Mapping a page creates a PTE that can be looked up."""
        pt = PageTable()
        pt.map_page(vpn=5, frame=42)

        pte = pt.lookup(5)
        assert pte is not None
        assert pte.frame_number == 42
        assert pte.present is True

    def test_lookup_unmapped_returns_none(self) -> None:
        """Looking up an unmapped VPN returns None."""
        pt = PageTable()
        pt.map_page(vpn=0, frame=0)
        assert pt.lookup(999) is None

    def test_map_with_permissions(self) -> None:
        """Permissions are set correctly during mapping."""
        pt = PageTable()
        pt.map_page(vpn=10, frame=20, writable=False, executable=True, user=False)

        pte = pt.lookup(10)
        assert pte is not None
        assert pte.writable is False
        assert pte.executable is True
        assert pte.user_accessible is False

    def test_unmap_returns_pte(self) -> None:
        """Unmapping a page returns the removed PTE."""
        pt = PageTable()
        pt.map_page(vpn=3, frame=7)

        pte = pt.unmap_page(3)
        assert pte is not None
        assert pte.frame_number == 7
        assert pt.lookup(3) is None

    def test_unmap_nonexistent_returns_none(self) -> None:
        """Unmapping a VPN that doesn't exist returns None."""
        pt = PageTable()
        assert pt.unmap_page(42) is None

    def test_mapped_count(self) -> None:
        """mapped_count() tracks the number of entries."""
        pt = PageTable()
        assert pt.mapped_count() == 0

        pt.map_page(vpn=0, frame=0)
        assert pt.mapped_count() == 1

        pt.map_page(vpn=1, frame=1)
        assert pt.mapped_count() == 2

        pt.unmap_page(0)
        assert pt.mapped_count() == 1

    def test_entries_returns_all_mappings(self) -> None:
        """entries() returns the full dictionary of mappings."""
        pt = PageTable()
        pt.map_page(vpn=10, frame=100)
        pt.map_page(vpn=20, frame=200)

        entries = pt.entries()
        assert len(entries) == 2
        assert 10 in entries
        assert 20 in entries
        assert entries[10].frame_number == 100
        assert entries[20].frame_number == 200

    def test_overwrite_mapping(self) -> None:
        """Mapping the same VPN again overwrites the previous mapping."""
        pt = PageTable()
        pt.map_page(vpn=5, frame=10)
        pt.map_page(vpn=5, frame=99)

        pte = pt.lookup(5)
        assert pte is not None
        assert pte.frame_number == 99

    def test_multiple_pages(self) -> None:
        """Multiple pages can be mapped independently."""
        pt = PageTable()
        for i in range(10):
            pt.map_page(vpn=i, frame=i * 10)

        assert pt.mapped_count() == 10
        for i in range(10):
            pte = pt.lookup(i)
            assert pte is not None
            assert pte.frame_number == i * 10
