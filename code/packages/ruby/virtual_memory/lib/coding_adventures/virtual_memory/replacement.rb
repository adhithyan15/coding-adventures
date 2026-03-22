# frozen_string_literal: true

# = Page Replacement Policies
#
# When physical memory is full and a new frame is needed, the OS must
# choose a page to evict. This is the page replacement problem -- one
# of the most studied problems in operating systems.
#
# The optimal algorithm (Belady's algorithm) would evict the page that
# won't be used for the longest time in the future. But we can't predict
# the future, so real systems use heuristics based on past behavior.
#
# == The Three Policies
#
#   +-------+------------------+--------------------+------------------+
#   | Policy| Eviction Rule    | Pros               | Cons             |
#   +-------+------------------+--------------------+------------------+
#   | FIFO  | Evict oldest     | Simple, O(1)       | Can evict hot    |
#   |       | page             | eviction           | pages            |
#   +-------+------------------+--------------------+------------------+
#   | LRU   | Evict least      | Good approximation | Expensive: every |
#   |       | recently used    | of optimal         | access must      |
#   |       |                  |                    | update timestamp |
#   +-------+------------------+--------------------+------------------+
#   | Clock | Evict page with  | Cheap LRU approx   | Slightly worse   |
#   |       | cleared accessed | using hardware bit | than true LRU    |
#   |       | bit              |                    |                  |
#   +-------+------------------+--------------------+------------------+

module CodingAdventures
  module VirtualMemory
    # = FIFO (First-In, First-Out) Page Replacement
    #
    # The simplest page replacement policy: evict the page that has been
    # in memory the longest. Uses a queue (FIFO order) to track arrival.
    #
    # == Example
    #
    # Given 3 frames and access sequence [A, B, C, D]:
    #   Load A: queue = [A]
    #   Load B: queue = [A, B]
    #   Load C: queue = [A, B, C]      -- memory full
    #   Load D: evict A (oldest), queue = [B, C, D]
    #
    # == Belady's Anomaly
    #
    # FIFO suffers from Belady's anomaly: adding more frames can actually
    # INCREASE the number of page faults! This counterintuitive behavior
    # occurs because FIFO doesn't consider how recently a page was used.
    class FIFOPolicy
      def initialize
        # Queue of frames in order of arrival.
        # Front of the queue = oldest = first to be evicted.
        @queue = []
      end

      # Record that a frame was accessed.
      #
      # FIFO ignores access patterns -- eviction is based solely on
      # arrival order. This method is a no-op for FIFO.
      #
      # @param frame [Integer] the frame that was accessed
      def record_access(frame)
        # FIFO does not track accesses; eviction is by arrival order only.
      end

      # Select a victim frame for eviction.
      #
      # Returns the oldest frame (front of the queue).
      #
      # @return [Integer, nil] the frame to evict, or nil if empty
      def select_victim
        @queue.shift
      end

      # Add a newly loaded frame to the replacement tracker.
      #
      # The frame goes to the back of the queue (newest).
      #
      # @param frame [Integer] the frame that was just loaded
      def add_frame(frame)
        @queue.push(frame)
      end

      # Remove a frame from the replacement tracker.
      #
      # Called when a frame is explicitly freed (not evicted).
      #
      # @param frame [Integer] the frame to remove
      def remove_frame(frame)
        @queue.delete(frame)
      end

      # How many frames are being tracked?
      #
      # @return [Integer] number of tracked frames
      def size
        @queue.size
      end
    end

    # = LRU (Least Recently Used) Page Replacement
    #
    # Evicts the page that has not been accessed for the longest time.
    # Based on the principle of temporal locality: recently used pages
    # are likely to be used again soon.
    #
    # == Implementation
    #
    # We maintain a logical clock that increments on every access.
    # Each frame records the clock value when it was last accessed.
    # The victim is the frame with the smallest (oldest) timestamp.
    #
    # == Example
    #
    # Given 3 frames and access sequence [A, B, C, A, D]:
    #   Load A: timestamps = {A:1}
    #   Load B: timestamps = {A:1, B:2}
    #   Load C: timestamps = {A:1, B:2, C:3}    -- memory full
    #   Access A: timestamps = {A:4, B:2, C:3}  -- A is refreshed
    #   Load D: evict B (timestamp 2, oldest), timestamps = {A:4, C:3, D:5}
    #
    # Note: B is evicted, not A, because A was re-accessed.
    class LRUPolicy
      def initialize
        # Maps frame number to its last access timestamp.
        @timestamps = {}

        # Logical clock -- incremented on every access.
        @clock = 0
      end

      # Record that a frame was accessed.
      #
      # Updates the frame's timestamp to the current clock value.
      # This "refreshes" the frame, moving it away from eviction.
      #
      # @param frame [Integer] the frame that was accessed
      def record_access(frame)
        @clock += 1
        @timestamps[frame] = @clock
      end

      # Select the least recently used frame for eviction.
      #
      # Finds the frame with the smallest (oldest) timestamp.
      #
      # @return [Integer, nil] the frame to evict, or nil if empty
      def select_victim
        return nil if @timestamps.empty?

        # Find the frame with the minimum timestamp (least recently used).
        victim = @timestamps.min_by { |_frame, timestamp| timestamp }
        frame = victim[0]
        @timestamps.delete(frame)
        frame
      end

      # Add a newly loaded frame to the replacement tracker.
      #
      # @param frame [Integer] the frame that was just loaded
      def add_frame(frame)
        @clock += 1
        @timestamps[frame] = @clock
      end

      # Remove a frame from the replacement tracker.
      #
      # @param frame [Integer] the frame to remove
      def remove_frame(frame)
        @timestamps.delete(frame)
      end

      # How many frames are being tracked?
      #
      # @return [Integer] number of tracked frames
      def size
        @timestamps.size
      end
    end

    # = Clock (Second-Chance) Page Replacement
    #
    # A practical approximation of LRU that is much cheaper to implement.
    # Instead of tracking the exact access time of every frame, it uses
    # a single "use bit" per frame.
    #
    # == How It Works
    #
    # Imagine the frames arranged in a circle with a clock hand pointing
    # at one of them:
    #
    #       +---+
    #   +---| A |<-- use_bit=1 → clear bit, move on
    #   |   |   |
    #   |   +---+
    #   |     |
    # +-+-+ | +---+
    # | D | +--| B |<-- use_bit=0 → EVICT THIS ONE
    # |   |    |   |
    # +---+    +---+
    #   |        |
    #   |  +---+ |
    #   +--| C |-+
    #      |   |
    #      +---+
    #
    # When we need a victim:
    #   1. Look at the frame under the clock hand.
    #   2. If its use bit is 0 → evict it.
    #   3. If its use bit is 1 → give it a "second chance":
    #      clear the bit and advance the hand.
    #   4. Repeat until we find a frame with use_bit=0.
    #
    # The name "second chance" comes from step 3: a page that was recently
    # accessed gets one more pass around the clock before eviction.
    class ClockPolicy
      def initialize
        # Circular buffer of frame numbers.
        @frames = []

        # Use bit for each frame. true = recently accessed.
        @use_bits = {}

        # The clock hand position (index into @frames).
        @hand = 0
      end

      # Record that a frame was accessed.
      #
      # Sets the frame's use bit to true. When the clock hand reaches
      # this frame, it will get a second chance instead of being evicted.
      #
      # @param frame [Integer] the frame that was accessed
      def record_access(frame)
        @use_bits[frame] = true
      end

      # Select a victim frame using the clock algorithm.
      #
      # Sweeps the clock hand around the circular buffer:
      #   - If use_bit is 0 → evict this frame
      #   - If use_bit is 1 → clear the bit (second chance), advance hand
      #
      # In the worst case (all bits set), the hand sweeps the entire
      # circle, clearing all bits, then evicts the frame it started at.
      #
      # @return [Integer, nil] the frame to evict, or nil if empty
      def select_victim
        return nil if @frames.empty?

        # We may need to go around the entire circle twice in the worst case:
        # once to clear all use bits, once more to find the victim.
        (2 * @frames.size).times do
          @hand = 0 if @hand >= @frames.size
          frame = @frames[@hand]

          if @use_bits[frame]
            # Second chance: clear the use bit and move on.
            @use_bits[frame] = false
            @hand += 1
          else
            # Use bit is clear → this frame is the victim.
            @frames.delete_at(@hand)
            @use_bits.delete(frame)
            @hand = 0 if @hand >= @frames.size
            return frame
          end
        end

        # Safety fallback: evict the frame at the hand position.
        # This should never be reached with correct logic.
        frame = @frames.delete_at(@hand)
        @use_bits.delete(frame) if frame
        @hand = 0 if !@frames.empty? && @hand >= @frames.size
        frame
      end

      # Add a newly loaded frame to the clock.
      #
      # New frames enter with use_bit=true (they were just loaded,
      # so they are "recently used").
      #
      # @param frame [Integer] the frame that was just loaded
      def add_frame(frame)
        @frames.push(frame)
        @use_bits[frame] = true
      end

      # Remove a frame from the clock.
      #
      # @param frame [Integer] the frame to remove
      def remove_frame(frame)
        idx = @frames.index(frame)
        if idx
          @frames.delete_at(idx)
          @use_bits.delete(frame)
          # Adjust hand if it was pointing past the removed element.
          @hand = 0 if !@frames.empty? && @hand >= @frames.size
        end
      end

      # How many frames are being tracked?
      #
      # @return [Integer] number of tracked frames
      def size
        @frames.size
      end
    end
  end
end
