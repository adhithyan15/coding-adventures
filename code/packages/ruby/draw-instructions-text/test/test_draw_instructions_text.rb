# frozen_string_literal: true

require "minitest/autorun"
require "coding_adventures_draw_instructions_text"

# == Test Suite for DrawInstructionsText
#
# These tests verify that the text renderer correctly converts draw
# instruction scenes into box-drawing character strings. All tests use
# 1:1 scale for easy reasoning about coordinates.
class TestDrawInstructionsText < Minitest::Test
  DI = CodingAdventures::DrawInstructions
  DIT = CodingAdventures::DrawInstructionsText

  # ------------------------------------------------------------------
  # Version
  # ------------------------------------------------------------------

  def test_version_exists
    refute_nil DIT::VERSION
    assert_match(/\A\d+\.\d+\.\d+\z/, DIT::VERSION)
  end

  # ------------------------------------------------------------------
  # Stroked rectangles
  # ------------------------------------------------------------------

  def test_stroked_rect_draws_box_with_corners_and_edges
    scene = DI.create_scene(width: 5, height: 3, instructions: [
      DI.draw_rect(x: 0, y: 0, width: 4, height: 2, fill: "transparent",
                   stroke: "#000", stroke_width: 1)
    ])
    result = DIT.render_text(scene, scale_x: 1, scale_y: 1)

    expected = "\u250C\u2500\u2500\u2500\u2510\n" \
               "\u2502   \u2502\n" \
               "\u2514\u2500\u2500\u2500\u2518"
    assert_equal expected, result
  end

  # ------------------------------------------------------------------
  # Filled rectangles
  # ------------------------------------------------------------------

  def test_filled_rect_uses_block_characters
    scene = DI.create_scene(width: 3, height: 2, instructions: [
      DI.draw_rect(x: 0, y: 0, width: 2, height: 1, fill: "#000")
    ])
    result = DIT.render_text(scene, scale_x: 1, scale_y: 1)
    assert_includes result, "\u2588"
  end

  # ------------------------------------------------------------------
  # Horizontal lines
  # ------------------------------------------------------------------

  def test_horizontal_line
    scene = DI.create_scene(width: 5, height: 1, instructions: [
      DI.draw_line(x1: 0, y1: 0, x2: 4, y2: 0)
    ])
    result = DIT.render_text(scene, scale_x: 1, scale_y: 1)
    assert_equal "\u2500\u2500\u2500\u2500\u2500", result
  end

  # ------------------------------------------------------------------
  # Vertical lines
  # ------------------------------------------------------------------

  def test_vertical_line
    scene = DI.create_scene(width: 1, height: 3, instructions: [
      DI.draw_line(x1: 0, y1: 0, x2: 0, y2: 2)
    ])
    result = DIT.render_text(scene, scale_x: 1, scale_y: 1)
    assert_equal "\u2502\n\u2502\n\u2502", result
  end

  # ------------------------------------------------------------------
  # Line intersections
  # ------------------------------------------------------------------

  def test_crossing_lines_produce_cross
    scene = DI.create_scene(width: 5, height: 3, instructions: [
      DI.draw_line(x1: 0, y1: 1, x2: 4, y2: 1),
      DI.draw_line(x1: 2, y1: 0, x2: 2, y2: 2)
    ])
    result = DIT.render_text(scene, scale_x: 1, scale_y: 1)
    lines = result.split("\n")

    # Row 0: vertical at col 2
    assert_equal "\u2502", lines[0][2]
    # Row 1: cross at col 2
    assert_equal "\u253C", lines[1][2]
    # Row 2: vertical at col 2
    assert_equal "\u2502", lines[2][2]
  end

  # ------------------------------------------------------------------
  # Box with internal lines (table grid)
  # ------------------------------------------------------------------

  def test_table_grid
    scene = DI.create_scene(width: 7, height: 3, instructions: [
      DI.draw_rect(x: 0, y: 0, width: 6, height: 2, fill: "transparent",
                   stroke: "#000", stroke_width: 1),
      DI.draw_line(x1: 0, y1: 1, x2: 6, y2: 1)
    ])
    result = DIT.render_text(scene, scale_x: 1, scale_y: 1)
    lines = result.split("\n")

    assert_equal "\u250C\u2500\u2500\u2500\u2500\u2500\u2510", lines[0]
    assert_equal "\u251C", lines[1][0]  # left tee
    assert_equal "\u2524", lines[1][6]  # right tee
    assert_equal "\u2514\u2500\u2500\u2500\u2500\u2500\u2518", lines[2]
  end

  # ------------------------------------------------------------------
  # Text rendering
  # ------------------------------------------------------------------

  def test_text_at_position
    scene = DI.create_scene(width: 10, height: 1, instructions: [
      DI.draw_text(x: 0, y: 0, value: "Hello", align: "start")
    ])
    result = DIT.render_text(scene, scale_x: 1, scale_y: 1)
    assert_equal "Hello", result
  end

  def test_text_middle_alignment
    scene = DI.create_scene(width: 10, height: 1, instructions: [
      DI.draw_text(x: 5, y: 0, value: "Hi", align: "middle")
    ])
    result = DIT.render_text(scene, scale_x: 1, scale_y: 1)
    assert_equal "H", result[4]
    assert_equal "i", result[5]
  end

  def test_text_end_alignment
    scene = DI.create_scene(width: 10, height: 1, instructions: [
      DI.draw_text(x: 9, y: 0, value: "End", align: "end")
    ])
    result = DIT.render_text(scene, scale_x: 1, scale_y: 1)
    assert_equal "E", result[6]
    assert_equal "n", result[7]
    assert_equal "d", result[8]
  end

  # ------------------------------------------------------------------
  # Text inside a box
  # ------------------------------------------------------------------

  def test_text_inside_box
    scene = DI.create_scene(width: 12, height: 3, instructions: [
      DI.draw_rect(x: 0, y: 0, width: 11, height: 2, fill: "transparent",
                   stroke: "#000", stroke_width: 1),
      DI.draw_text(x: 1, y: 1, value: "Hello", align: "start")
    ])
    result = DIT.render_text(scene, scale_x: 1, scale_y: 1)
    lines = result.split("\n")

    assert_equal "\u250C\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2510", lines[0]
    assert_equal "\u2502Hello     \u2502", lines[1]
    assert_equal "\u2514\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2518", lines[2]
  end

  # ------------------------------------------------------------------
  # Clips
  # ------------------------------------------------------------------

  def test_clip_truncates_text
    scene = DI.create_scene(width: 10, height: 1, instructions: [
      DI.draw_clip(x: 0, y: 0, width: 3, height: 1, children: [
        DI.draw_text(x: 0, y: 0, value: "Hello World", align: "start")
      ])
    ])
    result = DIT.render_text(scene, scale_x: 1, scale_y: 1)
    assert_equal "Hel", result
  end

  # ------------------------------------------------------------------
  # Groups
  # ------------------------------------------------------------------

  def test_group_recurses_into_children
    scene = DI.create_scene(width: 5, height: 1, instructions: [
      DI.draw_group(children: [
        DI.draw_text(x: 0, y: 0, value: "AB", align: "start"),
        DI.draw_text(x: 3, y: 0, value: "CD", align: "start")
      ])
    ])
    result = DIT.render_text(scene, scale_x: 1, scale_y: 1)
    assert_equal "AB CD", result
  end

  # ------------------------------------------------------------------
  # Complete table demo
  # ------------------------------------------------------------------

  def test_complete_table
    scene = DI.create_scene(width: 13, height: 6, instructions: [
      DI.draw_rect(x: 0, y: 0, width: 12, height: 5, fill: "transparent",
                   stroke: "#000", stroke_width: 1),
      DI.draw_line(x1: 6, y1: 0, x2: 6, y2: 5),
      DI.draw_line(x1: 0, y1: 2, x2: 12, y2: 2),
      DI.draw_text(x: 1, y: 1, value: "Name", align: "start"),
      DI.draw_text(x: 7, y: 1, value: "Age", align: "start"),
      DI.draw_text(x: 1, y: 3, value: "Alice", align: "start"),
      DI.draw_text(x: 7, y: 3, value: "30", align: "start"),
      DI.draw_text(x: 1, y: 4, value: "Bob", align: "start"),
      DI.draw_text(x: 7, y: 4, value: "25", align: "start")
    ])
    result = DIT.render_text(scene, scale_x: 1, scale_y: 1)
    lines = result.split("\n")

    assert_equal "\u250C\u2500\u2500\u2500\u2500\u2500\u252C\u2500\u2500\u2500\u2500\u2500\u2510", lines[0]
    assert_includes lines[1], "Name"
    assert_includes lines[1], "Age"
    assert_equal "\u251C", lines[2][0]
    assert_equal "\u253C", lines[2][6]
    assert_equal "\u2524", lines[2][12]
    assert_includes lines[3], "Alice"
    assert_includes lines[3], "30"
    assert_includes lines[4], "Bob"
    assert_includes lines[4], "25"
    assert_equal "\u2514\u2500\u2500\u2500\u2500\u2500\u2534\u2500\u2500\u2500\u2500\u2500\u2518", lines[5]
  end

  # ------------------------------------------------------------------
  # Scale factor
  # ------------------------------------------------------------------

  def test_default_scale_maps_px_to_chars
    scene = DI.create_scene(width: 88, height: 48, instructions: [
      DI.draw_rect(x: 0, y: 0, width: 80, height: 32, fill: "transparent",
                   stroke: "#000", stroke_width: 1)
    ])
    result = DIT.render_text(scene)
    lines = result.split("\n")
    assert_equal 3, lines.length
    assert_equal "\u250C", lines[0][0]
    assert_equal "\u2514", lines[2][0]
  end

  def test_custom_scale
    renderer = DIT::TextRenderer.new(scale_x: 4, scale_y: 4)
    scene = DI.create_scene(width: 12, height: 8, instructions: [
      DI.draw_line(x1: 0, y1: 0, x2: 12, y2: 0)
    ])
    result = renderer.render(scene)
    assert_includes result, "\u2500"
  end

  # ------------------------------------------------------------------
  # TextRenderer class (duck-typed renderer)
  # ------------------------------------------------------------------

  def test_text_renderer_works_with_render_with
    scene = DI.create_scene(width: 5, height: 1, instructions: [
      DI.draw_text(x: 0, y: 0, value: "OK", align: "start")
    ])
    renderer = DIT::TextRenderer.new(scale_x: 1, scale_y: 1)
    result = DI.render_with(scene, renderer)
    assert_equal "OK", result
  end

  # ------------------------------------------------------------------
  # Empty scene
  # ------------------------------------------------------------------

  def test_empty_scene
    scene = DI.create_scene(width: 0, height: 0, instructions: [])
    result = DIT.render_text(scene, scale_x: 1, scale_y: 1)
    assert_equal "", result
  end

  # ------------------------------------------------------------------
  # Transparent rect is not rendered
  # ------------------------------------------------------------------

  def test_transparent_rect_not_rendered
    scene = DI.create_scene(width: 5, height: 3, instructions: [
      DI.draw_rect(x: 0, y: 0, width: 4, height: 2, fill: "transparent")
    ])
    result = DIT.render_text(scene, scale_x: 1, scale_y: 1)
    assert_equal "", result
  end

  # ------------------------------------------------------------------
  # Diagonal line
  # ------------------------------------------------------------------

  def test_diagonal_line
    scene = DI.create_scene(width: 5, height: 5, instructions: [
      DI.draw_line(x1: 0, y1: 0, x2: 4, y2: 4)
    ])
    result = DIT.render_text(scene, scale_x: 1, scale_y: 1)
    refute_empty result
    assert result.split("\n").length >= 3
  end

  # ------------------------------------------------------------------
  # Text overrides box-drawing
  # ------------------------------------------------------------------

  def test_text_overrides_box_drawing
    scene = DI.create_scene(width: 10, height: 1, instructions: [
      DI.draw_line(x1: 0, y1: 0, x2: 9, y2: 0),
      DI.draw_text(x: 2, y: 0, value: "AB", align: "start")
    ])
    result = DIT.render_text(scene, scale_x: 1, scale_y: 1)
    assert_equal "A", result[2]
    assert_equal "B", result[3]
  end

  # ------------------------------------------------------------------
  # Box-drawing does not overwrite text
  # ------------------------------------------------------------------

  def test_box_drawing_does_not_overwrite_text
    scene = DI.create_scene(width: 10, height: 1, instructions: [
      DI.draw_text(x: 2, y: 0, value: "X", align: "start"),
      DI.draw_line(x1: 0, y1: 0, x2: 9, y2: 0)
    ])
    result = DIT.render_text(scene, scale_x: 1, scale_y: 1)
    assert_equal "X", result[2]
  end

  # ------------------------------------------------------------------
  # Nested clip
  # ------------------------------------------------------------------

  def test_nested_clip
    inner = DI.draw_clip(x: 0, y: 0, width: 3, height: 1, children: [
      DI.draw_text(x: 0, y: 0, value: "ABCDEFGH", align: "start")
    ])
    outer = DI.draw_clip(x: 0, y: 0, width: 5, height: 1, children: [inner])
    scene = DI.create_scene(width: 10, height: 1, instructions: [outer])
    result = DIT.render_text(scene, scale_x: 1, scale_y: 1)
    assert_equal "ABC", result
  end

  # ------------------------------------------------------------------
  # Reversed lines
  # ------------------------------------------------------------------

  def test_reversed_horizontal_line
    scene = DI.create_scene(width: 5, height: 1, instructions: [
      DI.draw_line(x1: 4, y1: 0, x2: 0, y2: 0)
    ])
    result = DIT.render_text(scene, scale_x: 1, scale_y: 1)
    assert_equal "\u2500\u2500\u2500\u2500\u2500", result
  end

  def test_reversed_vertical_line
    scene = DI.create_scene(width: 1, height: 3, instructions: [
      DI.draw_line(x1: 0, y1: 2, x2: 0, y2: 0)
    ])
    result = DIT.render_text(scene, scale_x: 1, scale_y: 1)
    assert_equal "\u2502\n\u2502\n\u2502", result
  end

  # ------------------------------------------------------------------
  # "none" fill rect not rendered
  # ------------------------------------------------------------------

  def test_none_fill_rect_not_rendered
    scene = DI.create_scene(width: 5, height: 3, instructions: [
      DI.draw_rect(x: 0, y: 0, width: 4, height: 2, fill: "none")
    ])
    result = DIT.render_text(scene, scale_x: 1, scale_y: 1)
    assert_equal "", result
  end

  # ------------------------------------------------------------------
  # Steep diagonal
  # ------------------------------------------------------------------

  def test_steep_diagonal_uses_vertical_chars
    scene = DI.create_scene(width: 3, height: 7, instructions: [
      DI.draw_line(x1: 0, y1: 0, x2: 2, y2: 6)
    ])
    result = DIT.render_text(scene, scale_x: 1, scale_y: 1)
    refute_empty result
    assert_includes result, "\u2502"
  end
end
