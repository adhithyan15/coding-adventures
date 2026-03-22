# frozen_string_literal: true

require_relative "test_helper"

# ============================================================================
# Tests for combinational circuits -- MUX, DEMUX, decoder, encoder,
# priority encoder, and tri-state buffer.
# ============================================================================

module CodingAdventures
  Comb = LogicGates::Combinational
end

# === MUX2 ===

class TestMux2 < Minitest::Test
  def test_sel_0_returns_d0
    assert_equal 0, CodingAdventures::Comb.mux2(0, 1, 0)
    assert_equal 1, CodingAdventures::Comb.mux2(1, 0, 0)
  end

  def test_sel_1_returns_d1
    assert_equal 1, CodingAdventures::Comb.mux2(0, 1, 1)
    assert_equal 0, CodingAdventures::Comb.mux2(1, 0, 1)
  end

  def test_all_combinations
    # Exhaustive truth table for 2:1 MUX
    assert_equal 0, CodingAdventures::Comb.mux2(0, 0, 0)
    assert_equal 0, CodingAdventures::Comb.mux2(0, 0, 1)
    assert_equal 0, CodingAdventures::Comb.mux2(0, 1, 0)
    assert_equal 1, CodingAdventures::Comb.mux2(0, 1, 1)
    assert_equal 1, CodingAdventures::Comb.mux2(1, 0, 0)
    assert_equal 0, CodingAdventures::Comb.mux2(1, 0, 1)
    assert_equal 1, CodingAdventures::Comb.mux2(1, 1, 0)
    assert_equal 1, CodingAdventures::Comb.mux2(1, 1, 1)
  end

  def test_validates_inputs
    assert_raises(TypeError) { CodingAdventures::Comb.mux2("0", 1, 0) }
    assert_raises(ArgumentError) { CodingAdventures::Comb.mux2(2, 1, 0) }
  end
end

# === MUX4 ===

class TestMux4 < Minitest::Test
  def test_selects_d0
    assert_equal 1, CodingAdventures::Comb.mux4(1, 0, 0, 0, [0, 0])
  end

  def test_selects_d1
    assert_equal 1, CodingAdventures::Comb.mux4(0, 1, 0, 0, [1, 0])
  end

  def test_selects_d2
    assert_equal 1, CodingAdventures::Comb.mux4(0, 0, 1, 0, [0, 1])
  end

  def test_selects_d3
    assert_equal 1, CodingAdventures::Comb.mux4(0, 0, 0, 1, [1, 1])
  end

  def test_invalid_sel_length
    assert_raises(ArgumentError) do
      CodingAdventures::Comb.mux4(0, 0, 0, 0, [0])
    end
  end

  def test_invalid_sel_type
    assert_raises(ArgumentError) do
      CodingAdventures::Comb.mux4(0, 0, 0, 0, 0)
    end
  end
end

# === MUX_N ===

class TestMuxN < Minitest::Test
  def test_2_input_mux
    assert_equal 0, CodingAdventures::Comb.mux_n([0, 1], [0])
    assert_equal 1, CodingAdventures::Comb.mux_n([0, 1], [1])
  end

  def test_4_input_mux
    assert_equal 1, CodingAdventures::Comb.mux_n([1, 0, 0, 0], [0, 0])
    assert_equal 1, CodingAdventures::Comb.mux_n([0, 1, 0, 0], [1, 0])
    assert_equal 1, CodingAdventures::Comb.mux_n([0, 0, 1, 0], [0, 1])
    assert_equal 1, CodingAdventures::Comb.mux_n([0, 0, 0, 1], [1, 1])
  end

  def test_8_input_mux
    data = [0] * 8
    data[5] = 1
    # 5 in binary LSB-first: [1, 0, 1]
    assert_equal 1, CodingAdventures::Comb.mux_n(data, [1, 0, 1])
  end

  def test_16_input_mux
    data = [0] * 16
    data[10] = 1
    # 10 in binary LSB-first: [0, 1, 0, 1]
    assert_equal 1, CodingAdventures::Comb.mux_n(data, [0, 1, 0, 1])
  end

  def test_non_power_of_2_raises
    assert_raises(ArgumentError) do
      CodingAdventures::Comb.mux_n([0, 0, 0], [0, 0])
    end
  end

  def test_too_few_inputs_raises
    assert_raises(ArgumentError) do
      CodingAdventures::Comb.mux_n([0], [0])
    end
  end

  def test_wrong_sel_length_raises
    assert_raises(ArgumentError) do
      CodingAdventures::Comb.mux_n([0, 1, 0, 1], [0])
    end
  end
end

# === DEMUX ===

class TestDemux < Minitest::Test
  def test_route_to_output_0
    assert_equal [1, 0, 0, 0], CodingAdventures::Comb.demux(1, [0, 0], 4)
  end

  def test_route_to_output_1
    assert_equal [0, 1, 0, 0], CodingAdventures::Comb.demux(1, [1, 0], 4)
  end

  def test_route_to_output_2
    assert_equal [0, 0, 1, 0], CodingAdventures::Comb.demux(1, [0, 1], 4)
  end

  def test_route_to_output_3
    assert_equal [0, 0, 0, 1], CodingAdventures::Comb.demux(1, [1, 1], 4)
  end

  def test_data_0_all_outputs_0
    assert_equal [0, 0, 0, 0], CodingAdventures::Comb.demux(0, [1, 0], 4)
  end

  def test_8_outputs
    result = CodingAdventures::Comb.demux(1, [1, 0, 1], 8)
    expected = [0, 0, 0, 0, 0, 1, 0, 0]
    assert_equal expected, result
  end

  def test_invalid_n_outputs
    assert_raises(ArgumentError) do
      CodingAdventures::Comb.demux(1, [0], 3)
    end
  end
end

# === DECODER ===

class TestDecoder < Minitest::Test
  def test_1_bit_decoder
    assert_equal [1, 0], CodingAdventures::Comb.decoder([0])
    assert_equal [0, 1], CodingAdventures::Comb.decoder([1])
  end

  def test_2_bit_decoder
    assert_equal [1, 0, 0, 0], CodingAdventures::Comb.decoder([0, 0])
    assert_equal [0, 1, 0, 0], CodingAdventures::Comb.decoder([1, 0])
    assert_equal [0, 0, 1, 0], CodingAdventures::Comb.decoder([0, 1])
    assert_equal [0, 0, 0, 1], CodingAdventures::Comb.decoder([1, 1])
  end

  def test_3_bit_decoder
    result = CodingAdventures::Comb.decoder([1, 0, 1])
    # 1 + 0*2 + 1*4 = 5 -> output[5] = 1
    expected = [0, 0, 0, 0, 0, 1, 0, 0]
    assert_equal expected, result
  end

  def test_exactly_one_output_is_1
    result = CodingAdventures::Comb.decoder([0, 1, 0])
    assert_equal 1, result.sum
  end

  def test_empty_inputs_raises
    assert_raises(ArgumentError) { CodingAdventures::Comb.decoder([]) }
  end
end

# === ENCODER ===

class TestEncoder < Minitest::Test
  def test_4_to_2_index_0
    assert_equal [0, 0], CodingAdventures::Comb.encoder([1, 0, 0, 0])
  end

  def test_4_to_2_index_1
    assert_equal [1, 0], CodingAdventures::Comb.encoder([0, 1, 0, 0])
  end

  def test_4_to_2_index_2
    assert_equal [0, 1], CodingAdventures::Comb.encoder([0, 0, 1, 0])
  end

  def test_4_to_2_index_3
    assert_equal [1, 1], CodingAdventures::Comb.encoder([0, 0, 0, 1])
  end

  def test_8_to_3_index_5
    inputs = [0, 0, 0, 0, 0, 1, 0, 0]
    # 5 in binary LSB-first: [1, 0, 1]
    assert_equal [1, 0, 1], CodingAdventures::Comb.encoder(inputs)
  end

  def test_not_one_hot_raises
    assert_raises(ArgumentError) do
      CodingAdventures::Comb.encoder([1, 1, 0, 0])
    end
  end

  def test_no_active_bit_raises
    assert_raises(ArgumentError) do
      CodingAdventures::Comb.encoder([0, 0, 0, 0])
    end
  end

  def test_non_power_of_2_raises
    assert_raises(ArgumentError) do
      CodingAdventures::Comb.encoder([1, 0, 0])
    end
  end
end

# === PRIORITY ENCODER ===

class TestPriorityEncoder < Minitest::Test
  def test_no_active_input
    output, valid = CodingAdventures::Comb.priority_encoder([0, 0, 0, 0])
    assert_equal [0, 0], output
    assert_equal 0, valid
  end

  def test_single_active_input
    output, valid = CodingAdventures::Comb.priority_encoder([0, 0, 1, 0])
    assert_equal [0, 1], output
    assert_equal 1, valid
  end

  def test_highest_priority_wins
    # Both I0 and I2 active, I2 should win (highest index)
    output, valid = CodingAdventures::Comb.priority_encoder([1, 0, 1, 0])
    assert_equal [0, 1], output
    assert_equal 1, valid
  end

  def test_highest_index_always_wins
    output, valid = CodingAdventures::Comb.priority_encoder([1, 1, 1, 1])
    assert_equal [1, 1], output
    assert_equal 1, valid
  end

  def test_index_0_only
    output, valid = CodingAdventures::Comb.priority_encoder([1, 0, 0, 0])
    assert_equal [0, 0], output
    assert_equal 1, valid
  end

  def test_8_inputs
    inputs = [1, 0, 0, 0, 0, 1, 0, 0]
    output, valid = CodingAdventures::Comb.priority_encoder(inputs)
    # 5 in binary LSB-first: [1, 0, 1]
    assert_equal [1, 0, 1], output
    assert_equal 1, valid
  end

  def test_non_power_of_2_raises
    assert_raises(ArgumentError) do
      CodingAdventures::Comb.priority_encoder([0, 0, 0])
    end
  end
end

# === TRI-STATE BUFFER ===

class TestTriState < Minitest::Test
  def test_enabled_passes_data_0
    assert_equal 0, CodingAdventures::Comb.tri_state(0, 1)
  end

  def test_enabled_passes_data_1
    assert_equal 1, CodingAdventures::Comb.tri_state(1, 1)
  end

  def test_disabled_returns_nil
    assert_nil CodingAdventures::Comb.tri_state(0, 0)
    assert_nil CodingAdventures::Comb.tri_state(1, 0)
  end

  def test_validates_data
    assert_raises(TypeError) { CodingAdventures::Comb.tri_state("1", 1) }
    assert_raises(ArgumentError) { CodingAdventures::Comb.tri_state(2, 1) }
  end

  def test_validates_enable
    assert_raises(TypeError) { CodingAdventures::Comb.tri_state(1, "1") }
    assert_raises(ArgumentError) { CodingAdventures::Comb.tri_state(1, 2) }
  end
end
