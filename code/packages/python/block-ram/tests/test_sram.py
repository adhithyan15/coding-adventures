"""Tests for SRAM cells and arrays.

Coverage targets:
- SRAMCell: read, write, hold behavior, initial state, validation
- SRAMArray: read, write, shape, row validation, data validation
- _validate_bit: type checks, range checks
"""

from __future__ import annotations

import pytest

from block_ram.sram import SRAMArray, SRAMCell, _validate_bit

# ─── _validate_bit ────────────────────────────────────────────────────

class TestValidateBit:
    """Tests for the bit validation helper."""

    def test_valid_zero(self) -> None:
        _validate_bit(0, "test")

    def test_valid_one(self) -> None:
        _validate_bit(1, "test")

    def test_rejects_two(self) -> None:
        with pytest.raises(ValueError, match="must be 0 or 1"):
            _validate_bit(2, "test")

    def test_rejects_negative(self) -> None:
        with pytest.raises(ValueError, match="must be 0 or 1"):
            _validate_bit(-1, "test")

    def test_rejects_bool_true(self) -> None:
        with pytest.raises(TypeError, match="must be an int"):
            _validate_bit(True, "test")  # type: ignore[arg-type]

    def test_rejects_bool_false(self) -> None:
        with pytest.raises(TypeError, match="must be an int"):
            _validate_bit(False, "test")  # type: ignore[arg-type]

    def test_rejects_float(self) -> None:
        with pytest.raises(TypeError, match="must be an int"):
            _validate_bit(1.0, "test")  # type: ignore[arg-type]

    def test_rejects_string(self) -> None:
        with pytest.raises(TypeError, match="must be an int"):
            _validate_bit("1", "test")  # type: ignore[arg-type]

    def test_rejects_none(self) -> None:
        with pytest.raises(TypeError, match="must be an int"):
            _validate_bit(None, "test")  # type: ignore[arg-type]


# ─── SRAMCell ─────────────────────────────────────────────────────────

class TestSRAMCell:
    """Tests for the single-bit SRAM cell."""

    def test_initial_value_is_zero(self) -> None:
        cell = SRAMCell()
        assert cell.value == 0

    def test_read_when_selected(self) -> None:
        cell = SRAMCell()
        assert cell.read(word_line=1) == 0

    def test_read_when_not_selected_returns_none(self) -> None:
        cell = SRAMCell()
        assert cell.read(word_line=0) is None

    def test_write_one(self) -> None:
        cell = SRAMCell()
        cell.write(word_line=1, bit_line=1)
        assert cell.value == 1
        assert cell.read(word_line=1) == 1

    def test_write_zero_after_one(self) -> None:
        cell = SRAMCell()
        cell.write(word_line=1, bit_line=1)
        cell.write(word_line=1, bit_line=0)
        assert cell.value == 0

    def test_write_ignored_when_not_selected(self) -> None:
        """When word_line=0, writes have no effect (hold mode)."""
        cell = SRAMCell()
        cell.write(word_line=1, bit_line=1)
        cell.write(word_line=0, bit_line=0)  # Should be ignored
        assert cell.value == 1

    def test_hold_preserves_value(self) -> None:
        """Cell retains value when not selected."""
        cell = SRAMCell()
        cell.write(word_line=1, bit_line=1)
        # Multiple reads with word_line=0 shouldn't change anything
        for _ in range(10):
            cell.read(word_line=0)
        assert cell.value == 1

    def test_read_validates_word_line(self) -> None:
        cell = SRAMCell()
        with pytest.raises(ValueError, match="must be 0 or 1"):
            cell.read(word_line=2)

    def test_write_validates_word_line(self) -> None:
        cell = SRAMCell()
        with pytest.raises(ValueError, match="must be 0 or 1"):
            cell.write(word_line=2, bit_line=0)

    def test_write_validates_bit_line(self) -> None:
        cell = SRAMCell()
        with pytest.raises(ValueError, match="must be 0 or 1"):
            cell.write(word_line=1, bit_line=2)

    def test_write_validates_bit_line_type(self) -> None:
        cell = SRAMCell()
        with pytest.raises(TypeError, match="must be an int"):
            cell.write(word_line=1, bit_line=True)  # type: ignore[arg-type]


# ─── SRAMArray ────────────────────────────────────────────────────────

class TestSRAMArray:
    """Tests for the 2D SRAM array."""

    def test_shape(self) -> None:
        arr = SRAMArray(4, 8)
        assert arr.shape == (4, 8)

    def test_initial_values_all_zero(self) -> None:
        arr = SRAMArray(2, 4)
        assert arr.read(0) == [0, 0, 0, 0]
        assert arr.read(1) == [0, 0, 0, 0]

    def test_write_and_read_row(self) -> None:
        arr = SRAMArray(4, 8)
        data = [1, 0, 1, 0, 0, 1, 0, 1]
        arr.write(0, data)
        assert arr.read(0) == data

    def test_write_does_not_affect_other_rows(self) -> None:
        arr = SRAMArray(4, 4)
        arr.write(0, [1, 1, 1, 1])
        assert arr.read(1) == [0, 0, 0, 0]
        assert arr.read(2) == [0, 0, 0, 0]
        assert arr.read(3) == [0, 0, 0, 0]

    def test_overwrite_row(self) -> None:
        arr = SRAMArray(2, 4)
        arr.write(0, [1, 1, 1, 1])
        arr.write(0, [0, 0, 0, 0])
        assert arr.read(0) == [0, 0, 0, 0]

    def test_multiple_rows(self) -> None:
        arr = SRAMArray(4, 2)
        arr.write(0, [0, 0])
        arr.write(1, [0, 1])
        arr.write(2, [1, 0])
        arr.write(3, [1, 1])
        assert arr.read(0) == [0, 0]
        assert arr.read(1) == [0, 1]
        assert arr.read(2) == [1, 0]
        assert arr.read(3) == [1, 1]

    def test_single_cell_array(self) -> None:
        arr = SRAMArray(1, 1)
        assert arr.shape == (1, 1)
        assert arr.read(0) == [0]
        arr.write(0, [1])
        assert arr.read(0) == [1]

    def test_read_returns_copy(self) -> None:
        """Modifying returned list shouldn't change stored data."""
        arr = SRAMArray(1, 4)
        arr.write(0, [1, 0, 1, 0])
        result = arr.read(0)
        result[0] = 0  # Mutate the returned list
        assert arr.read(0) == [1, 0, 1, 0]  # Original unchanged

    # ── Validation ────────────────────────────────────────────────

    def test_rejects_zero_rows(self) -> None:
        with pytest.raises(ValueError, match="rows must be >= 1"):
            SRAMArray(0, 4)

    def test_rejects_negative_rows(self) -> None:
        with pytest.raises(ValueError, match="rows must be >= 1"):
            SRAMArray(-1, 4)

    def test_rejects_zero_cols(self) -> None:
        with pytest.raises(ValueError, match="cols must be >= 1"):
            SRAMArray(4, 0)

    def test_rejects_negative_cols(self) -> None:
        with pytest.raises(ValueError, match="cols must be >= 1"):
            SRAMArray(4, -1)

    def test_read_rejects_out_of_range_row(self) -> None:
        arr = SRAMArray(4, 4)
        with pytest.raises(ValueError, match="out of range"):
            arr.read(4)

    def test_read_rejects_negative_row(self) -> None:
        arr = SRAMArray(4, 4)
        with pytest.raises(ValueError, match="out of range"):
            arr.read(-1)

    def test_read_rejects_bool_row(self) -> None:
        arr = SRAMArray(4, 4)
        with pytest.raises(TypeError, match="must be an int"):
            arr.read(True)  # type: ignore[arg-type]

    def test_write_rejects_wrong_data_length(self) -> None:
        arr = SRAMArray(4, 4)
        with pytest.raises(ValueError, match="does not match cols"):
            arr.write(0, [1, 0])

    def test_write_rejects_non_list_data(self) -> None:
        arr = SRAMArray(4, 4)
        with pytest.raises(TypeError, match="must be a list"):
            arr.write(0, (1, 0, 1, 0))  # type: ignore[arg-type]

    def test_write_rejects_invalid_bit(self) -> None:
        arr = SRAMArray(4, 4)
        with pytest.raises(ValueError, match="must be 0 or 1"):
            arr.write(0, [1, 0, 2, 0])

    def test_write_rejects_out_of_range_row(self) -> None:
        arr = SRAMArray(4, 4)
        with pytest.raises(ValueError, match="out of range"):
            arr.write(4, [0, 0, 0, 0])
