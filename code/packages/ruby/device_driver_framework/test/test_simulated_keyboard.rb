# frozen_string_literal: true

require_relative "test_helper"

module CodingAdventures
  module DeviceDriverFramework
    class TestSimulatedKeyboard < Minitest::Test
      def setup
        @kb = SimulatedKeyboard.new
        @kb.init
      end

      # --- Basic properties ---

      def test_default_name
        assert_equal "keyboard0", @kb.name
      end

      def test_device_type_is_character
        assert_equal DeviceType::CHARACTER, @kb.device_type
      end

      def test_major_number_is_2
        assert_equal 2, @kb.major
      end

      def test_interrupt_number_is_33
        assert_equal 33, @kb.interrupt_number
      end

      # --- Reading from empty buffer ---

      def test_read_empty_buffer_returns_empty_array
        result = @kb.read(10)
        assert_empty result
      end

      # --- Enqueue and read ---

      def test_enqueue_then_read
        @kb.enqueue_bytes([0x48, 0x69])  # "Hi"
        result = @kb.read(2)
        assert_equal [0x48, 0x69], result
      end

      def test_fifo_order
        # Bytes should come out in the order they went in
        @kb.enqueue_bytes([1, 2, 3, 4, 5])
        result = @kb.read(5)
        assert_equal [1, 2, 3, 4, 5], result
      end

      def test_read_fewer_than_available
        @kb.enqueue_bytes([1, 2, 3, 4, 5])
        result = @kb.read(3)
        assert_equal [1, 2, 3], result
        # Remaining bytes should still be in buffer
        result2 = @kb.read(2)
        assert_equal [4, 5], result2
      end

      def test_read_more_than_available
        # Only 3 bytes in buffer, asking for 10
        @kb.enqueue_bytes([0x41, 0x42, 0x43])
        result = @kb.read(10)
        assert_equal [0x41, 0x42, 0x43], result
      end

      # --- Write always fails ---

      def test_write_returns_negative_one
        result = @kb.write([0x48, 0x69])
        assert_equal(-1, result)
      end

      # --- Buffer management ---

      def test_buffer_size
        assert_equal 0, @kb.buffer_size
        @kb.enqueue_bytes([1, 2, 3])
        assert_equal 3, @kb.buffer_size
        @kb.read(1)
        assert_equal 2, @kb.buffer_size
      end

      def test_init_clears_buffer
        @kb.enqueue_bytes([1, 2, 3])
        @kb.init
        assert_equal 0, @kb.buffer_size
        assert_empty @kb.read(10)
      end

      # --- Custom name and minor ---

      def test_custom_name_and_minor
        kb = SimulatedKeyboard.new(name: "keyboard1", minor: 1)
        assert_equal "keyboard1", kb.name
        assert_equal 1, kb.minor
      end

      # --- Multiple enqueue calls ---

      def test_multiple_enqueues_concatenate
        @kb.enqueue_bytes([1, 2])
        @kb.enqueue_bytes([3, 4])
        result = @kb.read(4)
        assert_equal [1, 2, 3, 4], result
      end
    end
  end
end
