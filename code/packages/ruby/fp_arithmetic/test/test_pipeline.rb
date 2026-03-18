# frozen_string_literal: true

require_relative "test_helper"

class TestPipeline < Minitest::Test
  FPA = CodingAdventures::FpArithmetic
  FP32 = FPA::FP32
  ClockGenerator = CodingAdventures::Clock::ClockGenerator

  # --- PipelinedFPAdder ---

  def test_pipelined_adder_basic
    clock = ClockGenerator.new
    adder = FPA::PipelinedFPAdder.new(clock)

    a = FPA.float_to_bits(1.5)
    b = FPA.float_to_bits(2.5)
    adder.submit(a, b)

    # 5 stages = 5 full cycles for result to emerge
    5.times { clock.full_cycle }

    assert_equal 1, adder.results.length
    assert_in_delta 4.0, FPA.bits_to_float(adder.results[0]), 1e-6
  end

  def test_pipelined_adder_multiple_operations
    clock = ClockGenerator.new
    adder = FPA::PipelinedFPAdder.new(clock)

    # Submit 3 operations
    adder.submit(FPA.float_to_bits(1.0), FPA.float_to_bits(2.0))
    adder.submit(FPA.float_to_bits(3.0), FPA.float_to_bits(4.0))
    adder.submit(FPA.float_to_bits(10.0), FPA.float_to_bits(20.0))

    # Run enough cycles for all results (5 + 2 extra for pipeline)
    8.times { clock.full_cycle }

    assert_equal 3, adder.results.length
    assert_in_delta 3.0, FPA.bits_to_float(adder.results[0]), 1e-6
    assert_in_delta 7.0, FPA.bits_to_float(adder.results[1]), 1e-6
    assert_in_delta 30.0, FPA.bits_to_float(adder.results[2]), 1e-6
  end

  def test_pipelined_adder_nan
    clock = ClockGenerator.new
    adder = FPA::PipelinedFPAdder.new(clock)

    adder.submit(FPA.float_to_bits(Float::NAN), FPA.float_to_bits(1.0))
    5.times { clock.full_cycle }

    assert_equal 1, adder.results.length
    assert FPA.bits_to_float(adder.results[0]).nan?
  end

  def test_pipelined_adder_inf
    clock = ClockGenerator.new
    adder = FPA::PipelinedFPAdder.new(clock)

    adder.submit(FPA.float_to_bits(Float::INFINITY), FPA.float_to_bits(1.0))
    5.times { clock.full_cycle }

    assert_equal 1, adder.results.length
    assert_equal Float::INFINITY, FPA.bits_to_float(adder.results[0])
  end

  def test_pipelined_adder_zeros
    clock = ClockGenerator.new
    adder = FPA::PipelinedFPAdder.new(clock)

    adder.submit(FPA.float_to_bits(0.0), FPA.float_to_bits(0.0))
    5.times { clock.full_cycle }

    assert_equal 1, adder.results.length
    assert_equal 0.0, FPA.bits_to_float(adder.results[0])
  end

  def test_pipelined_adder_cycle_count
    clock = ClockGenerator.new
    adder = FPA::PipelinedFPAdder.new(clock)

    3.times { clock.full_cycle }
    assert_equal 3, adder.cycle_count
  end

  # --- PipelinedFPMultiplier ---

  def test_pipelined_multiplier_basic
    clock = ClockGenerator.new
    mul = FPA::PipelinedFPMultiplier.new(clock)

    mul.submit(FPA.float_to_bits(3.0), FPA.float_to_bits(4.0))
    4.times { clock.full_cycle }

    assert_equal 1, mul.results.length
    assert_in_delta 12.0, FPA.bits_to_float(mul.results[0]), 1e-6
  end

  def test_pipelined_multiplier_multiple
    clock = ClockGenerator.new
    mul = FPA::PipelinedFPMultiplier.new(clock)

    mul.submit(FPA.float_to_bits(2.0), FPA.float_to_bits(3.0))
    mul.submit(FPA.float_to_bits(5.0), FPA.float_to_bits(10.0))
    6.times { clock.full_cycle }

    assert_equal 2, mul.results.length
    assert_in_delta 6.0, FPA.bits_to_float(mul.results[0]), 1e-6
    assert_in_delta 50.0, FPA.bits_to_float(mul.results[1]), 1e-6
  end

  def test_pipelined_multiplier_nan
    clock = ClockGenerator.new
    mul = FPA::PipelinedFPMultiplier.new(clock)

    mul.submit(FPA.float_to_bits(Float::NAN), FPA.float_to_bits(2.0))
    4.times { clock.full_cycle }

    assert FPA.bits_to_float(mul.results[0]).nan?
  end

  def test_pipelined_multiplier_inf_times_zero
    clock = ClockGenerator.new
    mul = FPA::PipelinedFPMultiplier.new(clock)

    mul.submit(FPA.float_to_bits(Float::INFINITY), FPA.float_to_bits(0.0))
    4.times { clock.full_cycle }

    assert FPA.bits_to_float(mul.results[0]).nan?
  end

  def test_pipelined_multiplier_zero
    clock = ClockGenerator.new
    mul = FPA::PipelinedFPMultiplier.new(clock)

    mul.submit(FPA.float_to_bits(0.0), FPA.float_to_bits(42.0))
    4.times { clock.full_cycle }

    assert_equal 0.0, FPA.bits_to_float(mul.results[0])
  end

  # --- PipelinedFMA ---

  def test_pipelined_fma_basic
    clock = ClockGenerator.new
    fma = FPA::PipelinedFMA.new(clock)

    # 2 * 3 + 1 = 7
    fma.submit(FPA.float_to_bits(2.0), FPA.float_to_bits(3.0), FPA.float_to_bits(1.0))
    6.times { clock.full_cycle }

    assert_equal 1, fma.results.length
    assert_in_delta 7.0, FPA.bits_to_float(fma.results[0]), 1e-6
  end

  def test_pipelined_fma_multiple
    clock = ClockGenerator.new
    fma = FPA::PipelinedFMA.new(clock)

    fma.submit(FPA.float_to_bits(2.0), FPA.float_to_bits(3.0), FPA.float_to_bits(1.0))
    fma.submit(FPA.float_to_bits(4.0), FPA.float_to_bits(5.0), FPA.float_to_bits(2.0))
    8.times { clock.full_cycle }

    assert_equal 2, fma.results.length
    assert_in_delta 7.0, FPA.bits_to_float(fma.results[0]), 1e-6
    assert_in_delta 22.0, FPA.bits_to_float(fma.results[1]), 1e-6
  end

  def test_pipelined_fma_nan
    clock = ClockGenerator.new
    fma = FPA::PipelinedFMA.new(clock)

    fma.submit(FPA.float_to_bits(Float::NAN), FPA.float_to_bits(1.0), FPA.float_to_bits(1.0))
    6.times { clock.full_cycle }

    assert FPA.bits_to_float(fma.results[0]).nan?
  end

  def test_pipelined_fma_inf_times_zero
    clock = ClockGenerator.new
    fma = FPA::PipelinedFMA.new(clock)

    fma.submit(FPA.float_to_bits(Float::INFINITY), FPA.float_to_bits(0.0), FPA.float_to_bits(1.0))
    6.times { clock.full_cycle }

    assert FPA.bits_to_float(fma.results[0]).nan?
  end

  def test_pipelined_fma_zero_product
    clock = ClockGenerator.new
    fma = FPA::PipelinedFMA.new(clock)

    fma.submit(FPA.float_to_bits(0.0), FPA.float_to_bits(5.0), FPA.float_to_bits(42.0))
    6.times { clock.full_cycle }

    assert_in_delta 42.0, FPA.bits_to_float(fma.results[0]), 1e-6
  end

  # --- FPUnit ---

  def test_fp_unit_all_pipelines
    clock = ClockGenerator.new
    fp_unit = FPA::FPUnit.new(clock)

    # Submit to all three pipelines
    fp_unit.adder.submit(FPA.float_to_bits(1.0), FPA.float_to_bits(2.0))
    fp_unit.multiplier.submit(FPA.float_to_bits(3.0), FPA.float_to_bits(4.0))
    fp_unit.fma.submit(FPA.float_to_bits(2.0), FPA.float_to_bits(3.0), FPA.float_to_bits(1.0))

    fp_unit.tick(10)

    assert_equal 1, fp_unit.adder.results.length
    assert_equal 1, fp_unit.multiplier.results.length
    assert_equal 1, fp_unit.fma.results.length

    assert_in_delta 3.0, FPA.bits_to_float(fp_unit.adder.results[0]), 1e-6
    assert_in_delta 12.0, FPA.bits_to_float(fp_unit.multiplier.results[0]), 1e-6
    assert_in_delta 7.0, FPA.bits_to_float(fp_unit.fma.results[0]), 1e-6
  end

  def test_fp_unit_tick
    clock = ClockGenerator.new
    fp_unit = FPA::FPUnit.new(clock)
    fp_unit.tick(5)
    assert_equal 5, clock.cycle
  end

  # --- Pipeline with cancellation ---

  def test_pipelined_adder_subtraction_to_zero
    clock = ClockGenerator.new
    adder = FPA::PipelinedFPAdder.new(clock)

    a = FPA.float_to_bits(5.0)
    b = FPA.float_to_bits(-5.0)
    adder.submit(a, b)
    5.times { clock.full_cycle }

    assert_equal 1, adder.results.length
    assert_in_delta 0.0, FPA.bits_to_float(adder.results[0]), 1e-6
  end

  def test_pipelined_adder_inf_plus_neg_inf
    clock = ClockGenerator.new
    adder = FPA::PipelinedFPAdder.new(clock)

    adder.submit(FPA.float_to_bits(Float::INFINITY), FPA.float_to_bits(-Float::INFINITY))
    5.times { clock.full_cycle }

    assert FPA.bits_to_float(adder.results[0]).nan?
  end

  def test_pipelined_adder_zero_plus_value
    clock = ClockGenerator.new
    adder = FPA::PipelinedFPAdder.new(clock)

    adder.submit(FPA.float_to_bits(0.0), FPA.float_to_bits(7.0))
    5.times { clock.full_cycle }

    assert_in_delta 7.0, FPA.bits_to_float(adder.results[0]), 1e-6
  end

  def test_pipelined_multiplier_inf
    clock = ClockGenerator.new
    mul = FPA::PipelinedFPMultiplier.new(clock)

    mul.submit(FPA.float_to_bits(Float::INFINITY), FPA.float_to_bits(2.0))
    4.times { clock.full_cycle }

    assert_equal Float::INFINITY, FPA.bits_to_float(mul.results[0])
  end

  def test_pipelined_fma_c_inf
    clock = ClockGenerator.new
    fma = FPA::PipelinedFMA.new(clock)

    fma.submit(FPA.float_to_bits(1.0), FPA.float_to_bits(1.0), FPA.float_to_bits(Float::INFINITY))
    6.times { clock.full_cycle }

    assert_equal Float::INFINITY, FPA.bits_to_float(fma.results[0])
  end

  def test_pipelined_fma_inf_product_neg_inf_c
    clock = ClockGenerator.new
    fma = FPA::PipelinedFMA.new(clock)

    fma.submit(FPA.float_to_bits(Float::INFINITY), FPA.float_to_bits(1.0), FPA.float_to_bits(-Float::INFINITY))
    6.times { clock.full_cycle }

    assert FPA.bits_to_float(fma.results[0]).nan?
  end

  def test_pipelined_fma_zero_product_zero_c
    clock = ClockGenerator.new
    fma = FPA::PipelinedFMA.new(clock)

    fma.submit(FPA.float_to_bits(0.0), FPA.float_to_bits(1.0), FPA.float_to_bits(0.0))
    6.times { clock.full_cycle }

    assert FPA.zero?(fma.results[0])
  end

  def test_pipelined_fma_cancellation
    clock = ClockGenerator.new
    fma = FPA::PipelinedFMA.new(clock)

    # 3 * 4 + (-12) = 0
    fma.submit(FPA.float_to_bits(3.0), FPA.float_to_bits(4.0), FPA.float_to_bits(-12.0))
    6.times { clock.full_cycle }

    assert_in_delta 0.0, FPA.bits_to_float(fma.results[0]), 1e-6
  end

  def test_pipeline_empty_cycles_produce_no_results
    clock = ClockGenerator.new
    adder = FPA::PipelinedFPAdder.new(clock)

    # Run cycles with no submissions
    10.times { clock.full_cycle }
    assert_equal 0, adder.results.length
  end

  def test_pipelined_adder_negative_numbers
    clock = ClockGenerator.new
    adder = FPA::PipelinedFPAdder.new(clock)

    adder.submit(FPA.float_to_bits(-3.0), FPA.float_to_bits(-7.0))
    5.times { clock.full_cycle }

    assert_in_delta(-10.0, FPA.bits_to_float(adder.results[0]), 1e-6)
  end
end
