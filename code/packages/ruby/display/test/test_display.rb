# frozen_string_literal: true

# Comprehensive tests for the display package.
# Tests cover all operations specified in S05-display.md.

require_relative "test_helper"

module CodingAdventures
  module Display
    # ============================================================
    # Test helpers
    # ============================================================

    def self.new_test_driver
      config = COMPACT_40X10
      memory = Array.new(config.columns * config.rows * BYTES_PER_CELL, 0)
      DisplayDriver.new(config, memory)
    end

    def self.new_standard_driver
      config = DisplayConfig.new
      memory = Array.new(config.columns * config.rows * BYTES_PER_CELL, 0)
      DisplayDriver.new(config, memory)
    end

    # ============================================================
    # Config tests
    # ============================================================

    class TestConfig < Minitest::Test
      def test_default_config
        config = DisplayConfig.new
        assert_equal 80, config.columns
        assert_equal 25, config.rows
        assert_equal 0xFFFB0000, config.framebuffer_base
        assert_equal 0x07, config.default_attribute
      end

      def test_make_attribute_white_on_blue
        assert_equal 0x1F, Display.make_attribute(COLOR_WHITE, COLOR_BLUE)
      end

      def test_make_attribute_default
        assert_equal 0x07, Display.make_attribute(COLOR_LIGHT_GRAY, COLOR_BLACK)
      end

      def test_make_attribute_white_on_red
        assert_equal 0x4F, Display.make_attribute(COLOR_WHITE, COLOR_RED)
      end

      def test_make_attribute_green_on_black
        assert_equal 0x02, Display.make_attribute(COLOR_GREEN, COLOR_BLACK)
      end

      def test_predefined_vga
        assert_equal 80, VGA_80X25.columns
        assert_equal 25, VGA_80X25.rows
      end

      def test_predefined_compact
        assert_equal 40, COMPACT_40X10.columns
        assert_equal 10, COMPACT_40X10.rows
      end
    end

    # ============================================================
    # Constructor tests
    # ============================================================

    class TestConstructor < Minitest::Test
      def test_clears_screen
        d = Display.new_test_driver
        d.config.rows.times do |row|
          d.config.columns.times do |col|
            cell = d.get_cell(row, col)
            assert_equal 0x20, cell.character, "cell(#{row},#{col}) char"
            assert_equal DEFAULT_ATTRIBUTE, cell.attribute, "cell(#{row},#{col}) attr"
          end
        end
      end

      def test_cursor_at_origin
        d = Display.new_test_driver
        pos = d.get_cursor
        assert_equal 0, pos.row
        assert_equal 0, pos.col
      end
    end

    # ============================================================
    # PutChar tests
    # ============================================================

    class TestPutChar < Minitest::Test
      def test_basic_write
        d = Display.new_test_driver
        d.put_char(0x41) # 'A'
        cell = d.get_cell(0, 0)
        assert_equal 0x41, cell.character
        assert_equal DEFAULT_ATTRIBUTE, cell.attribute
      end

      def test_cursor_advance
        d = Display.new_test_driver
        d.put_char(0x41)
        pos = d.get_cursor
        assert_equal 0, pos.row
        assert_equal 1, pos.col
      end

      def test_multiple_characters
        d = Display.new_test_driver
        d.put_char(0x48) # 'H'
        d.put_char(0x69) # 'i'
        assert_equal 0x48, d.get_cell(0, 0).character
        assert_equal 0x69, d.get_cell(0, 1).character
        assert_equal 2, d.get_cursor.col
      end

      def test_newline
        d = Display.new_test_driver
        d.put_char(0x41)
        d.put_char(0x0A) # '\n'
        pos = d.get_cursor
        assert_equal 1, pos.row
        assert_equal 0, pos.col
      end

      def test_carriage_return
        d = Display.new_test_driver
        5.times { d.put_char(0x78) } # 'x'
        d.put_char(0x0D) # '\r'
        pos = d.get_cursor
        assert_equal 0, pos.row
        assert_equal 0, pos.col
      end

      def test_tab
        d = Display.new_test_driver
        d.put_char(0x09) # '\t'
        assert_equal 8, d.get_cursor.col
      end

      def test_tab_from_col_1
        d = Display.new_test_driver
        d.put_char(0x78) # 'x'
        d.put_char(0x09) # '\t'
        assert_equal 8, d.get_cursor.col
      end

      def test_backspace
        d = Display.new_test_driver
        d.put_char(0x41) # 'A'
        d.put_char(0x42) # 'B'
        d.put_char(0x08) # '\b'
        assert_equal 1, d.get_cursor.col
      end

      def test_backspace_at_col_zero
        d = Display.new_test_driver
        d.put_char(0x08)
        assert_equal 0, d.get_cursor.col
      end
    end

    # ============================================================
    # PutCharAt tests
    # ============================================================

    class TestPutCharAt < Minitest::Test
      def test_basic_write
        d = Display.new_test_driver
        d.put_char_at(5, 10, 0x58, 0x0F) # 'X'
        cell = d.get_cell(5, 10)
        assert_equal 0x58, cell.character
        assert_equal 0x0F, cell.attribute
      end

      def test_does_not_move_cursor
        d = Display.new_test_driver
        d.set_cursor(0, 0)
        d.put_char_at(5, 10, 0x58, 0x07)
        pos = d.get_cursor
        assert_equal 0, pos.row
        assert_equal 0, pos.col
      end

      def test_out_of_bounds
        d = Display.new_test_driver
        # Should not crash.
        d.put_char_at(30, 0, 0x58, 0x07)
        d.put_char_at(-1, 0, 0x58, 0x07)
        d.put_char_at(0, -1, 0x58, 0x07)
        d.put_char_at(0, 100, 0x58, 0x07)
      end
    end

    # ============================================================
    # Puts tests
    # ============================================================

    class TestPuts < Minitest::Test
      def test_simple_string
        d = Display.new_test_driver
        d.puts_str("Hello")
        "Hello".bytes.each_with_index do |ch, i|
          assert_equal ch, d.get_cell(0, i).character
        end
        assert_equal 5, d.get_cursor.col
      end

      def test_with_newline
        d = Display.new_test_driver
        d.puts_str("Hi\nBye")
        snap = d.snapshot
        assert_equal "Hi", snap.lines[0]
        assert_equal "Bye", snap.lines[1]
      end

      def test_empty_string
        d = Display.new_test_driver
        d.puts_str("")
        pos = d.get_cursor
        assert_equal 0, pos.row
        assert_equal 0, pos.col
      end
    end

    # ============================================================
    # Line wrap tests
    # ============================================================

    class TestLineWrap < Minitest::Test
      def test_wrap_at_end_of_row
        d = Display.new_test_driver
        d.config.columns.times { d.put_char(0x41) }
        pos = d.get_cursor
        assert_equal 1, pos.row
        assert_equal 0, pos.col
      end

      def test_wrap_next_char
        d = Display.new_test_driver
        d.config.columns.times { d.put_char(0x41) }
        d.put_char(0x42) # 'B'
        assert_equal 0x42, d.get_cell(1, 0).character
      end

      def test_multi_line_wrap
        d = Display.new_test_driver
        total = d.config.columns * 2 + 1
        total.times { d.put_char(0x78) }
        pos = d.get_cursor
        assert_equal 2, pos.row
        assert_equal 1, pos.col
      end
    end

    # ============================================================
    # Scroll tests
    # ============================================================

    class TestScroll < Minitest::Test
      def test_scroll_trigger
        d = Display.new_test_driver
        d.config.rows.times do |row|
          d.put_char_at(row, 0, 0x41 + row, DEFAULT_ATTRIBUTE)
        end
        row1_char = d.get_cell(1, 0).character

        d.set_cursor(d.config.rows - 1, 0)
        d.put_char(0x0A) # '\n'

        assert_equal row1_char, d.get_cell(0, 0).character
      end

      def test_last_row_cleared
        d = Display.new_test_driver
        d.config.rows.times do |row|
          d.config.columns.times do |col|
            d.put_char_at(row, col, 0x58, DEFAULT_ATTRIBUTE)
          end
        end

        d.set_cursor(d.config.rows - 1, 0)
        d.put_char(0x0A)

        d.config.columns.times do |col|
          assert_equal 0x20, d.get_cell(d.config.rows - 1, col).character
        end
      end

      def test_cursor_after_scroll
        d = Display.new_test_driver
        d.set_cursor(d.config.rows - 1, 0)
        d.put_char(0x0A)
        pos = d.get_cursor
        assert_equal d.config.rows - 1, pos.row
        assert_equal 0, pos.col
      end

      def test_multiple_scrolls
        d = Display.new_test_driver
        30.times do
          d.puts_str("Line")
          d.put_char(0x0A)
        end
        snap = d.snapshot
        assert snap.contains?("Line")
      end

      def test_scroll_preserves_attributes
        d = Display.new_test_driver
        custom_attr = Display.make_attribute(COLOR_WHITE, COLOR_BLUE)
        d.put_char_at(1, 0, 0x5A, custom_attr) # 'Z'

        d.set_cursor(d.config.rows - 1, 0)
        d.put_char(0x0A)

        cell = d.get_cell(0, 0)
        assert_equal 0x5A, cell.character
        assert_equal custom_attr, cell.attribute
      end
    end

    # ============================================================
    # Clear tests
    # ============================================================

    class TestClear < Minitest::Test
      def test_clear_display
        d = Display.new_test_driver
        d.puts_str("Hello World")
        d.clear

        d.config.rows.times do |row|
          d.config.columns.times do |col|
            cell = d.get_cell(row, col)
            assert_equal 0x20, cell.character
            assert_equal DEFAULT_ATTRIBUTE, cell.attribute
          end
        end
      end

      def test_clear_resets_cursor
        d = Display.new_test_driver
        d.puts_str("Hello")
        d.clear
        pos = d.get_cursor
        assert_equal 0, pos.row
        assert_equal 0, pos.col
      end
    end

    # ============================================================
    # Snapshot tests
    # ============================================================

    class TestSnapshot < Minitest::Test
      def test_basic
        d = Display.new_test_driver
        d.puts_str("Hello World")
        snap = d.snapshot
        assert_equal "Hello World", snap.lines[0]
      end

      def test_trailing_spaces_trimmed
        d = Display.new_test_driver
        d.puts_str("Hi")
        snap = d.snapshot
        assert_equal "Hi", snap.lines[0]
      end

      def test_empty_lines
        d = Display.new_test_driver
        snap = d.snapshot
        snap.lines.each { |line| assert_equal "", line }
      end

      def test_contains_positive
        d = Display.new_test_driver
        d.puts_str("Hello World")
        snap = d.snapshot
        assert snap.contains?("Hello World")
      end

      def test_contains_negative
        d = Display.new_test_driver
        d.puts_str("Hello World")
        snap = d.snapshot
        refute snap.contains?("Goodbye")
      end

      def test_contains_partial
        d = Display.new_test_driver
        d.puts_str("Hello World")
        snap = d.snapshot
        assert snap.contains?("World")
      end

      def test_string_output
        d = Display.new_test_driver
        d.puts_str("Hello")
        snap = d.snapshot
        s = snap.to_s
        lines = s.split("\n")
        assert_equal d.config.rows, lines.length
        lines.each { |line| assert_equal d.config.columns, line.length }
      end

      def test_cursor_in_snapshot
        d = Display.new_test_driver
        d.set_cursor(5, 10)
        snap = d.snapshot
        assert_equal 5, snap.cursor.row
        assert_equal 10, snap.cursor.col
      end

      def test_line_at
        d = Display.new_test_driver
        d.puts_str("Line 0")
        d.put_char(0x0A)
        d.puts_str("Line 1")
        snap = d.snapshot
        assert_equal "Line 0", snap.line_at(0)
        assert_equal "Line 1", snap.line_at(1)
        assert_equal "", snap.line_at(-1)
        assert_equal "", snap.line_at(100)
      end

      def test_rows_and_columns
        d = Display.new_test_driver
        snap = d.snapshot
        assert_equal d.config.rows, snap.rows
        assert_equal d.config.columns, snap.columns
      end
    end

    # ============================================================
    # Attribute tests
    # ============================================================

    class TestAttributes < Minitest::Test
      def test_default_attribute
        d = Display.new_test_driver
        d.put_char(0x41)
        assert_equal 0x07, d.get_cell(0, 0).attribute
      end

      def test_custom_attribute
        d = Display.new_test_driver
        d.put_char_at(0, 0, 0x41, 0x1F)
        assert_equal 0x1F, d.get_cell(0, 0).attribute
      end
    end

    # ============================================================
    # Cursor management tests
    # ============================================================

    class TestCursorManagement < Minitest::Test
      def test_set_cursor_clamps_negative
        d = Display.new_test_driver
        d.set_cursor(-5, -5)
        pos = d.get_cursor
        assert_equal 0, pos.row
        assert_equal 0, pos.col
      end

      def test_set_cursor_clamps_large
        d = Display.new_test_driver
        d.set_cursor(100, 100)
        pos = d.get_cursor
        assert_equal d.config.rows - 1, pos.row
        assert_equal d.config.columns - 1, pos.col
      end
    end

    # ============================================================
    # Edge case tests
    # ============================================================

    class TestEdgeCases < Minitest::Test
      def test_full_framebuffer
        d = Display.new_test_driver
        total = d.config.columns * d.config.rows
        total.times { d.put_char(0x58) }
        snap = d.snapshot
        assert snap.contains?("X")
      end

      def test_rapid_scrolling
        d = Display.new_test_driver
        100.times do
          d.puts_str("Line")
          d.put_char(0x0A)
        end
        snap = d.snapshot
        assert snap.contains?("Line")
      end

      def test_null_character
        d = Display.new_test_driver
        d.put_char(0x00)
        assert_equal 0x00, d.get_cell(0, 0).character
      end

      def test_all_ascii_values
        d = Display.new_standard_driver
        256.times do |i|
          row = i / d.config.columns
          col = i % d.config.columns
          d.put_char_at(row, col, i, DEFAULT_ATTRIBUTE)
        end
        256.times do |i|
          row = i / d.config.columns
          col = i % d.config.columns
          assert_equal i, d.get_cell(row, col).character
        end
      end

      def test_get_cell_out_of_bounds
        d = Display.new_test_driver
        cell = d.get_cell(-1, 0)
        assert_equal 0x20, cell.character
        assert_equal DEFAULT_ATTRIBUTE, cell.attribute
      end

      def test_tab_wrap_to_next_row
        d = Display.new_test_driver
        d.set_cursor(0, 39)
        d.put_char(0x09)
        pos = d.get_cursor
        assert_equal 1, pos.row
        assert_equal 0, pos.col
      end
    end

    # ============================================================
    # Standard 80x25 tests
    # ============================================================

    class TestStandard80x25 < Minitest::Test
      def test_put_char
        d = Display.new_standard_driver
        d.put_char(0x41)
        cell = d.get_cell(0, 0)
        assert_equal 0x41, cell.character
        assert_equal 0x07, cell.attribute
      end

      def test_line_wrap
        d = Display.new_standard_driver
        81.times { d.put_char(0x41) }
        pos = d.get_cursor
        assert_equal 1, pos.row
        assert_equal 1, pos.col
      end

      def test_scroll
        d = Display.new_standard_driver
        25.times { |row| d.put_char_at(row, 0, 0x41 + (row % 26), DEFAULT_ATTRIBUTE) }
        d.set_cursor(24, 0)
        d.put_char(0x0A)
        assert_equal 0x42, d.get_cell(0, 0).character # 'B'
      end
    end
  end
end
