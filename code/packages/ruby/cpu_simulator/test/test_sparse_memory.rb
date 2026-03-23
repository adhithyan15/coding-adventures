# frozen_string_literal: true

require_relative "test_helper"

module CodingAdventures
  module CpuSimulator
    # === Test helpers ===

    def self.make_test_sparse_memory
      SparseMemory.new([
        MemoryRegion.new(base: 0x00000000, size: 4096, name: "RAM"),
        MemoryRegion.new(base: 0xFFFF0000, size: 256, name: "ROM", read_only: true)
      ])
    end

    # === Construction tests ===

    class TestNewSparseMemory < Minitest::Test
      def test_allocates_regions
        mem = CpuSimulator.make_test_sparse_memory
        assert_equal 2, mem.region_count

        assert_equal "RAM", mem.regions[0].name
        assert_equal 0x00000000, mem.regions[0].base
        assert_equal 4096, mem.regions[0].size
        assert_equal 4096, mem.regions[0].data.size
        refute mem.regions[0].read_only

        assert_equal "ROM", mem.regions[1].name
        assert mem.regions[1].read_only
      end

      def test_pre_populated_data
        rom_data = Array.new(64, 0)
        rom_data[0] = 0xAA
        rom_data[63] = 0xBB

        mem = SparseMemory.new([
          MemoryRegion.new(base: 0x1000, size: 64, data: rom_data, name: "ROM", read_only: true)
        ])

        assert_equal 0xAA, mem.read_byte(0x1000)
        assert_equal 0xBB, mem.read_byte(0x103F)
      end

      def test_zero_initialized
        mem = CpuSimulator.make_test_sparse_memory
        16.times do |i|
          assert_equal 0, mem.read_byte(i)
        end
      end
    end

    # === Byte read/write tests ===

    class TestReadWriteByte < Minitest::Test
      def test_basic
        mem = CpuSimulator.make_test_sparse_memory
        mem.write_byte(0x0000, 0x42)
        mem.write_byte(0x0001, 0xFF)
        mem.write_byte(0x0FFF, 0x99)

        assert_equal 0x42, mem.read_byte(0x0000)
        assert_equal 0xFF, mem.read_byte(0x0001)
        assert_equal 0x99, mem.read_byte(0x0FFF)
      end

      def test_read_only_write_silently_ignored
        mem = CpuSimulator.make_test_sparse_memory
        assert_equal 0, mem.read_byte(0xFFFF0000)

        mem.write_byte(0xFFFF0000, 0xDE)
        assert_equal 0, mem.read_byte(0xFFFF0000)
      end
    end

    # === Word read/write tests ===

    class TestReadWriteWord < Minitest::Test
      def test_little_endian
        mem = CpuSimulator.make_test_sparse_memory
        mem.write_word(0x0100, 0xDEADBEEF)

        assert_equal 0xEF, mem.read_byte(0x0100)
        assert_equal 0xBE, mem.read_byte(0x0101)
        assert_equal 0xAD, mem.read_byte(0x0102)
        assert_equal 0xDE, mem.read_byte(0x0103)

        assert_equal 0xDEADBEEF, mem.read_word(0x0100)
      end

      def test_write_word_read_only
        mem = CpuSimulator.make_test_sparse_memory
        mem.write_word(0xFFFF0000, 0x12345678)
        assert_equal 0x00000000, mem.read_word(0xFFFF0000)
      end

      def test_word_round_trip
        mem = CpuSimulator.make_test_sparse_memory
        [
          [0x0000, 0x00000000],
          [0x0004, 0xFFFFFFFF],
          [0x0008, 0x00000001],
          [0x000C, 0x80000000],
          [0x0010, 0x7FFFFFFF],
          [0x0014, 0x01020304]
        ].each do |addr, val|
          mem.write_word(addr, val)
          assert_equal val, mem.read_word(addr), "at 0x#{format("%04X", addr)}"
        end
      end
    end

    # === LoadBytes tests ===

    class TestLoadBytes < Minitest::Test
      def test_basic
        mem = CpuSimulator.make_test_sparse_memory
        data = [0x01, 0x02, 0x03, 0x04, 0x05]
        mem.load_bytes(0x0200, data)

        data.each_with_index do |expected, i|
          assert_equal expected, mem.read_byte(0x0200 + i)
        end
      end

      def test_into_read_only_region
        mem = CpuSimulator.make_test_sparse_memory
        data = [0xAA, 0xBB, 0xCC, 0xDD]
        mem.load_bytes(0xFFFF0000, data)

        assert_equal 0xAA, mem.read_byte(0xFFFF0000)
        assert_equal 0xDD, mem.read_byte(0xFFFF0003)

        # Subsequent writes should still be ignored
        mem.write_byte(0xFFFF0000, 0x00)
        assert_equal 0xAA, mem.read_byte(0xFFFF0000)
      end
    end

    # === Dump tests ===

    class TestDump < Minitest::Test
      def test_basic
        mem = CpuSimulator.make_test_sparse_memory
        mem.write_byte(0x0010, 0xAA)
        mem.write_byte(0x0011, 0xBB)
        mem.write_byte(0x0012, 0xCC)

        dumped = mem.dump(0x0010, 3)
        assert_equal 3, dumped.size
        assert_equal [0xAA, 0xBB, 0xCC], dumped
      end

      def test_is_copy
        mem = CpuSimulator.make_test_sparse_memory
        mem.write_byte(0x0000, 0xFF)

        dumped = mem.dump(0x0000, 4)
        dumped[0] = 0x00

        assert_equal 0xFF, mem.read_byte(0x0000)
      end
    end

    # === Unmapped address tests ===

    class TestUnmappedAddresses < Minitest::Test
      def test_read_byte_unmapped_raises
        mem = CpuSimulator.make_test_sparse_memory
        assert_raises(RuntimeError) { mem.read_byte(0x80000000) }
      end

      def test_write_byte_unmapped_raises
        mem = CpuSimulator.make_test_sparse_memory
        assert_raises(RuntimeError) { mem.write_byte(0x80000000, 0xFF) }
      end

      def test_read_word_unmapped_raises
        mem = CpuSimulator.make_test_sparse_memory
        assert_raises(RuntimeError) { mem.read_word(0x80000000) }
      end

      def test_write_word_unmapped_raises
        mem = CpuSimulator.make_test_sparse_memory
        assert_raises(RuntimeError) { mem.write_word(0x80000000, 0xDEAD) }
      end

      def test_read_word_crosses_boundary_raises
        mem = CpuSimulator.make_test_sparse_memory
        assert_raises(RuntimeError) { mem.read_word(0x0FFE) }
      end
    end

    # === Multiple non-contiguous region tests ===

    class TestMultipleRegions < Minitest::Test
      def test_isolation
        mem = SparseMemory.new([
          MemoryRegion.new(base: 0x00000000, size: 1024, name: "RAM"),
          MemoryRegion.new(base: 0x10000000, size: 256, name: "SRAM"),
          MemoryRegion.new(base: 0xFFFF0000, size: 128, name: "IO")
        ])

        mem.write_byte(0x00000000, 0x11)
        mem.write_byte(0x10000000, 0x22)
        mem.write_byte(0xFFFF0000, 0x33)

        assert_equal 0x11, mem.read_byte(0x00000000)
        assert_equal 0x22, mem.read_byte(0x10000000)
        assert_equal 0x33, mem.read_byte(0xFFFF0000)
      end
    end

    # === High address region tests ===

    class TestHighAddressRegion < Minitest::Test
      def test_near_top_of_address_space
        mem = SparseMemory.new([
          MemoryRegion.new(base: 0xFFFB0000, size: 0x50000, name: "HIGH_IO")
        ])

        mem.write_byte(0xFFFB0000, 0x01)
        mem.write_byte(0xFFFFFFFE, 0xFE)
        mem.write_word(0xFFFFFFFC, 0xCAFEBABE)

        assert_equal 0x01, mem.read_byte(0xFFFB0000)
        assert_equal 0xCAFEBABE, mem.read_word(0xFFFFFFFC)
      end
    end

    # === LoadBytes as program loader ===

    class TestLoadProgram < Minitest::Test
      def test_load_program
        mem = SparseMemory.new([
          MemoryRegion.new(base: 0x00000000, size: 0x10000, name: "RAM")
        ])

        program = [0x93, 0x00, 0xA0, 0x02, 0x13, 0x01, 0x30, 0x00,
                   0xB3, 0x01, 0x21, 0x00, 0x73, 0x00, 0x00, 0x00]
        mem.load_bytes(0x0000, program)

        word0 = mem.read_word(0x0000)
        assert_equal 0x02A00093, word0
      end
    end

    # === findRegion edge cases ===

    class TestFindRegionEdgeCases < Minitest::Test
      def test_load_bytes_unmapped_raises
        mem = CpuSimulator.make_test_sparse_memory
        assert_raises(RuntimeError) { mem.load_bytes(0x80000000, [0x01, 0x02]) }
      end

      def test_dump_unmapped_raises
        mem = CpuSimulator.make_test_sparse_memory
        assert_raises(RuntimeError) { mem.dump(0x80000000, 4) }
      end

      def test_empty_regions
        mem = SparseMemory.new([])
        assert_raises(RuntimeError) { mem.read_byte(0x0000) }
      end

      def test_region_count
        mem = SparseMemory.new([
          MemoryRegion.new(base: 0, size: 16, name: "A"),
          MemoryRegion.new(base: 0x1000, size: 16, name: "B"),
          MemoryRegion.new(base: 0x2000, size: 16, name: "C")
        ])
        assert_equal 3, mem.region_count
      end
    end
  end
end
