defmodule CodingAdventures.DrawInstructionsText do
  @moduledoc """
  ASCII/Unicode text renderer for the draw-instructions scene model.

  This renderer proves the draw-instructions abstraction is truly backend-
  neutral: the same DrawScene that produces SVG can also render as box-drawing
  characters in a terminal.

  ## How It Works

  The renderer maps pixel-coordinate scenes to a fixed-width character grid.
  Each cell in the grid is one character. The mapping uses a configurable
  scale factor (default: 8px per char width, 16px per char height).

  ```
  Scene coordinates (pixels)     Character grid
  +---------------------+        +----------+
  | rect at (0,0,200,32)|   ->   |##########|
  |                     |        |##########|
  +---------------------+        +----------+
  ```

  ## Character Palette

  Box-drawing characters create clean table grids:

  ```
  +------+-----+     Corners: top-left top-right bottom-left bottom-right
  | Name | Age |     Edges:   horizontal vertical
  +------+-----+     Tees:    top-tee bottom-tee left-tee right-tee
  | Alice|  30 |     Cross:   cross
  +------+-----+     Fill:    full-block
  ```

  ## Intersection Logic

  When two drawing operations overlap at the same cell, the renderer
  merges them into the correct junction character. A horizontal line
  crossing a vertical line becomes a cross. A line meeting a box corner
  becomes the appropriate tee.

  This is tracked via a "tag" buffer parallel to the character buffer.
  Each cell records which directions have lines passing through it
  (up, down, left, right), and the tag is resolved to the correct
  box-drawing character on each write.

  ## Direction Bitmask

  ```
         UP (1)
          |
  LEFT(8)-+-RIGHT(2)
          |
        DOWN(4)
  ```

  The FILL flag (16) marks cells filled with block characters.
  The TEXT flag (32) marks cells containing text characters.
  """

  @behaviour CodingAdventures.DrawInstructions

  import Bitwise

  # ---------------------------------------------------------------------------
  # Constants: direction bitmask flags
  #
  # Each cell in the tag buffer stores a bitmask of these flags. When
  # multiple drawing operations overlap, we OR the flags together and
  # resolve the combined tag to the correct box-drawing character.
  # ---------------------------------------------------------------------------

  @up 1
  @right 2
  @down 4
  @left 8
  @fill 16
  @text 32

  # ---------------------------------------------------------------------------
  # Box-drawing character lookup table
  #
  # Given a bitmask of directions (UP | DOWN | LEFT | RIGHT), return the
  # correct Unicode box-drawing character. This map covers all 16
  # combinations of the 4 direction bits.
  #
  #   Bitmask        | Char | Meaning
  #   ---------------|------|------------------
  #   LEFT|RIGHT     |  --  | horizontal line
  #   UP|DOWN        |  |   | vertical line
  #   DOWN|RIGHT     | top-left corner
  #   DOWN|LEFT      | top-right corner
  #   UP|RIGHT       | bottom-left corner
  #   UP|LEFT        | bottom-right corner
  #   L|R|DOWN       | top tee
  #   L|R|UP         | bottom tee
  #   U|D|RIGHT      | left tee
  #   U|D|LEFT       | right tee
  #   U|D|L|R        | cross
  # ---------------------------------------------------------------------------

  @box_chars %{
    (@left ||| @right) => "\u2500",
    (@up ||| @down) => "\u2502",
    (@down ||| @right) => "\u250C",
    (@down ||| @left) => "\u2510",
    (@up ||| @right) => "\u2514",
    (@up ||| @left) => "\u2518",
    (@left ||| @right ||| @down) => "\u252C",
    (@left ||| @right ||| @up) => "\u2534",
    (@up ||| @down ||| @right) => "\u251C",
    (@up ||| @down ||| @left) => "\u2524",
    (@up ||| @down ||| @left ||| @right) => "\u253C",
    @right => "\u2500",
    @left => "\u2500",
    @up => "\u2502",
    @down => "\u2502"
  }

  # ---------------------------------------------------------------------------
  # Public API
  # ---------------------------------------------------------------------------

  @doc """
  Render a draw scene to a Unicode box-drawing text string.

  This is the `DrawInstructions` behaviour callback. It uses the default
  scale of 8px per character column and 16px per character row.

  ## Example

      iex> scene = CodingAdventures.DrawInstructions.create_scene(16, 16, [
      ...>   CodingAdventures.DrawInstructions.draw_text(0, 0, "Hi", align: "start")
      ...> ])
      iex> CodingAdventures.DrawInstructionsText.render(scene)
      "Hi"
  """
  @impl true
  @spec render(CodingAdventures.DrawInstructions.draw_scene()) :: String.t()
  def render(scene) do
    render_text(scene)
  end

  @doc """
  Render a draw scene to text with custom scale options.

  ## Options

    - `:scale_x` - pixels per character column (default: 8)
    - `:scale_y` - pixels per character row (default: 16)

  ## Example

      iex> scene = CodingAdventures.DrawInstructions.create_scene(5, 3, [
      ...>   CodingAdventures.DrawInstructions.draw_rect(0, 0, 4, 2, "transparent",
      ...>     stroke: "#000", stroke_width: 1)
      ...> ])
      iex> CodingAdventures.DrawInstructionsText.render_text(scene, scale_x: 1, scale_y: 1)
      "\\u250C\\u2500\\u2500\\u2500\\u2510\\n\\u2502   \\u2502\\n\\u2514\\u2500\\u2500\\u2500\\u2518"
  """
  @spec render_text(CodingAdventures.DrawInstructions.draw_scene(), keyword()) :: String.t()
  def render_text(scene, opts \\ []) do
    sx = Keyword.get(opts, :scale_x, 8)
    sy = Keyword.get(opts, :scale_y, 16)

    cols = ceil(scene.width / sx)
    rows = ceil(scene.height / sy)

    # Early exit for zero-sized scenes
    if cols <= 0 or rows <= 0 do
      ""
    else
      # Initialize the character and tag buffers as maps for efficient access.
      # chars: %{{row, col} => char_string}
      # tags:  %{{row, col} => integer_bitmask}
      buf = %{rows: rows, cols: cols, chars: %{}, tags: %{}}

      full_clip = %{
        min_col: 0,
        min_row: 0,
        max_col: cols,
        max_row: rows
      }

      buf =
        Enum.reduce(scene.instructions, buf, fn inst, acc ->
          render_instruction(inst, acc, sx, sy, full_clip)
        end)

      buffer_to_string(buf)
    end
  end

  # ---------------------------------------------------------------------------
  # Buffer operations
  #
  # The buffer is a map with :rows, :cols, :chars, and :tags keys.
  # :chars maps {row, col} to a single-character string.
  # :tags maps {row, col} to an integer bitmask of direction flags.
  #
  # This approach avoids mutable state: each write_tag or write_char
  # returns a new buffer map with the updated cell.
  # ---------------------------------------------------------------------------

  defp write_tag(buf, row, col, dir_flags, clip) do
    cond do
      row < clip.min_row or row >= clip.max_row -> buf
      col < clip.min_col or col >= clip.max_col -> buf
      row < 0 or row >= buf.rows -> buf
      col < 0 or col >= buf.cols -> buf
      true ->
        existing = Map.get(buf.tags, {row, col}, 0)

        # Don't overwrite text with box-drawing
        if (existing &&& @text) != 0 do
          buf
        else
          merged = existing ||| dir_flags
          char = if (dir_flags &&& @fill) != 0, do: "\u2588", else: resolve_box_char(merged)

          buf
          |> buf_put([:tags, {row, col}], merged)
          |> buf_put([:chars, {row, col}], char)
        end
    end
  end

  defp write_char(buf, row, col, ch, clip) do
    cond do
      row < clip.min_row or row >= clip.max_row -> buf
      col < clip.min_col or col >= clip.max_col -> buf
      row < 0 or row >= buf.rows -> buf
      col < 0 or col >= buf.cols -> buf
      true ->
        buf
        |> buf_put([:chars, {row, col}], ch)
        |> buf_put([:tags, {row, col}], @text)
    end
  end

  # Helper to put a value into a nested map path like [:chars, {0, 1}]
  # Helper to set a value in a nested map: buf[key1][key2] = value
  defp buf_put(buf, [key1, key2], value) do
    inner = Map.get(buf, key1, %{})
    Map.put(buf, key1, Map.put(inner, key2, value))
  end

  @doc false
  defp resolve_box_char(tag) do
    cond do
      (tag &&& @fill) != 0 -> "\u2588"
      (tag &&& @text) != 0 -> ""
      true ->
        direction_bits = tag &&& (@up ||| @down ||| @left ||| @right)
        Map.get(@box_chars, direction_bits, "+")
    end
  end

  # Convert the buffer to a string: join each row's characters, trim trailing
  # whitespace per line, then join rows with newlines and trim trailing blank
  # lines.
  defp buffer_to_string(buf) do
    0..(buf.rows - 1)
    |> Enum.map(fn row ->
      0..(buf.cols - 1)
      |> Enum.map(fn col ->
        Map.get(buf.chars, {row, col}, " ")
      end)
      |> Enum.join("")
      |> String.trim_trailing()
    end)
    |> Enum.join("\n")
    |> String.trim_trailing()
  end

  # ---------------------------------------------------------------------------
  # Coordinate mapping
  #
  # Scene coordinates are in pixels; the buffer is in character cells.
  # We round to the nearest cell to handle non-integer boundaries.
  # ---------------------------------------------------------------------------

  defp to_col(x, sx), do: round(x / sx)
  defp to_row(y, sy), do: round(y / sy)

  # ---------------------------------------------------------------------------
  # Instruction renderers
  # ---------------------------------------------------------------------------

  defp render_instruction(%{kind: :rect} = inst, buf, sx, sy, clip) do
    render_rect(inst, buf, sx, sy, clip)
  end

  defp render_instruction(%{kind: :line} = inst, buf, sx, sy, clip) do
    render_line(inst, buf, sx, sy, clip)
  end

  defp render_instruction(%{kind: :text} = inst, buf, sx, sy, clip) do
    render_text_inst(inst, buf, sx, sy, clip)
  end

  defp render_instruction(%{kind: :group} = inst, buf, sx, sy, clip) do
    render_group(inst, buf, sx, sy, clip)
  end

  defp render_instruction(%{kind: :clip} = inst, buf, sx, sy, clip) do
    render_clip(inst, buf, sx, sy, clip)
  end

  # ---------------------------------------------------------------------------
  # Rect rendering
  #
  # Stroked rects produce box-drawing outlines: corners at the four vertices,
  # horizontal edges along the top and bottom, vertical edges along left and
  # right.
  #
  # Filled rects (non-transparent, non-"none" fill with no stroke) produce
  # solid block characters covering the entire rect area.
  #
  # Transparent rects with no stroke produce nothing (they are invisible).
  # ---------------------------------------------------------------------------

  defp render_rect(inst, buf, sx, sy, clip) do
    c1 = to_col(inst.x, sx)
    r1 = to_row(inst.y, sy)
    c2 = to_col(inst.x + inst.width, sx)
    r2 = to_row(inst.y + inst.height, sy)

    has_stroke = inst.stroke != nil and inst.stroke != ""
    has_fill = inst.fill != "" and inst.fill != "transparent" and inst.fill != "none"

    cond do
      has_stroke ->
        # Draw box outline: corners first, then edges
        buf
        |> write_tag(r1, c1, @down ||| @right, clip)
        |> write_tag(r1, c2, @down ||| @left, clip)
        |> write_tag(r2, c1, @up ||| @right, clip)
        |> write_tag(r2, c2, @up ||| @left, clip)
        |> draw_horizontal_edge(r1, c1 + 1, c2, clip)
        |> draw_horizontal_edge(r2, c1 + 1, c2, clip)
        |> draw_vertical_edge(c1, r1 + 1, r2, clip)
        |> draw_vertical_edge(c2, r1 + 1, r2, clip)

      has_fill ->
        # Fill interior with block characters
        Enum.reduce(r1..r2, buf, fn r, acc ->
          Enum.reduce(c1..c2, acc, fn c, acc2 ->
            write_tag(acc2, r, c, @fill, clip)
          end)
        end)

      true ->
        # Transparent rect with no stroke: invisible
        buf
    end
  end

  # Draw a horizontal edge of box-drawing characters from col `from` to
  # col `to` (exclusive) at the given row.
  defp draw_horizontal_edge(buf, _row, from, to, _clip) when from >= to, do: buf

  defp draw_horizontal_edge(buf, row, from, to, clip) do
    Enum.reduce(from..(to - 1), buf, fn c, acc ->
      write_tag(acc, row, c, @left ||| @right, clip)
    end)
  end

  # Draw a vertical edge of box-drawing characters from row `from` to
  # row `to` (exclusive) at the given column.
  defp draw_vertical_edge(buf, _col, from, to, _clip) when from >= to, do: buf

  defp draw_vertical_edge(buf, col, from, to, clip) do
    Enum.reduce(from..(to - 1), buf, fn r, acc ->
      write_tag(acc, r, col, @up ||| @down, clip)
    end)
  end

  # ---------------------------------------------------------------------------
  # Line rendering
  #
  # Lines can be horizontal, vertical, or diagonal.
  #
  # Endpoint-aware direction flags: at endpoints, only the inward direction
  # is set. This way a line endpoint meeting a perpendicular box edge
  # resolves to the correct tee character instead of a cross.
  #
  # For example, a horizontal line's left endpoint gets only the RIGHT flag,
  # so when it merges with a vertical box edge (UP|DOWN), the result is
  # UP|DOWN|RIGHT which resolves to the left-tee character.
  # ---------------------------------------------------------------------------

  defp render_line(inst, buf, sx, sy, clip) do
    c1 = to_col(inst.x1, sx)
    r1 = to_row(inst.y1, sy)
    c2 = to_col(inst.x2, sx)
    r2 = to_row(inst.y2, sy)

    cond do
      # Horizontal line
      r1 == r2 ->
        min_c = min(c1, c2)
        max_c = max(c1, c2)

        Enum.reduce(min_c..max_c, buf, fn c, acc ->
          flags =
            cond do
              # Single-cell line
              min_c == max_c -> @left ||| @right
              # Left endpoint: only points right (inward)
              c == min_c -> @right
              # Right endpoint: only points left (inward)
              c == max_c -> @left
              # Interior: both directions
              true -> @left ||| @right
            end

          write_tag(acc, r1, c, flags, clip)
        end)

      # Vertical line
      c1 == c2 ->
        min_r = min(r1, r2)
        max_r = max(r1, r2)

        Enum.reduce(min_r..max_r, buf, fn r, acc ->
          flags =
            cond do
              # Single-cell line
              min_r == max_r -> @up ||| @down
              # Top endpoint: only points down (inward)
              r == min_r -> @down
              # Bottom endpoint: only points up (inward)
              r == max_r -> @up
              # Interior: both directions
              true -> @up ||| @down
            end

          write_tag(acc, r, c1, flags, clip)
        end)

      # Diagonal line: approximate with Bresenham's algorithm
      true ->
        dr = abs(r2 - r1)
        dc = abs(c2 - c1)
        sr = if r1 < r2, do: 1, else: -1
        sc = if c1 < c2, do: 1, else: -1
        dominant_flags = if dc > dr, do: @left ||| @right, else: @up ||| @down

        bresenham(buf, r1, c1, r2, c2, dc - dr, sr, sc, dr, dc, dominant_flags, clip)
    end
  end

  # Bresenham's line algorithm for diagonal lines.
  # Walks from (r, c) to (r2, c2), writing the dominant direction's
  # box-drawing character at each step.
  defp bresenham(buf, r, c, r2, c2, err, sr, sc, dr, dc, flags, clip) do
    buf = write_tag(buf, r, c, flags, clip)

    if r == r2 and c == c2 do
      buf
    else
      e2 = 2 * err

      {err, c} =
        if e2 > -dr do
          {err - dr, c + sc}
        else
          {err, c}
        end

      {err, r} =
        if e2 < dc do
          {err + dc, r + sr}
        else
          {err, r}
        end

      bresenham(buf, r, c, r2, c2, err, sr, sc, dr, dc, flags, clip)
    end
  end

  # ---------------------------------------------------------------------------
  # Text rendering
  #
  # Text is placed directly into the character buffer, overwriting any
  # existing content. The align field controls where the text anchor is:
  #
  #   "start"  - text starts at the x coordinate
  #   "middle" - text is centered on the x coordinate
  #   "end"    - text ends at the x coordinate
  # ---------------------------------------------------------------------------

  defp render_text_inst(inst, buf, sx, sy, clip) do
    row = to_row(inst.y, sy)
    text = inst.value

    start_col =
      case Map.get(inst, :align, "middle") do
        "middle" -> to_col(inst.x, sx) - div(String.length(text), 2)
        "end" -> to_col(inst.x, sx) - String.length(text)
        _ -> to_col(inst.x, sx)
      end

    text
    |> String.graphemes()
    |> Enum.with_index()
    |> Enum.reduce(buf, fn {ch, i}, acc ->
      write_char(acc, row, start_col + i, ch, clip)
    end)
  end

  # ---------------------------------------------------------------------------
  # Group rendering: recursively render all children
  # ---------------------------------------------------------------------------

  defp render_group(inst, buf, sx, sy, clip) do
    Enum.reduce(inst.children, buf, fn child, acc ->
      render_instruction(child, acc, sx, sy, clip)
    end)
  end

  # ---------------------------------------------------------------------------
  # Clip rendering
  #
  # A clip instruction constrains its children to a rectangular region.
  # We compute the intersection of the new clip bounds with the parent
  # clip bounds, then render children with the tighter clip.
  # ---------------------------------------------------------------------------

  defp render_clip(inst, buf, sx, sy, parent_clip) do
    new_clip = %{
      min_col: max(parent_clip.min_col, to_col(inst.x, sx)),
      min_row: max(parent_clip.min_row, to_row(inst.y, sy)),
      max_col: min(parent_clip.max_col, to_col(inst.x + inst.width, sx)),
      max_row: min(parent_clip.max_row, to_row(inst.y + inst.height, sy))
    }

    Enum.reduce(inst.children, buf, fn child, acc ->
      render_instruction(child, acc, sx, sy, new_clip)
    end)
  end
end
