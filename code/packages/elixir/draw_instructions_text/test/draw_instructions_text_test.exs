defmodule CodingAdventures.DrawInstructionsTextTest do
  use ExUnit.Case

  alias CodingAdventures.DrawInstructions
  alias CodingAdventures.DrawInstructionsText

  # Use 1:1 scale for easy reasoning in tests
  @opts [scale_x: 1, scale_y: 1]

  # ---------------------------------------------------------------------------
  # Module loads
  # ---------------------------------------------------------------------------

  test "module loads" do
    assert Code.ensure_loaded?(DrawInstructionsText)
  end

  # ---------------------------------------------------------------------------
  # Stroked rectangles
  # ---------------------------------------------------------------------------

  test "draws a box with corners and edges" do
    scene =
      DrawInstructions.create_scene(5, 3, [
        DrawInstructions.draw_rect(0, 0, 4, 2, "transparent", stroke: "#000", stroke_width: 1)
      ])

    result = DrawInstructionsText.render_text(scene, @opts)

    # Expected:
    # ┌───┐
    # │   │
    # └───┘
    assert result ==
             "\u250C\u2500\u2500\u2500\u2510\n" <>
               "\u2502   \u2502\n" <>
               "\u2514\u2500\u2500\u2500\u2518"
  end

  test "draws a minimal 1x1 stroked rect" do
    scene =
      DrawInstructions.create_scene(2, 2, [
        DrawInstructions.draw_rect(0, 0, 1, 1, "transparent", stroke: "#000", stroke_width: 1)
      ])

    result = DrawInstructionsText.render_text(scene, @opts)

    # A 1x1 stroked rect has corners touching
    assert String.contains?(result, "\u250C")
    assert String.contains?(result, "\u2518")
  end

  # ---------------------------------------------------------------------------
  # Filled rectangles
  # ---------------------------------------------------------------------------

  test "fills with block characters" do
    scene =
      DrawInstructions.create_scene(3, 2, [
        DrawInstructions.draw_rect(0, 0, 2, 1, "#000")
      ])

    result = DrawInstructionsText.render_text(scene, @opts)
    assert String.contains?(result, "\u2588")
  end

  test "transparent rect with no stroke is invisible" do
    scene =
      DrawInstructions.create_scene(5, 3, [
        DrawInstructions.draw_rect(0, 0, 4, 2, "transparent")
      ])

    result = DrawInstructionsText.render_text(scene, @opts)
    assert result == ""
  end

  test "rect with fill 'none' and no stroke is invisible" do
    scene =
      DrawInstructions.create_scene(5, 3, [
        DrawInstructions.draw_rect(0, 0, 4, 2, "none")
      ])

    result = DrawInstructionsText.render_text(scene, @opts)
    assert result == ""
  end

  # ---------------------------------------------------------------------------
  # Horizontal lines
  # ---------------------------------------------------------------------------

  test "draws a horizontal line" do
    scene =
      DrawInstructions.create_scene(5, 1, [
        DrawInstructions.draw_line(0, 0, 4, 0)
      ])

    result = DrawInstructionsText.render_text(scene, @opts)
    assert result == "\u2500\u2500\u2500\u2500\u2500"
  end

  # ---------------------------------------------------------------------------
  # Vertical lines
  # ---------------------------------------------------------------------------

  test "draws a vertical line" do
    scene =
      DrawInstructions.create_scene(1, 3, [
        DrawInstructions.draw_line(0, 0, 0, 2)
      ])

    result = DrawInstructionsText.render_text(scene, @opts)
    assert result == "\u2502\n\u2502\n\u2502"
  end

  # ---------------------------------------------------------------------------
  # Line intersections
  # ---------------------------------------------------------------------------

  test "crossing lines produce a cross character" do
    scene =
      DrawInstructions.create_scene(5, 3, [
        DrawInstructions.draw_line(0, 1, 4, 1),
        DrawInstructions.draw_line(2, 0, 2, 2)
      ])

    result = DrawInstructionsText.render_text(scene, @opts)
    lines = String.split(result, "\n")

    # Row 0: vertical line at col 2
    assert String.at(Enum.at(lines, 0), 2) == "\u2502"
    # Row 1: cross at col 2
    assert String.at(Enum.at(lines, 1), 2) == "\u253C"
    # Row 2: vertical line at col 2
    assert String.at(Enum.at(lines, 2), 2) == "\u2502"
  end

  # ---------------------------------------------------------------------------
  # Box with internal lines (table grid)
  # ---------------------------------------------------------------------------

  test "produces a table-like grid with tee characters" do
    scene =
      DrawInstructions.create_scene(7, 3, [
        DrawInstructions.draw_rect(0, 0, 6, 2, "transparent", stroke: "#000", stroke_width: 1),
        DrawInstructions.draw_line(0, 1, 6, 1)
      ])

    result = DrawInstructionsText.render_text(scene, @opts)
    lines = String.split(result, "\n")

    # Top row: ┌─────┐
    assert Enum.at(lines, 0) == "\u250C\u2500\u2500\u2500\u2500\u2500\u2510"
    # Middle row: starts with ├, ends with ┤
    assert String.at(Enum.at(lines, 1), 0) == "\u251C"
    assert String.at(Enum.at(lines, 1), 6) == "\u2524"
    # Bottom row: └─────┘
    assert Enum.at(lines, 2) == "\u2514\u2500\u2500\u2500\u2500\u2500\u2518"
  end

  # ---------------------------------------------------------------------------
  # Text rendering
  # ---------------------------------------------------------------------------

  test "writes text with start alignment" do
    scene =
      DrawInstructions.create_scene(10, 1, [
        DrawInstructions.draw_text(0, 0, "Hello", align: "start")
      ])

    result = DrawInstructionsText.render_text(scene, @opts)
    assert result == "Hello"
  end

  test "centers text with middle alignment" do
    scene =
      DrawInstructions.create_scene(10, 1, [
        DrawInstructions.draw_text(5, 0, "Hi", align: "middle")
      ])

    result = DrawInstructionsText.render_text(scene, @opts)
    # "Hi" centered at col 5: starts at col 4
    assert String.at(result, 4) == "H"
    assert String.at(result, 5) == "i"
  end

  test "right-aligns text with end alignment" do
    scene =
      DrawInstructions.create_scene(10, 1, [
        DrawInstructions.draw_text(9, 0, "End", align: "end")
      ])

    result = DrawInstructionsText.render_text(scene, @opts)
    assert String.at(result, 6) == "E"
    assert String.at(result, 7) == "n"
    assert String.at(result, 8) == "d"
  end

  # ---------------------------------------------------------------------------
  # Text inside a box
  # ---------------------------------------------------------------------------

  test "renders text inside a stroked rectangle" do
    scene =
      DrawInstructions.create_scene(12, 3, [
        DrawInstructions.draw_rect(0, 0, 11, 2, "transparent", stroke: "#000", stroke_width: 1),
        DrawInstructions.draw_text(1, 1, "Hello", align: "start")
      ])

    result = DrawInstructionsText.render_text(scene, @opts)
    lines = String.split(result, "\n")

    assert Enum.at(lines, 0) ==
             "\u250C\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2510"

    assert Enum.at(lines, 1) == "\u2502Hello     \u2502"

    assert Enum.at(lines, 2) ==
             "\u2514\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2518"
  end

  # ---------------------------------------------------------------------------
  # Clips
  # ---------------------------------------------------------------------------

  test "clips text that extends beyond the region" do
    scene =
      DrawInstructions.create_scene(10, 1, [
        DrawInstructions.draw_clip(0, 0, 3, 1, [
          DrawInstructions.draw_text(0, 0, "Hello World", align: "start")
        ])
      ])

    result = DrawInstructionsText.render_text(scene, @opts)
    assert result == "Hel"
  end

  test "nested clips intersect properly" do
    scene =
      DrawInstructions.create_scene(10, 1, [
        DrawInstructions.draw_clip(0, 0, 5, 1, [
          DrawInstructions.draw_clip(2, 0, 5, 1, [
            DrawInstructions.draw_text(0, 0, "ABCDEFGHIJ", align: "start")
          ])
        ])
      ])

    result = DrawInstructionsText.render_text(scene, @opts)
    # Outer clip: cols 0-4, inner clip: cols 2-6
    # Intersection: cols 2-4, so only chars at positions 2,3,4 are visible
    assert result == "  CDE"
  end

  # ---------------------------------------------------------------------------
  # Groups
  # ---------------------------------------------------------------------------

  test "recurses into group children" do
    scene =
      DrawInstructions.create_scene(5, 1, [
        DrawInstructions.draw_group([
          DrawInstructions.draw_text(0, 0, "AB", align: "start"),
          DrawInstructions.draw_text(3, 0, "CD", align: "start")
        ])
      ])

    result = DrawInstructionsText.render_text(scene, @opts)
    assert result == "AB CD"
  end

  test "nested groups work" do
    inner =
      DrawInstructions.draw_group([
        DrawInstructions.draw_text(0, 0, "X", align: "start")
      ])

    outer = DrawInstructions.draw_group([inner])
    scene = DrawInstructions.create_scene(3, 1, [outer])
    result = DrawInstructionsText.render_text(scene, @opts)
    assert result == "X"
  end

  # ---------------------------------------------------------------------------
  # Table demo
  # ---------------------------------------------------------------------------

  test "renders a complete table with headers and data" do
    scene =
      DrawInstructions.create_scene(13, 6, [
        # Outer border
        DrawInstructions.draw_rect(0, 0, 12, 5, "transparent", stroke: "#000", stroke_width: 1),
        # Vertical divider at x=6
        DrawInstructions.draw_line(6, 0, 6, 5),
        # Horizontal divider at y=2
        DrawInstructions.draw_line(0, 2, 12, 2),
        # Header text
        DrawInstructions.draw_text(1, 1, "Name", align: "start"),
        DrawInstructions.draw_text(7, 1, "Age", align: "start"),
        # Data row 1
        DrawInstructions.draw_text(1, 3, "Alice", align: "start"),
        DrawInstructions.draw_text(7, 3, "30", align: "start"),
        # Data row 2
        DrawInstructions.draw_text(1, 4, "Bob", align: "start"),
        DrawInstructions.draw_text(7, 4, "25", align: "start")
      ])

    result = DrawInstructionsText.render_text(scene, @opts)
    lines = String.split(result, "\n")

    # ┌─────┬─────┐
    assert Enum.at(lines, 0) ==
             "\u250C\u2500\u2500\u2500\u2500\u2500\u252C\u2500\u2500\u2500\u2500\u2500\u2510"

    assert String.contains?(Enum.at(lines, 1), "Name")
    assert String.contains?(Enum.at(lines, 1), "Age")

    # ├─────┼─────┤
    assert String.at(Enum.at(lines, 2), 0) == "\u251C"
    assert String.at(Enum.at(lines, 2), 6) == "\u253C"
    assert String.at(Enum.at(lines, 2), 12) == "\u2524"

    assert String.contains?(Enum.at(lines, 3), "Alice")
    assert String.contains?(Enum.at(lines, 3), "30")
    assert String.contains?(Enum.at(lines, 4), "Bob")
    assert String.contains?(Enum.at(lines, 4), "25")

    # └─────┴─────┘
    assert Enum.at(lines, 5) ==
             "\u2514\u2500\u2500\u2500\u2500\u2500\u2534\u2500\u2500\u2500\u2500\u2500\u2518"
  end

  # ---------------------------------------------------------------------------
  # Scale factor
  # ---------------------------------------------------------------------------

  test "maps pixel coordinates to characters using default scale" do
    # Default scale: 8px/col, 16px/row
    # A rect at (0,0) with width=80 height=32 -> 10 cols, 2 rows
    scene =
      DrawInstructions.create_scene(88, 48, [
        DrawInstructions.draw_rect(0, 0, 80, 32, "transparent", stroke: "#000", stroke_width: 1)
      ])

    result = DrawInstructionsText.render_text(scene)
    lines = String.split(result, "\n")

    assert length(lines) == 3
    assert String.at(Enum.at(lines, 0), 0) == "\u250C"
    assert String.at(Enum.at(lines, 2), 0) == "\u2514"
  end

  test "respects custom scale factor" do
    result =
      DrawInstructions.create_scene(12, 8, [
        DrawInstructions.draw_line(0, 0, 12, 0)
      ])
      |> DrawInstructionsText.render_text(scale_x: 4, scale_y: 4)

    assert String.contains?(result, "\u2500")
  end

  # ---------------------------------------------------------------------------
  # render_with integration
  # ---------------------------------------------------------------------------

  test "works with render_with from DrawInstructions" do
    scene =
      DrawInstructions.create_scene(5, 1, [
        DrawInstructions.draw_text(0, 0, "OK", align: "start")
      ])

    # Using the behaviour callback through render_with
    result = DrawInstructions.render_with(scene, DrawInstructionsText)
    # Default scale will map 5px wide -> 1 col at 8px/col scale
    # But the text "OK" at position 0,0 -> col 0, row 0
    assert String.contains?(result, "O")
  end

  # ---------------------------------------------------------------------------
  # Empty scene
  # ---------------------------------------------------------------------------

  test "returns empty string for empty scene" do
    scene = DrawInstructions.create_scene(0, 0, [])
    result = DrawInstructionsText.render_text(scene, @opts)
    assert result == ""
  end

  test "returns empty string for scene with no instructions" do
    scene = DrawInstructions.create_scene(5, 5, [])
    result = DrawInstructionsText.render_text(scene, @opts)
    assert result == ""
  end

  # ---------------------------------------------------------------------------
  # Text overwrites box-drawing
  # ---------------------------------------------------------------------------

  test "text overwrites box-drawing characters" do
    scene =
      DrawInstructions.create_scene(5, 1, [
        DrawInstructions.draw_line(0, 0, 4, 0),
        DrawInstructions.draw_text(1, 0, "X", align: "start")
      ])

    result = DrawInstructionsText.render_text(scene, @opts)
    assert String.at(result, 0) == "\u2500"
    assert String.at(result, 1) == "X"
    assert String.at(result, 2) == "\u2500"
  end

  test "box-drawing does not overwrite text" do
    scene =
      DrawInstructions.create_scene(5, 1, [
        DrawInstructions.draw_text(1, 0, "X", align: "start"),
        DrawInstructions.draw_line(0, 0, 4, 0)
      ])

    result = DrawInstructionsText.render_text(scene, @opts)
    # Text was placed first; box-drawing should not overwrite it
    assert String.at(result, 1) == "X"
  end

  # ---------------------------------------------------------------------------
  # Diagonal lines
  # ---------------------------------------------------------------------------

  test "draws a diagonal line" do
    scene =
      DrawInstructions.create_scene(4, 4, [
        DrawInstructions.draw_line(0, 0, 3, 3)
      ])

    result = DrawInstructionsText.render_text(scene, @opts)
    # Should produce some characters along the diagonal
    assert String.length(result) > 0
    lines = String.split(result, "\n")
    assert length(lines) >= 3
  end

  # ---------------------------------------------------------------------------
  # Trailing whitespace trimming
  # ---------------------------------------------------------------------------

  test "trims trailing whitespace per line" do
    scene =
      DrawInstructions.create_scene(10, 2, [
        DrawInstructions.draw_text(0, 0, "Hi", align: "start"),
        DrawInstructions.draw_text(0, 1, "Lo", align: "start")
      ])

    result = DrawInstructionsText.render_text(scene, @opts)
    lines = String.split(result, "\n")

    Enum.each(lines, fn line ->
      assert line == String.trim_trailing(line)
    end)
  end
end
