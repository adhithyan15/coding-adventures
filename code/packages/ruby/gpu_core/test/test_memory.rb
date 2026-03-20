# frozen_string_literal: true

require_relative "test_helper"

module CodingAdventures
  module GpuCore
    # =========================================================================
    # Tests for LocalMemory.
    # =========================================================================
    #
    # Local memory is the GPU core's private scratchpad -- byte-addressable
    # storage for floats that don't fit in registers.

    class TestMemoryConstruction < Minitest::Test
      # Default memory is 4096 bytes.
      def test_default_size
        mem = LocalMemory.new
        assert_equal 4096, mem.size
      end

      # Can create memory with custom size.
      def test_custom_size
        mem = LocalMemory.new(size: 256)
        assert_equal 256, mem.size
      end

      # Cannot create memory with size < 1.
      def test_invalid_size
        err = assert_raises(ArgumentError) { LocalMemory.new(size: 0) }
        assert_match(/positive/, err.message)
      end

      # Memory starts as all zeros.
      def test_initialized_to_zero
        mem = LocalMemory.new(size: 16)
        16.times do |i|
          assert_equal 0, mem.read_byte(i)
        end
      end
    end

    class TestByteAccess < Minitest::Test
      # Write a byte and read it back.
      def test_read_write_byte
        mem = LocalMemory.new
        mem.write_byte(0, 0x42)
        assert_equal 0x42, mem.read_byte(0)
      end

      # Values are masked to 8 bits.
      def test_byte_masking
        mem = LocalMemory.new
        mem.write_byte(0, 0x1FF) # 9 bits
        assert_equal 0xFF, mem.read_byte(0) # truncated to 8
      end

      # Write and read multiple bytes.
      def test_read_write_bytes
        mem = LocalMemory.new
        data = [0x01, 0x02, 0x03, 0x04]
        mem.write_bytes(0, data)
        assert_equal data, mem.read_bytes(0, 4)
      end

      # Reading past memory bounds raises IndexError.
      def test_out_of_bounds_read
        mem = LocalMemory.new(size: 8)
        err = assert_raises(IndexError) { mem.read_byte(8) }
        assert_match(/out of bounds/, err.message)
      end

      # Writing past memory bounds raises IndexError.
      def test_out_of_bounds_write
        mem = LocalMemory.new(size: 8)
        err = assert_raises(IndexError) { mem.write_byte(8, 0) }
        assert_match(/out of bounds/, err.message)
      end

      # Negative addresses are out of bounds.
      def test_negative_address
        mem = LocalMemory.new
        assert_raises(IndexError) { mem.read_byte(-1) }
      end

      # Multi-byte access that extends past the end fails.
      def test_multi_byte_out_of_bounds
        mem = LocalMemory.new(size: 8)
        assert_raises(IndexError) do
          mem.read_bytes(6, 4) # bytes 6,7,8,9 -- 8 and 9 out of bounds
        end
      end
    end

    class TestFloatAccess < Minitest::Test
      # Store and load an FP32 value.
      def test_store_load_fp32
        mem = LocalMemory.new
        value = FpArithmetic.float_to_bits(3.14, FpArithmetic::FP32)
        mem.store_float(0, value)
        result = mem.load_float(0, FpArithmetic::FP32)
        assert_in_delta 3.14, FpArithmetic.bits_to_float(result), 1e-5
      end

      # Store and load an FP16 value (2 bytes).
      def test_store_load_fp16
        mem = LocalMemory.new
        value = FpArithmetic.float_to_bits(1.0, FpArithmetic::FP16)
        mem.store_float(0, value)
        result = mem.load_float(0, FpArithmetic::FP16)
        assert_equal 1.0, FpArithmetic.bits_to_float(result)
      end

      # Store and load a BF16 value (2 bytes).
      def test_store_load_bf16
        mem = LocalMemory.new
        value = FpArithmetic.float_to_bits(2.0, FpArithmetic::BF16)
        mem.store_float(0, value)
        result = mem.load_float(0, FpArithmetic::BF16)
        assert_equal 2.0, FpArithmetic.bits_to_float(result)
      end

      # FP32 store writes exactly 4 bytes.
      def test_fp32_uses_4_bytes
        mem = LocalMemory.new
        value = FpArithmetic.float_to_bits(1.0, FpArithmetic::FP32)
        mem.store_float(0, value)
        raw = mem.read_bytes(0, 4)
        assert_equal 4, raw.length
        refute_equal [0, 0, 0, 0], raw
      end

      # FP16 store writes exactly 2 bytes.
      def test_fp16_uses_2_bytes
        mem = LocalMemory.new
        value = FpArithmetic.float_to_bits(1.0, FpArithmetic::FP16)
        mem.store_float(0, value)
        raw = mem.read_bytes(0, 2)
        assert_equal 2, raw.length
        refute_equal [0, 0], raw
      end

      # Store multiple floats at non-overlapping addresses.
      def test_multiple_floats_at_different_addresses
        mem = LocalMemory.new
        mem.store_ruby_float(0, 1.0)
        mem.store_ruby_float(4, 2.0)
        mem.store_ruby_float(8, 3.0)
        assert_equal 1.0, mem.load_float_as_ruby(0)
        assert_equal 2.0, mem.load_float_as_ruby(4)
        assert_equal 3.0, mem.load_float_as_ruby(8)
      end

      # Store and load a negative float.
      def test_store_negative
        mem = LocalMemory.new
        mem.store_ruby_float(0, -42.5)
        assert_equal(-42.5, mem.load_float_as_ruby(0))
      end

      # Store and load zero.
      def test_store_zero
        mem = LocalMemory.new
        mem.store_ruby_float(0, 0.0)
        assert_equal 0.0, mem.load_float_as_ruby(0)
      end

      # Test store_ruby_float and load_float_as_ruby.
      def test_convenience_methods
        mem = LocalMemory.new
        mem.store_ruby_float(0, 2.71828, FpArithmetic::FP32)
        result = mem.load_float_as_ruby(0, FpArithmetic::FP32)
        assert_in_delta 2.71828, result, 1e-5
      end

      # Loading a float past memory end raises IndexError.
      def test_float_out_of_bounds
        mem = LocalMemory.new(size: 8)
        assert_raises(IndexError) do
          mem.load_float(6, FpArithmetic::FP32) # needs 4 bytes at 6, goes to 10
        end
      end
    end

    class TestMemoryDump < Minitest::Test
      # Dump of fresh memory is all zeros.
      def test_dump_zeros
        mem = LocalMemory.new(size: 16)
        assert_equal [0] * 16, mem.dump(0, 16)
      end

      # Dump reflects written bytes.
      def test_dump_after_write
        mem = LocalMemory.new
        mem.write_byte(0, 0xFF)
        mem.write_byte(1, 0x42)
        d = mem.dump(0, 4)
        assert_equal 0xFF, d[0]
        assert_equal 0x42, d[1]
        assert_equal 0, d[2]
        assert_equal 0, d[3]
      end

      # to_s shows size and non-zero count.
      def test_to_s
        mem = LocalMemory.new(size: 64)
        assert_includes mem.to_s, "64 bytes"
        assert_includes mem.to_s, "0 non-zero"
        mem.write_byte(0, 1)
        assert_includes mem.to_s, "1 non-zero"
      end
    end
  end
end
