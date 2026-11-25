# frozen_string_literal: true

module CodingAdventures
  module TestBootstrap
    module Pbt
      # Integrated shrinking (VER05 4, VER05e 4, VER05h 3.1).
      #
      # A property that fails on [913, -4, 77, 0, 2] has told you nothing; one
      # that fails on [0] has told you what is wrong.  Shrinking walks from the
      # first failing candidate toward the smallest one that still fails.  It is
      # "integrated" because the generator that made the value knows its own
      # bounds, so a shrunk `Gens.integer(5, 10)` never becomes 0.
      #
      # The public shrinkers take a block that returns truthy when the property
      # HOLDS (the corpus convention) and return `[minimal, steps]`, where steps
      # counts ACCEPTED moves only.
      module Shrink
        module_function

        def in_bounds?(value, lo, hi)
          (lo.nil? || lo <= value) && (hi.nil? || value <= hi)
        end

        # Bisects then unit-steps `current` toward `target`.
        #
        # Each turn tries, in order,
        #   1. the bisection candidate  current - floor(diff / 2)   when |diff| > 1,
        #   2. the unit candidate       current -/+ 1 toward the target.
        # A candidate is accepted only if inside [lo, hi] (nil = unbounded), different
        # from current and still falsifying.  No acceptable candidate ends the walk.
        #
        # Worked example (corpus shrink_int/54321_le_100, property x <= 100):
        # 54321 -> 27161 -> 13581 -> ... -> 101 in 15 accepted moves; 100 would
        # satisfy the property, so neither candidate from 101 is accepted.
        def int_walk(current, target, lo, hi, falsifies, budget)
          steps = 0
          while steps < budget && current != target
            diff = current - target
            moved = false
            if diff.abs > 1
              cand = Hooks.bisect_candidate.call(current, diff)
              if cand != current && in_bounds?(cand, lo, hi) && falsifies.call(cand)
                current = cand
                steps += 1
                moved = true
              end
            end
            unless moved
              cand = Hooks.unit_candidate.call(current, diff)
              if cand != current && in_bounds?(cand, lo, hi) && falsifies.call(cand)
                current = cand
                steps += 1
                moved = true
              end
            end
            break unless moved
          end
          [current, steps]
        end

        # The target probe followed by the walk (VER05 4.2).
        def shrink_int_core(initial, target, lo, hi, falsifies, max_steps)
          return [initial, 0] if max_steps == 0 # no budget, no shrinking

          # the target itself fails: cannot do better
          return [target, 1] if in_bounds?(target, lo, hi) && target != initial && falsifies.call(target)

          int_walk(initial, target, lo, hi, falsifies, max_steps)
        end

        # A byte value shrunk toward 0 by the integer walk.
        BYTE_WALK = ->(value, falsifies, budget) { int_walk(value, 0, 0, 255, falsifies, budget) }

        # Shrinks a falsifying sequence (VER05 4.3).  `falsifies` always receives a
        # FRESH array it may keep or mutate; the ELEMENTS are not cloned.
        #
        #   probe        min_len == 0: does the empty list fail?  Then it is the
        #                answer, one step.  (min_len > 0: probe the length-min_len
        #                prefix and carry on from it.)
        #   bisection    keep the left half, else the right half, while one fails.
        #   deletion     single elements, right to left.
        #   element-wise left to right via `shrink_elem` (nil means none).
        #
        # Worked example (corpus list_sum_le_20, [10 5 8 4]): the empty list and both
        # halves satisfy the property; deleting index 3 leaves [10 5 8] (sum 23, still
        # failing); element-wise 10 -> 8 gives the 1-minimal [8 5 8].
        #
        # Every candidate is CHARGED to the budget tree BEFORE it is built; once
        # refused, every loop ends.  When max_steps == 0 or the budget is already
        # spent the input is returned as is (it may be the same object).
        def shrink_sequence(initial, falsifies, max_steps, min_len, shrink_elem)
          return [initial, 0] if max_steps == 0

          ShrinkBudget.enter(max_steps) do |budget|
            return [initial, 0] unless budget.charge_copy(initial.size)

            current = initial.dup
            steps = 0

            if current.size > min_len && budget.charge(min_len) && falsifies.call(current[0, min_len])
              return [[], 1] if min_len == 0

              current.slice!(min_len..)
              steps = 1
            end

            floor = [1, min_len].max

            # Phase 1: bisection (left half first, then right half).
            while current.size > floor && steps < max_steps
              mid = current.size / 2
              if mid >= min_len && budget.charge(mid) && falsifies.call(current[0, mid])
                current.slice!(mid..)
                steps += 1
                next
              end
              right_len = current.size - mid
              if right_len >= min_len && budget.charge(right_len) && falsifies.call(current[mid, right_len])
                current.slice!(0, mid)
                steps += 1
                next
              end
              break
            end

            # Phase 2: single-element deletion, right to left.
            unless Hooks.skip_deletion_phase
              i = current.size - 1
              while i >= 0 && current.size > floor && steps < max_steps && budget.room?
                break unless budget.charge(current.size - 1) # charge BEFORE copying

                cand = current.dup
                cand.delete_at(i)
                if falsifies.call(cand)
                  current.delete_at(i)
                  steps += 1
                  i = [i, current.size - 1].min
                else
                  i -= 1
                end
              end
            end

            # Phase 3: element-wise, left to right.
            if shrink_elem
              size = current.size
              index = 0
              while index < size
                remaining = max_steps - steps
                break if remaining <= 0 || !budget.room?

                at = index
                with_element = lambda do |candidate|
                  next false unless budget.charge(current.size)

                  probe = current.dup
                  probe[at] = candidate
                  falsifies.call(probe)
                end
                value, used = shrink_elem.call(current[index], with_element, remaining)
                if used > 0
                  current[index] = value
                  steps += used
                end
                index += 1
              end
            end

            [current, steps]
          end
        end

        # Shrinks a falsifying integer toward `target` inside [lo, hi] (the corpus
        # calls these min and max).  The block is true when the property holds.
        # Raises NotFalsifyingError when `initial` satisfies it, ArgumentError for
        # lo > hi, an initial outside the bounds or a budget outside [0, MAX_SHRINK_STEPS].
        def shrink_int(initial, target, lo, hi, max_steps = 1000, &prop)
          raise ArgumentError, "a property block is required" unless prop

          [initial, target, lo, hi].zip(%w[initial target min max]).each do |v, name|
            raise TypeError, "#{name} must be an Integer, not #{v.class}" unless v.is_a?(Integer)
          end
          Limits.check_budget!(max_steps)
          raise ArgumentError, "min exceeds max" if lo > hi
          raise ArgumentError, "initial value must lie within [min, max]" unless in_bounds?(initial, lo, hi)

          ShrinkBudget.enter_child(max_steps) do |budget|
            next [initial, 0] unless budget.charge(1) # starved by an enclosing shrink

            raise NotFalsifyingError if prop.call(initial)

            falsifies = ->(v) { budget.charge(1) && !prop.call(v) }
            shrink_int_core(initial, target, lo, hi, falsifies, max_steps)
          end
        end

        # Shrinks a falsifying list of integers: empty probe, halves, single deletion,
        # then each element toward `elem_target` (unbounded).
        def shrink_list(initial, elem_target, max_steps = 1000, &prop)
          raise ArgumentError, "a property block is required" unless prop
          raise TypeError, "initial must be an Array" unless initial.is_a?(Array)
          raise TypeError, "elem_target must be an Integer" unless elem_target.is_a?(Integer)

          initial.each { |v| raise TypeError, "element must be an Integer" unless v.is_a?(Integer) }
          Limits.check_budget!(max_steps)

          ShrinkBudget.enter_child(max_steps) do |budget|
            # charged before the first copies (the working copy and the copy the
            # first property call receives) and for the initial evaluation
            next [initial, 0] if !budget.charge_copy(2 * initial.size) || !budget.charge(1)

            items = initial.dup
            raise NotFalsifyingError if prop.call(items.dup)

            walk = ->(value, falsifies, steps) { int_walk(value, elem_target, nil, nil, falsifies, steps) }
            shrink_sequence(items, ->(c) { !prop.call(c) }, max_steps, 0, walk)
          end
        end

        # Shrinks falsifying bytes (a binary String): sequence shrinking over byte
        # values toward 0x00.
        def shrink_bytes(initial, max_steps = 1000, &prop)
          raise ArgumentError, "a property block is required" unless prop
          raise TypeError, "initial must be a String" unless initial.is_a?(String)

          Limits.check_budget!(max_steps)
          ShrinkBudget.enter_child(max_steps) do |budget|
            # the copy, the array conversion and the string conversion back
            next [initial, 0] if !budget.charge_copy(3 * initial.bytesize) || !budget.charge(1)

            raise NotFalsifyingError if prop.call(initial.b)

            # each attempt also converts its candidate back to a String: charge that copy too
            falsifies = ->(c) { budget.charge_copy(c.size) && !prop.call(to_bytes(c)) }
            list, steps = shrink_sequence(initial.bytes, falsifies, max_steps, 0, BYTE_WALK)
            [to_bytes(list), steps]
          end
        end

        def to_bytes(list) = list.pack("C*")

        # Replays a failure from nothing but `seed`: draw up to 1000 values
        # bounded(1000); the first that falsifies is the initial failure, then
        # shrunk toward 0 over [0, 1000].  Same seed, same pair, forever.
        # Returns [initial_fail, minimal].
        def simulate_replay(seed, &prop)
          raise ArgumentError, "a property block is required" unless prop

          ShrinkBudget.enter_child(1000) do |budget|
            rng = Hooks.new_rng.call(seed)
            first = nil
            1000.times do
              break unless budget.charge(1)

              v = rng.bounded(1000)
              unless prop.call(v)
                first = v
                break
              end
            end
            raise ArgumentError, "property was not falsified within 1000 draws" if first.nil?

            minimal, = shrink_int(first, 0, 0, 1000, 1000, &prop)
            [first, minimal]
          end
        end
      end
    end
  end
end
