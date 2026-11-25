# frozen_string_literal: true

module CodingAdventures
  module TestBootstrap
    # Deterministic randomness and hashing for test runs (TF01 5.1, VER05 2).
    #
    # Ruby integers never overflow, which is the opposite of what these
    # algorithms were designed for: SplitMix64 and FNV-1a are defined on 64-bit
    # words that WRAP.  So every add and every multiply below is followed by an
    # explicit `& MASK64`.  Forgetting one mask gives a perfectly plausible
    # 70-bit number and a silently different random stream.
    module Prng
      MASK64 = 0xFFFF_FFFF_FFFF_FFFF
      GOLDEN_GAMMA = 0x9E37_79B9_7F4A_7C15
      MIX_MUL1 = 0xBF58_476D_1CE4_E5B9
      MIX_MUL2 = 0x94D0_49BB_1331_11EB
      FNV_OFFSET_BASIS = 0xCBF2_9CE4_8422_2325
      FNV_PRIME = 0x0000_0100_0000_01B3
      TWO64 = 1 << 64

      # Strict integer check: Ruby `true` is not an Integer, but a Float is a
      # Numeric, and neither is a valid seed or count.
      def self.check_int!(value, name)
        raise TypeError, "#{name} must be an Integer, not #{value.class}" unless value.is_a?(Integer)
      end

      # SplitMix64: state += golden gamma; then a three-step bit mixer.
      #
      #   z1 = (s ^ (s >> 30)) * 0xbf58476d1ce4e5b9   (mod 2^64)
      #   z2 = (z1 ^ (z1 >> 27)) * 0x94d049bb133111eb (mod 2^64)
      #   out = z2 ^ (z2 >> 31)
      #
      # Seed 0's first output is 0xe220a8397b1dcdaf.
      class SplitMix64
        attr_reader :state

        def initialize(seed = 0)
          Prng.check_int!(seed, "seed")
          @state = seed & MASK64
        end

        # The state IS the stream: never printed.
        def inspect = "#<Prng::SplitMix64>"

        alias_method :to_s, :inspect

        def next_u64
          @state = (@state + GOLDEN_GAMMA) & MASK64
          z = @state
          z = ((z ^ (z >> 30)) * MIX_MUL1) & MASK64
          z = ((z ^ (z >> 27)) * MIX_MUL2) & MASK64
          (z ^ (z >> 31)) & MASK64
        end

        # Uniform integer in [0, n) for 1 <= n <= 2**64, by rejection sampling.
        #
        # The threshold is t = (2**64 - n) % n.  Raws below t are redrawn so the
        # remaining 2**64 - t values split into whole residue classes.  Why not
        # `(-n) % n`?  In C `-n` wraps to 2**64 - n; in Ruby it is the negative
        # number, and floored modulo makes `(-n) % n` exactly 0 for every n: no
        # draw is ever rejected and every span that does not divide 2**64 is biased.
        #
        # Worked example, n = 3: 2**64 = 3 * 6148914691236517205 + 1, so t = 1 and
        # the single raw 0 is thrown away.
        #
        # bounded(1) still consumes one word (the VER05 2.2 loop run once): the
        # stream position must be identical in every lane.
        def bounded(n)
          Prng.check_int!(n, "bounded(n)")
          raise ArgumentError, "bounded(n) requires n >= 1, got #{n}" if n < 1
          raise ArgumentError, "bounded(n) requires n <= 2**64, got #{n}" if n > TWO64
          return next_u64 if n == TWO64

          threshold = (TWO64 - n) % n
          loop do
            x = next_u64
            return x % n if x >= threshold
          end
        end

        # Uniform integer in [0, span) for any span >= 1 (VER05 7.2).
        #
        #   span <= 2**64      bounded(span)
        #   span  > 2**64      k-word rejection: k = max(2, ceil(bitlen(span-1)/64)),
        #                      first draw most significant, t = (2**(64k) - span) % span
        def bounded_big(span)
          Prng.check_int!(span, "span")
          raise ArgumentError, "span must be >= 1, got #{span}" if span < 1
          return bounded(span) if span <= TWO64

          words = [2, ((span - 1).bit_length + 63) / 64].max
          threshold = ((1 << (64 * words)) - span) % span
          loop do
            raw = 0
            words.times { raw = (raw << 64) | next_u64 }
            return raw % span if raw >= threshold
          end
        end
      end

      # 64-bit FNV-1a of a String (hashed as its UTF-8 or raw bytes).
      def self.fnv1a64(data)
        raise TypeError, "fnv1a64 expects a String, not #{data.class}" unless data.is_a?(String)

        hash = FNV_OFFSET_BASIS
        data.each_byte { |b| hash = ((hash ^ b) * FNV_PRIME) & MASK64 }
        hash
      end

      # sub_seed(run_seed, id) = SplitMix64(run_seed ^ fnv1a64(id)).next_u64  (VER05 2.3)
      def self.sub_seed(run_seed, property_id)
        check_int!(run_seed, "run_seed")
        raise TypeError, "property_id must be a String" unless property_id.is_a?(String)

        SplitMix64.new((run_seed & MASK64) ^ fnv1a64(property_id)).next_u64
      end
    end
  end
end
