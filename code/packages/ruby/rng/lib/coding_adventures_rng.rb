# frozen_string_literal: true

# coding_adventures_rng — three classic pseudorandom number generators.
#
# This file is the single entry-point for the gem. It wires together the
# version constant and the three generator classes that live below.
#
# Algorithms implemented:
#   - LCG        (Linear Congruential Generator, Knuth 1948)
#   - Xorshift64 (Marsaglia 2003)
#   - PCG32      (O'Neill 2014 — Permuted Congruential Generator)
#
# All three generators expose an identical API:
#
#   g = CodingAdventures::Rng::LCG.new(42)
#   g.next_u32              # => Integer in [0, 2^32)
#   g.next_u64              # => Integer in [0, 2^64)
#   g.next_float            # => Float  in [0.0, 1.0)
#   g.next_int_in_range(1, 6) # => Integer in [1, 6] inclusive
#
# Ruby integers are arbitrary precision (BigNum), so every operation that
# simulates unsigned 64-bit or 32-bit wrap-around must mask explicitly:
#
#   MASK64 = 0xFFFFFFFFFFFFFFFF   (64 one-bits)
#   MASK32 = 0xFFFFFFFF           (32 one-bits)

require_relative "coding_adventures/rng/version"

module CodingAdventures
  # Top-level namespace shared by all coding-adventures gems.
  module Rng
    # ── Shared arithmetic constants ────────────────────────────────────────
    #
    # These are the Knuth / Numerical Recipes constants used by both LCG and
    # PCG32. They satisfy the Hull-Dobell theorem: with these values the LCG
    # recurrence has a full period of 2^64 (every 64-bit value is visited
    # exactly once before the cycle repeats).
    #
    # Multiplier: 6364136223846793005
    # Increment:  1442695040888963407  (must be odd for full period)
    LCG_MULTIPLIER = 6_364_136_223_846_793_005
    LCG_INCREMENT  = 1_442_695_040_888_963_407

    # Bit masks used to emulate unsigned 64-bit and 32-bit overflow.
    MASK64 = 0xFFFF_FFFF_FFFF_FFFF
    MASK32 = 0xFFFF_FFFF

    # Divisor used to map a u32 into [0.0, 1.0).  Equals 2^32.
    FLOAT_DIV = 4_294_967_296.0

    # ── LCG ──────────────────────────────────────────────────────────────────
    #
    # A Linear Congruential Generator is the simplest useful PRNG.
    #
    # State recurrence (mod 2^64):
    #
    #   state = (state × a + c) mod 2^64
    #
    # where a = LCG_MULTIPLIER and c = LCG_INCREMENT.
    #
    # Output: upper 32 bits of state.  Lower bits have shorter sub-periods, so
    # taking the top half gives better statistical quality.
    #
    # Strengths:  extremely fast, no branching, full period 2^64.
    # Weaknesses: consecutive outputs are visibly correlated; fails spectral
    #             tests in high dimensions.  Good for simple simulations; not
    #             suitable for cryptography.
    #
    # Reference values for seed=1 (first three next_u32 calls):
    #   [1817669548, 2187888307, 2784682393]
    class LCG
      # Seed is loaded directly into state; any 64-bit value is valid.
      def initialize(seed)
        # Mask to 64 bits in case caller passes a value wider than 64 bits.
        @state = seed & MASK64
      end

      # Advance state and return the upper 32 bits as an unsigned integer.
      #
      # The multiply-add must be masked to 64 bits to emulate C uint64_t
      # wraparound, because Ruby integers grow unboundedly.
      def next_u32
        @state = (@state * LCG_MULTIPLIER + LCG_INCREMENT) & MASK64
        (@state >> 32) & MASK32
      end

      # Return a 64-bit unsigned integer from two consecutive next_u32 calls.
      #
      # The high word goes into bits [63:32] and the low word into [31:0],
      # matching the Go reference implementation.
      def next_u64
        hi = next_u32
        lo = next_u32
        ((hi << 32) | lo) & MASK64
      end

      # Return a Float uniformly distributed in [0.0, 1.0).
      #
      # Dividing a u32 by 2^32 gives a value in [0, 1).  The division is
      # exact for all u32 values because Float64 has 53 bits of mantissa and
      # u32 only uses 32 bits.
      def next_float
        next_u32.to_f / FLOAT_DIV
      end

      # Return a uniform random integer in [min, max] inclusive.
      #
      # Naïve modulo is biased when 2^32 is not divisible by range_size:
      # values in [0, 2^32 mod range_size) would appear one more time than
      # the rest.  Rejection sampling eliminates this bias.
      #
      # Threshold derivation:
      #   threshold = (-range_size) mod 2^32 mod range_size
      #
      # Ruby's % operator is always non-negative (sign follows divisor), so
      # (-range_size) % (1 << 32) is correct without extra sign handling.
      # Any draw r < threshold is rejected; the expected number of extra
      # draws is less than 2 for every possible range_size.
      def next_int_in_range(min, max)
        range_size = (max - min + 1).to_i
        threshold = (-range_size) % (1 << 32) % range_size
        loop do
          r = next_u32
          return min + (r % range_size) if r >= threshold
        end
      end
    end

    # ── Xorshift64 ───────────────────────────────────────────────────────────
    #
    # George Marsaglia (2003) showed that three XOR-and-shift operations can
    # produce a full-period sequence of 2^64 − 1 values using only 64 bits of
    # state and no multiplication.
    #
    # The recurrence (each step is applied to the same accumulator x):
    #
    #   x ^= x << 13
    #   x ^= x >> 7
    #   x ^= x << 17
    #
    # Output: lower 32 bits of state.
    #
    # Period: 2^64 − 1.  State 0 is a fixed point (0 XOR anything = 0) and
    # would produce an infinite stream of zeros.  Seed 0 is therefore replaced
    # with 1.
    #
    # Strengths:  no multiplication, very fast, long period.
    # Weaknesses: linear structure means it fails some linear-complexity tests.
    #
    # Reference values for seed=1 (first three next_u32 calls):
    #   [1082269761, 201397313, 1854285353]
    class Xorshift64
      # Replace seed 0 with 1 to avoid the degenerate all-zeros fixed point.
      def initialize(seed)
        @state = seed.zero? ? 1 : seed & MASK64
      end

      # Apply the three XOR-shifts and return the lower 32 bits.
      def next_u32
        x = @state
        x ^= (x << 13) & MASK64
        x ^= (x >> 7)
        x ^= (x << 17) & MASK64
        @state = x & MASK64
        @state & MASK32
      end

      # 64-bit output: two consecutive next_u32 calls, high word first.
      def next_u64
        hi = next_u32
        lo = next_u32
        ((hi << 32) | lo) & MASK64
      end

      # Float in [0.0, 1.0).
      def next_float
        next_u32.to_f / FLOAT_DIV
      end

      # Unbiased integer in [min, max] using rejection sampling.
      def next_int_in_range(min, max)
        range_size = (max - min + 1).to_i
        threshold = (-range_size) % (1 << 32) % range_size
        loop do
          r = next_u32
          return min + (r % range_size) if r >= threshold
        end
      end
    end

    # ── PCG32 ────────────────────────────────────────────────────────────────
    #
    # PCG32 (Melissa O'Neill, 2014) uses the same LCG recurrence as LCG but
    # applies an output permutation — XSH RR (XOR-Shift High / Random Rotate)
    # — before returning any bits.  The permutation breaks the linear
    # correlation that makes plain LCG weak.
    #
    # Output permutation steps (applied to old_state, before the LCG advance):
    #
    #  1. xorshifted = ((old_state >> 18) ^ old_state) >> 27
    #     Mix bits [63:18] down through [45:0], then keep the top 32 of the
    #     64-bit value — i.e., shift right by 27 more to land in a u32.
    #
    #  2. rot = old_state >> 59
    #     Take the top 5 bits as a rotation amount in [0, 31].
    #
    #  3. output = rotr32(xorshifted, rot)
    #     Rotate xorshifted right by rot positions.  In Ruby:
    #     ((xorshifted >> rot) | ((xorshifted << (32 - rot)) & MASK32)) & MASK32
    #
    # Initialisation ("initseq" warm-up, from O'Neill's reference C code):
    #
    #   state = 0
    #   Advance once   → mixes the increment into state
    #   state += seed  → XOR the seed into state
    #   Advance once   → scatter seed bits throughout all 64 state bits
    #
    # This warm-up ensures that even seeds 0 and 1 produce high-quality output
    # from the very first call.
    #
    # Reference values for seed=1 (first three next_u32 calls):
    #   [1412771199, 1791099446, 124312908]
    class PCG32
      # The increment for the LCG recurrence inside PCG32. It must be odd for
      # full period; lcgIncrement is already odd, and ORing with 1 is a no-op
      # that makes the requirement explicit.
      PCG_INCREMENT = LCG_INCREMENT | 1

      # Warm-up initialisation: two LCG advances around the seed injection.
      def initialize(seed)
        @state     = 0
        @increment = PCG_INCREMENT
        # First advance: incorporate the increment before the seed arrives.
        lcg_advance
        # Inject the seed.
        @state = (@state + (seed & MASK64)) & MASK64
        # Second advance: scatter seed bits throughout the full 64-bit state.
        lcg_advance
      end

      # Advance state and return the XSH-RR permuted 32-bit output.
      def next_u32
        old_state = @state
        lcg_advance

        # XSH RR permutation on old_state.
        # Step 1: mix bits [63:18] down, then shift to 32-bit range.
        xorshifted = (((old_state >> 18) ^ old_state) >> 27) & MASK32
        # Step 2: rotation amount from the top 5 bits.
        rot = (old_state >> 59) & 0x1F
        # Step 3: right-rotate xorshifted by rot positions.
        ((xorshifted >> rot) | ((xorshifted << (32 - rot)) & MASK32)) & MASK32
      end

      # 64-bit output: two consecutive next_u32 calls, high word first.
      def next_u64
        hi = next_u32
        lo = next_u32
        ((hi << 32) | lo) & MASK64
      end

      # Float in [0.0, 1.0).
      def next_float
        next_u32.to_f / FLOAT_DIV
      end

      # Unbiased integer in [min, max] using rejection sampling.
      def next_int_in_range(min, max)
        range_size = (max - min + 1).to_i
        threshold = (-range_size) % (1 << 32) % range_size
        loop do
          r = next_u32
          return min + (r % range_size) if r >= threshold
        end
      end

      private

      # One step of the LCG recurrence that drives PCG32's internal state.
      def lcg_advance
        @state = (@state * LCG_MULTIPLIER + @increment) & MASK64
      end
    end
  end
end
