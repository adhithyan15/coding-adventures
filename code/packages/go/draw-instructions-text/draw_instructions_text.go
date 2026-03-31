// Package drawinstructionstext renders draw-instructions scenes as
// ASCII/Unicode box-drawing character strings.
//
// This renderer proves the draw-instructions abstraction is truly backend-
// neutral: the same DrawScene that produces SVG can also render as box-drawing
// characters in a terminal.
//
// === How It Works ===
//
// The renderer maps pixel-coordinate scenes to a fixed-width character grid.
// Each cell in the grid is one character. The mapping uses a configurable
// scale factor (default: 8px per char column, 16px per char row).
//
//	Scene coordinates (pixels)     Character grid
//	+---------------------+        +----------+
//	| rect at (0,0,80,32) |   ->   |##########|
//	|                     |        |##########|
//	+---------------------+        +----------+
//
// === Character Palette ===
//
// Box-drawing characters create clean table grids:
//
//	+------+-----+     Corners: + + + +
//	| Name | Age |     Edges:   - |
//	+------+-----+     Tees:    T (top) (bottom) (left) (right)
//	| Alice|  30 |     Cross:   +
//	+------+-----+     Fill:    #
//
// === Intersection Logic ===
//
// When two drawing operations overlap at the same cell, the renderer
// merges them into the correct junction character. A horizontal line
// crossing a vertical line becomes a cross. A line meeting a box corner
// becomes the appropriate tee character.
//
// This is tracked via a "tag" buffer parallel to the character buffer.
// Each cell records which directions have lines passing through it
// (up, down, left, right), and the tag is resolved to the correct
// box-drawing character on each write.
//
// === Usage ===
//
//	scene := drawinstructions.CreateScene(160, 48, []drawinstructions.DrawInstruction{
//	    rect, line, text,
//	}, "", nil)
//	result := drawinstructionstext.RenderText(scene, nil)
//	fmt.Println(result)
package drawinstructionstext

import (
	"math"
	"strings"

	drawinstructions "github.com/adhithyan15/coding-adventures/code/packages/go/draw-instructions"
)

const Version = "0.1.0"

// ---------------------------------------------------------------------------
// Options
// ---------------------------------------------------------------------------

// TextRendererOptions controls how pixel coordinates map to character cells.
type TextRendererOptions struct {
	// ScaleX is the number of pixels per character column. Default: 8.
	ScaleX int
	// ScaleY is the number of pixels per character row. Default: 16.
	ScaleY int
}

// defaults returns the options with zero values replaced by defaults.
func (o *TextRendererOptions) defaults() TextRendererOptions {
	if o == nil {
		return TextRendererOptions{ScaleX: 8, ScaleY: 16}
	}
	out := *o
	if out.ScaleX == 0 {
		out.ScaleX = 8
	}
	if out.ScaleY == 0 {
		out.ScaleY = 16
	}
	return out
}

// ---------------------------------------------------------------------------
// Direction flags
//
// Each cell in the tag buffer stores a bitmask of directions. When
// multiple drawing operations overlap, we OR the flags together and
// resolve the combined tag to the correct box-drawing character.
//
//	      UP (1)
//	       |
//	LEFT(8)-+-RIGHT(2)
//	       |
//	     DOWN(4)
// ---------------------------------------------------------------------------

const (
	dirUp    = 1
	dirRight = 2
	dirDown  = 4
	dirLeft  = 8
	dirFill  = 16
	dirText  = 32
)

// clipBounds represents the visible region in character coordinates.
type clipBounds struct {
	minCol, minRow, maxCol, maxRow int
}

// ---------------------------------------------------------------------------
// Box-drawing character resolution
//
// Given a bitmask of directions (UP | DOWN | LEFT | RIGHT), return the
// correct Unicode box-drawing character. This table covers all 16
// combinations of the 4 direction bits.
// ---------------------------------------------------------------------------

var boxChars = map[int]string{
	dirLeft | dirRight:                         "\u2500", // horizontal  -
	dirUp | dirDown:                            "\u2502", // vertical    |
	dirDown | dirRight:                         "\u250C", // top-left    +
	dirDown | dirLeft:                          "\u2510", // top-right   +
	dirUp | dirRight:                           "\u2514", // bottom-left +
	dirUp | dirLeft:                            "\u2518", // bottom-right+
	dirLeft | dirRight | dirDown:               "\u252C", // top tee     T
	dirLeft | dirRight | dirUp:                 "\u2534", // bottom tee  _|_
	dirUp | dirDown | dirRight:                 "\u251C", // left tee    |-
	dirUp | dirDown | dirLeft:                  "\u2524", // right tee   -|
	dirUp | dirDown | dirLeft | dirRight:       "\u253C", // cross       +
	dirRight:                                   "\u2500", // half-lines default to full
	dirLeft:                                    "\u2500",
	dirUp:                                      "\u2502",
	dirDown:                                    "\u2502",
}

// resolveBoxChar maps a direction bitmask to a box-drawing character.
// Falls back to "+" if the combination is not in our table (should not
// happen in practice).
func resolveBoxChar(tag int) string {
	if tag&dirFill != 0 {
		return "\u2588" // full block
	}
	if tag&dirText != 0 {
		return "" // text chars are stored directly, not via tags
	}
	ch, ok := boxChars[tag&(dirUp|dirDown|dirLeft|dirRight)]
	if ok {
		return ch
	}
	return "+"
}

// ---------------------------------------------------------------------------
// charBuffer
//
// A 2D character buffer with a parallel tag buffer for intersection logic.
// The char buffer stores the actual character at each cell. The tag buffer
// stores a bitmask of directions passing through each cell. When writing
// a box-drawing character, we update the tag buffer and resolve the correct
// character from the combined tag.
// ---------------------------------------------------------------------------

type charBuffer struct {
	rows, cols int
	chars      [][]string
	tags       [][]int
}

func newCharBuffer(rows, cols int) *charBuffer {
	chars := make([][]string, rows)
	tags := make([][]int, rows)
	for r := 0; r < rows; r++ {
		chars[r] = make([]string, cols)
		tags[r] = make([]int, cols)
		for c := 0; c < cols; c++ {
			chars[r][c] = " "
		}
	}
	return &charBuffer{rows: rows, cols: cols, chars: chars, tags: tags}
}

// writeTag adds direction flags at (row, col) and resolves the character.
// It respects clip bounds and does not overwrite text cells.
func (b *charBuffer) writeTag(row, col, dirFlags int, clip clipBounds) {
	if row < clip.minRow || row >= clip.maxRow {
		return
	}
	if col < clip.minCol || col >= clip.maxCol {
		return
	}
	if row < 0 || row >= b.rows || col < 0 || col >= b.cols {
		return
	}

	existing := b.tags[row][col]

	// Don't overwrite text with box-drawing
	if existing&dirText != 0 {
		return
	}

	merged := existing | dirFlags
	b.tags[row][col] = merged
	if dirFlags&dirFill != 0 {
		b.chars[row][col] = "\u2588"
	} else {
		b.chars[row][col] = resolveBoxChar(merged)
	}
}

// writeChar places a text character directly at (row, col).
// Text overwrites any existing content.
func (b *charBuffer) writeChar(row, col int, ch string, clip clipBounds) {
	if row < clip.minRow || row >= clip.maxRow {
		return
	}
	if col < clip.minCol || col >= clip.maxCol {
		return
	}
	if row < 0 || row >= b.rows || col < 0 || col >= b.cols {
		return
	}

	b.chars[row][col] = ch
	b.tags[row][col] = dirText
}

// String joins all rows, trims trailing whitespace per line, and
// trims trailing blank lines from the result.
func (b *charBuffer) String() string {
	lines := make([]string, b.rows)
	for r := 0; r < b.rows; r++ {
		lines[r] = strings.TrimRight(strings.Join(b.chars[r], ""), " ")
	}
	return strings.TrimRight(strings.Join(lines, "\n"), "\n ")
}

// ---------------------------------------------------------------------------
// Coordinate mapping
// ---------------------------------------------------------------------------

func toCol(x float64, scaleX int) int {
	return int(math.Round(x / float64(scaleX)))
}

func toRow(y float64, scaleY int) int {
	return int(math.Round(y / float64(scaleY)))
}

// ---------------------------------------------------------------------------
// Instruction renderers
// ---------------------------------------------------------------------------

func renderRect(inst drawinstructions.DrawRectInstruction, buf *charBuffer, sx, sy int, clip clipBounds) {
	c1 := toCol(float64(inst.X), sx)
	r1 := toRow(float64(inst.Y), sy)
	c2 := toCol(float64(inst.X+inst.Width), sx)
	r2 := toRow(float64(inst.Y+inst.Height), sy)

	hasStroke := inst.Stroke != ""
	hasFill := inst.Fill != "" && inst.Fill != "transparent" && inst.Fill != "none"

	if hasStroke {
		// Draw the box outline

		// Corners
		buf.writeTag(r1, c1, dirDown|dirRight, clip)
		buf.writeTag(r1, c2, dirDown|dirLeft, clip)
		buf.writeTag(r2, c1, dirUp|dirRight, clip)
		buf.writeTag(r2, c2, dirUp|dirLeft, clip)

		// Top edge
		for c := c1 + 1; c < c2; c++ {
			buf.writeTag(r1, c, dirLeft|dirRight, clip)
		}
		// Bottom edge
		for c := c1 + 1; c < c2; c++ {
			buf.writeTag(r2, c, dirLeft|dirRight, clip)
		}
		// Left edge
		for r := r1 + 1; r < r2; r++ {
			buf.writeTag(r, c1, dirUp|dirDown, clip)
		}
		// Right edge
		for r := r1 + 1; r < r2; r++ {
			buf.writeTag(r, c2, dirUp|dirDown, clip)
		}
	} else if hasFill {
		// Fill the interior with block characters
		for r := r1; r <= r2; r++ {
			for c := c1; c <= c2; c++ {
				buf.writeTag(r, c, dirFill, clip)
			}
		}
	}
}

func renderLine(inst drawinstructions.DrawLineInstruction, buf *charBuffer, sx, sy int, clip clipBounds) {
	c1 := toCol(inst.X1, sx)
	r1 := toRow(inst.Y1, sy)
	c2 := toCol(inst.X2, sx)
	r2 := toRow(inst.Y2, sy)

	if r1 == r2 {
		// Horizontal line
		// At endpoints, only set the direction pointing inward so that
		// junctions with perpendicular elements resolve correctly.
		minC := c1
		maxC := c2
		if c2 < c1 {
			minC, maxC = c2, c1
		}
		for c := minC; c <= maxC; c++ {
			flags := 0
			if c > minC {
				flags |= dirLeft
			}
			if c < maxC {
				flags |= dirRight
			}
			if c == minC && c == maxC {
				flags = dirLeft | dirRight // single-cell line
			}
			buf.writeTag(r1, c, flags, clip)
		}
	} else if c1 == c2 {
		// Vertical line -- same endpoint logic
		minR := r1
		maxR := r2
		if r2 < r1 {
			minR, maxR = r2, r1
		}
		for r := minR; r <= maxR; r++ {
			flags := 0
			if r > minR {
				flags |= dirUp
			}
			if r < maxR {
				flags |= dirDown
			}
			if r == minR && r == maxR {
				flags = dirUp | dirDown // single-cell line
			}
			buf.writeTag(r, c1, flags, clip)
		}
	} else {
		// Diagonal -- approximate with Bresenham's algorithm
		dr := int(math.Abs(float64(r2 - r1)))
		dc := int(math.Abs(float64(c2 - c1)))
		sr := 1
		if r1 >= r2 {
			sr = -1
		}
		sc := 1
		if c1 >= c2 {
			sc = -1
		}
		err := dc - dr
		r := r1
		c := c1

		for {
			// Use the dominant direction's character
			if dc > dr {
				buf.writeTag(r, c, dirLeft|dirRight, clip)
			} else {
				buf.writeTag(r, c, dirUp|dirDown, clip)
			}
			if r == r2 && c == c2 {
				break
			}
			e2 := 2 * err
			if e2 > -dr {
				err -= dr
				c += sc
			}
			if e2 < dc {
				err += dc
				r += sr
			}
		}
	}
}

func renderTextInst(inst drawinstructions.DrawTextInstruction, buf *charBuffer, sx, sy int, clip clipBounds) {
	row := toRow(float64(inst.Y), sy)
	text := inst.Value

	var startCol int
	switch inst.Align {
	case "middle":
		startCol = toCol(float64(inst.X), sx) - len(text)/2
	case "end":
		startCol = toCol(float64(inst.X), sx) - len(text)
	default: // "start"
		startCol = toCol(float64(inst.X), sx)
	}

	for i, ch := range text {
		buf.writeChar(row, startCol+i, string(ch), clip)
	}
}

func renderGroup(inst drawinstructions.DrawGroupInstruction, buf *charBuffer, sx, sy int, clip clipBounds) {
	for _, child := range inst.Children {
		renderInstruction(child, buf, sx, sy, clip)
	}
}

func renderClip(inst drawinstructions.DrawClipInstruction, buf *charBuffer, sx, sy int, parentClip clipBounds) {
	// Intersect the new clip with the parent clip
	newClip := clipBounds{
		minCol: max(parentClip.minCol, toCol(inst.X, sx)),
		minRow: max(parentClip.minRow, toRow(inst.Y, sy)),
		maxCol: min(parentClip.maxCol, toCol(inst.X+inst.Width, sx)),
		maxRow: min(parentClip.maxRow, toRow(inst.Y+inst.Height, sy)),
	}

	for _, child := range inst.Children {
		renderInstruction(child, buf, sx, sy, newClip)
	}
}

func renderInstruction(inst drawinstructions.DrawInstruction, buf *charBuffer, sx, sy int, clip clipBounds) {
	switch v := inst.(type) {
	case drawinstructions.DrawRectInstruction:
		renderRect(v, buf, sx, sy, clip)
	case drawinstructions.DrawTextInstruction:
		renderTextInst(v, buf, sx, sy, clip)
	case drawinstructions.DrawGroupInstruction:
		renderGroup(v, buf, sx, sy, clip)
	case drawinstructions.DrawLineInstruction:
		renderLine(v, buf, sx, sy, clip)
	case drawinstructions.DrawClipInstruction:
		renderClip(v, buf, sx, sy, clip)
	}
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

// TextRenderer implements drawinstructions.Renderer[string] and converts
// a DrawScene into a box-drawing character string.
type TextRenderer struct {
	opts TextRendererOptions
}

// NewTextRenderer creates a renderer with the given scale options.
// Pass nil for default options (8px per column, 16px per row).
func NewTextRenderer(opts *TextRendererOptions) *TextRenderer {
	resolved := opts.defaults()
	return &TextRenderer{opts: resolved}
}

// Render converts a DrawScene to a string of box-drawing characters.
func (tr *TextRenderer) Render(scene drawinstructions.DrawScene) string {
	sx := tr.opts.ScaleX
	sy := tr.opts.ScaleY

	cols := int(math.Ceil(float64(scene.Width) / float64(sx)))
	rows := int(math.Ceil(float64(scene.Height) / float64(sy)))

	buf := newCharBuffer(rows, cols)

	fullClip := clipBounds{
		minCol: 0,
		minRow: 0,
		maxCol: cols,
		maxRow: rows,
	}

	for _, inst := range scene.Instructions {
		renderInstruction(inst, buf, sx, sy, fullClip)
	}

	return buf.String()
}

// DefaultTextRenderer uses standard scale (8px/col, 16px/row).
var DefaultTextRenderer = NewTextRenderer(nil)

// RenderText is a convenience function: scene in, text string out.
// Pass nil for opts to use defaults.
func RenderText(scene drawinstructions.DrawScene, opts *TextRendererOptions) string {
	if opts == nil {
		return DefaultTextRenderer.Render(scene)
	}
	return NewTextRenderer(opts).Render(scene)
}
