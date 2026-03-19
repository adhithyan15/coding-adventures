# frozen_string_literal: true

# ─── Branch Target Buffer (BTB) ──────────────────────────────────────────────
#
# The branch predictor answers "WILL this branch be taken?"
# The BTB answers "WHERE does it go?"
#
# Both are needed for high-performance fetch. Without a BTB, even a perfect
# direction predictor would cause a 1-cycle bubble: the predictor says "taken"
# in the fetch stage, but the target address isn't known until decode (when
# the instruction's immediate field is extracted). With a BTB, the target
# is available in the SAME cycle as the prediction, enabling zero-bubble
# fetch redirection.
#
# How the BTB fits into the pipeline:
#
#     Cycle 1 (Fetch):
#         1. Read PC
#         2. Direction predictor: "taken" or "not taken"?
#         3. BTB lookup: if "taken", where does it go?
#         4. Redirect fetch to target (BTB hit) or PC+4 (not taken / BTB miss)
#
# BTB organization (this implementation):
#     - Direct-mapped cache indexed by (pc % size)
#     - Each entry stores: valid bit, tag (full PC), target, branch type
#     - On lookup: check valid bit and tag match
#     - On miss: return nil (fall through to PC+4)
#
# Real-world BTB sizes:
#     Intel Skylake: 4096 entries (L1 BTB) + 4096 entries (L2 BTB)
#     ARM Cortex-A72: 64 entries (micro BTB) + 4096 entries (main BTB)
#     AMD Zen 2: 512 entries (L1 BTB) + 7168 entries (L2 BTB)

module CodingAdventures
  module BranchPredictor
    # A single BTB entry -- caches the target address of one branch.
    #
    # Attributes:
    #   valid       -- is this entry occupied?
    #   tag         -- the full PC (for disambiguation on aliasing)
    #   target      -- the branch target address (the whole point of the BTB)
    #   branch_type -- what kind of branch ("conditional", "unconditional",
    #                  "call", "return")
    class BTBEntry
      attr_accessor :valid, :tag, :target, :branch_type

      def initialize(valid: false, tag: 0, target: 0, branch_type: "")
        @valid = valid
        @tag = tag
        @target = target
        @branch_type = branch_type
      end
    end

    # Branch Target Buffer -- works alongside any direction predictor to
    # provide target addresses.
    #
    # The BTB is a separate structure from the direction predictor. In a real
    # CPU, both are consulted in parallel during the fetch stage:
    #
    #   1. Direction predictor says: "taken" or "not taken"
    #   2. BTB says: "if taken, the target is 0x1234" (or miss)
    #
    # @example
    #   btb = BranchTargetBuffer.new(size: 256)
    #   btb.lookup(pc: 0x100) # => nil (miss -- never seen)
    #   btb.update(pc: 0x100, target: 0x200, branch_type: "conditional")
    #   btb.lookup(pc: 0x100) # => 0x200 (hit!)
    class BranchTargetBuffer
      # @return [Integer] total number of BTB lookups
      attr_reader :lookups

      # @return [Integer] number of BTB hits (target found)
      attr_reader :hits

      # @return [Integer] number of BTB misses (target not found)
      attr_reader :misses

      # @param size [Integer] number of entries in the BTB (should be power of 2)
      def initialize(size: 256)
        @size = size
        # Pre-allocate all entries as invalid. In hardware, this is a SRAM
        # array with valid bits cleared on reset.
        @entries = Array.new(size) { BTBEntry.new }
        @lookups = 0
        @hits = 0
        @misses = 0
      end

      # Look up the predicted target for a branch at +pc+.
      #
      # Returns the cached target address on a hit, or nil on a miss.
      # A miss occurs when:
      #   - The entry at this index is not valid (never written)
      #   - The entry's tag doesn't match the PC (aliasing conflict)
      #
      # @param pc [Integer] the program counter
      # @return [Integer, nil] the predicted target, or nil on miss
      def lookup(pc:)
        @lookups += 1
        index = pc % @size
        entry = @entries[index]

        # Check valid bit AND tag match (just like a cache)
        if entry.valid && entry.tag == pc
          @hits += 1
          entry.target
        else
          @misses += 1
          nil
        end
      end

      # Record a branch target after execution.
      #
      # Writes the target and metadata into the BTB. If another branch was
      # occupying this index (aliasing), it gets evicted -- direct-mapped policy.
      #
      # @param pc [Integer] the program counter
      # @param target [Integer] the actual target address
      # @param branch_type [String] the kind of branch
      def update(pc:, target:, branch_type: "conditional")
        index = pc % @size
        @entries[index] = BTBEntry.new(
          valid: true,
          tag: pc,
          target: target,
          branch_type: branch_type
        )
      end

      # Inspect the BTB entry for a given PC (for testing/debugging).
      #
      # @param pc [Integer] the program counter
      # @return [BTBEntry, nil] the entry if found, nil otherwise
      def get_entry(pc:)
        index = pc % @size
        entry = @entries[index]
        if entry.valid && entry.tag == pc
          entry
        end
      end

      # BTB hit rate as a percentage (0.0 to 100.0).
      #
      # @return [Float]
      def hit_rate
        return 0.0 if @lookups == 0
        (@hits.to_f / @lookups) * 100.0
      end

      # Reset all BTB state -- entries and statistics.
      def reset
        @entries = Array.new(@size) { BTBEntry.new }
        @lookups = 0
        @hits = 0
        @misses = 0
      end
    end
  end
end
