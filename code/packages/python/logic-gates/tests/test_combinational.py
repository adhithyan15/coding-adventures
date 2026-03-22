"""Tests for combinational circuits: MUX, DEMUX, decoder, encoder, tri-state buffer.

Every circuit is tested against its complete truth table where feasible.
For larger circuits, representative test vectors cover normal operation,
edge cases, and error conditions.
"""

from __future__ import annotations

import pytest

from logic_gates.combinational import (
    decoder,
    demux,
    encoder,
    mux2,
    mux4,
    mux8,
    mux_n,
    priority_encoder,
    tri_state,
)


# ===========================================================================
# MUX2 — 2-to-1 Multiplexer
# ===========================================================================


class TestMux2:
    """Verify the 2-to-1 multiplexer against its complete truth table."""

    def test_sel_0_selects_d0(self) -> None:
        """When sel=0, output should equal d0 regardless of d1."""
        assert mux2(0, 0, 0) == 0
        assert mux2(0, 1, 0) == 0
        assert mux2(1, 0, 0) == 1
        assert mux2(1, 1, 0) == 1

    def test_sel_1_selects_d1(self) -> None:
        """When sel=1, output should equal d1 regardless of d0."""
        assert mux2(0, 0, 1) == 0
        assert mux2(0, 1, 1) == 1
        assert mux2(1, 0, 1) == 0
        assert mux2(1, 1, 1) == 1

    def test_invalid_inputs_raise_error(self) -> None:
        """Non-binary inputs must raise TypeError or ValueError."""
        with pytest.raises(TypeError):
            mux2(True, 0, 0)
        with pytest.raises(ValueError):
            mux2(2, 0, 0)
        with pytest.raises(TypeError):
            mux2(0, 0, True)


# ===========================================================================
# MUX4 — 4-to-1 Multiplexer
# ===========================================================================


class TestMux4:
    """Verify the 4-to-1 multiplexer selects the correct input."""

    def test_all_select_combinations(self) -> None:
        """Each sel value should route the corresponding input."""
        # Set one input to 1 at a time, verify it's selected
        assert mux4(1, 0, 0, 0, [0, 0]) == 1  # sel=00 → d0
        assert mux4(0, 1, 0, 0, [1, 0]) == 1  # sel=01 → d1
        assert mux4(0, 0, 1, 0, [0, 1]) == 1  # sel=10 → d2
        assert mux4(0, 0, 0, 1, [1, 1]) == 1  # sel=11 → d3

    def test_unselected_inputs_ignored(self) -> None:
        """Inputs not selected should not affect the output."""
        assert mux4(0, 1, 1, 1, [0, 0]) == 0  # sel=00 → d0=0, others ignored
        assert mux4(1, 0, 1, 1, [1, 0]) == 0  # sel=01 → d1=0

    def test_invalid_sel_length(self) -> None:
        """sel must be exactly 2 bits."""
        with pytest.raises(ValueError):
            mux4(0, 0, 0, 0, [0])
        with pytest.raises(ValueError):
            mux4(0, 0, 0, 0, [0, 0, 0])

    def test_invalid_sel_type(self) -> None:
        """sel must be a list."""
        with pytest.raises(ValueError):
            mux4(0, 0, 0, 0, 0)  # type: ignore[arg-type]


# ===========================================================================
# MUX8 — 8-to-1 Multiplexer
# ===========================================================================


class TestMux8:
    """Verify the 8-to-1 multiplexer selects the correct input."""

    def test_each_input_selectable(self) -> None:
        """Each of 8 inputs should be individually selectable."""
        for i in range(8):
            inputs = [0] * 8
            inputs[i] = 1
            # Convert index to 3-bit binary (LSB first)
            sel = [(i >> b) & 1 for b in range(3)]
            assert mux8(inputs, sel) == 1, f"Failed to select input {i} with sel={sel}"

    def test_all_zeros(self) -> None:
        """All inputs 0 should always produce 0."""
        for s0 in range(2):
            for s1 in range(2):
                for s2 in range(2):
                    assert mux8([0] * 8, [s0, s1, s2]) == 0

    def test_all_ones(self) -> None:
        """All inputs 1 should always produce 1."""
        for s0 in range(2):
            for s1 in range(2):
                for s2 in range(2):
                    assert mux8([1] * 8, [s0, s1, s2]) == 1

    def test_invalid_inputs_length(self) -> None:
        """inputs must be exactly 8 elements."""
        with pytest.raises(ValueError):
            mux8([0, 0, 0, 0], [0, 0, 0])

    def test_invalid_sel_length(self) -> None:
        """sel must be exactly 3 bits."""
        with pytest.raises(ValueError):
            mux8([0] * 8, [0, 0])


# ===========================================================================
# MUX_N — General N-to-1 Multiplexer
# ===========================================================================


class TestMuxN:
    """Verify the general N-to-1 multiplexer for various sizes."""

    def test_2_to_1(self) -> None:
        """N=2 should behave like mux2."""
        assert mux_n([0, 1], [0]) == 0
        assert mux_n([0, 1], [1]) == 1

    def test_4_to_1(self) -> None:
        """N=4 should select the correct input."""
        for i in range(4):
            inputs = [0] * 4
            inputs[i] = 1
            sel = [(i >> b) & 1 for b in range(2)]
            assert mux_n(inputs, sel) == 1

    def test_16_to_1(self) -> None:
        """N=16 should select the correct input."""
        for i in range(16):
            inputs = [0] * 16
            inputs[i] = 1
            sel = [(i >> b) & 1 for b in range(4)]
            assert mux_n(inputs, sel) == 1, f"Failed for index {i}"

    def test_non_power_of_2_raises(self) -> None:
        """Non-power-of-2 input counts should raise ValueError."""
        with pytest.raises(ValueError, match="power of 2"):
            mux_n([0, 0, 0], [0, 0])

    def test_wrong_sel_length_raises(self) -> None:
        """sel length must match log2(N)."""
        with pytest.raises(ValueError):
            mux_n([0, 0, 0, 0], [0])  # Need 2 sel bits, got 1

    def test_single_input_raises(self) -> None:
        """Must have at least 2 inputs."""
        with pytest.raises(ValueError):
            mux_n([0], [])


# ===========================================================================
# DEMUX — 1-to-N Demultiplexer
# ===========================================================================


class TestDemux:
    """Verify the demultiplexer routes data to the correct output."""

    def test_1_to_4_all_positions(self) -> None:
        """Data=1 should appear at the selected output, zeros elsewhere."""
        assert demux(1, [0, 0], 4) == [1, 0, 0, 0]
        assert demux(1, [1, 0], 4) == [0, 1, 0, 0]
        assert demux(1, [0, 1], 4) == [0, 0, 1, 0]
        assert demux(1, [1, 1], 4) == [0, 0, 0, 1]

    def test_data_0_all_zeros(self) -> None:
        """When data=0, all outputs should be 0 regardless of sel."""
        assert demux(0, [0, 0], 4) == [0, 0, 0, 0]
        assert demux(0, [1, 1], 4) == [0, 0, 0, 0]

    def test_1_to_2(self) -> None:
        """Smallest DEMUX: 1-to-2."""
        assert demux(1, [0], 2) == [1, 0]
        assert demux(1, [1], 2) == [0, 1]

    def test_1_to_8(self) -> None:
        """8-output DEMUX should route to correct position."""
        result = demux(1, [1, 0, 1], 8)  # index = 5
        expected = [0, 0, 0, 0, 0, 1, 0, 0]
        assert result == expected

    def test_invalid_n_outputs(self) -> None:
        """n_outputs must be a power of 2 >= 2."""
        with pytest.raises(ValueError):
            demux(1, [0], 3)
        with pytest.raises(ValueError):
            demux(1, [], 1)


# ===========================================================================
# DECODER — Binary to One-Hot
# ===========================================================================


class TestDecoder:
    """Verify the decoder produces correct one-hot outputs."""

    def test_1_to_2(self) -> None:
        """1-bit input → 2 outputs."""
        assert decoder([0]) == [1, 0]
        assert decoder([1]) == [0, 1]

    def test_2_to_4_exhaustive(self) -> None:
        """2-bit input → 4 outputs, all combinations."""
        assert decoder([0, 0]) == [1, 0, 0, 0]
        assert decoder([1, 0]) == [0, 1, 0, 0]
        assert decoder([0, 1]) == [0, 0, 1, 0]
        assert decoder([1, 1]) == [0, 0, 0, 1]

    def test_3_to_8_exhaustive(self) -> None:
        """3-bit input → 8 outputs, all combinations."""
        for i in range(8):
            input_bits = [(i >> b) & 1 for b in range(3)]
            result = decoder(input_bits)
            expected = [0] * 8
            expected[i] = 1
            assert result == expected, f"Failed for input {input_bits}"

    def test_exactly_one_hot(self) -> None:
        """Every decoder output should have exactly one 1."""
        for i in range(16):
            input_bits = [(i >> b) & 1 for b in range(4)]
            result = decoder(input_bits)
            assert sum(result) == 1, f"Not one-hot for input {input_bits}: {result}"

    def test_empty_input_raises(self) -> None:
        """Empty input list should raise ValueError."""
        with pytest.raises(ValueError):
            decoder([])

    def test_invalid_bit_raises(self) -> None:
        """Non-binary values should raise an error."""
        with pytest.raises(ValueError):
            decoder([2])


# ===========================================================================
# ENCODER — One-Hot to Binary
# ===========================================================================


class TestEncoder:
    """Verify the encoder converts one-hot to binary correctly."""

    def test_4_to_2_all_positions(self) -> None:
        """4-input encoder should produce correct binary for each position."""
        assert encoder([1, 0, 0, 0]) == [0, 0]  # index 0
        assert encoder([0, 1, 0, 0]) == [1, 0]  # index 1
        assert encoder([0, 0, 1, 0]) == [0, 1]  # index 2
        assert encoder([0, 0, 0, 1]) == [1, 1]  # index 3

    def test_8_to_3_all_positions(self) -> None:
        """8-input encoder should produce correct 3-bit binary."""
        for i in range(8):
            inputs = [0] * 8
            inputs[i] = 1
            result = encoder(inputs)
            expected = [(i >> b) & 1 for b in range(3)]
            assert result == expected, f"Failed for index {i}"

    def test_non_one_hot_raises(self) -> None:
        """Multiple active inputs should raise ValueError."""
        with pytest.raises(ValueError, match="one-hot"):
            encoder([1, 1, 0, 0])

    def test_no_active_input_raises(self) -> None:
        """All-zero input should raise ValueError."""
        with pytest.raises(ValueError, match="one-hot"):
            encoder([0, 0, 0, 0])

    def test_non_power_of_2_raises(self) -> None:
        """Input length must be power of 2."""
        with pytest.raises(ValueError, match="power of 2"):
            encoder([1, 0, 0])

    def test_roundtrip_with_decoder(self) -> None:
        """Encoder should be the inverse of decoder for all inputs."""
        for n_bits in [2, 3, 4]:
            n_inputs = 1 << n_bits
            for i in range(n_inputs):
                input_bits = [(i >> b) & 1 for b in range(n_bits)]
                one_hot = decoder(input_bits)
                recovered = encoder(one_hot)
                assert recovered == input_bits, (
                    f"Roundtrip failed for {input_bits}: "
                    f"decoder={one_hot}, encoder={recovered}"
                )


# ===========================================================================
# PRIORITY ENCODER
# ===========================================================================


class TestPriorityEncoder:
    """Verify the priority encoder picks the highest-priority active input."""

    def test_single_input_active(self) -> None:
        """With one input active, output is that input's index."""
        output, valid = priority_encoder([1, 0, 0, 0])
        assert output == [0, 0] and valid == 1

        output, valid = priority_encoder([0, 0, 0, 1])
        assert output == [1, 1] and valid == 1

    def test_highest_priority_wins(self) -> None:
        """When multiple inputs active, highest index wins."""
        output, valid = priority_encoder([1, 0, 1, 0])
        assert output == [0, 1] and valid == 1  # I2 wins over I0

        output, valid = priority_encoder([1, 1, 1, 1])
        assert output == [1, 1] and valid == 1  # I3 wins

    def test_no_input_active(self) -> None:
        """All zeros should produce valid=0."""
        output, valid = priority_encoder([0, 0, 0, 0])
        assert valid == 0
        assert output == [0, 0]  # Output is don't-care, but we produce zeros

    def test_8_input_priority(self) -> None:
        """8-input priority encoder picks highest active."""
        output, valid = priority_encoder([0, 1, 0, 0, 0, 1, 0, 0])
        assert output == [1, 0, 1] and valid == 1  # I5 (binary 101) wins

    def test_only_lowest_active(self) -> None:
        """When only I0 is active, output is index 0."""
        output, valid = priority_encoder([1, 0, 0, 0, 0, 0, 0, 0])
        assert output == [0, 0, 0] and valid == 1

    def test_non_power_of_2_raises(self) -> None:
        """Input length must be power of 2."""
        with pytest.raises(ValueError, match="power of 2"):
            priority_encoder([1, 0, 0])


# ===========================================================================
# TRI-STATE BUFFER
# ===========================================================================


class TestTriState:
    """Verify the tri-state buffer output states."""

    def test_enabled_passes_data(self) -> None:
        """When enable=1, output should equal data."""
        assert tri_state(0, 1) == 0
        assert tri_state(1, 1) == 1

    def test_disabled_returns_none(self) -> None:
        """When enable=0, output should be None (high-impedance)."""
        assert tri_state(0, 0) is None
        assert tri_state(1, 0) is None

    def test_invalid_data_raises(self) -> None:
        """Non-binary data should raise an error."""
        with pytest.raises(ValueError):
            tri_state(2, 1)
        with pytest.raises(TypeError):
            tri_state(True, 1)

    def test_invalid_enable_raises(self) -> None:
        """Non-binary enable should raise an error."""
        with pytest.raises(ValueError):
            tri_state(0, 2)
        with pytest.raises(TypeError):
            tri_state(0, False)
