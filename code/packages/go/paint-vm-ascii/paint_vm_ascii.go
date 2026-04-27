// Package paintvmascii renders PaintScene values to a terminal-friendly string.
package paintvmascii

import (
	"fmt"
	"math"
	"strings"

	paintinstructions "github.com/adhithyan15/coding-adventures/code/packages/go/paint-instructions"
)

const Version = "0.1.0"

// AsciiOptions controls how scene coordinates map to character cells.
type AsciiOptions struct {
	ScaleX int
	ScaleY int
}

func (o *AsciiOptions) defaults() AsciiOptions {
	if o == nil {
		return AsciiOptions{ScaleX: 8, ScaleY: 16}
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

type charBuffer struct {
	rows, cols int
	chars      [][]string
}

func newCharBuffer(rows, cols int) *charBuffer {
	chars := make([][]string, rows)
	for r := 0; r < rows; r++ {
		chars[r] = make([]string, cols)
		for c := 0; c < cols; c++ {
			chars[r][c] = " "
		}
	}
	return &charBuffer{rows: rows, cols: cols, chars: chars}
}

func (b *charBuffer) writeChar(row, col int, ch string) {
	if row < 0 || row >= b.rows || col < 0 || col >= b.cols {
		return
	}
	b.chars[row][col] = ch
}

func (b *charBuffer) String() string {
	lines := make([]string, b.rows)
	for r := 0; r < b.rows; r++ {
		lines[r] = strings.TrimRight(strings.Join(b.chars[r], ""), " ")
	}
	return strings.TrimRight(strings.Join(lines, "\n"), "\n ")
}

func toCol(x float64, scaleX int) int {
	return int(math.Round(x / float64(scaleX)))
}

func toRow(y float64, scaleY int) int {
	return int(math.Round(y / float64(scaleY)))
}

func renderRect(inst paintinstructions.PaintRectInstruction, buf *charBuffer, sx, sy int) {
	if inst.Fill == "" || inst.Fill == "transparent" || inst.Fill == "none" {
		return
	}
	c1 := toCol(float64(inst.X), sx)
	r1 := toRow(float64(inst.Y), sy)
	c2 := toCol(float64(inst.X+inst.Width), sx)
	r2 := toRow(float64(inst.Y+inst.Height), sy)

	for r := r1; r <= r2; r++ {
		for c := c1; c <= c2; c++ {
			buf.writeChar(r, c, "█")
		}
	}
}

// Render executes a rect-based PaintScene into a terminal string.
func Render(scene paintinstructions.PaintScene, options *AsciiOptions) (string, error) {
	opts := options.defaults()
	cols := int(math.Ceil(float64(scene.Width) / float64(opts.ScaleX)))
	rows := int(math.Ceil(float64(scene.Height) / float64(opts.ScaleY)))
	buf := newCharBuffer(rows, cols)

	for _, instruction := range scene.Instructions {
		switch current := instruction.(type) {
		case paintinstructions.PaintRectInstruction:
			renderRect(current, buf, opts.ScaleX, opts.ScaleY)
		case *paintinstructions.PaintRectInstruction:
			if current != nil {
				renderRect(*current, buf, opts.ScaleX, opts.ScaleY)
			}
		default:
			return "", fmt.Errorf("paint-vm-ascii: unsupported paint instruction kind: %s", instruction.InstructionKind())
		}
	}

	return buf.String(), nil
}
