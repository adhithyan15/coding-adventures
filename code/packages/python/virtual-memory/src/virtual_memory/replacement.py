"""Page Replacement Policies — FIFO, LRU, and Clock (Second Chance).

When physical memory is full and a new page needs to be loaded, the operating
system must choose which existing page to EVICT (remove from memory) to make
room. The choice of which page to evict has a dramatic impact on performance:

    - Evict a page that will be needed soon -> immediate page fault (bad!)
    - Evict a page that won't be needed for a long time -> smooth sailing

The optimal algorithm (Belady's MIN) would evict the page that won't be used
for the longest time in the future. But we can't see the future, so we use
heuristics based on past access patterns.

Comparison of Policies
======================

    +--------+------------+---------------------------------------------+
    | Policy | Complexity | Strategy                                    |
    +--------+------------+---------------------------------------------+
    | FIFO   | O(1)       | Evict the oldest page (first loaded)        |
    | LRU    | O(n)       | Evict the least recently accessed page      |
    | Clock  | O(n) worst | LRU approximation using a "use bit"         |
    +--------+------------+---------------------------------------------+

FIFO is simplest but can evict hot pages. LRU is best in practice but
expensive to maintain. Clock is a practical compromise used by real OSes.

Protocol (Interface)
====================

All three policies implement the same interface:

    record_access(frame)  - Note that a frame was accessed (for LRU/Clock)
    select_victim()       - Choose a frame to evict
    add_frame(frame)      - Register a new frame (just allocated)
    remove_frame(frame)   - Remove a frame from tracking (freed externally)
"""

from typing import Protocol, runtime_checkable


@runtime_checkable
class ReplacementPolicy(Protocol):
    """Protocol defining the interface for page replacement policies.

    Any class that implements these four methods can serve as a replacement
    policy for the MMU. This is Python's version of an interface — it uses
    structural subtyping (duck typing with type checking).
    """

    def record_access(self, frame: int) -> None:
        """Record that a frame was accessed (read or write).

        Used by LRU and Clock to track access patterns. FIFO ignores this
        because it only cares about insertion order, not access order.

        Args:
            frame: The physical frame number that was accessed.
        """
        ...

    def select_victim(self) -> int | None:
        """Select a frame to evict.

        Returns the frame number of the victim page, or None if there are
        no frames being tracked (nothing to evict).

        Returns:
            Frame number to evict, or None.
        """
        ...

    def add_frame(self, frame: int) -> None:
        """Register a newly allocated frame for tracking.

        Called when a new frame is brought into memory. The policy starts
        tracking it for future eviction decisions.

        Args:
            frame: The physical frame number that was just allocated.
        """
        ...

    def remove_frame(self, frame: int) -> None:
        """Stop tracking a frame (it was freed externally).

        Called when a frame is freed by the MMU (e.g., process exit).
        The policy should remove it from its internal data structures.

        Args:
            frame: The physical frame number to stop tracking.
        """
        ...


# =============================================================================
# FIFO — First-In, First-Out
# =============================================================================


class FIFOPolicy:
    """First-In, First-Out page replacement.

    The simplest policy: always evict the page that has been in memory the
    longest. This is like a queue at a grocery store — the person who arrived
    first gets served (evicted) first.

    Pros:
        - Extremely simple to implement (just a queue)
        - O(1) eviction (pop from front)
        - No overhead on memory accesses (no access tracking needed)

    Cons:
        - Can evict frequently-used pages just because they were loaded
          a long time ago
        - Belady's anomaly: adding more frames can actually INCREASE the
          number of page faults (counterintuitive!)

    Example:
        Queue: [A, B, C, D]  (A is oldest)
        Need to evict -> evict A
        Load new page E -> Queue: [B, C, D, E]
    """

    def __init__(self) -> None:
        """Initialize with an empty queue."""
        # The queue tracks frames in insertion order.
        # Front = oldest (first to be evicted).
        # Back = newest (last to be evicted).
        self._queue: list[int] = []

    def record_access(self, frame: int) -> None:
        """FIFO ignores access events.

        FIFO only cares about when a page was LOADED, not when it was last
        ACCESSED. This is what makes it simple but suboptimal — a frequently
        accessed page gets no special treatment.

        Args:
            frame: The frame that was accessed (ignored).
        """
        # Intentionally empty — FIFO does not track accesses.

    def select_victim(self) -> int | None:
        """Evict the oldest frame (first one loaded).

        Removes and returns the frame at the front of the queue.

        Returns:
            Frame number of the oldest page, or None if the queue is empty.
        """
        if not self._queue:
            return None
        return self._queue.pop(0)

    def add_frame(self, frame: int) -> None:
        """Add a newly loaded frame to the back of the queue.

        Args:
            frame: The physical frame number that was just loaded.
        """
        self._queue.append(frame)

    def remove_frame(self, frame: int) -> None:
        """Remove a frame from tracking.

        Args:
            frame: The frame to remove.
        """
        if frame in self._queue:
            self._queue.remove(frame)


# =============================================================================
# LRU — Least Recently Used
# =============================================================================


class LRUPolicy:
    """Least Recently Used page replacement.

    Evicts the page that hasn't been accessed for the longest time. Based on
    the principle of TEMPORAL LOCALITY: if a page was used recently, it will
    probably be used again soon. Conversely, if a page hasn't been used for
    a long time, it probably won't be needed soon.

    Implementation:
        We maintain a logical clock that increments on every access. Each
        frame records the clock value of its last access. To find the victim,
        we pick the frame with the smallest (oldest) clock value.

    Pros:
        - Generally the best-performing policy
        - No Belady's anomaly (more frames always helps)

    Cons:
        - Every memory access must update the clock — O(1) per access but
          adds overhead to every single memory operation
        - select_victim() is O(n) — must scan all frames to find the minimum

    Example:
        Access order: [C:1, A:3, D:2, B:4]  (timestamps)
        C was accessed at time 1 (oldest) -> evict C
    """

    def __init__(self) -> None:
        """Initialize with an empty access time map and clock at 0."""
        # Maps frame_number -> last access timestamp.
        self._access_times: dict[int, int] = {}

        # Logical clock — increments on every record_access() call.
        # Not tied to real time; just a monotonically increasing counter
        # that gives us a total ordering of accesses.
        self._clock: int = 0

    def record_access(self, frame: int) -> None:
        """Record that a frame was accessed, updating its timestamp.

        The frame's timestamp is set to the current clock value, then the
        clock advances. This ensures that the most recently accessed frame
        always has the highest timestamp.

        Args:
            frame: The physical frame that was accessed.
        """
        self._access_times[frame] = self._clock
        self._clock += 1

    def select_victim(self) -> int | None:
        """Evict the frame with the oldest (smallest) access timestamp.

        Scans all tracked frames to find the one with the minimum timestamp.
        This is O(n) where n is the number of tracked frames.

        Returns:
            Frame number of the least recently used page, or None if empty.
        """
        if not self._access_times:
            return None

        # Find the frame with the minimum (oldest) access time.
        victim = min(self._access_times, key=self._access_times.get)  # type: ignore[arg-type]
        del self._access_times[victim]
        return victim

    def add_frame(self, frame: int) -> None:
        """Register a new frame with the current timestamp.

        Args:
            frame: The physical frame that was just allocated.
        """
        self._access_times[frame] = self._clock
        self._clock += 1

    def remove_frame(self, frame: int) -> None:
        """Stop tracking a frame.

        Args:
            frame: The frame to remove from tracking.
        """
        self._access_times.pop(frame, None)


# =============================================================================
# Clock — Second Chance
# =============================================================================


class ClockPolicy:
    """Clock (Second Chance) page replacement.

    A practical approximation of LRU used by real operating systems (Linux,
    Windows, macOS). Pages are arranged in a circular buffer, and a "clock
    hand" sweeps around looking for a victim.

    Each page has a USE BIT:
        - use_bit = True:  page was recently accessed -> give it a second chance
        - use_bit = False: page was NOT recently accessed -> evict it

    When looking for a victim, the clock hand moves around the circle:
        1. Look at the page under the hand.
        2. If use_bit is False -> EVICT this page (it wasn't used recently).
        3. If use_bit is True  -> CLEAR the bit (second chance) and move on.
        4. Repeat until a victim is found.

    The "second chance" name comes from step 3: a page with its use bit set
    gets one more pass before eviction. If it is accessed again before the
    hand comes back around, its use bit will be set again and it survives
    another round.

    Visualization:
            +---+
        +---| A |<-- use=1 -> clear, move on
        |   |   |
        |   +---+
        |     |
      +-+-+   |  +---+
      | D |   +--| B |<-- use=0 -> EVICT
      |   |      |   |
      +---+      +---+
        |          |
        |   +---+  |
        +---| C |--+
            |   |
            +---+

    Pros:
        - O(1) amortized eviction (usually finds a victim quickly)
        - Very low overhead on memory accesses (just set a bit)
        - Good approximation of LRU in practice

    Cons:
        - Worst case O(n) if all use bits are set (must clear them all)
        - Not as optimal as true LRU
    """

    def __init__(self) -> None:
        """Initialize with an empty circular buffer and hand at position 0."""
        # The circular buffer of frame numbers.
        self._frames: list[int] = []

        # Use bits: True = recently accessed, False = not recently accessed.
        self._use_bits: dict[int, bool] = {}

        # The clock hand position (index into _frames).
        self._hand: int = 0

    def record_access(self, frame: int) -> None:
        """Set the use bit for an accessed frame.

        When a page is accessed (read or write), the hardware sets its use
        bit to True. This tells the clock algorithm that the page was
        recently used and should get a second chance before eviction.

        Args:
            frame: The physical frame that was accessed.
        """
        if frame in self._use_bits:
            self._use_bits[frame] = True

    def select_victim(self) -> int | None:
        """Find a victim using the clock (second chance) algorithm.

        Sweeps the clock hand around the circular buffer:
        - If use_bit is False -> evict this frame.
        - If use_bit is True  -> clear the bit and advance the hand.

        Returns:
            Frame number of the evicted page, or None if no frames are tracked.
        """
        if not self._frames:
            return None

        # In the worst case, we need to go around the entire circle twice:
        # once to clear all use bits, once more to find a cleared one.
        max_iterations = len(self._frames) * 2

        for _ in range(max_iterations):
            # Wrap the hand around the circular buffer.
            if self._hand >= len(self._frames):
                self._hand = 0

            frame = self._frames[self._hand]

            if not self._use_bits.get(frame, False):
                # Use bit is clear -> evict this frame.
                self._frames.pop(self._hand)
                del self._use_bits[frame]
                # Don't advance hand — the next frame slides into this position.
                if self._hand >= len(self._frames) and self._frames:
                    self._hand = 0
                return frame

            # Use bit is set -> give it a second chance.
            # Clear the bit and move to the next frame.
            self._use_bits[frame] = False
            self._hand += 1

        # Should never reach here unless the buffer is empty.
        return None  # pragma: no cover

    def add_frame(self, frame: int) -> None:
        """Add a newly loaded frame to the clock buffer.

        New frames start with use_bit = True (they were just loaded, so
        they were "accessed").

        Args:
            frame: The physical frame that was just allocated.
        """
        self._frames.append(frame)
        self._use_bits[frame] = True

    def remove_frame(self, frame: int) -> None:
        """Remove a frame from the clock buffer.

        Args:
            frame: The frame to stop tracking.
        """
        if frame in self._use_bits:
            del self._use_bits[frame]
        if frame in self._frames:
            idx = self._frames.index(frame)
            self._frames.remove(frame)
            # Adjust hand if it was pointing past the removed element.
            if idx < self._hand:
                self._hand -= 1
            if self._hand >= len(self._frames) and self._frames:
                self._hand = 0
