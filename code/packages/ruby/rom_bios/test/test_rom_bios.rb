# frozen_string_literal: true

require "minitest/autorun"
require "coding_adventures/rom_bios/rom"
require "coding_adventures/rom_bios/hardware_info"
require "coding_adventures/rom_bios/bios"

module CodingAdventures
  module RomBios
    # ═══════════════════════════════════════════════════════════════
    # ROM Tests
    # ═══════════════════════════════════════════════════════════════

    class TestROM < Minitest::Test
      def test_new_rom_loads_firmware
        rom = ROM.new(ROMConfig.new, [0xAA, 0xBB, 0xCC, 0xDD])
        assert_equal DEFAULT_ROM_SIZE, rom.size
      end

      def test_raises_on_oversized_firmware
        assert_raises(ArgumentError) do
          ROM.new(ROMConfig.new, Array.new(DEFAULT_ROM_SIZE + 1, 0))
        end
      end

      def test_read_byte
        firmware = [0x12, 0x34, 0x56, 0x78, 0x9A, 0xBC, 0xDE, 0xF0]
        rom = ROM.new(ROMConfig.new, firmware)
        base = DEFAULT_ROM_BASE

        assert_equal 0x12, rom.read(base)
        assert_equal 0x34, rom.read(base + 1)
        assert_equal 0x78, rom.read(base + 3)
        assert_equal 0xF0, rom.read(base + 7)
      end

      def test_read_word
        firmware = [0x78, 0x56, 0x34, 0x12, 0xF0, 0xDE, 0xBC, 0x9A]
        rom = ROM.new(ROMConfig.new, firmware)
        base = DEFAULT_ROM_BASE

        assert_equal 0x12345678, rom.read_word(base)
        assert_equal 0x9ABCDEF0, rom.read_word(base + 4)
      end

      def test_write_is_ignored
        rom = ROM.new(ROMConfig.new, [0xAA, 0xBB, 0xCC, 0xDD])
        rom.write(DEFAULT_ROM_BASE, 0xFF)
        assert_equal 0xAA, rom.read(DEFAULT_ROM_BASE)
      end

      def test_out_of_range_returns_zero
        rom = ROM.new(ROMConfig.new, [0xAA])
        assert_equal 0, rom.read(0x00000000)
        assert_equal 0, rom.read_word(0x00000000)
      end

      def test_firmware_smaller_than_rom
        rom = ROM.new(ROMConfig.new, [0xAA, 0xBB])
        assert_equal 0, rom.read(DEFAULT_ROM_BASE + 2)
        assert_equal 0, rom.read(DEFAULT_ROM_BASE + 100)
      end

      def test_custom_config
        config = ROMConfig.new(base_address: 0x10000000, size: 256)
        rom = ROM.new(config, [0x11, 0x22, 0x33, 0x44])
        assert_equal 256, rom.size
        assert_equal 0x10000000, rom.base_address
        assert_equal 0x11, rom.read(0x10000000)
        assert_equal 0, rom.read(DEFAULT_ROM_BASE)
      end

      def test_contains
        rom = ROM.new(ROMConfig.new, [0xAA])
        assert rom.contains?(DEFAULT_ROM_BASE)
        assert rom.contains?(DEFAULT_ROM_BASE + DEFAULT_ROM_SIZE - 1)
        refute rom.contains?(DEFAULT_ROM_BASE - 1)
        refute rom.contains?(0x00000000)
      end

      def test_boundary_reads
        firmware = Array.new(DEFAULT_ROM_SIZE, 0)
        firmware[0..3] = [0x01, 0x02, 0x03, 0x04]
        firmware[-4..] = [0xA1, 0xA2, 0xA3, 0xA4]
        rom = ROM.new(ROMConfig.new, firmware)

        assert_equal 0x04030201, rom.read_word(DEFAULT_ROM_BASE)
        assert_equal 0xA4A3A2A1, rom.read_word(0xFFFFFFFC)
      end

      def test_empty_firmware
        rom = ROM.new(ROMConfig.new)
        assert_equal 0, rom.read_word(DEFAULT_ROM_BASE)
      end
    end

    # ═══════════════════════════════════════════════════════════════
    # HardwareInfo Tests
    # ═══════════════════════════════════════════════════════════════

    class TestHardwareInfo < Minitest::Test
      def test_defaults
        info = HardwareInfo.new
        assert_equal 0, info.memory_size
        assert_equal 80, info.display_columns
        assert_equal 25, info.display_rows
        assert_equal 0xFFFB0000, info.framebuffer_base
        assert_equal 0x00000000, info.idt_base
        assert_equal 256, info.idt_entries
        assert_equal 0x00010000, info.bootloader_entry
      end

      def test_to_bytes_roundtrip
        info = HardwareInfo.new(memory_size: 64 * 1024 * 1024)
        data = info.to_bytes
        assert_equal HARDWARE_INFO_SIZE, data.length
        restored = HardwareInfo.from_bytes(data)
        assert_equal info, restored
      end

      def test_to_bytes_layout
        info = HardwareInfo.new(memory_size: 0x04000000)
        data = info.to_bytes
        assert_equal 0x00, data[0]
        assert_equal 0x04, data[3]
        assert_equal 80, data[4] # display_columns
      end

      def test_from_bytes_raises_on_short_data
        assert_raises(ArgumentError) { HardwareInfo.from_bytes([0x01, 0x02]) }
      end
    end

    # ═══════════════════════════════════════════════════════════════
    # BIOS Firmware Generation Tests
    # ═══════════════════════════════════════════════════════════════

    class TestBIOSFirmware < Minitest::Test
      def test_generate_non_empty
        code = BIOSFirmware.new(BIOSConfig.new).generate
        refute_empty code
      end

      def test_generate_word_aligned
        code = BIOSFirmware.new(BIOSConfig.new).generate
        assert_equal 0, code.length % 4
      end

      def test_generate_deterministic
        config = BIOSConfig.new
        code1 = BIOSFirmware.new(config).generate
        code2 = BIOSFirmware.new(config).generate
        assert_equal code1, code2
      end

      def test_configurable_different_output
        code1 = BIOSFirmware.new(BIOSConfig.new).generate
        code2 = BIOSFirmware.new(BIOSConfig.new(memory_size: 128 * 1024 * 1024)).generate
        refute_equal code1, code2
      end

      def test_configured_memory_shorter
        probe_code = BIOSFirmware.new(BIOSConfig.new).generate
        fixed_code = BIOSFirmware.new(BIOSConfig.new(memory_size: 64 * 1024 * 1024)).generate
        assert fixed_code.length < probe_code.length
      end

      def test_fits_in_rom
        code = BIOSFirmware.new(BIOSConfig.new).generate
        assert code.length <= DEFAULT_ROM_SIZE
      end

      def test_load_into_rom
        bios = BIOSFirmware.new(BIOSConfig.new)
        code = bios.generate
        rom = ROM.new(ROMConfig.new, code)
        first_word = rom.read_word(DEFAULT_ROM_BASE)
        expected = code[0] | (code[1] << 8) | (code[2] << 16) | (code[3] << 24)
        assert_equal expected, first_word
      end
    end

    # ═══════════════════════════════════════════════════════════════
    # Annotated Output Tests
    # ═══════════════════════════════════════════════════════════════

    class TestAnnotatedOutput < Minitest::Test
      def test_matches_generate
        bios = BIOSFirmware.new(BIOSConfig.new)
        code = bios.generate
        annotated = bios.generate_with_comments
        assert_equal annotated.length * 4, code.length

        annotated.each_with_index do |inst, i|
          off = i * 4
          expected = code[off] | (code[off + 1] << 8) | (code[off + 2] << 16) | (code[off + 3] << 24)
          assert_equal expected, inst.machine_code, "instruction #{i} mismatch"
        end
      end

      def test_address_continuity
        annotated = BIOSFirmware.new(BIOSConfig.new).generate_with_comments
        refute_empty annotated
        assert_equal DEFAULT_ROM_BASE, annotated[0].address
        (1...annotated.length).each do |i|
          assert_equal annotated[i - 1].address + 4, annotated[i].address
        end
      end

      def test_non_empty_strings
        BIOSFirmware.new(BIOSConfig.new).generate_with_comments.each_with_index do |inst, i|
          refute_empty inst.assembly, "instruction #{i} has empty assembly"
          refute_empty inst.comment, "instruction #{i} has empty comment"
        end
      end

      def test_contains_riscv_mnemonics
        annotated = BIOSFirmware.new(BIOSConfig.new).generate_with_comments
        mnemonics = %w[lui addi sw jalr]
        found = mnemonics.select { |m| annotated.any? { |inst| inst.assembly.start_with?("#{m} ") } }
        assert_equal mnemonics, found, "missing mnemonics"
      end

      def test_last_instruction_is_jump
        annotated = BIOSFirmware.new(BIOSConfig.new).generate_with_comments
        assert annotated.last.assembly.start_with?("jalr")
      end
    end

    # ═══════════════════════════════════════════════════════════════
    # Default Config Tests
    # ═══════════════════════════════════════════════════════════════

    class TestDefaults < Minitest::Test
      def test_default_rom_config
        config = ROMConfig.new
        assert_equal 0xFFFF0000, config.base_address
        assert_equal 65536, config.size
      end

      def test_default_bios_config
        config = BIOSConfig.new
        assert_equal 0, config.memory_size
        assert_equal 80, config.display_columns
        assert_equal 25, config.display_rows
        assert_equal 0xFFFB0000, config.framebuffer_base
        assert_equal 0x00010000, config.bootloader_entry
      end
    end
  end
end
