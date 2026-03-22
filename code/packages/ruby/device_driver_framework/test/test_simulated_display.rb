# frozen_string_literal: true

require_relative "test_helper"

module CodingAdventures
  module DeviceDriverFramework
    class TestSimulatedDisplay < Minitest::Test
      def setup
        @display = SimulatedDisplay.new
        @display.init
      end

      # --- Basic properties ---

      def test_default_name
        assert_equal "display0", @display.name
      end

      def test_device_type_is_character
        assert_equal DeviceType::CHARACTER, @display.device_type
      end

      def test_major_number_is_1
        assert_equal 1, @display.major
      end

      def test_no_interrupt
        assert_equal(-1, @display.interrupt_number)
      end

      # --- Framebuffer dimensions ---

      def test_framebuffer_size
        assert_equal 4000, @display.framebuffer.length
      end

      def test_framebuffer_starts_zeroed
        assert @display.framebuffer.all? { |b| b == 0 }
      end

      # --- Writing ---

      def test_write_single_character
        @display.write([0x48])  # 'H'
        # Character at (0,0) should be 0x48
        assert_equal 0x48, @display.char_at(0, 0)
        # Color attribute should be default (0x07)
        assert_equal 0x07, @display.framebuffer[1]
      end

      def test_write_two_characters
        @display.write([0x48, 0x69])  # "Hi"
        assert_equal 0x48, @display.char_at(0, 0)
        assert_equal 0x69, @display.char_at(0, 1)
      end

      def test_write_returns_byte_count
        result = @display.write([0x48, 0x69])
        assert_equal 2, result
      end

      # --- Reading always fails ---

      def test_read_returns_negative_one
        result = @display.read(10)
        assert_equal(-1, result)
      end

      # --- Cursor movement ---

      def test_cursor_starts_at_origin
        assert_equal 0, @display.cursor_row
        assert_equal 0, @display.cursor_col
      end

      def test_cursor_advances_after_write
        @display.write([0x41])  # 'A'
        assert_equal 0, @display.cursor_row
        assert_equal 1, @display.cursor_col
      end

      def test_cursor_wraps_at_end_of_row
        # Write 80 characters to fill row 0
        @display.write([0x41] * 80)
        assert_equal 1, @display.cursor_row
        assert_equal 0, @display.cursor_col
      end

      def test_cursor_wraps_at_bottom_of_screen
        # Write enough to fill the entire screen (80 * 25 = 2000 chars)
        @display.write([0x41] * (80 * 25))
        # Should wrap back to (0, 0)
        assert_equal 0, @display.cursor_row
        assert_equal 0, @display.cursor_col
      end

      # --- Clear screen ---

      def test_clear_screen_zeros_framebuffer
        @display.write([0x48, 0x69])
        @display.clear_screen
        assert @display.framebuffer.all? { |b| b == 0 }
      end

      def test_clear_screen_resets_cursor
        @display.write([0x41] * 10)
        @display.clear_screen
        assert_equal 0, @display.cursor_row
        assert_equal 0, @display.cursor_col
      end

      def test_init_clears_screen
        display = SimulatedDisplay.new
        # Manually set some data before init
        display.put_char(0x41)
        display.init
        assert_equal 0, display.cursor_row
        assert_equal 0, display.cursor_col
        assert_equal 0, display.char_at(0, 0)
      end

      # --- Custom name and minor ---

      def test_custom_name_and_minor
        d = SimulatedDisplay.new(name: "display1", minor: 1)
        assert_equal "display1", d.name
        assert_equal 1, d.minor
      end

      # --- put_char directly ---

      def test_put_char_writes_character_and_color
        @display.put_char(0x58)  # 'X'
        assert_equal 0x58, @display.framebuffer[0]
        assert_equal SimulatedDisplay::DEFAULT_COLOR, @display.framebuffer[1]
      end

      # --- char_at helper ---

      def test_char_at_reads_specific_position
        # Write at position (0,5) by writing 6 characters
        @display.write([0x41] * 5 + [0x5A])  # 5 'A's then 'Z'
        assert_equal 0x5A, @display.char_at(0, 5)
      end
    end
  end
end
