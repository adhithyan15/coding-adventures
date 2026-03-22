"""Tests for TwoLevelPageTable — Sv32 two-level page table."""

from virtual_memory.multi_level_page_table import TwoLevelPageTable


class TestAddressSplitting:
    """Verify the L1/L2/offset extraction from virtual addresses."""

    def test_split_zero_address(self) -> None:
        """Address 0x00000000 splits into all zeros."""
        l1, l2, offset = TwoLevelPageTable._split_address(0x00000000)
        assert l1 == 0
        assert l2 == 0
        assert offset == 0

    def test_split_simple_address(self) -> None:
        """Address 0x00012ABC: VPN[1]=0, VPN[0]=0x12, offset=0xABC."""
        l1, l2, offset = TwoLevelPageTable._split_address(0x00012ABC)
        assert l1 == 0  # bits 31-22 = 0
        assert l2 == 0x12  # bits 21-12 = 18
        assert offset == 0xABC  # bits 11-0 = 2748

    def test_split_high_address(self) -> None:
        """Address with nonzero L1 index."""
        # 0x00400000 = bit 22 set -> L1 = 1
        l1, l2, offset = TwoLevelPageTable._split_address(0x00400000)
        assert l1 == 1
        assert l2 == 0
        assert offset == 0

    def test_split_max_address(self) -> None:
        """Address 0xFFFFFFFF: all bits set."""
        l1, l2, offset = TwoLevelPageTable._split_address(0xFFFFFFFF)
        assert l1 == 0x3FF  # all 10 bits set = 1023
        assert l2 == 0x3FF  # all 10 bits set = 1023
        assert offset == 0xFFF  # all 12 bits set = 4095

    def test_split_only_offset(self) -> None:
        """An address within page 0 has only offset bits set."""
        l1, l2, offset = TwoLevelPageTable._split_address(0x00000ABC)
        assert l1 == 0
        assert l2 == 0
        assert offset == 0xABC

    def test_split_l2_boundary(self) -> None:
        """Address at the L2 boundary (bit 12)."""
        # 0x1000 = VPN[0] = 1, offset = 0
        l1, l2, offset = TwoLevelPageTable._split_address(0x00001000)
        assert l1 == 0
        assert l2 == 1
        assert offset == 0

    def test_split_l1_boundary(self) -> None:
        """Address at the L1 boundary (bit 22)."""
        # 0x00800000 = L1 = 2
        l1, l2, offset = TwoLevelPageTable._split_address(0x00800000)
        assert l1 == 2
        assert l2 == 0
        assert offset == 0


class TestTwoLevelPageTable:
    """Test the two-level page table operations."""

    def test_empty_table_translate(self) -> None:
        """Translating in an empty table returns None."""
        pt = TwoLevelPageTable()
        assert pt.translate(0x1000) is None

    def test_map_and_translate(self) -> None:
        """Map a page and verify translation produces correct physical address."""
        pt = TwoLevelPageTable()
        pt.map(0x00001000, physical_frame=42)

        result = pt.translate(0x00001ABC)
        assert result is not None
        physical_addr, pte = result
        # physical = (42 << 12) | 0xABC = 0x2AABC
        assert physical_addr == (42 << 12) | 0xABC
        assert pte.frame_number == 42
        assert pte.present is True

    def test_translate_unmapped_region(self) -> None:
        """Translating an address in an unmapped L1 region returns None."""
        pt = TwoLevelPageTable()
        pt.map(0x00001000, physical_frame=1)
        # Address in a different L1 region.
        assert pt.translate(0x00400000) is None

    def test_translate_unmapped_page_in_mapped_region(self) -> None:
        """Translating an unmapped page within a mapped L1 region returns None."""
        pt = TwoLevelPageTable()
        pt.map(0x00001000, physical_frame=1)
        # Same L1 region (0), different L2 index.
        assert pt.translate(0x00002000) is None

    def test_multiple_pages_same_region(self) -> None:
        """Multiple pages in the same L1 region share one L2 table."""
        pt = TwoLevelPageTable()
        pt.map(0x00001000, physical_frame=10)  # L2 index 1
        pt.map(0x00002000, physical_frame=20)  # L2 index 2

        r1 = pt.translate(0x00001000)
        r2 = pt.translate(0x00002000)
        assert r1 is not None
        assert r2 is not None
        assert r1[1].frame_number == 10
        assert r2[1].frame_number == 20

    def test_multiple_pages_different_regions(self) -> None:
        """Pages in different L1 regions create separate L2 tables."""
        pt = TwoLevelPageTable()
        pt.map(0x00001000, physical_frame=10)  # L1 = 0
        pt.map(0x00401000, physical_frame=20)  # L1 = 1

        r1 = pt.translate(0x00001000)
        r2 = pt.translate(0x00401000)
        assert r1 is not None and r1[1].frame_number == 10
        assert r2 is not None and r2[1].frame_number == 20

    def test_unmap(self) -> None:
        """Unmapping a page removes the translation."""
        pt = TwoLevelPageTable()
        pt.map(0x00001000, physical_frame=5)

        pte = pt.unmap(0x00001000)
        assert pte is not None
        assert pte.frame_number == 5
        assert pt.translate(0x00001000) is None

    def test_unmap_unmapped_returns_none(self) -> None:
        """Unmapping an unmapped address returns None."""
        pt = TwoLevelPageTable()
        assert pt.unmap(0x00001000) is None

    def test_unmap_from_unmapped_region(self) -> None:
        """Unmapping from an L1 region with no L2 table returns None."""
        pt = TwoLevelPageTable()
        assert pt.unmap(0x00400000) is None

    def test_permissions_preserved(self) -> None:
        """Permissions set during map() are preserved in the PTE."""
        pt = TwoLevelPageTable()
        pt.map(
            0x00005000, physical_frame=1,
            writable=False, executable=True, user=False,
        )

        result = pt.translate(0x00005000)
        assert result is not None
        _, pte = result
        assert pte.writable is False
        assert pte.executable is True
        assert pte.user_accessible is False

    def test_lookup_vpn(self) -> None:
        """lookup_vpn() works with pre-extracted VPN."""
        pt = TwoLevelPageTable()
        pt.map(0x00005000, physical_frame=42)

        vpn = 0x00005000 >> 12  # = 5
        pte = pt.lookup_vpn(vpn)
        assert pte is not None
        assert pte.frame_number == 42

    def test_lookup_vpn_unmapped(self) -> None:
        """lookup_vpn() returns None for unmapped VPN."""
        pt = TwoLevelPageTable()
        assert pt.lookup_vpn(999) is None

    def test_map_vpn(self) -> None:
        """map_vpn() maps using a pre-extracted VPN."""
        pt = TwoLevelPageTable()
        pt.map_vpn(vpn=5, physical_frame=42)

        result = pt.translate(0x00005000)
        assert result is not None
        assert result[1].frame_number == 42

    def test_offset_preserved_in_translation(self) -> None:
        """The page offset is carried through to the physical address."""
        pt = TwoLevelPageTable()
        pt.map(0x00003000, physical_frame=7)

        # Access byte 0x123 within page 3.
        result = pt.translate(0x00003123)
        assert result is not None
        physical_addr, _ = result
        # physical = (7 << 12) | 0x123 = 0x7123
        assert physical_addr == 0x7123

    def test_directory_property(self) -> None:
        """directory property gives raw access for iteration."""
        pt = TwoLevelPageTable()
        assert len(pt.directory) == 1024
        assert all(entry is None for entry in pt.directory)

        pt.map(0x00001000, physical_frame=1)
        assert pt.directory[0] is not None
