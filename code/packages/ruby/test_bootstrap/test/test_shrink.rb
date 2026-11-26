# frozen_string_literal: true

require_relative "test_helper"

class TestShrink < HookedTest
  # -- integers ------------------------------------------------------------------

  def test_the_corpus_headline_example
    assert_equal [101, 15], Pbt::Shrink.shrink_int(54_321, 0, 0, 100_000) { |x| x <= 100 }
  end

  def test_the_target_probe_is_one_step
    assert_equal [0, 1], Pbt::Shrink.shrink_int(500, 0, 0, 1000) { |x| x < -5 } # everything fails
  end

  def test_negative_bisection_floors
    # diff = -81 => floor(-81/2) = -41, so the candidate is -81 + 41 = -40 (truncation would give -41)
    seen = []
    Pbt::Shrink.shrink_int(-81, 0, -200, 200) { |x| (seen << x) && x >= -10 }
    assert_equal(-40, seen[2]) # [initial, target probe 0, first bisection]
  end

  def test_unit_steps_when_bisection_cannot_progress
    assert_equal [51, 49], Pbt::Shrink.shrink_int(100, 0, -100, 500) { |x| x.between?(-10, 50) }
  end

  def test_result_stays_in_bounds_and_falsifies
    (1..60).each do |t|
      [-300, 300, 77].each do |start|
        next if start < t && start >= -t # holds: not a valid start

        min, = Pbt::Shrink.shrink_int(start, 0, -400, 400) { |x| x.abs < t }
        assert_equal t, min.abs
      end
    end
  end

  def test_zero_budget_returns_the_start
    assert_equal [99, 0], Pbt::Shrink.shrink_int(99, 0, 0, 100, 0) { |x| x < 5 }
  end

  def test_budget_limits_accepted_moves
    value, steps = Pbt::Shrink.shrink_int(54_321, 0, 0, 100_000, 3) { |x| x <= 100 }
    assert_equal 3, steps
    assert_operator value, :>, 101
  end

  def test_integer_argument_validation
    prop = ->(x) { x < 5 }
    assert_raises(Pbt::NotFalsifyingError) { Pbt::Shrink.shrink_int(1, 0, 0, 10, &prop) }
    assert_raises(ArgumentError) { Pbt::Shrink.shrink_int(50, 0, 10, 0, &prop) }
    assert_raises(ArgumentError) { Pbt::Shrink.shrink_int(50, 0, 0, 10, &prop) } # outside bounds
    assert_raises(ArgumentError) { Pbt::Shrink.shrink_int(5, 0, 0, 10, -1, &prop) }
    assert_raises(ArgumentError) { Pbt::Shrink.shrink_int(5, 0, 0, 10, 10_001, &prop) }
    assert_raises(TypeError) { Pbt::Shrink.shrink_int(5.0, 0, 0, 10, &prop) }
    assert_raises(TypeError) { Pbt::Shrink.shrink_int(5, 0, 0, 10, "3", &prop) }
    assert_raises(ArgumentError) { Pbt::Shrink.shrink_int(5, 0, 0, 10) }
  end

  def test_target_outside_the_bounds_is_not_probed
    # refinement: target 10 is inside; target 5 below min 10 is not
    assert_equal [10, 1], Pbt::Shrink.shrink_int(99, 10, 10, 500) { |x| x < 5 }
    min, = Pbt::Shrink.shrink_int(99, 5, 10, 500) { |x| x < 50 }
    assert_equal 50, min
  end

  # -- lists: bisection halves, pinned -------------------------------------------

  # Left half, then right half, in one run, with the exact order of candidates.
  def test_bisection_accepts_the_right_half_then_the_left_half
    calls = []
    prop = lambda do |l|
      calls << l.dup
      !l.include?(5)
    end
    minimal, steps = Pbt::Shrink.shrink_list([0, 0, 0, 0, 5, 0, 0, 0], 0, &prop)
    assert_equal [[5], 3], [minimal, steps]
    assert_equal(
      [[0, 0, 0, 0, 5, 0, 0, 0], # the initial check
        [],                      # empty probe
        [0, 0, 0, 0],            # left half holds: rejected
        [5, 0, 0, 0],            # right half falsifies: ACCEPTED
        [5, 0],                  # left half falsifies: accepted
        [5],                     # left half falsifies: accepted
        [3], [4]],               # element phase: bisect then unit candidates, both hold
      calls
    )
  end

  def test_bisection_only_left_halves
    _, steps = Pbt::Shrink.shrink_list([5, 0, 0, 0, 0, 0, 0, 0], 0) { |l| !l.include?(5) }
    assert_equal 3, steps
  end

  def test_bisection_only_right_halves
    minimal, steps = Pbt::Shrink.shrink_list([0, 0, 0, 0, 0, 0, 0, 5], 0) { |l| !l.include?(5) }
    assert_equal [[5], 3], [minimal, steps]
  end

  def test_empty_probe_wins_in_one_step
    assert_equal [[], 1], Pbt::Shrink.shrink_list([1, 2, 3], 0) { |_l| false }
  end

  def test_deletion_is_right_to_left_and_one_minimal
    minimal, = Pbt::Shrink.shrink_list([1, 2, 3, 3, 4, 5], 0) { |l| l.each_cons(2).none? { |a, b| a == b } }
    assert_equal [3, 3], minimal
  end

  def test_element_walk_does_not_go_below_the_element_target
    minimal, = Pbt::Shrink.shrink_list([9, 9, 9], 4) { |l| l.sum < 10 }
    assert_operator minimal.sum, :>=, 10
    assert(minimal.all? { |x| x >= 4 })
  end

  def test_list_validation_and_starvation_free_failure
    assert_raises(Pbt::NotFalsifyingError) { Pbt::Shrink.shrink_list([1], 0) { |_l| true } }
    assert_raises(TypeError) { Pbt::Shrink.shrink_list("x", 0) { |_l| false } }
    assert_raises(TypeError) { Pbt::Shrink.shrink_list([1.5], 0) { |_l| false } }
    assert_raises(TypeError) { Pbt::Shrink.shrink_list([1], nil) { |_l| false } }
    assert_raises(ArgumentError) { Pbt::Shrink.shrink_list([1], 0) }
  end

  def test_a_property_that_scribbles_on_its_argument_cannot_corrupt_the_shrink
    minimal, = Pbt::Shrink.shrink_list([7, 8, 9], 0) do |l|
      ok = l.sum < 10
      l.clear
      ok
    end
    assert_operator minimal.size, :<=, 3
    refute_empty minimal
  end

  # -- bytes ---------------------------------------------------------------------

  def test_shrink_bytes_zeroes_and_truncates
    minimal, steps = Pbt::Shrink.shrink_bytes("\x01\x02\x03\x04\x05\x06\x07\x08".b) { |b| b.bytesize < 5 }
    assert_equal ["\x00\x00\x00\x00\x00".b, 16], [minimal, steps]
    assert_equal Encoding::BINARY, minimal.encoding
  end

  def test_shrink_bytes_validation
    assert_raises(Pbt::NotFalsifyingError) { Pbt::Shrink.shrink_bytes("a".b) { |_b| true } }
    assert_raises(TypeError) { Pbt::Shrink.shrink_bytes(5) { |_b| false } }
    assert_raises(ArgumentError) { Pbt::Shrink.shrink_bytes("a") }
  end

  # -- replay --------------------------------------------------------------------

  def test_simulate_replay_is_deterministic
    a = Pbt::Shrink.simulate_replay(42) { |x| x <= 100 }
    assert_equal [413, 101], a
    assert_equal a, Pbt::Shrink.simulate_replay(42) { |x| x <= 100 }
  end

  def test_simulate_replay_needs_a_falsifiable_property
    assert_raises(ArgumentError) { Pbt::Shrink.simulate_replay(1) { |_x| true } }
    assert_raises(ArgumentError) { Pbt::Shrink.simulate_replay(1) }
  end

  # -- sequence internals --------------------------------------------------------

  def test_shrink_sequence_zero_steps_returns_the_input_object
    input = [1, 2, 3]
    assert_same input, Pbt::Shrink.shrink_sequence(input, ->(_l) { true }, 0, 0, nil).first
  end

  def test_shrink_sequence_respects_min_len
    out, = Pbt::Shrink.shrink_sequence([1, 2, 3, 4, 5, 6], ->(_l) { true }, 100, 2, nil)
    assert_equal 2, out.size
    out, = Pbt::Shrink.shrink_sequence([1, 2, 3, 4], ->(l) { l.size >= 3 }, 100, 3, nil)
    assert_equal 3, out.size
  end

  def test_sabotaged_deletion_phase_changes_the_answer
    prop = ->(l) { l.each_cons(2).none? { |a, b| a == b } }
    honest, = Pbt::Shrink.shrink_list([1, 2, 3, 3, 4, 5], 0, &prop)
    Pbt::Hooks.skip_deletion_phase = true
    sabotaged, = Pbt::Shrink.shrink_list([1, 2, 3, 3, 4, 5], 0, &prop)
    refute_equal honest, sabotaged
  end
end
