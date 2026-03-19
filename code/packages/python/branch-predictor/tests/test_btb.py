"""Tests for the Branch Target Buffer (BTB).

The BTB caches branch target addresses — it answers "WHERE does the branch go?"
These tests verify lookup/update behavior, eviction, aliasing, and statistics.
"""

from __future__ import annotations

from branch_predictor.btb import BTBEntry, BranchTargetBuffer


class TestBTBLookupMiss:
    """Cold-start behavior — lookups before any updates."""

    def test_lookup_miss_on_cold_start(self) -> None:
        """Fresh BTB returns None for any lookup."""
        btb = BranchTargetBuffer(size=256)
        assert btb.lookup(pc=0x100) is None

    def test_miss_increments_counter(self) -> None:
        btb = BranchTargetBuffer(size=256)
        btb.lookup(pc=0x100)
        assert btb.misses == 1
        assert btb.hits == 0
        assert btb.lookups == 1


class TestBTBUpdateAndHit:
    """Update the BTB, then look up — should hit."""

    def test_update_then_lookup_hits(self) -> None:
        btb = BranchTargetBuffer(size=256)
        btb.update(pc=0x100, target=0x200, branch_type="conditional")
        target = btb.lookup(pc=0x100)
        assert target == 0x200

    def test_hit_increments_counter(self) -> None:
        btb = BranchTargetBuffer(size=256)
        btb.update(pc=0x100, target=0x200)
        btb.lookup(pc=0x100)
        assert btb.hits == 1
        assert btb.misses == 0

    def test_update_overwrites_previous_target(self) -> None:
        """Updating the same branch with a new target overwrites the old one."""
        btb = BranchTargetBuffer(size=256)
        btb.update(pc=0x100, target=0x200)
        btb.update(pc=0x100, target=0x300)
        assert btb.lookup(pc=0x100) == 0x300

    def test_multiple_branches(self) -> None:
        """Multiple different branches can coexist in the BTB."""
        btb = BranchTargetBuffer(size=256)
        # Use addresses that don't alias (different index = pc % 256)
        btb.update(pc=0x01, target=0x200)
        btb.update(pc=0x02, target=0x400)
        btb.update(pc=0x03, target=0x600)

        assert btb.lookup(pc=0x01) == 0x200
        assert btb.lookup(pc=0x02) == 0x400
        assert btb.lookup(pc=0x03) == 0x600


class TestBTBBranchTypes:
    """Branch type tracking in BTB entries."""

    def test_default_branch_type(self) -> None:
        btb = BranchTargetBuffer(size=256)
        btb.update(pc=0x100, target=0x200)
        entry = btb.get_entry(pc=0x100)
        assert entry is not None
        assert entry.branch_type == "conditional"

    def test_custom_branch_types(self) -> None:
        btb = BranchTargetBuffer(size=256)
        for pc, btype in [
            (0x01, "conditional"),
            (0x02, "unconditional"),
            (0x03, "call"),
            (0x04, "return"),
        ]:
            btb.update(pc=pc, target=pc + 0x100, branch_type=btype)

        for pc, btype in [
            (0x01, "conditional"),
            (0x02, "unconditional"),
            (0x03, "call"),
            (0x04, "return"),
        ]:
            entry = btb.get_entry(pc=pc)
            assert entry is not None
            assert entry.branch_type == btype


class TestBTBEviction:
    """Eviction due to aliasing (direct-mapped conflict)."""

    def test_eviction_on_aliasing(self) -> None:
        """Two branches aliasing to the same slot: second evicts first.

        With size=4:
          Branch A at 0x100 → index 0 (0x100 % 4 = 0)
          Branch B at 0x104 → index 0 (0x104 % 4 = 0)
        """
        btb = BranchTargetBuffer(size=4)
        btb.update(pc=0x100, target=0x200)
        assert btb.lookup(pc=0x100) == 0x200

        # Branch B evicts Branch A
        btb.update(pc=0x104, target=0x300)
        assert btb.lookup(pc=0x104) == 0x300

        # Branch A is now evicted — tag mismatch → miss
        assert btb.lookup(pc=0x100) is None

    def test_no_eviction_with_large_table(self) -> None:
        """Large table avoids aliasing for nearby branches."""
        btb = BranchTargetBuffer(size=4096)
        btb.update(pc=0x100, target=0x200)
        btb.update(pc=0x104, target=0x300)
        assert btb.lookup(pc=0x100) == 0x200
        assert btb.lookup(pc=0x104) == 0x300


class TestBTBEntry:
    """Tests for the BTBEntry dataclass."""

    def test_default_entry(self) -> None:
        entry = BTBEntry()
        assert entry.valid is False
        assert entry.tag == 0
        assert entry.target == 0
        assert entry.branch_type == ""

    def test_custom_entry(self) -> None:
        entry = BTBEntry(valid=True, tag=0x100, target=0x200, branch_type="call")
        assert entry.valid is True
        assert entry.tag == 0x100
        assert entry.target == 0x200
        assert entry.branch_type == "call"


class TestBTBGetEntry:
    """Tests for the get_entry debug/inspection method."""

    def test_get_entry_returns_none_on_miss(self) -> None:
        btb = BranchTargetBuffer(size=256)
        assert btb.get_entry(pc=0x100) is None

    def test_get_entry_returns_entry_on_hit(self) -> None:
        btb = BranchTargetBuffer(size=256)
        btb.update(pc=0x100, target=0x200, branch_type="unconditional")
        entry = btb.get_entry(pc=0x100)
        assert entry is not None
        assert entry.valid is True
        assert entry.tag == 0x100
        assert entry.target == 0x200

    def test_get_entry_returns_none_on_tag_mismatch(self) -> None:
        """An occupied entry with wrong tag → returns None."""
        btb = BranchTargetBuffer(size=4)
        btb.update(pc=0x100, target=0x200)  # index 0
        # PC 0x104 also maps to index 0 but different tag
        assert btb.get_entry(pc=0x104) is None


class TestBTBStatistics:
    """Test hit rate and counter statistics."""

    def test_hit_rate_zero_lookups(self) -> None:
        btb = BranchTargetBuffer(size=256)
        assert btb.hit_rate == 0.0

    def test_hit_rate_all_hits(self) -> None:
        btb = BranchTargetBuffer(size=256)
        btb.update(pc=0x100, target=0x200)
        for _ in range(10):
            btb.lookup(pc=0x100)
        assert btb.hit_rate == 100.0

    def test_hit_rate_all_misses(self) -> None:
        btb = BranchTargetBuffer(size=256)
        for _ in range(10):
            btb.lookup(pc=0x100)
        assert btb.hit_rate == 0.0

    def test_hit_rate_mixed(self) -> None:
        btb = BranchTargetBuffer(size=256)
        btb.update(pc=0x100, target=0x200)
        btb.lookup(pc=0x100)  # hit
        btb.lookup(pc=0x100)  # hit
        btb.lookup(pc=0x200)  # miss (never updated)
        btb.lookup(pc=0x300)  # miss
        assert btb.hit_rate == 50.0


class TestBTBReset:
    """Reset clears all entries and statistics."""

    def test_reset_clears_entries(self) -> None:
        btb = BranchTargetBuffer(size=256)
        btb.update(pc=0x100, target=0x200)
        btb.reset()
        assert btb.lookup(pc=0x100) is None

    def test_reset_clears_statistics(self) -> None:
        btb = BranchTargetBuffer(size=256)
        btb.update(pc=0x100, target=0x200)
        btb.lookup(pc=0x100)
        btb.reset()
        assert btb.lookups == 0
        assert btb.hits == 0
        assert btb.misses == 0
