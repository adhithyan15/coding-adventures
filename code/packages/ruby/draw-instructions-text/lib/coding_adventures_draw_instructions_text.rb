# frozen_string_literal: true

require "coding_adventures_draw_instructions"
require_relative "coding_adventures/draw_instructions_text/version"

module CodingAdventures
  # ASCII/Unicode text renderer for the draw-instructions scene model.
  #
  # This renderer proves the draw-instructions abstraction is truly backend-
  # neutral: the same DrawScene that produces SVG or paints a Canvas can also
  # render as box-drawing characters in a terminal.
  #
  # == How It Works
  #
  # The renderer maps pixel-coordinate scenes to a fixed-width character grid.
  # Each cell is one character. The mapping uses a configurable scale factor
  # (default: 8px per char width, 16px per char height).
  #
  # == Character Palette
  #
  # Box-drawing characters create clean table grids:
  #
  #   Corners: + + + +     Edges: - |
  #   Tees:    T (top/bottom/left/right)
  #   Cross:   +           Fill:  #
  #
  # == Intersection Logic
  #
  # When two drawing operations overlap at the same cell, the renderer
  # merges them into the correct junction character. This is tracked via
  # a "tag" buffer parallel to the character buffer, where each cell
  # records a bitmask of directions (up, down, left, right).
  module DrawInstructionsText
    module_function

    # Direction flag constants for the tag bitmask.
    #
    # Each cell in the tag buffer stores a bitmask of directions. When
    # multiple drawing operations overlap, we OR the flags together and
    # resolve the combined tag to the correct box-drawing character.
    #
    #        UP (1)
    #         |
    # LEFT(8)-+-RIGHT(2)
    #         |
    #       DOWN(4)
    DIR_UP    = 1
    DIR_RIGHT = 2
    DIR_DOWN  = 4
    DIR_LEFT  = 8
    DIR_FILL  = 16
    DIR_TEXT  = 32

    # Box-drawing character lookup table.
    #
    # Given a bitmask of directions (UP | DOWN | LEFT | RIGHT), returns the
    # correct Unicode box-drawing character. Covers all 16 combinations of
    # the 4 direction bits.
    BOX_CHARS = {
      (DIR_LEFT | DIR_RIGHT) => "\u2500",                        # horizontal
      (DIR_UP | DIR_DOWN) => "\u2502",                           # vertical
      (DIR_DOWN | DIR_RIGHT) => "\u250C",                        # top-left corner
      (DIR_DOWN | DIR_LEFT) => "\u2510",                         # top-right corner
      (DIR_UP | DIR_RIGHT) => "\u2514",                          # bottom-left corner
      (DIR_UP | DIR_LEFT) => "\u2518",                           # bottom-right corner
      (DIR_LEFT | DIR_RIGHT | DIR_DOWN) => "\u252C",             # top tee
      (DIR_LEFT | DIR_RIGHT | DIR_UP) => "\u2534",               # bottom tee
      (DIR_UP | DIR_DOWN | DIR_RIGHT) => "\u251C",               # left tee
      (DIR_UP | DIR_DOWN | DIR_LEFT) => "\u2524",                # right tee
      (DIR_UP | DIR_DOWN | DIR_LEFT | DIR_RIGHT) => "\u253C",   # cross
      DIR_RIGHT => "\u2500",                                     # half-lines
      DIR_LEFT => "\u2500",
      DIR_UP => "\u2502",
      DIR_DOWN => "\u2502"
    }.freeze

    # Resolves a direction bitmask to a box-drawing character.
    # Falls back to "+" if the combination is not in our table.
    def resolve_box_char(tag)
      return "\u2588" if (tag & DIR_FILL) != 0
      return "" if (tag & DIR_TEXT) != 0
      BOX_CHARS[tag & (DIR_UP | DIR_DOWN | DIR_LEFT | DIR_RIGHT)] || "+"
    end

    # ------------------------------------------------------------------
    # CharBuffer
    #
    # A 2D character buffer with a parallel tag buffer for intersection
    # logic. The char buffer stores the actual character at each cell.
    # The tag buffer stores a bitmask of directions passing through each
    # cell.
    # ------------------------------------------------------------------

    class CharBuffer
      attr_reader :rows, :cols

      def initialize(rows, cols)
        @rows = rows
        @cols = cols
        @chars = Array.new(rows) { Array.new(cols, " ") }
        @tags = Array.new(rows) { Array.new(cols, 0) }
      end

      # Writes a box-drawing element at (row, col) by adding direction
      # flags. The actual character is resolved from the combined tag.
      def write_tag(row, col, dir_flags, clip)
        return if row < clip[:min_row] || row >= clip[:max_row]
        return if col < clip[:min_col] || col >= clip[:max_col]
        return if row < 0 || row >= @rows || col < 0 || col >= @cols

        existing = @tags[row][col]

        # Don't overwrite text with box-drawing
        return if (existing & DIR_TEXT) != 0

        merged = existing | dir_flags
        @tags[row][col] = merged
        @chars[row][col] = if (dir_flags & DIR_FILL) != 0
          "\u2588"
        else
          DrawInstructionsText.resolve_box_char(merged)
        end
      end

      # Writes a text character directly at (row, col).
      # Text overwrites any existing content.
      def write_char(row, col, ch, clip)
        return if row < clip[:min_row] || row >= clip[:max_row]
        return if col < clip[:min_col] || col >= clip[:max_col]
        return if row < 0 || row >= @rows || col < 0 || col >= @cols

        @chars[row][col] = ch
        @tags[row][col] = DIR_TEXT
      end

      # Joins all rows, trims trailing whitespace per line, and returns
      # the result with trailing blank lines removed.
      def to_s
        @chars.map { |row| row.join.rstrip }.join("\n").rstrip
      end
    end

    # ------------------------------------------------------------------
    # Coordinate mapping
    # ------------------------------------------------------------------

    def to_col(x, scale_x)
      (x.to_f / scale_x).round
    end

    def to_row(y, scale_y)
      (y.to_f / scale_y).round
    end

    # ------------------------------------------------------------------
    # Instruction renderers
    # ------------------------------------------------------------------

    def render_rect(inst, buf, sx, sy, clip)
      c1 = to_col(inst.x, sx)
      r1 = to_row(inst.y, sy)
      c2 = to_col(inst.x + inst.width, sx)
      r2 = to_row(inst.y + inst.height, sy)

      has_stroke = inst.stroke && !inst.stroke.empty?
      has_fill = inst.fill && !inst.fill.empty? &&
        inst.fill != "transparent" && inst.fill != "none"

      if has_stroke
        # Corners
        buf.write_tag(r1, c1, DIR_DOWN | DIR_RIGHT, clip)
        buf.write_tag(r1, c2, DIR_DOWN | DIR_LEFT, clip)
        buf.write_tag(r2, c1, DIR_UP | DIR_RIGHT, clip)
        buf.write_tag(r2, c2, DIR_UP | DIR_LEFT, clip)

        # Top edge
        ((c1 + 1)...c2).each { |c| buf.write_tag(r1, c, DIR_LEFT | DIR_RIGHT, clip) }
        # Bottom edge
        ((c1 + 1)...c2).each { |c| buf.write_tag(r2, c, DIR_LEFT | DIR_RIGHT, clip) }
        # Left edge
        ((r1 + 1)...r2).each { |r| buf.write_tag(r, c1, DIR_UP | DIR_DOWN, clip) }
        # Right edge
        ((r1 + 1)...r2).each { |r| buf.write_tag(r, c2, DIR_UP | DIR_DOWN, clip) }
      elsif has_fill
        (r1..r2).each do |r|
          (c1..c2).each { |c| buf.write_tag(r, c, DIR_FILL, clip) }
        end
      end
    end

    def render_line(inst, buf, sx, sy, clip)
      c1 = to_col(inst.x1, sx)
      r1 = to_row(inst.y1, sy)
      c2 = to_col(inst.x2, sx)
      r2 = to_row(inst.y2, sy)

      if r1 == r2
        # Horizontal line with endpoint-aware direction flags
        min_c = [c1, c2].min
        max_c = [c1, c2].max
        (min_c..max_c).each do |c|
          flags = 0
          flags |= DIR_LEFT if c > min_c
          flags |= DIR_RIGHT if c < max_c
          flags = DIR_LEFT | DIR_RIGHT if min_c == max_c # single-cell
          buf.write_tag(r1, c, flags, clip)
        end
      elsif c1 == c2
        # Vertical line with endpoint-aware direction flags
        min_r = [r1, r2].min
        max_r = [r1, r2].max
        (min_r..max_r).each do |r|
          flags = 0
          flags |= DIR_UP if r > min_r
          flags |= DIR_DOWN if r < max_r
          flags = DIR_UP | DIR_DOWN if min_r == max_r # single-cell
          buf.write_tag(r, c1, flags, clip)
        end
      else
        # Diagonal -- Bresenham's algorithm
        dr = (r2 - r1).abs
        dc = (c2 - c1).abs
        sr = r1 < r2 ? 1 : -1
        sc = c1 < c2 ? 1 : -1
        err = dc - dr
        r = r1
        c = c1

        loop do
          dir = dc > dr ? (DIR_LEFT | DIR_RIGHT) : (DIR_UP | DIR_DOWN)
          buf.write_tag(r, c, dir, clip)
          break if r == r2 && c == c2
          e2 = 2 * err
          if e2 > -dr
            err -= dr
            c += sc
          end
          if e2 < dc
            err += dc
            r += sr
          end
        end
      end
    end

    def render_text_inst(inst, buf, sx, sy, clip)
      row = to_row(inst.y, sy)
      text = inst.value

      start_col = case inst.align
        when "middle"
          to_col(inst.x, sx) - (text.length / 2)
        when "end"
          to_col(inst.x, sx) - text.length
        else # "start"
          to_col(inst.x, sx)
        end

      text.chars.each_with_index do |ch, i|
        buf.write_char(row, start_col + i, ch, clip)
      end
    end

    def render_group(inst, buf, sx, sy, clip)
      inst.children.each { |child| render_instruction(child, buf, sx, sy, clip) }
    end

    def render_clip(inst, buf, sx, sy, parent_clip)
      new_clip = {
        min_col: [parent_clip[:min_col], to_col(inst.x, sx)].max,
        min_row: [parent_clip[:min_row], to_row(inst.y, sy)].max,
        max_col: [parent_clip[:max_col], to_col(inst.x + inst.width, sx)].min,
        max_row: [parent_clip[:max_row], to_row(inst.y + inst.height, sy)].min
      }

      inst.children.each { |child| render_instruction(child, buf, sx, sy, new_clip) }
    end

    def render_instruction(inst, buf, sx, sy, clip)
      case inst.kind
      when "rect" then render_rect(inst, buf, sx, sy, clip)
      when "line" then render_line(inst, buf, sx, sy, clip)
      when "text" then render_text_inst(inst, buf, sx, sy, clip)
      when "group" then render_group(inst, buf, sx, sy, clip)
      when "clip" then render_clip(inst, buf, sx, sy, clip)
      end
    end

    # ------------------------------------------------------------------
    # TextRenderer class (duck-typed renderer)
    #
    # Responds to +render(scene)+ so it can be used with
    # DrawInstructions.render_with.
    # ------------------------------------------------------------------

    class TextRenderer
      attr_reader :scale_x, :scale_y

      def initialize(scale_x: 8, scale_y: 16)
        @scale_x = scale_x
        @scale_y = scale_y
      end

      def render(scene)
        DrawInstructionsText.render_text(scene, scale_x: @scale_x, scale_y: @scale_y)
      end
    end

    # ------------------------------------------------------------------
    # Public API
    # ------------------------------------------------------------------

    # Render a DrawScene to a box-drawing character string.
    #
    #   text = DrawInstructionsText.render_text(scene)
    #   text = DrawInstructionsText.render_text(scene, scale_x: 4, scale_y: 4)
    #
    def render_text(scene, scale_x: 8, scale_y: 16)
      sx = scale_x
      sy = scale_y

      cols = (scene.width.to_f / sx).ceil
      rows = (scene.height.to_f / sy).ceil

      buf = CharBuffer.new(rows, cols)

      full_clip = {
        min_col: 0,
        min_row: 0,
        max_col: cols,
        max_row: rows
      }

      scene.instructions.each do |inst|
        render_instruction(inst, buf, sx, sy, full_clip)
      end

      buf.to_s
    end
  end
end
