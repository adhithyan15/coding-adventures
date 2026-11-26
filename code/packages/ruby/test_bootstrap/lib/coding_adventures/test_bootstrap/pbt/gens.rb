# frozen_string_literal: true

module CodingAdventures
  module TestBootstrap
    module Pbt
      # The generator algebra (VER05 3, VER05e 3).
      #
      # DRAW ORDER IS THE COMPATIBILITY CONTRACT between language lanes: the
      # pbt-v1 vectors pin exact outputs per seed, so reordering a draw breaks
      # conformance.  Nested generators are length-bounded per level, not in total
      # (`list(list(g, 0, n), 0, n)` can produce n x n elements); shrinking, by
      # contrast, shares one work budget across all levels.
      #
      # Values: integers are Integer (u8, u64 and i128 included), bytes are binary
      # Strings, lists and tuples are Arrays.
      module Gens
        module_function

        TWO127 = 1 << 127
        TWO128 = 1 << 128
        MIN_I128 = -TWO127
        MAX_I128 = TWO127 - 1

        # The in-range point nearest zero: the integer shrink target.
        def clamp_zero(lo, hi)
          return lo if lo > 0
          return hi if hi < 0

          0
        end

        def int_shrinker(lo, hi)
          target = clamp_zero(lo, hi)
          ->(value, falsifies, max_steps) { Shrink.shrink_int_core(value, target, lo, hi, falsifies, max_steps) }
        end

        # Always generates `value` and draws nothing.
        def constant(value) = Gen.new(->(_rng) { value })

        # bounded(2) == 1.  Shrinks true to false.
        def bool
          Gen.new(
            ->(rng) { rng.bounded(2) == 1 },
            lambda do |value, falsifies, max_steps|
              (value && max_steps > 0 && falsifies.call(false)) ? [false, 1] : [value, 0]
            end
          )
        end

        # [0, 255]: bounded(256).
        def u8 = Gen.new(->(rng) { rng.bounded(256) }, int_shrinker(0, 255))

        # A full 64-bit word: next_u64.
        def u64 = Gen.new(->(rng) { rng.next_u64 }, int_shrinker(0, Prng::MASK64))

        # Two stream words, high first, read as two's complement:
        #   words 0x8000...0 : 0   -> -2**127  (top bit set => subtract 2**128)
        #   words 0x7fff...f : f.. ->  2**127 - 1
        def sample_i128(rng)
          hi = rng.next_u64
          lo = rng.next_u64
          v = (hi << 64) | lo
          (v >= TWO127) ? v - TWO128 : v
        end

        def i128 = Gen.new(method(:sample_i128), int_shrinker(MIN_I128, MAX_I128))

        # A uniform integer over the inclusive range [min, max], any width.  The
        # offset from min is drawn by bounded_big (VER05 7.2).  Shrinks toward the
        # in-range point nearest zero and never leaves [min, max].
        def integer(min, max)
          raise TypeError, "integer bounds must be Integers" unless min.is_a?(Integer) && max.is_a?(Integer)
          raise ArgumentError, "integer needs min <= max" if min > max

          Gen.new(->(rng) { bounded_int(rng, min, max) }, int_shrinker(min, max))
        end

        def bounded_int(rng, lo, hi) = lo + rng.bounded_big(hi - lo + 1)

        def check_lengths!(min_len, max_len, limit, what)
          raise TypeError, "#{what} lengths must be Integers" unless min_len.is_a?(Integer) && max_len.is_a?(Integer)
          raise ArgumentError, "#{what} needs 0 <= min_len <= max_len" if min_len < 0 || max_len < min_len
          raise ArgumentError, "#{what} max_len #{max_len} exceeds #{limit}" if max_len > limit
        end

        # A byte string: L = min_len + bounded(span), then L draws of bounded(256).
        # Shrinks by sequence shrinking, respecting min_len.
        def bytes(min_len, max_len)
          check_lengths!(min_len, max_len, Limits::MAX_INPUT_BYTES, "bytes")
          span = max_len - min_len + 1
          sampler = lambda do |rng|
            n = min_len + rng.bounded(span)
            Array.new(n) { rng.bounded(256) }.pack("C*")
          end
          shrinker = lambda do |value, falsifies, max_steps|
            # The array conversion (and the string one on the way out) are copies
            # too: charge them first, and do nothing once the budget is spent.
            ShrinkBudget.enter(max_steps) do |budget|
              next [value, 0] if max_steps == 0 || !budget.charge_copy(2 * value.bytesize)

              wrapped = ->(c) { budget.charge_copy(c.size) && falsifies.call(Shrink.to_bytes(c)) }
              list, steps = Shrink.shrink_sequence(value.bytes, wrapped, max_steps, min_len, Shrink::BYTE_WALK)
              [Shrink.to_bytes(list), steps]
            end
          end
          Gen.new(sampler, shrinker)
        end

        # Transforms generated values.  Drops shrinking: the block has no inverse.
        def map(gen, &fn)
          raise ArgumentError, "a block is required" unless fn

          Gen.new(->(rng) { fn.call(gen.sample(rng)) })
        end

        # Keeps only values satisfying the block.  After exactly `max_retries`
        # rejected samples sampling raises ExhaustedError.  Shrinking is preserved:
        # a candidate failing the predicate is simply not a valid input.
        def filter(gen, max_retries = Limits::DEFAULT_MAX_RETRIES, &pred)
          raise ArgumentError, "a predicate block is required" unless pred

          Limits.check_range!(max_retries, 1, Limits::MAX_RETRIES, "max_retries")
          sampler = lambda do |rng|
            # Every retry inside a check is charged, so stacked filters (each
            # retrying 10,000 times) cannot multiply uncharged work: in a NESTED
            # check a retry is an attempt charged up the chain; in a top-level
            # check it is one element of work against the root's 64M allowance.
            budget = ShrinkBudget.current
            nested = !budget.nil? && budget.nested?
            max_retries.times do
              if budget && !(nested ? budget.charge(1) : budget.charge_copy(1))
                raise ExhaustedError, "the enclosing shrink budget was spent while filtering"
              end

              v = gen.sample(rng)
              return v if pred.call(v)
            end
            raise ExhaustedError, "filter rejected #{max_retries} samples"
          end
          shrinker = lambda do |value, falsifies, max_steps|
            # Charge BEFORE the user predicate runs: it is property code and may be costly.
            budget = ShrinkBudget.current
            gen.shrink_nested(value, ->(c) { (budget.nil? || budget.charge(1)) && pred.call(c) && falsifies.call(c) }, max_steps)
          end
          Gen.new(sampler, shrinker)
        end

        # Dependent generation: sample, then sample from the block's generator.
        def flat_map(gen, &fn)
          raise ArgumentError, "a block is required" unless fn

          Gen.new(->(rng) { fn.call(gen.sample(rng)).sample(rng) })
        end

        # Picks one generator uniformly (bounded(len), then that arm's draws).
        def one_of(*gens)
          raise ArgumentError, "one_of needs at least one generator" if gens.empty?

          arms = gens.dup
          Gen.new(->(rng) { arms[rng.bounded(arms.size)].sample(rng) })
        end

        # A weighted choice among [weight, gen] pairs: pick = bounded(total); arms
        # are walked in order subtracting each weight, so a zero-weight arm can
        # never be selected.  Empty, negative weight, zero total or total above
        # 2**32 is refused.
        def frequency(*arms)
          raise ArgumentError, "frequency needs at least one arm" if arms.empty?

          total = 0
          arms.each do |arm|
            weight, = arm
            Limits.check_range!(weight, 0, Limits::MAX_FREQUENCY_TOTAL, "frequency weight")
            total += weight
            raise ArgumentError, "frequency total weight exceeds 2**32" if total > Limits::MAX_FREQUENCY_TOTAL
          end
          raise ArgumentError, "frequency total weight must be > 0" if total == 0

          owned = arms.map { |w, g| [w, g] }
          Gen.new(lambda do |rng|
            pick = rng.bounded(total)
            owned.each do |weight, gen|
              return gen.sample(rng) if pick < weight

              pick -= weight
            end
            # :nocov:
            raise "unreachable: pick < total"
            # :nocov:
          end)
        end

        # A list: L = min_len + bounded(span), then L elements in order.  Shrinks by
        # sequence shrinking with the element generator's shrinker for the
        # element-wise phase.
        def list(elem, min_len, max_len)
          check_lengths!(min_len, max_len, Limits::MAX_LIST_LEN, "list")
          span = max_len - min_len + 1
          sampler = lambda do |rng|
            n = min_len + rng.bounded(span)
            Array.new(n) { elem.sample(rng) }
          end
          shrinker = lambda do |value, falsifies, max_steps|
            elem_shrinker = ->(v, f, n) { elem.shrink_nested(v, f, n) }
            Shrink.shrink_sequence(value, falsifies, max_steps, min_len, elem_shrinker)
          end
          Gen.new(sampler, shrinker)
        end

        # One shrink-wide evaluation budget for a tuple: the repeat-until-no-progress
        # loop calls each component shrinker once per pass, so without a shared
        # counter the work is quadratic in max_steps.
        def global_budget(falsifies, max_steps)
          budget = Limits.attempt_budget(max_steps)
          spent = 0
          lock = Mutex.new
          lambda do |v|
            allowed = lock.synchronize { (spent += 1) <= budget }
            allowed && falsifies.call(v)
          end
        end

        # Samples each generator left to right (an Array) and shrinks one
        # coordinate at a time, in repeated passes until a pass makes no progress.
        def tuple(*gens)
          raise ArgumentError, "tuple needs at least one generator" if gens.empty?
          raise ArgumentError, "tuple arity exceeds #{Limits::MAX_TUPLE_ARITY}" if gens.size > Limits::MAX_TUPLE_ARITY

          sampler = ->(rng) { gens.map { |g| g.sample(rng) } }
          shrinker = lambda do |value, raw_falsifies, max_steps|
            falsifies = global_budget(raw_falsifies, max_steps)
            current = value.dup
            steps = 0
            progress = true
            while progress && steps < max_steps
              progress = false
              gens.each_with_index do |gen, index|
                remaining = max_steps - steps
                next unless remaining > 0

                slot = lambda do |candidate|
                  probe = current.dup
                  probe[index] = candidate
                  falsifies.call(probe)
                end
                shrunk, used = gen.shrink_nested(current[index], slot, remaining)
                if used > 0
                  current[index] = shrunk
                  steps += used
                  progress = true
                end
              end
            end
            [current, steps]
          end
          Gen.new(sampler, shrinker)
        end

        # -- corpus entry points (pbt-v1) ----------------------------------------

        # `count` primitives of `type_name` ("bool", "u8", "u64", "i128") from `seed`.
        def gen_primitive(seed, type_name, count)
          Limits.check_count!(count)
          rng = Hooks.new_rng.call(seed)
          sampler =
            case type_name
            when "bool" then ->(r) { r.bounded(2) == 1 }
            when "u8" then ->(r) { r.bounded(256) }
            when "u64" then ->(r) { r.next_u64 }
            when "i128" then method(:sample_i128)
            else raise ArgumentError, "unknown primitive type #{type_name.inspect}"
            end
          Array.new(count) { sampler.call(rng) }
        end

        # `count` uniform integers over [lo, hi] from `seed`.
        def gen_bounded(seed, lo, hi, count)
          raise TypeError, "bounds must be Integers" unless lo.is_a?(Integer) && hi.is_a?(Integer)
          raise ArgumentError, "need lo <= hi" if lo > hi

          Limits.check_count!(count)
          # Each draw is about as wide as the span; count x width bounds what the
          # returned list can hold (a 1,000-digit span x 10**6 draws would not).
          width = [1, ((hi - lo).bit_length + 7) / 8].max
          if count * width > Limits::MAX_GEN_BYTES_TOTAL
            raise ArgumentError, "count #{count} x #{width}-byte span exceeds the generation budget"
          end

          rng = Hooks.new_rng.call(seed)
          Array.new(count) { bounded_int(rng, lo, hi) }
        end

        # `count` byte strings with length in [min_len, max_len] from `seed`.
        def gen_bytes(seed, min_len, max_len, count)
          check_lengths!(min_len, max_len, Limits::MAX_INPUT_BYTES, "gen_bytes")
          Limits.check_count!(count) # applies even when max_len == 0
          if max_len > 0 && count > Limits::MAX_GEN_BYTES_TOTAL / max_len
            raise ArgumentError, "count #{count} x max_len #{max_len} exceeds the total of #{Limits::MAX_GEN_BYTES_TOTAL} bytes"
          end

          gen = bytes(min_len, max_len)
          rng = Hooks.new_rng.call(seed)
          Array.new(count) { gen.sample(rng) }
        end
      end
    end
  end
end
