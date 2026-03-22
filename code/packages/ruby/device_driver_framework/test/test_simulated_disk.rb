# frozen_string_literal: true

require_relative "test_helper"

module CodingAdventures
  module DeviceDriverFramework
    class TestSimulatedDisk < Minitest::Test
      def setup
        @disk = SimulatedDisk.new(total_blocks: 64)
        @disk.init
      end

      # --- Basic properties ---

      def test_default_name
        disk = SimulatedDisk.new
        assert_equal "disk0", disk.name
      end

      def test_device_type_is_block
        assert_equal DeviceType::BLOCK, @disk.device_type
      end

      def test_major_number_is_3
        assert_equal 3, @disk.major
      end

      def test_interrupt_number_is_34
        assert_equal 34, @disk.interrupt_number
      end

      def test_default_block_size_is_512
        assert_equal 512, @disk.block_size
      end

      def test_total_blocks_matches_config
        assert_equal 64, @disk.total_blocks
      end

      # --- Reading ---

      def test_fresh_disk_reads_all_zeros
        data = @disk.read_block(0)
        assert_equal 512, data.length
        assert data.all? { |b| b == 0 }
      end

      def test_read_block_returns_exact_block_size
        data = @disk.read_block(10)
        assert_equal @disk.block_size, data.length
      end

      # --- Writing and reading back ---

      def test_write_then_read_roundtrips
        # Write known data to block 5
        test_data = (0...512).map { |i| i % 256 }
        @disk.write_block(5, test_data)

        # Read it back
        result = @disk.read_block(5)
        assert_equal test_data, result
      end

      def test_write_does_not_affect_other_blocks
        # Write to block 5
        test_data = [0xFF] * 512
        @disk.write_block(5, test_data)

        # Block 4 should still be zeros
        assert @disk.read_block(4).all? { |b| b == 0 }
        # Block 6 should still be zeros
        assert @disk.read_block(6).all? { |b| b == 0 }
      end

      def test_overwrite_block
        # Write once
        @disk.write_block(3, [0xAA] * 512)
        # Overwrite
        @disk.write_block(3, [0xBB] * 512)
        # Should see the overwrite
        result = @disk.read_block(3)
        assert result.all? { |b| b == 0xBB }
      end

      # --- Boundary conditions ---

      def test_read_last_valid_block
        data = @disk.read_block(63)
        assert_equal 512, data.length
      end

      def test_read_out_of_bounds_raises_error
        assert_raises(ArgumentError) { @disk.read_block(64) }
      end

      def test_read_negative_block_raises_error
        assert_raises(ArgumentError) { @disk.read_block(-1) }
      end

      def test_write_out_of_bounds_raises_error
        assert_raises(ArgumentError) { @disk.write_block(64, [0] * 512) }
      end

      def test_write_wrong_size_data_raises_error
        # Too short
        assert_raises(ArgumentError) { @disk.write_block(0, [0] * 100) }
        # Too long
        assert_raises(ArgumentError) { @disk.write_block(0, [0] * 600) }
      end

      # --- Custom configuration ---

      def test_custom_block_size
        disk = SimulatedDisk.new(block_size: 1024, total_blocks: 10)
        disk.init
        assert_equal 1024, disk.block_size

        data = disk.read_block(0)
        assert_equal 1024, data.length
      end

      def test_custom_name_and_minor
        disk = SimulatedDisk.new(name: "disk1", minor: 1, total_blocks: 10)
        assert_equal "disk1", disk.name
        assert_equal 1, disk.minor
      end

      # --- Read returns independent copy ---

      def test_read_returns_copy
        @disk.write_block(0, [0x42] * 512)
        data1 = @disk.read_block(0)
        data2 = @disk.read_block(0)
        refute_same data1, data2
        assert_equal data1, data2
      end
    end
  end
end
