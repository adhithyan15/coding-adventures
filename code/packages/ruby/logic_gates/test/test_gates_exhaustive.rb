# frozen_string_literal: true

require_relative "test_helper"

# BigDecimal is a bundled gem since Ruby 3.4; use it when present without
# adding a dependency just for a test.
begin
  # Ruby 3.4 warns (then may raise LoadError) when bigdecimal is not in the
  # bundle; the BigDecimal rows are optional, so stay quiet either way.
  verbose, $VERBOSE = $VERBOSE, nil
  require "bigdecimal"
rescue LoadError
  nil
ensure
  $VERBOSE = verbose
end

# ============================================================================
# Exhaustive tests for the eleven basic gates (LG02 sections 7 and 9).
#
# Three questions, answered completely rather than by sampling:
#
#   1. Does every gate produce the literal truth table?      (small domain)
#   2. Does every gate reject every non-bit, in every
#      argument position, naming the right parameter?        (wide domain)
#   3. Does a valid call ever raise, or return a non-bit?    (no)
# ============================================================================

class TestGatesExhaustive < Minitest::Test
  LG = CodingAdventures::LogicGates
  Invalid = LG::InvalidBitError

  # name => [arity, rows]; each row is the inputs followed by the expected
  # output. Written out by hand, never computed from the code under test.
  TRUTH = {
    not_gate: [1, [[0, 1], [1, 0]]],
    and_gate: [2, [[0, 0, 0], [0, 1, 0], [1, 0, 0], [1, 1, 1]]],
    or_gate: [2, [[0, 0, 0], [0, 1, 1], [1, 0, 1], [1, 1, 1]]],
    xor_gate: [2, [[0, 0, 0], [0, 1, 1], [1, 0, 1], [1, 1, 0]]],
    nand_gate: [2, [[0, 0, 1], [0, 1, 1], [1, 0, 1], [1, 1, 0]]],
    nor_gate: [2, [[0, 0, 1], [0, 1, 0], [1, 0, 0], [1, 1, 0]]],
    xnor_gate: [2, [[0, 0, 1], [0, 1, 0], [1, 0, 0], [1, 1, 1]]],
    nand_not: [1, [[0, 1], [1, 0]]],
    nand_and: [2, [[0, 0, 0], [0, 1, 0], [1, 0, 0], [1, 1, 1]]],
    nand_or: [2, [[0, 0, 0], [0, 1, 1], [1, 0, 1], [1, 1, 1]]],
    nand_xor: [2, [[0, 0, 0], [0, 1, 1], [1, 0, 1], [1, 1, 0]]]
  }.freeze

  PARAMS = %w[a b].freeze

  # An object that claims to be 1 in every way short of being an Integer.
  class Liar
    def to_int = 1

    def to_i = 1

    def ==(other) = other == 1 || other.equal?(self)

    def coerce(other) = [other, 1]

    def class = Integer

    def is_a?(_) = true

    def to_s = raise("to_s must never be called")

    def inspect = raise("inspect must never be called")
  end

  class Hostile
    def to_s = raise("to_s must never be called")

    def inspect = raise("inspect must never be called")
  end

  # BasicObject has no #class, #to_s or #inspect at all.
  class Bare < BasicObject; end

  class Namer
    def self.name = "x" * 500
  end

  def bad_values
    ints = ((-1024..1024).to_a - [0, 1]) +
      [2**31, -(2**31), 2**63, -(2**63), 2**64, -(2**64), 10**100, -(10**100)]
    others = [
      true, false, nil, 1.0, 0.0, -0.0, 1.5, Float::NAN, Float::INFINITY,
      Rational(1), Rational(0), Complex(1, 0),
      "1", "0", "", :one, :"1", Object.new, [1], [], {1 => 1}, 1..1,
      Liar.new, Hostile.new, Bare.new, Namer.new, Class.new.new, Integer
    ]
    others += [BigDecimal(1), BigDecimal(0)] if defined?(BigDecimal)
    ints + others
  end

  # --- 1. truth tables ------------------------------------------------------

  def test_literal_truth_tables
    TRUTH.each do |gate, (_arity, rows)|
      rows.each do |row|
        *inputs, expected = row
        assert_equal expected, LG.public_send(gate, *inputs), "#{gate}(#{inputs.join(", ")})"
      end
    end
  end

  def test_every_gate_is_covered
    assert_equal 11, TRUTH.size
  end

  def test_nand_built_equals_direct
    [0, 1].each do |a|
      assert_equal LG.not_gate(a), LG.nand_not(a)
      [0, 1].each do |b|
        assert_equal LG.and_gate(a, b), LG.nand_and(a, b)
        assert_equal LG.or_gate(a, b), LG.nand_or(a, b)
        assert_equal LG.xor_gate(a, b), LG.nand_xor(a, b)
      end
    end
  end

  # --- 3. valid inputs never raise; results are Integer bits -----------------

  def test_valid_inputs_return_integer_bits
    TRUTH.each do |gate, (arity, _)|
      [0, 1].repeated_permutation(arity) do |args|
        result = LG.public_send(gate, *args)
        assert_equal Integer, result.class, "#{gate}#{args}"
        assert_includes [0, 1], result
      end
    end
  end

  # --- 2. rejection in every position ----------------------------------------

  def assert_rejected(gate, args, position)
    err = assert_raises(Invalid) { LG.public_send(gate, *args) }
    assert_equal PARAMS[position], err.param, "#{gate} reported #{err.param}"
    assert_kind_of ArgumentError, err
    # Every description is short: a class name capped at 64 characters, a
    # 64-bit integer, or an "Integer of N bits" summary.
    assert_operator err.value_description.length, :<=, 64
    assert_includes err.message, err.param
  end

  def test_every_non_bit_rejected_in_every_position_with_valid_partner
    bads = bad_values
    TRUTH.each do |gate, (arity, _)|
      arity.times do |pos|
        bads.each do |bad|
          args = Array.new(arity, 1)
          args[pos] = bad
          assert_rejected(gate, args, pos)
        end
      end
    end
  end

  def test_both_invalid_reports_the_first_parameter
    bads = bad_values
    TRUTH.each do |gate, (arity, _)|
      next if arity < 2

      bads.each do |bad|
        assert_rejected(gate, [bad, "x"], 0)
        assert_rejected(gate, [bad, bad], 0)
      end
    end
  end

  def test_inner_names_never_leak
    # NAND-built gates call nand_gate internally; they must still report a/b.
    %i[nand_not nand_and nand_or nand_xor].each do |gate|
      arity = TRUTH[gate][0]
      assert_equal "a", assert_raises(Invalid) { LG.public_send(gate, *([7] * arity)) }.param
    end
    %i[nand_and nand_or nand_xor].each do |gate|
      assert_equal "b", assert_raises(Invalid) { LG.public_send(gate, 1, 7) }.param
    end
  end

  # --- error identity and description ----------------------------------------

  def test_error_for_integer_carries_the_integer
    err = assert_raises(Invalid) { LG.and_gate(2, 1) }
    assert_equal "a", err.param
    assert_equal "2", err.value_description
    assert_includes err.message, "2"
  end

  def test_error_for_64_bit_integer_is_still_the_integer
    err = assert_raises(Invalid) { LG.not_gate(2**63) }
    assert_equal((2**63).to_s, err.value_description)
    err = assert_raises(Invalid) { LG.not_gate(-(2**63)) }
    assert_equal((-(2**63)).to_s, err.value_description)
  end

  def test_error_for_huge_integer_is_summarised_and_fast
    elapsed = Process.clock_gettime(Process::CLOCK_MONOTONIC)
    err = assert_raises(Invalid) { LG.not_gate(10**100_000) }
    elapsed = Process.clock_gettime(Process::CLOCK_MONOTONIC) - elapsed
    assert_match(/\Aan Integer of \d+ bits\z/, err.value_description)
    assert_operator err.message.length, :<, 100
    assert_operator elapsed, :<, 1.0
    assert_equal "an Integer of 66 bits", assert_raises(Invalid) { LG.not_gate(2**65) }.value_description
  end

  def test_error_for_non_integer_names_only_the_class
    assert_equal "a Float", assert_raises(Invalid) { LG.not_gate(1.0) }.value_description
    assert_equal "a boolean", assert_raises(Invalid) { LG.not_gate(true) }.value_description
    assert_equal "nil", assert_raises(Invalid) { LG.not_gate(nil) }.value_description
  end

  def test_description_never_calls_value_methods_and_never_names_the_class
    [Hostile, Liar, Namer, Bare, Class.new].each do |k|
      assert_equal "a value of another type", assert_raises(Invalid) { LG.not_gate(k.new) }.value_description
    end
  end

  def test_invalid_bit_error_is_an_argument_error_not_a_type_error
    refute_operator Invalid, :<, TypeError
    assert_operator Invalid, :<, ArgumentError
  end

  # --- arity -----------------------------------------------------------------

  def test_wrong_arity_raises_argument_error
    TRUTH.each do |gate, (arity, _)|
      assert_raises(ArgumentError) { LG.public_send(gate) }
      assert_raises(ArgumentError) { LG.public_send(gate, *([1] * (arity + 1))) }
      assert_raises(ArgumentError) { LG.public_send(gate, *([1] * (arity - 1))) } if arity > 1
    end
  end
end
