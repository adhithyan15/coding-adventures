"""Tests for page replacement policies — FIFO, LRU, and Clock."""

from virtual_memory.replacement import (
    ClockPolicy,
    FIFOPolicy,
    LRUPolicy,
    ReplacementPolicy,
)


class TestFIFOPolicy:
    """Test First-In, First-Out page replacement."""

    def test_implements_protocol(self) -> None:
        """FIFOPolicy satisfies the ReplacementPolicy protocol."""
        assert isinstance(FIFOPolicy(), ReplacementPolicy)

    def test_select_victim_empty(self) -> None:
        """No victim when no frames are tracked."""
        fifo = FIFOPolicy()
        assert fifo.select_victim() is None

    def test_evicts_oldest(self) -> None:
        """FIFO evicts the first frame that was added."""
        fifo = FIFOPolicy()
        fifo.add_frame(10)
        fifo.add_frame(20)
        fifo.add_frame(30)

        assert fifo.select_victim() == 10  # oldest
        assert fifo.select_victim() == 20  # next oldest
        assert fifo.select_victim() == 30

    def test_access_does_not_change_order(self) -> None:
        """FIFO ignores access events — insertion order is all that matters."""
        fifo = FIFOPolicy()
        fifo.add_frame(10)
        fifo.add_frame(20)
        fifo.add_frame(30)

        # Access frame 10 — should NOT move it to the back.
        fifo.record_access(10)

        assert fifo.select_victim() == 10  # still first

    def test_remove_frame(self) -> None:
        """Removing a frame takes it out of the eviction queue."""
        fifo = FIFOPolicy()
        fifo.add_frame(10)
        fifo.add_frame(20)
        fifo.add_frame(30)

        fifo.remove_frame(20)  # remove middle

        assert fifo.select_victim() == 10
        assert fifo.select_victim() == 30
        assert fifo.select_victim() is None

    def test_remove_nonexistent_frame(self) -> None:
        """Removing a frame not in the queue is a no-op."""
        fifo = FIFOPolicy()
        fifo.remove_frame(999)  # should not raise


class TestLRUPolicy:
    """Test Least Recently Used page replacement."""

    def test_implements_protocol(self) -> None:
        """LRUPolicy satisfies the ReplacementPolicy protocol."""
        assert isinstance(LRUPolicy(), ReplacementPolicy)

    def test_select_victim_empty(self) -> None:
        """No victim when no frames are tracked."""
        lru = LRUPolicy()
        assert lru.select_victim() is None

    def test_evicts_least_recently_used(self) -> None:
        """LRU evicts the frame that hasn't been accessed longest."""
        lru = LRUPolicy()
        lru.add_frame(10)  # time 0
        lru.add_frame(20)  # time 1
        lru.add_frame(30)  # time 2

        # Frame 10 has the oldest timestamp -> evict it.
        assert lru.select_victim() == 10

    def test_access_changes_eviction_order(self) -> None:
        """Accessing a frame updates its timestamp, preventing eviction."""
        lru = LRUPolicy()
        lru.add_frame(10)  # time 0
        lru.add_frame(20)  # time 1
        lru.add_frame(30)  # time 2

        # Access frame 10, making it most recent.
        lru.record_access(10)  # time 3

        # Now frame 20 is the least recently used.
        assert lru.select_victim() == 20

    def test_multiple_accesses(self) -> None:
        """Multiple accesses correctly reorder eviction priority."""
        lru = LRUPolicy()
        lru.add_frame(1)
        lru.add_frame(2)
        lru.add_frame(3)

        lru.record_access(1)  # 1 is now most recent
        lru.record_access(2)  # 2 is now most recent

        # 3 has the oldest timestamp.
        assert lru.select_victim() == 3

    def test_remove_frame(self) -> None:
        """Removing a frame excludes it from eviction."""
        lru = LRUPolicy()
        lru.add_frame(10)
        lru.add_frame(20)

        lru.remove_frame(10)

        assert lru.select_victim() == 20

    def test_remove_nonexistent_frame(self) -> None:
        """Removing a frame not tracked is a no-op."""
        lru = LRUPolicy()
        lru.remove_frame(999)  # should not raise


class TestClockPolicy:
    """Test Clock (Second Chance) page replacement."""

    def test_implements_protocol(self) -> None:
        """ClockPolicy satisfies the ReplacementPolicy protocol."""
        assert isinstance(ClockPolicy(), ReplacementPolicy)

    def test_select_victim_empty(self) -> None:
        """No victim when no frames are tracked."""
        clock = ClockPolicy()
        assert clock.select_victim() is None

    def test_evicts_frame_with_cleared_use_bit(self) -> None:
        """A frame whose use bit is already clear gets evicted immediately."""
        clock = ClockPolicy()
        clock.add_frame(10)  # use_bit = True
        clock.add_frame(20)  # use_bit = True
        clock.add_frame(30)  # use_bit = True

        # Clear use bit on frame 10 (simulate the clock clearing it).
        clock._use_bits[10] = False

        # Frame 10 should be evicted (use bit is clear).
        assert clock.select_victim() == 10

    def test_second_chance_clears_bit(self) -> None:
        """Frames with use_bit=True get their bit cleared (second chance)."""
        clock = ClockPolicy()
        clock.add_frame(10)  # use_bit = True
        clock.add_frame(20)  # use_bit = True

        # Clear use bit for frame 20 only.
        clock._use_bits[20] = False

        # The hand starts at 0, finds frame 10 with use=True -> clears it.
        # Then finds frame 20 with use=False -> evicts it.
        assert clock.select_victim() == 20

        # Frame 10's use bit should now be False (it was given a second chance).
        assert clock._use_bits[10] is False

    def test_all_use_bits_set_wraps_around(self) -> None:
        """When all use bits are set, the hand wraps and clears them all."""
        clock = ClockPolicy()
        clock.add_frame(10)
        clock.add_frame(20)
        clock.add_frame(30)

        # All use bits are True. The hand clears them all, then evicts
        # the first one on the second pass.
        victim = clock.select_victim()
        assert victim == 10

    def test_access_sets_use_bit(self) -> None:
        """record_access() sets the use bit, protecting the frame."""
        clock = ClockPolicy()
        clock.add_frame(10)
        clock.add_frame(20)

        # Clear both use bits.
        clock._use_bits[10] = False
        clock._use_bits[20] = False

        # Access frame 10 — sets its use bit back to True.
        clock.record_access(10)

        # Frame 10 gets second chance, frame 20 gets evicted.
        assert clock.select_victim() == 20

    def test_remove_frame(self) -> None:
        """Removing a frame takes it out of the clock buffer."""
        clock = ClockPolicy()
        clock.add_frame(10)
        clock.add_frame(20)
        clock.add_frame(30)

        clock.remove_frame(20)

        # Clear all use bits for predictable eviction.
        clock._use_bits[10] = False
        clock._use_bits[30] = False

        assert clock.select_victim() == 10
        assert clock.select_victim() == 30
        assert clock.select_victim() is None

    def test_remove_nonexistent_frame(self) -> None:
        """Removing a frame not in the buffer is a no-op."""
        clock = ClockPolicy()
        clock.remove_frame(999)  # should not raise

    def test_sequential_evictions(self) -> None:
        """Multiple evictions work correctly in sequence."""
        clock = ClockPolicy()
        for i in range(5):
            clock.add_frame(i)

        # Clear all use bits.
        for i in range(5):
            clock._use_bits[i] = False

        # Should evict 0, 1, 2, 3, 4 in order.
        for i in range(5):
            assert clock.select_victim() == i
