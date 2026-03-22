# frozen_string_literal: true

# = Translation Lookaside Buffer (TLB)
#
# The TLB is a small, fast cache that stores recent virtual-to-physical
# address translations. Without the TLB, every memory access would require
# walking the page table -- 2-3 additional memory accesses just to find
# where the data actually lives.
#
# == Why the TLB Matters
#
# Consider a simple loop that adds numbers in an array:
#
#   sum = 0
#   for i in 0..999
#     sum += array[i]   # Each access needs a virtual-to-physical translation
#   end
#
# Without a TLB, each array access requires:
#   1. Read the page directory entry (memory access #1)
#   2. Read the page table entry (memory access #2)
#   3. Read the actual data (memory access #3)
#
# That's 3x slower! But with a TLB, after the first access to a page,
# subsequent accesses to the same page are translated in one cycle.
# Since 1000 array elements fit in just ~1 page (assuming 4-byte ints),
# 999 of 1000 accesses hit the TLB.
#
# == Temporal and Spatial Locality
#
# TLBs work because programs exhibit *locality*:
#   - Temporal locality: if you accessed a page recently, you'll likely
#     access it again soon (loops, function calls, stack usage).
#   - Spatial locality: if you accessed address X, you'll likely access
#     X+1, X+2, etc. (sequential array traversal, instruction fetch).
#
# A 64-entry TLB covers 64 * 4 KB = 256 KB of memory. Most programs'
# "working set" (the set of pages actively in use) fits in 256 KB,
# giving hit rates above 95%.
#
# == Eviction Policy
#
# When the TLB is full and a new translation needs to be cached, we
# evict the least recently used entry (LRU). This ensures that actively
# used translations stay cached.
#
# == Flushing
#
# The TLB must be flushed (cleared) on context switch. When the OS
# switches from process A to process B, the TLB contains A's translations.
# If B accesses virtual page 5, the TLB might return A's frame for page 5,
# letting B read A's private memory! Flushing prevents this security hole.
#
# This is one reason context switches are expensive: every process switch
# means the TLB starts cold (empty), and the first few memory accesses
# are all TLB misses.

module CodingAdventures
  module VirtualMemory
    # Default TLB capacity. Real TLBs have 32-256 entries.
    # 64 is a reasonable simulation value.
    DEFAULT_TLB_CAPACITY = 64

    class TLB
      attr_reader :hits, :misses, :capacity

      # Create a new TLB with the given capacity.
      #
      # @param capacity [Integer] maximum number of cached translations
      def initialize(capacity: DEFAULT_TLB_CAPACITY)
        @capacity = capacity

        # The entries hash maps (pid, vpn) => [frame_number, pte].
        # We use a two-element array key [pid, vpn] for lookups.
        @entries = {}

        # Access order tracks which entries were used most recently.
        # The most recently accessed entry is at the end of the array.
        # On eviction, we remove the entry at the front (LRU).
        @access_order = []

        # Hit/miss counters for performance monitoring.
        # A healthy TLB has a hit rate above 95%.
        @hits = 0
        @misses = 0
      end

      # Look up a cached translation.
      #
      # If the (pid, vpn) pair is in the TLB, return the cached frame
      # number and increment the hit counter. Otherwise, return nil and
      # increment the miss counter.
      #
      # @param pid [Integer] the process ID
      # @param vpn [Integer] the virtual page number
      # @return [Integer, nil] the cached frame number, or nil on miss
      def lookup(pid, vpn)
        key = [pid, vpn]
        entry = @entries[key]

        if entry
          @hits += 1
          # Move this entry to the end of the access order (most recent).
          # This implements LRU: least recently used entries drift to the front.
          @access_order.delete(key)
          @access_order.push(key)
          entry[0] # frame_number
        else
          @misses += 1
          nil
        end
      end

      # Insert a translation into the TLB.
      #
      # If the TLB is full, evict the least recently used entry first.
      #
      # @param pid [Integer] the process ID
      # @param vpn [Integer] the virtual page number
      # @param frame_number [Integer] the physical frame number
      # @param pte [PageTableEntry] the page table entry (cached for metadata)
      def insert(pid, vpn, frame_number, pte = nil)
        key = [pid, vpn]

        # If this key already exists, update it in place.
        if @entries.key?(key)
          @entries[key] = [frame_number, pte]
          @access_order.delete(key)
          @access_order.push(key)
          return
        end

        # If the TLB is full, evict the least recently used entry.
        if @entries.size >= @capacity
          evict_key = @access_order.shift
          @entries.delete(evict_key)
        end

        @entries[key] = [frame_number, pte]
        @access_order.push(key)
      end

      # Invalidate (remove) a single TLB entry.
      #
      # Called when a specific mapping changes, e.g., after a page fault
      # resolves or when a page is unmapped. We must remove the stale
      # translation so the next access walks the (updated) page table.
      #
      # @param pid [Integer] the process ID
      # @param vpn [Integer] the virtual page number
      def invalidate(pid, vpn)
        key = [pid, vpn]
        @entries.delete(key)
        @access_order.delete(key)
      end

      # Flush (clear) the entire TLB.
      #
      # Called on context switch to prevent one process from seeing
      # another process's translations. Also called when the page
      # table structure changes globally (e.g., kernel memory mapping).
      def flush
        @entries.clear
        @access_order.clear
      end

      # Calculate the TLB hit rate.
      #
      # Hit rate = hits / (hits + misses).
      # A good TLB achieves >95% hit rate due to locality of reference.
      #
      # @return [Float] the hit rate (0.0 to 1.0), or 0.0 if no lookups
      def hit_rate
        total = @hits + @misses
        return 0.0 if total == 0
        @hits.to_f / total
      end

      # How many entries are currently in the TLB?
      #
      # @return [Integer] number of cached translations
      def size
        @entries.size
      end

      # Reset the hit/miss counters.
      #
      # Useful for benchmarking specific code sections without
      # historical data polluting the results.
      def reset_stats
        @hits = 0
        @misses = 0
      end
    end
  end
end
