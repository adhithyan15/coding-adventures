"""Tests for SparseMemory -- port of the Go test suite (24 tests)."""

import pytest

from cpu_simulator.sparse_memory import MemoryRegion, SparseMemory


# === Test helpers ===


def make_test_sparse_memory() -> SparseMemory:
    """Create a SparseMemory with two non-contiguous regions:
    - RAM at 0x00000000, 4096 bytes (read/write)
    - ROM at 0xFFFF0000, 256 bytes (read-only)
    """
    return SparseMemory([
        MemoryRegion(base=0x00000000, size=4096, name="RAM"),
        MemoryRegion(base=0xFFFF0000, size=256, name="ROM", read_only=True),
    ])


# === Construction tests ===


class TestNewSparseMemory:
    def test_allocates_regions(self) -> None:
        mem = make_test_sparse_memory()
        assert mem.region_count() == 2

        # Verify RAM region
        assert mem.regions[0].name == "RAM"
        assert mem.regions[0].base == 0x00000000
        assert mem.regions[0].size == 4096
        assert len(mem.regions[0].data) == 4096
        assert mem.regions[0].read_only is False

        # Verify ROM region
        assert mem.regions[1].name == "ROM"
        assert mem.regions[1].read_only is True

    def test_pre_populated_data(self) -> None:
        rom_data = bytearray(64)
        rom_data[0] = 0xAA
        rom_data[63] = 0xBB

        mem = SparseMemory([
            MemoryRegion(base=0x1000, size=64, data=rom_data, name="ROM", read_only=True),
        ])

        assert mem.read_byte(0x1000) == 0xAA
        assert mem.read_byte(0x103F) == 0xBB

    def test_zero_initialized(self) -> None:
        mem = make_test_sparse_memory()
        for i in range(16):
            assert mem.read_byte(i) == 0


# === Byte read/write tests ===


class TestReadWriteByte:
    def test_basic(self) -> None:
        mem = make_test_sparse_memory()
        mem.write_byte(0x0000, 0x42)
        mem.write_byte(0x0001, 0xFF)
        mem.write_byte(0x0FFF, 0x99)

        assert mem.read_byte(0x0000) == 0x42
        assert mem.read_byte(0x0001) == 0xFF
        assert mem.read_byte(0x0FFF) == 0x99

    def test_read_only_write_silently_ignored(self) -> None:
        mem = make_test_sparse_memory()
        assert mem.read_byte(0xFFFF0000) == 0

        mem.write_byte(0xFFFF0000, 0xDE)
        assert mem.read_byte(0xFFFF0000) == 0


# === Word read/write tests ===


class TestReadWriteWord:
    def test_little_endian(self) -> None:
        mem = make_test_sparse_memory()
        mem.write_word(0x0100, 0xDEADBEEF)

        # Check individual bytes (little-endian)
        assert mem.read_byte(0x0100) == 0xEF
        assert mem.read_byte(0x0101) == 0xBE
        assert mem.read_byte(0x0102) == 0xAD
        assert mem.read_byte(0x0103) == 0xDE

        # Read back as word
        assert mem.read_word(0x0100) == 0xDEADBEEF

    def test_write_word_read_only(self) -> None:
        mem = make_test_sparse_memory()
        mem.write_word(0xFFFF0000, 0x12345678)
        assert mem.read_word(0xFFFF0000) == 0x00000000

    def test_word_round_trip(self) -> None:
        mem = make_test_sparse_memory()
        test_cases = [
            (0x0000, 0x00000000),
            (0x0004, 0xFFFFFFFF),
            (0x0008, 0x00000001),
            (0x000C, 0x80000000),
            (0x0010, 0x7FFFFFFF),
            (0x0014, 0x01020304),
        ]
        for addr, val in test_cases:
            mem.write_word(addr, val)
            assert mem.read_word(addr) == val, f"at 0x{addr:04X}: wrote 0x{val:08X}"


# === LoadBytes tests ===


class TestLoadBytes:
    def test_basic(self) -> None:
        mem = make_test_sparse_memory()
        data = bytes([0x01, 0x02, 0x03, 0x04, 0x05])
        mem.load_bytes(0x0200, data)

        for i, expected in enumerate(data):
            assert mem.read_byte(0x0200 + i) == expected

    def test_into_read_only_region(self) -> None:
        """LoadBytes should bypass the read_only check -- for initialization."""
        mem = make_test_sparse_memory()
        data = bytes([0xAA, 0xBB, 0xCC, 0xDD])
        mem.load_bytes(0xFFFF0000, data)

        assert mem.read_byte(0xFFFF0000) == 0xAA
        assert mem.read_byte(0xFFFF0003) == 0xDD

        # Subsequent writes via write_byte should still be ignored
        mem.write_byte(0xFFFF0000, 0x00)
        assert mem.read_byte(0xFFFF0000) == 0xAA


# === Dump tests ===


class TestDump:
    def test_basic(self) -> None:
        mem = make_test_sparse_memory()
        mem.write_byte(0x0010, 0xAA)
        mem.write_byte(0x0011, 0xBB)
        mem.write_byte(0x0012, 0xCC)

        dumped = mem.dump(0x0010, 3)
        assert len(dumped) == 3
        assert dumped == [0xAA, 0xBB, 0xCC]

    def test_is_copy(self) -> None:
        mem = make_test_sparse_memory()
        mem.write_byte(0x0000, 0xFF)

        dumped = mem.dump(0x0000, 4)
        dumped[0] = 0x00  # modifying the copy

        # Original should be unchanged
        assert mem.read_byte(0x0000) == 0xFF


# === Unmapped address tests ===


class TestUnmappedAddresses:
    def test_read_byte_unmapped_raises(self) -> None:
        mem = make_test_sparse_memory()
        with pytest.raises(RuntimeError, match="unmapped"):
            mem.read_byte(0x80000000)

    def test_write_byte_unmapped_raises(self) -> None:
        mem = make_test_sparse_memory()
        with pytest.raises(RuntimeError, match="unmapped"):
            mem.write_byte(0x80000000, 0xFF)

    def test_read_word_unmapped_raises(self) -> None:
        mem = make_test_sparse_memory()
        with pytest.raises(RuntimeError, match="unmapped"):
            mem.read_word(0x80000000)

    def test_write_word_unmapped_raises(self) -> None:
        mem = make_test_sparse_memory()
        with pytest.raises(RuntimeError, match="unmapped"):
            mem.write_word(0x80000000, 0xDEAD)

    def test_read_word_crosses_boundary_raises(self) -> None:
        mem = make_test_sparse_memory()
        # RAM ends at 0x1000. A 4-byte read at 0x0FFE goes out of bounds.
        with pytest.raises(RuntimeError, match="unmapped"):
            mem.read_word(0x0FFE)


# === Multiple non-contiguous region tests ===


class TestMultipleRegions:
    def test_isolation(self) -> None:
        mem = SparseMemory([
            MemoryRegion(base=0x00000000, size=1024, name="RAM"),
            MemoryRegion(base=0x10000000, size=256, name="SRAM"),
            MemoryRegion(base=0xFFFF0000, size=128, name="IO"),
        ])

        mem.write_byte(0x00000000, 0x11)
        mem.write_byte(0x10000000, 0x22)
        mem.write_byte(0xFFFF0000, 0x33)

        assert mem.read_byte(0x00000000) == 0x11
        assert mem.read_byte(0x10000000) == 0x22
        assert mem.read_byte(0xFFFF0000) == 0x33


# === High address region tests ===


class TestHighAddressRegion:
    def test_near_top_of_address_space(self) -> None:
        mem = SparseMemory([
            MemoryRegion(base=0xFFFB0000, size=0x50000, name="HIGH_IO"),
        ])

        mem.write_byte(0xFFFB0000, 0x01)
        mem.write_byte(0xFFFFFFFE, 0xFE)
        mem.write_word(0xFFFFFFFC, 0xCAFEBABE)

        assert mem.read_byte(0xFFFB0000) == 0x01
        assert mem.read_word(0xFFFFFFFC) == 0xCAFEBABE


# === LoadBytes as program loader ===


class TestLoadProgram:
    def test_load_program(self) -> None:
        mem = SparseMemory([
            MemoryRegion(base=0x00000000, size=0x10000, name="RAM"),
        ])

        # Simulate loading a small RISC-V program (4 instructions)
        program = bytes([
            0x93, 0x00, 0xA0, 0x02,  # addi x1, x0, 42
            0x13, 0x01, 0x30, 0x00,  # addi x2, x0, 3
            0xB3, 0x01, 0x21, 0x00,  # add x3, x2, x2
            0x73, 0x00, 0x00, 0x00,  # ecall
        ])
        mem.load_bytes(0x0000, program)

        word0 = mem.read_word(0x0000)
        assert word0 == 0x02A00093


# === findRegion edge cases ===


class TestFindRegionEdgeCases:
    def test_load_bytes_unmapped_raises(self) -> None:
        mem = make_test_sparse_memory()
        with pytest.raises(RuntimeError, match="unmapped"):
            mem.load_bytes(0x80000000, bytes([0x01, 0x02]))

    def test_dump_unmapped_raises(self) -> None:
        mem = make_test_sparse_memory()
        with pytest.raises(RuntimeError, match="unmapped"):
            mem.dump(0x80000000, 4)

    def test_empty_regions(self) -> None:
        mem = SparseMemory([])
        with pytest.raises(RuntimeError, match="unmapped"):
            mem.read_byte(0x0000)

    def test_region_count(self) -> None:
        mem = SparseMemory([
            MemoryRegion(base=0, size=16, name="A"),
            MemoryRegion(base=0x1000, size=16, name="B"),
            MemoryRegion(base=0x2000, size=16, name="C"),
        ])
        assert mem.region_count() == 3
