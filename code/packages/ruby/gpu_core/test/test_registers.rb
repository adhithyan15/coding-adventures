# frozen_string_literal: true

require_relative "test_helper"

module CodingAdventures
  module GpuCore
    # =========================================================================
    # Tests for the FPRegisterFile.
    # =========================================================================
    #
    # The register file is the fastest storage in the GPU core. These tests
    # verify that it correctly stores, retrieves, and reports FloatBits values.

    class TestRegisterConstruction < Minitest::Test
      # Default: 32 FP32 registers, all zero.
      def test_default_construction
        rf = FPRegisterFile.new
        assert_equal 32, rf.num_registers
        assert_equal FpArithmetic::FP32, rf.fmt
        assert_equal 0.0, rf.read_float(0)
        assert_equal 0.0, rf.read_float(31)
      end

      # Can create register files with different sizes.
      def test_custom_register_count
        rf = FPRegisterFile.new(num_registers: 64)
        assert_equal 64, rf.num_registers
        rf.write_float(63, 1.0)
        assert_equal 1.0, rf.read_float(63)
      end

      # NVIDIA cores support up to 255 registers.
      def test_nvidia_scale
        rf = FPRegisterFile.new(num_registers: 255)
        rf.write_float(254, 42.0)
        assert_equal 42.0, rf.read_float(254)
      end

      # Maximum is 256 registers.
      def test_max_256_registers
        rf = FPRegisterFile.new(num_registers: 256)
        assert_equal 256, rf.num_registers
      end

      # Cannot create a register file with 0 registers.
      def test_invalid_zero_registers
        err = assert_raises(ArgumentError) { FPRegisterFile.new(num_registers: 0) }
        assert_match(/num_registers must be 1-256/, err.message)
      end

      # Cannot create a register file with >256 registers.
      def test_invalid_too_many_registers
        err = assert_raises(ArgumentError) { FPRegisterFile.new(num_registers: 257) }
        assert_match(/num_registers must be 1-256/, err.message)
      end

      # Register file can use FP16 format.
      def test_fp16_format
        rf = FPRegisterFile.new(fmt: FpArithmetic::FP16)
        assert_equal FpArithmetic::FP16, rf.fmt
        rf.write_float(0, 1.0)
        assert_equal 1.0, rf.read_float(0)
      end

      # Register file can use BF16 format.
      def test_bf16_format
        rf = FPRegisterFile.new(fmt: FpArithmetic::BF16)
        assert_equal FpArithmetic::BF16, rf.fmt
        rf.write_float(0, 1.0)
        assert_equal 1.0, rf.read_float(0)
      end
    end

    class TestRegisterReadWrite < Minitest::Test
      # Write a FloatBits and read it back.
      def test_write_and_read_floatbits
        rf = FPRegisterFile.new
        value = FpArithmetic.float_to_bits(3.14, FpArithmetic::FP32)
        rf.write(0, value)
        result = rf.read(0)
        assert_equal value, result
      end

      # Write a Ruby float and read it back.
      def test_write_and_read_float
        rf = FPRegisterFile.new
        rf.write_float(5, 2.71828)
        result = rf.read_float(5)
        assert_in_delta 2.71828, result, 1e-5
      end

      # Write a negative value.
      def test_write_negative
        rf = FPRegisterFile.new
        rf.write_float(0, -42.0)
        assert_equal(-42.0, rf.read_float(0))
      end

      # Write zero explicitly.
      def test_write_zero
        rf = FPRegisterFile.new
        rf.write_float(0, 99.0)
        rf.write_float(0, 0.0)
        assert_equal 0.0, rf.read_float(0)
      end

      # Writing to a register overwrites the previous value.
      def test_overwrite
        rf = FPRegisterFile.new
        rf.write_float(0, 1.0)
        rf.write_float(0, 2.0)
        assert_equal 2.0, rf.read_float(0)
      end

      # Writing to one register doesn't affect others.
      def test_independent_registers
        rf = FPRegisterFile.new
        rf.write_float(0, 1.0)
        rf.write_float(1, 2.0)
        assert_equal 1.0, rf.read_float(0)
        assert_equal 2.0, rf.read_float(1)
      end

      # Reading past the register count raises IndexError.
      def test_read_out_of_bounds
        rf = FPRegisterFile.new(num_registers: 8)
        err = assert_raises(IndexError) { rf.read(8) }
        assert_match(/Register index 8/, err.message)
      end

      # Writing past the register count raises IndexError.
      def test_write_out_of_bounds
        rf = FPRegisterFile.new(num_registers: 8)
        assert_raises(IndexError) do
          rf.write(8, FpArithmetic.float_to_bits(1.0, FpArithmetic::FP32))
        end
      end

      # Negative register indices are invalid.
      def test_negative_index
        rf = FPRegisterFile.new
        assert_raises(IndexError) { rf.read(-1) }
      end
    end

    class TestRegisterDump < Minitest::Test
      # Dump of all-zero registers returns empty hash.
      def test_dump_empty
        rf = FPRegisterFile.new
        assert_equal({}, rf.dump)
      end

      # Dump includes only non-zero registers.
      def test_dump_non_zero
        rf = FPRegisterFile.new
        rf.write_float(0, 1.0)
        rf.write_float(5, 3.14)
        result = rf.dump
        assert_includes result.keys, "R0"
        assert_includes result.keys, "R5"
        assert_equal 2, result.length
      end

      # dump_all includes all registers including zeros.
      def test_dump_all
        rf = FPRegisterFile.new(num_registers: 4)
        rf.write_float(0, 1.0)
        result = rf.dump_all
        assert_equal 4, result.length
        assert_equal 1.0, result["R0"]
        assert_equal 0.0, result["R1"]
      end

      # to_s shows 'all zero' for fresh register file.
      def test_to_s_empty
        rf = FPRegisterFile.new
        assert_includes rf.to_s, "all zero"
      end

      # to_s shows non-zero register values.
      def test_to_s_with_values
        rf = FPRegisterFile.new
        rf.write_float(0, 3.0)
        assert_includes rf.to_s, "R0=3.0"
      end
    end
  end
end
