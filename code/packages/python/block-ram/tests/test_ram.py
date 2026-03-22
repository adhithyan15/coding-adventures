"""Tests for RAM modules — SinglePortRAM and DualPortRAM.

Coverage targets:
- SinglePortRAM: all three read modes, rising edge detection, validation
- DualPortRAM: dual-port access, write collision, all read modes
"""

from __future__ import annotations

import pytest

from block_ram.ram import (
    DualPortRAM,
    ReadMode,
    SinglePortRAM,
    WriteCollisionError,
)

# ─── Helper ───────────────────────────────────────────────────────────

def write_cycle(
    ram: SinglePortRAM,
    address: int,
    data: list[int],
) -> list[int]:
    """Perform a full write cycle (clock 0 then 1)."""
    ram.tick(0, address, data, write_enable=1)
    return ram.tick(1, address, data, write_enable=1)


def read_cycle(
    ram: SinglePortRAM,
    address: int,
) -> list[int]:
    """Perform a full read cycle (clock 0 then 1)."""
    zeros = [0] * ram.width
    ram.tick(0, address, zeros, write_enable=0)
    return ram.tick(1, address, zeros, write_enable=0)


# ─── SinglePortRAM ────────────────────────────────────────────────────

class TestSinglePortRAM:
    """Tests for single-port synchronous RAM."""

    def test_initial_read_is_zeros(self) -> None:
        ram = SinglePortRAM(depth=4, width=8)
        result = read_cycle(ram, 0)
        assert result == [0] * 8

    def test_write_then_read(self) -> None:
        ram = SinglePortRAM(depth=4, width=8)
        data = [1, 0, 1, 0, 0, 1, 0, 1]
        write_cycle(ram, 0, data)
        result = read_cycle(ram, 0)
        assert result == data

    def test_write_to_different_addresses(self) -> None:
        ram = SinglePortRAM(depth=4, width=4)
        write_cycle(ram, 0, [1, 0, 0, 0])
        write_cycle(ram, 1, [0, 1, 0, 0])
        write_cycle(ram, 2, [0, 0, 1, 0])
        write_cycle(ram, 3, [0, 0, 0, 1])
        assert read_cycle(ram, 0) == [1, 0, 0, 0]
        assert read_cycle(ram, 1) == [0, 1, 0, 0]
        assert read_cycle(ram, 2) == [0, 0, 1, 0]
        assert read_cycle(ram, 3) == [0, 0, 0, 1]

    def test_overwrite(self) -> None:
        ram = SinglePortRAM(depth=4, width=4)
        write_cycle(ram, 0, [1, 1, 1, 1])
        write_cycle(ram, 0, [0, 0, 0, 0])
        assert read_cycle(ram, 0) == [0, 0, 0, 0]

    def test_no_rising_edge_no_operation(self) -> None:
        """Operations only happen on 0→1 transition."""
        ram = SinglePortRAM(depth=4, width=4)
        # Stay at 0 — nothing should happen
        result = ram.tick(0, 0, [1, 1, 1, 1], write_enable=1)
        assert result == [0, 0, 0, 0]  # last_read is still zeros
        # 1→1 is not a rising edge
        ram.tick(1, 0, [1, 1, 1, 1], write_enable=1)  # This IS a rising edge
        result = ram.tick(1, 0, [0, 0, 0, 0], write_enable=0)
        # Not a rising edge (1→1), so returns last_read
        assert result == [0, 0, 0, 0]

    def test_properties(self) -> None:
        ram = SinglePortRAM(depth=256, width=16)
        assert ram.depth == 256
        assert ram.width == 16

    def test_dump(self) -> None:
        ram = SinglePortRAM(depth=4, width=2)
        write_cycle(ram, 0, [1, 0])
        write_cycle(ram, 2, [0, 1])
        contents = ram.dump()
        assert contents == [[1, 0], [0, 0], [0, 1], [0, 0]]

    # ── Read modes ────────────────────────────────────────────────

    def test_read_first_returns_old_value_during_write(self) -> None:
        """READ_FIRST: data_out = old value before the write takes effect."""
        ram = SinglePortRAM(depth=4, width=4, read_mode=ReadMode.READ_FIRST)
        write_cycle(ram, 0, [1, 0, 1, 0])
        # Now write new data — should return the OLD value
        result = write_cycle(ram, 0, [0, 1, 0, 1])
        assert result == [1, 0, 1, 0]  # Old value
        # But reading back should show new value
        assert read_cycle(ram, 0) == [0, 1, 0, 1]

    def test_write_first_returns_new_value_during_write(self) -> None:
        """WRITE_FIRST: data_out = new value being written."""
        ram = SinglePortRAM(depth=4, width=4, read_mode=ReadMode.WRITE_FIRST)
        write_cycle(ram, 0, [1, 0, 1, 0])
        # Now write new data — should return the NEW value
        result = write_cycle(ram, 0, [0, 1, 0, 1])
        assert result == [0, 1, 0, 1]  # New value

    def test_no_change_retains_previous_output_during_write(self) -> None:
        """NO_CHANGE: data_out retains previous read value during writes."""
        ram = SinglePortRAM(depth=4, width=4, read_mode=ReadMode.NO_CHANGE)
        # Read address 0 to set last_read
        initial = read_cycle(ram, 0)
        assert initial == [0, 0, 0, 0]
        # Write — output should remain [0,0,0,0] (the last read value)
        result = write_cycle(ram, 0, [1, 1, 1, 1])
        assert result == [0, 0, 0, 0]
        # But the data IS written
        assert read_cycle(ram, 0) == [1, 1, 1, 1]

    # ── Validation ────────────────────────────────────────────────

    def test_rejects_zero_depth(self) -> None:
        with pytest.raises(ValueError, match="depth must be >= 1"):
            SinglePortRAM(depth=0, width=8)

    def test_rejects_zero_width(self) -> None:
        with pytest.raises(ValueError, match="width must be >= 1"):
            SinglePortRAM(depth=4, width=0)

    def test_rejects_out_of_range_address(self) -> None:
        ram = SinglePortRAM(depth=4, width=4)
        with pytest.raises(ValueError, match="out of range"):
            ram.tick(1, 4, [0, 0, 0, 0], write_enable=0)

    def test_rejects_negative_address(self) -> None:
        ram = SinglePortRAM(depth=4, width=4)
        with pytest.raises(ValueError, match="out of range"):
            ram.tick(1, -1, [0, 0, 0, 0], write_enable=0)

    def test_rejects_bool_address(self) -> None:
        ram = SinglePortRAM(depth=4, width=4)
        with pytest.raises(TypeError, match="must be an int"):
            ram.tick(1, True, [0, 0, 0, 0], write_enable=0)  # type: ignore[arg-type]

    def test_rejects_wrong_data_length(self) -> None:
        ram = SinglePortRAM(depth=4, width=4)
        with pytest.raises(ValueError, match="does not match width"):
            ram.tick(1, 0, [0, 0], write_enable=0)

    def test_rejects_non_list_data(self) -> None:
        ram = SinglePortRAM(depth=4, width=4)
        with pytest.raises(TypeError, match="must be a list"):
            ram.tick(1, 0, (0, 0, 0, 0), write_enable=0)  # type: ignore[arg-type]

    def test_rejects_invalid_clock(self) -> None:
        ram = SinglePortRAM(depth=4, width=4)
        with pytest.raises(ValueError, match="must be 0 or 1"):
            ram.tick(2, 0, [0, 0, 0, 0], write_enable=0)

    def test_rejects_invalid_write_enable(self) -> None:
        ram = SinglePortRAM(depth=4, width=4)
        with pytest.raises(ValueError, match="must be 0 or 1"):
            ram.tick(1, 0, [0, 0, 0, 0], write_enable=2)

    def test_rejects_invalid_bit_in_data(self) -> None:
        ram = SinglePortRAM(depth=4, width=4)
        with pytest.raises(ValueError, match="must be 0 or 1"):
            ram.tick(1, 0, [0, 2, 0, 0], write_enable=0)


# ─── DualPortRAM ──────────────────────────────────────────────────────

class TestDualPortRAM:
    """Tests for dual-port synchronous RAM."""

    def test_simultaneous_reads(self) -> None:
        """Both ports can read different addresses simultaneously."""
        ram = DualPortRAM(depth=4, width=4)
        # Write data to two addresses using port A
        zeros = [0] * 4
        ram.tick(0, 0, [1, 0, 1, 0], 1, 1, zeros, 0)
        ram.tick(1, 0, [1, 0, 1, 0], 1, 1, zeros, 0)
        ram.tick(0, 1, [0, 1, 0, 1], 1, 2, zeros, 0)
        ram.tick(1, 1, [0, 1, 0, 1], 1, 2, zeros, 0)

        # Read both addresses simultaneously
        ram.tick(0, 0, zeros, 0, 1, zeros, 0)
        out_a, out_b = ram.tick(1, 0, zeros, 0, 1, zeros, 0)
        assert out_a == [1, 0, 1, 0]
        assert out_b == [0, 1, 0, 1]

    def test_write_a_read_b_different_addresses(self) -> None:
        """Port A writes while port B reads a different address."""
        ram = DualPortRAM(depth=4, width=4)
        zeros = [0] * 4
        # Write via A, read via B simultaneously
        ram.tick(0, 0, [1, 1, 0, 0], 1, 1, zeros, 0)
        out_a, out_b = ram.tick(1, 0, [1, 1, 0, 0], 1, 1, zeros, 0)
        # Port B reads address 1 which is still zeros
        assert out_b == [0, 0, 0, 0]
        # Port A wrote address 0 (READ_FIRST: returns old value)
        assert out_a == [0, 0, 0, 0]

    def test_write_collision_raises_error(self) -> None:
        """Both ports writing to the same address = collision."""
        ram = DualPortRAM(depth=4, width=4)
        ram.tick(0, 0, [1, 0, 0, 0], 1, 0, [0, 1, 0, 0], 1)
        with pytest.raises(WriteCollisionError, match="address 0"):
            ram.tick(1, 0, [1, 0, 0, 0], 1, 0, [0, 1, 0, 0], 1)

    def test_write_collision_error_has_address(self) -> None:
        """WriteCollisionError stores the conflicting address."""
        err = WriteCollisionError(42)
        assert err.address == 42
        assert "42" in str(err)

    def test_both_writes_different_addresses_ok(self) -> None:
        """Both ports can write simultaneously to different addresses."""
        ram = DualPortRAM(depth=4, width=4)
        ram.tick(0, 0, [1, 0, 0, 0], 1, 1, [0, 1, 0, 0], 1)
        ram.tick(1, 0, [1, 0, 0, 0], 1, 1, [0, 1, 0, 0], 1)
        # Verify both writes succeeded
        zeros = [0] * 4
        ram.tick(0, 0, zeros, 0, 1, zeros, 0)
        out_a, out_b = ram.tick(1, 0, zeros, 0, 1, zeros, 0)
        assert out_a == [1, 0, 0, 0]
        assert out_b == [0, 1, 0, 0]

    def test_no_rising_edge_no_operation(self) -> None:
        ram = DualPortRAM(depth=4, width=4)
        zeros = [0] * 4
        out_a, out_b = ram.tick(0, 0, zeros, 0, 0, zeros, 0)
        assert out_a == [0, 0, 0, 0]
        assert out_b == [0, 0, 0, 0]

    def test_properties(self) -> None:
        ram = DualPortRAM(depth=128, width=16)
        assert ram.depth == 128
        assert ram.width == 16

    # ── Read modes per port ───────────────────────────────────────

    def test_read_first_on_port_a(self) -> None:
        ram = DualPortRAM(depth=4, width=4, read_mode_a=ReadMode.READ_FIRST)
        zeros = [0] * 4
        # Write [1,1,1,1] to address 0 via port A
        ram.tick(0, 0, [1, 1, 1, 1], 1, 0, zeros, 0)
        ram.tick(1, 0, [1, 1, 1, 1], 1, 0, zeros, 0)
        # Now overwrite — READ_FIRST should return old value
        ram.tick(0, 0, [0, 0, 0, 0], 1, 1, zeros, 0)
        out_a, _ = ram.tick(1, 0, [0, 0, 0, 0], 1, 1, zeros, 0)
        assert out_a == [1, 1, 1, 1]

    def test_write_first_on_port_b(self) -> None:
        ram = DualPortRAM(
            depth=4, width=4,
            read_mode_a=ReadMode.READ_FIRST,
            read_mode_b=ReadMode.WRITE_FIRST,
        )
        zeros = [0] * 4
        # Write via port B with WRITE_FIRST — should return new value
        ram.tick(0, 0, zeros, 0, 0, [1, 0, 1, 0], 1)
        _, out_b = ram.tick(1, 0, zeros, 0, 0, [1, 0, 1, 0], 1)
        assert out_b == [1, 0, 1, 0]

    def test_no_change_on_port_a(self) -> None:
        ram = DualPortRAM(
            depth=4, width=4,
            read_mode_a=ReadMode.NO_CHANGE,
        )
        zeros = [0] * 4
        # Read first to set last_read_a
        ram.tick(0, 0, zeros, 0, 0, zeros, 0)
        ram.tick(1, 0, zeros, 0, 0, zeros, 0)
        # Write via port A — NO_CHANGE should keep previous read
        ram.tick(0, 0, [1, 1, 1, 1], 1, 1, zeros, 0)
        out_a, _ = ram.tick(1, 0, [1, 1, 1, 1], 1, 1, zeros, 0)
        assert out_a == [0, 0, 0, 0]  # Previous read value

    # ── Validation ────────────────────────────────────────────────

    def test_rejects_zero_depth(self) -> None:
        with pytest.raises(ValueError, match="depth must be >= 1"):
            DualPortRAM(depth=0, width=8)

    def test_rejects_zero_width(self) -> None:
        with pytest.raises(ValueError, match="width must be >= 1"):
            DualPortRAM(depth=4, width=0)

    def test_rejects_invalid_address_a(self) -> None:
        ram = DualPortRAM(depth=4, width=4)
        zeros = [0] * 4
        with pytest.raises(ValueError, match="out of range"):
            ram.tick(1, 4, zeros, 0, 0, zeros, 0)

    def test_rejects_invalid_address_b(self) -> None:
        ram = DualPortRAM(depth=4, width=4)
        zeros = [0] * 4
        with pytest.raises(ValueError, match="out of range"):
            ram.tick(1, 0, zeros, 0, 4, zeros, 0)

    def test_rejects_bool_address(self) -> None:
        ram = DualPortRAM(depth=4, width=4)
        zeros = [0] * 4
        with pytest.raises(TypeError, match="must be an int"):
            ram.tick(1, True, zeros, 0, 0, zeros, 0)  # type: ignore[arg-type]

    def test_rejects_wrong_data_length_a(self) -> None:
        ram = DualPortRAM(depth=4, width=4)
        zeros = [0] * 4
        with pytest.raises(ValueError, match="does not match width"):
            ram.tick(1, 0, [0, 0], 0, 0, zeros, 0)

    def test_rejects_wrong_data_length_b(self) -> None:
        ram = DualPortRAM(depth=4, width=4)
        zeros = [0] * 4
        with pytest.raises(ValueError, match="does not match width"):
            ram.tick(1, 0, zeros, 0, 0, [0, 0], 0)

    def test_rejects_non_list_data(self) -> None:
        ram = DualPortRAM(depth=4, width=4)
        zeros = [0] * 4
        with pytest.raises(TypeError, match="must be a list"):
            ram.tick(1, 0, (0, 0, 0, 0), 0, 0, zeros, 0)  # type: ignore[arg-type]

    def test_rejects_invalid_clock(self) -> None:
        ram = DualPortRAM(depth=4, width=4)
        zeros = [0] * 4
        with pytest.raises(ValueError, match="must be 0 or 1"):
            ram.tick(2, 0, zeros, 0, 0, zeros, 0)

    def test_rejects_invalid_write_enable_a(self) -> None:
        ram = DualPortRAM(depth=4, width=4)
        zeros = [0] * 4
        with pytest.raises(ValueError, match="must be 0 or 1"):
            ram.tick(1, 0, zeros, 2, 0, zeros, 0)

    def test_rejects_invalid_write_enable_b(self) -> None:
        ram = DualPortRAM(depth=4, width=4)
        zeros = [0] * 4
        with pytest.raises(ValueError, match="must be 0 or 1"):
            ram.tick(1, 0, zeros, 0, 0, zeros, 2)
