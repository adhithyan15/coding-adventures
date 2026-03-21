"""Tests for the S02 Bootloader package."""

from bootloader import (
    BOOT_PROTOCOL_MAGIC,
    DEFAULT_DISK_SIZE,
    DEFAULT_ENTRY_ADDRESS,
    DEFAULT_KERNEL_DISK_OFFSET,
    DEFAULT_KERNEL_LOAD_ADDRESS,
    DEFAULT_STACK_BASE,
    DISK_KERNEL_OFFSET,
    DISK_MEMORY_MAP_BASE,
    AnnotatedInstruction,
    Bootloader,
    BootloaderConfig,
    DefaultBootloaderConfig,
    DiskImage,
)


# =========================================================================
# BootloaderConfig Tests
# =========================================================================


class TestBootloaderConfig:
    def test_default_config(self) -> None:
        config = DefaultBootloaderConfig()
        assert config.entry_address == DEFAULT_ENTRY_ADDRESS
        assert config.kernel_disk_offset == DEFAULT_KERNEL_DISK_OFFSET
        assert config.kernel_load_address == DEFAULT_KERNEL_LOAD_ADDRESS
        assert config.kernel_size == 0
        assert config.stack_base == DEFAULT_STACK_BASE

    def test_custom_config(self) -> None:
        config = BootloaderConfig(
            entry_address=0x1000,
            kernel_disk_offset=0x2000,
            kernel_load_address=0x3000,
            kernel_size=4096,
            stack_base=0x5000,
        )
        assert config.entry_address == 0x1000
        assert config.kernel_size == 4096


# =========================================================================
# Bootloader Generation Tests
# =========================================================================


class TestBootloaderGeneration:
    def test_generate_returns_bytes(self) -> None:
        config = DefaultBootloaderConfig()
        config.kernel_size = 256
        bl = Bootloader(config)
        binary = bl.generate()
        assert isinstance(binary, bytes)
        assert len(binary) > 0

    def test_generate_word_aligned(self) -> None:
        config = DefaultBootloaderConfig()
        config.kernel_size = 256
        bl = Bootloader(config)
        binary = bl.generate()
        assert len(binary) % 4 == 0

    def test_generate_with_comments(self) -> None:
        config = DefaultBootloaderConfig()
        config.kernel_size = 256
        bl = Bootloader(config)
        annotated = bl.generate_with_comments()
        assert len(annotated) > 0
        for instr in annotated:
            assert isinstance(instr, AnnotatedInstruction)
            assert instr.assembly != ""
            assert instr.comment != ""

    def test_instructions_sequential_addresses(self) -> None:
        config = DefaultBootloaderConfig()
        config.kernel_size = 256
        bl = Bootloader(config)
        annotated = bl.generate_with_comments()
        for i in range(1, len(annotated)):
            assert annotated[i].address == annotated[i - 1].address + 4

    def test_first_instruction_at_entry_address(self) -> None:
        config = DefaultBootloaderConfig()
        config.kernel_size = 256
        bl = Bootloader(config)
        annotated = bl.generate_with_comments()
        assert annotated[0].address == config.entry_address

    def test_instruction_count(self) -> None:
        config = DefaultBootloaderConfig()
        config.kernel_size = 256
        bl = Bootloader(config)
        count = bl.instruction_count()
        assert count > 0
        assert count == len(bl.generate_with_comments())

    def test_estimate_cycles(self) -> None:
        config = DefaultBootloaderConfig()
        config.kernel_size = 4096
        bl = Bootloader(config)
        cycles = bl.estimate_cycles()
        assert cycles > 0
        # 4096 bytes / 4 bytes per word = 1024 iterations
        # 1024 * 6 + 20 = 6164
        assert cycles == 6164

    def test_zero_kernel_size(self) -> None:
        config = DefaultBootloaderConfig()
        config.kernel_size = 0
        bl = Bootloader(config)
        binary = bl.generate()
        assert len(binary) > 0

    def test_generate_consistency(self) -> None:
        config = DefaultBootloaderConfig()
        config.kernel_size = 256
        bl = Bootloader(config)
        b1 = bl.generate()
        b2 = bl.generate()
        assert b1 == b2

    def test_comments_mention_phases(self) -> None:
        config = DefaultBootloaderConfig()
        config.kernel_size = 256
        bl = Bootloader(config)
        annotated = bl.generate_with_comments()
        comments = " ".join(i.comment for i in annotated)
        assert "Phase 1" in comments
        assert "Phase 2" in comments
        assert "Phase 3" in comments
        assert "Phase 4" in comments

    def test_halt_instruction_present(self) -> None:
        config = DefaultBootloaderConfig()
        config.kernel_size = 256
        bl = Bootloader(config)
        annotated = bl.generate_with_comments()
        halt_found = any("Halt" in i.comment for i in annotated)
        assert halt_found

    def test_magic_number_check(self) -> None:
        config = DefaultBootloaderConfig()
        config.kernel_size = 256
        bl = Bootloader(config)
        annotated = bl.generate_with_comments()
        magic_ref = any("0xB007CAFE" in i.comment for i in annotated)
        assert magic_ref


# =========================================================================
# DiskImage Tests
# =========================================================================


class TestDiskImage:
    def test_create_default_size(self) -> None:
        disk = DiskImage()
        assert disk.size() == DEFAULT_DISK_SIZE

    def test_create_custom_size(self) -> None:
        disk = DiskImage(1024)
        assert disk.size() == 1024

    def test_load_kernel(self) -> None:
        disk = DiskImage()
        kernel = bytes([0xDE, 0xAD, 0xBE, 0xEF])
        disk.load_kernel(kernel)
        assert disk.read_byte_at(DISK_KERNEL_OFFSET) == 0xDE
        assert disk.read_byte_at(DISK_KERNEL_OFFSET + 1) == 0xAD

    def test_load_at(self) -> None:
        disk = DiskImage()
        disk.load_at(0x100, b"\x42\x43")
        assert disk.read_byte_at(0x100) == 0x42
        assert disk.read_byte_at(0x101) == 0x43

    def test_load_at_exceeds_size(self) -> None:
        disk = DiskImage(16)
        try:
            disk.load_at(15, b"\x01\x02")
            assert False, "Should raise ValueError"  # noqa: B011
        except ValueError:
            pass

    def test_read_word(self) -> None:
        disk = DiskImage()
        disk.load_at(0, bytes([0x78, 0x56, 0x34, 0x12]))
        assert disk.read_word(0) == 0x12345678

    def test_read_word_out_of_bounds(self) -> None:
        disk = DiskImage(4)
        assert disk.read_word(2) == 0

    def test_read_byte_at_out_of_bounds(self) -> None:
        disk = DiskImage(4)
        assert disk.read_byte_at(10) == 0
        assert disk.read_byte_at(-1) == 0

    def test_data_returns_bytearray(self) -> None:
        disk = DiskImage(16)
        assert isinstance(disk.data(), bytearray)
        assert len(disk.data()) == 16

    def test_load_user_program(self) -> None:
        disk = DiskImage()
        disk.load_user_program(b"\xAA\xBB", 0x100000)
        assert disk.read_byte_at(0x100000) == 0xAA


# =========================================================================
# Constants Tests
# =========================================================================


class TestConstants:
    def test_magic_number(self) -> None:
        assert BOOT_PROTOCOL_MAGIC == 0xB007CAFE

    def test_disk_memory_map_base(self) -> None:
        assert DISK_MEMORY_MAP_BASE == 0x10000000

    def test_entry_address(self) -> None:
        assert DEFAULT_ENTRY_ADDRESS == 0x00010000
