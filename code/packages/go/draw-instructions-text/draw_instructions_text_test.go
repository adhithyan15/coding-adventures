package drawinstructionstext

import (
	"strings"
	"testing"

	drawinstructions "github.com/adhithyan15/coding-adventures/code/packages/go/draw-instructions"
)

// helper creates a stroked rect instruction (transparent fill + stroke).
func strokedRect(x, y, w, h int) drawinstructions.DrawRectInstruction {
	r := drawinstructions.DrawRect(x, y, w, h, "transparent", nil)
	r.Stroke = "#000"
	r.StrokeWidth = 1
	return r
}

// textInst creates a text instruction with the given alignment.
func textInst(x, y int, value, align string) drawinstructions.DrawTextInstruction {
	t := drawinstructions.DrawText(x, y, value, nil)
	t.Align = align
	return t
}

// opts1x1 returns 1:1 scale options for easy reasoning in tests.
func opts1x1() *TextRendererOptions {
	return &TextRendererOptions{ScaleX: 1, ScaleY: 1}
}

// ---------------------------------------------------------------------------
// Version
// ---------------------------------------------------------------------------

func TestVersion(t *testing.T) {
	if Version == "" {
		t.Fatal("Version should not be empty")
	}
}

// ---------------------------------------------------------------------------
// Stroked rectangles
// ---------------------------------------------------------------------------

func TestStrokedRectDrawsBoxWithCornersAndEdges(t *testing.T) {
	scene := drawinstructions.CreateScene(5, 3, []drawinstructions.DrawInstruction{
		strokedRect(0, 0, 4, 2),
	}, "", nil)
	result := RenderText(scene, opts1x1())

	expected := "\u250C\u2500\u2500\u2500\u2510\n" +
		"\u2502   \u2502\n" +
		"\u2514\u2500\u2500\u2500\u2518"
	if result != expected {
		t.Fatalf("stroked rect:\ngot:\n%s\nexpected:\n%s", result, expected)
	}
}

// ---------------------------------------------------------------------------
// Filled rectangles
// ---------------------------------------------------------------------------

func TestFilledRectUsesBlockCharacters(t *testing.T) {
	scene := drawinstructions.CreateScene(3, 2, []drawinstructions.DrawInstruction{
		drawinstructions.DrawRect(0, 0, 2, 1, "#000", nil),
	}, "", nil)
	result := RenderText(scene, opts1x1())

	if !strings.Contains(result, "\u2588") {
		t.Fatalf("filled rect should contain block character, got:\n%s", result)
	}
}

// ---------------------------------------------------------------------------
// Horizontal lines
// ---------------------------------------------------------------------------

func TestHorizontalLine(t *testing.T) {
	scene := drawinstructions.CreateScene(5, 1, []drawinstructions.DrawInstruction{
		drawinstructions.DrawLine(0, 0, 4, 0, "#000", 1, nil),
	}, "", nil)
	result := RenderText(scene, opts1x1())

	expected := "\u2500\u2500\u2500\u2500\u2500"
	if result != expected {
		t.Fatalf("horizontal line:\ngot:      %q\nexpected: %q", result, expected)
	}
}

// ---------------------------------------------------------------------------
// Vertical lines
// ---------------------------------------------------------------------------

func TestVerticalLine(t *testing.T) {
	scene := drawinstructions.CreateScene(1, 3, []drawinstructions.DrawInstruction{
		drawinstructions.DrawLine(0, 0, 0, 2, "#000", 1, nil),
	}, "", nil)
	result := RenderText(scene, opts1x1())

	expected := "\u2502\n\u2502\n\u2502"
	if result != expected {
		t.Fatalf("vertical line:\ngot:      %q\nexpected: %q", result, expected)
	}
}

// ---------------------------------------------------------------------------
// Line intersections
// ---------------------------------------------------------------------------

func TestCrossingLinesProduceCross(t *testing.T) {
	// Horizontal at y=1, vertical at x=2, crossing at (2,1)
	scene := drawinstructions.CreateScene(5, 3, []drawinstructions.DrawInstruction{
		drawinstructions.DrawLine(0, 1, 4, 1, "#000", 1, nil),
		drawinstructions.DrawLine(2, 0, 2, 2, "#000", 1, nil),
	}, "", nil)
	result := RenderText(scene, opts1x1())

	lines := strings.Split(result, "\n")
	if len(lines) < 3 {
		t.Fatalf("expected 3 lines, got %d:\n%s", len(lines), result)
	}

	// Row 0: vertical at col 2
	if len(lines[0]) < 3 || string([]rune(lines[0])[2]) != "\u2502" {
		t.Fatalf("expected vertical pipe at row 0 col 2, got: %q", lines[0])
	}
	// Row 1: cross at col 2
	if len(lines[1]) < 3 || string([]rune(lines[1])[2]) != "\u253C" {
		t.Fatalf("expected cross at row 1 col 2, got: %q", lines[1])
	}
	// Row 2: vertical at col 2
	if len(lines[2]) < 3 || string([]rune(lines[2])[2]) != "\u2502" {
		t.Fatalf("expected vertical pipe at row 2 col 2, got: %q", lines[2])
	}
}

// ---------------------------------------------------------------------------
// Box with internal lines (table grid)
// ---------------------------------------------------------------------------

func TestTableGrid(t *testing.T) {
	// A 7x3 box with a horizontal divider at y=1
	scene := drawinstructions.CreateScene(7, 3, []drawinstructions.DrawInstruction{
		strokedRect(0, 0, 6, 2),
		drawinstructions.DrawLine(0, 1, 6, 1, "#000", 1, nil),
	}, "", nil)
	result := RenderText(scene, opts1x1())

	lines := strings.Split(result, "\n")
	if len(lines) < 3 {
		t.Fatalf("expected 3 lines, got %d:\n%s", len(lines), result)
	}

	// Top edge
	if lines[0] != "\u250C\u2500\u2500\u2500\u2500\u2500\u2510" {
		t.Fatalf("top edge wrong: %q", lines[0])
	}
	// Middle: left tee, right tee
	runes1 := []rune(lines[1])
	if string(runes1[0]) != "\u251C" {
		t.Fatalf("expected left tee at row 1 col 0, got: %q", string(runes1[0]))
	}
	if string(runes1[6]) != "\u2524" {
		t.Fatalf("expected right tee at row 1 col 6, got: %q", string(runes1[6]))
	}
	// Bottom edge
	if lines[2] != "\u2514\u2500\u2500\u2500\u2500\u2500\u2518" {
		t.Fatalf("bottom edge wrong: %q", lines[2])
	}
}

// ---------------------------------------------------------------------------
// Text rendering
// ---------------------------------------------------------------------------

func TestTextAtPosition(t *testing.T) {
	scene := drawinstructions.CreateScene(10, 1, []drawinstructions.DrawInstruction{
		textInst(0, 0, "Hello", "start"),
	}, "", nil)
	result := RenderText(scene, opts1x1())
	if result != "Hello" {
		t.Fatalf("expected 'Hello', got: %q", result)
	}
}

func TestTextMiddleAlignment(t *testing.T) {
	scene := drawinstructions.CreateScene(10, 1, []drawinstructions.DrawInstruction{
		textInst(5, 0, "Hi", "middle"),
	}, "", nil)
	result := RenderText(scene, opts1x1())
	runes := []rune(result)
	// "Hi" centered at col 5: starts at col 4
	if len(runes) < 6 || string(runes[4]) != "H" || string(runes[5]) != "i" {
		t.Fatalf("expected 'Hi' centered at col 5, got: %q", result)
	}
}

func TestTextEndAlignment(t *testing.T) {
	scene := drawinstructions.CreateScene(10, 1, []drawinstructions.DrawInstruction{
		textInst(9, 0, "End", "end"),
	}, "", nil)
	result := RenderText(scene, opts1x1())
	runes := []rune(result)
	// "End" ending at col 9: starts at col 6
	if len(runes) < 9 || string(runes[6]) != "E" || string(runes[7]) != "n" || string(runes[8]) != "d" {
		t.Fatalf("expected 'End' right-aligned at col 9, got: %q", result)
	}
}

// ---------------------------------------------------------------------------
// Text inside a box
// ---------------------------------------------------------------------------

func TestTextInsideBox(t *testing.T) {
	scene := drawinstructions.CreateScene(12, 3, []drawinstructions.DrawInstruction{
		strokedRect(0, 0, 11, 2),
		textInst(1, 1, "Hello", "start"),
	}, "", nil)
	result := RenderText(scene, opts1x1())

	lines := strings.Split(result, "\n")
	if len(lines) < 3 {
		t.Fatalf("expected 3 lines, got %d:\n%s", len(lines), result)
	}
	if lines[0] != "\u250C\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2510" {
		t.Fatalf("top edge wrong: %q", lines[0])
	}
	if lines[1] != "\u2502Hello     \u2502" {
		t.Fatalf("middle row wrong: %q", lines[1])
	}
	if lines[2] != "\u2514\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2518" {
		t.Fatalf("bottom edge wrong: %q", lines[2])
	}
}

// ---------------------------------------------------------------------------
// Clips
// ---------------------------------------------------------------------------

func TestClipTruncatesText(t *testing.T) {
	clip := drawinstructions.DrawClipRegion(0, 0, 3, 1, []drawinstructions.DrawInstruction{
		textInst(0, 0, "Hello World", "start"),
	}, nil)
	scene := drawinstructions.CreateScene(10, 1, []drawinstructions.DrawInstruction{clip}, "", nil)
	result := RenderText(scene, opts1x1())
	if result != "Hel" {
		t.Fatalf("expected 'Hel', got: %q", result)
	}
}

// ---------------------------------------------------------------------------
// Groups
// ---------------------------------------------------------------------------

func TestGroupRecursesIntoChildren(t *testing.T) {
	group := drawinstructions.DrawGroup([]drawinstructions.DrawInstruction{
		textInst(0, 0, "AB", "start"),
		textInst(3, 0, "CD", "start"),
	}, nil)
	scene := drawinstructions.CreateScene(5, 1, []drawinstructions.DrawInstruction{group}, "", nil)
	result := RenderText(scene, opts1x1())
	if result != "AB CD" {
		t.Fatalf("expected 'AB CD', got: %q", result)
	}
}

// ---------------------------------------------------------------------------
// Table demo
// ---------------------------------------------------------------------------

func TestCompleteTable(t *testing.T) {
	// A small 2-column table: "Name" (6 wide) | "Age" (4 wide)
	scene := drawinstructions.CreateScene(13, 6, []drawinstructions.DrawInstruction{
		// Outer border
		strokedRect(0, 0, 12, 5),
		// Vertical divider at x=6
		drawinstructions.DrawLine(6, 0, 6, 5, "#000", 1, nil),
		// Horizontal divider at y=2 (below headers)
		drawinstructions.DrawLine(0, 2, 12, 2, "#000", 1, nil),
		// Header text
		textInst(1, 1, "Name", "start"),
		textInst(7, 1, "Age", "start"),
		// Data row 1
		textInst(1, 3, "Alice", "start"),
		textInst(7, 3, "30", "start"),
		// Data row 2
		textInst(1, 4, "Bob", "start"),
		textInst(7, 4, "25", "start"),
	}, "", nil)
	result := RenderText(scene, opts1x1())

	lines := strings.Split(result, "\n")
	if len(lines) < 6 {
		t.Fatalf("expected 6 lines, got %d:\n%s", len(lines), result)
	}

	// Top: corner-dash-tee-dash-corner
	if lines[0] != "\u250C\u2500\u2500\u2500\u2500\u2500\u252C\u2500\u2500\u2500\u2500\u2500\u2510" {
		t.Fatalf("top row wrong: %q", lines[0])
	}
	if !strings.Contains(lines[1], "Name") || !strings.Contains(lines[1], "Age") {
		t.Fatalf("header row should contain Name and Age: %q", lines[1])
	}
	// Divider row: left tee, cross, right tee
	runes2 := []rune(lines[2])
	if string(runes2[0]) != "\u251C" {
		t.Fatalf("expected left tee at divider col 0, got: %q", string(runes2[0]))
	}
	if string(runes2[6]) != "\u253C" {
		t.Fatalf("expected cross at divider col 6, got: %q", string(runes2[6]))
	}
	if string(runes2[12]) != "\u2524" {
		t.Fatalf("expected right tee at divider col 12, got: %q", string(runes2[12]))
	}
	// Data rows
	if !strings.Contains(lines[3], "Alice") || !strings.Contains(lines[3], "30") {
		t.Fatalf("data row 1 wrong: %q", lines[3])
	}
	if !strings.Contains(lines[4], "Bob") || !strings.Contains(lines[4], "25") {
		t.Fatalf("data row 2 wrong: %q", lines[4])
	}
	// Bottom: corner-dash-tee-dash-corner
	if lines[5] != "\u2514\u2500\u2500\u2500\u2500\u2500\u2534\u2500\u2500\u2500\u2500\u2500\u2518" {
		t.Fatalf("bottom row wrong: %q", lines[5])
	}
}

// ---------------------------------------------------------------------------
// Scale factor
// ---------------------------------------------------------------------------

func TestDefaultScaleMapsPxToChars(t *testing.T) {
	// Default scale: 8px/col, 16px/row
	// A rect at (0,0) with width=80 height=32 -> 10 cols, 2 rows
	// That's a 3-row box: row 0 (top), row 1 (middle), row 2 (bottom)
	r := drawinstructions.DrawRect(0, 0, 80, 32, "transparent", nil)
	r.Stroke = "#000"
	r.StrokeWidth = 1
	scene := drawinstructions.CreateScene(88, 48, []drawinstructions.DrawInstruction{r}, "", nil)
	result := RenderText(scene, nil) // default scale

	lines := strings.Split(result, "\n")
	if len(lines) != 3 {
		t.Fatalf("expected 3 lines with default scale, got %d:\n%s", len(lines), result)
	}
	runes0 := []rune(lines[0])
	if string(runes0[0]) != "\u250C" {
		t.Fatalf("expected top-left corner, got: %q", string(runes0[0]))
	}
	runes2 := []rune(lines[2])
	if string(runes2[0]) != "\u2514" {
		t.Fatalf("expected bottom-left corner, got: %q", string(runes2[0]))
	}
}

func TestCustomScale(t *testing.T) {
	opts := &TextRendererOptions{ScaleX: 4, ScaleY: 4}
	renderer := NewTextRenderer(opts)
	scene := drawinstructions.CreateScene(12, 8, []drawinstructions.DrawInstruction{
		drawinstructions.DrawLine(0, 0, 12, 0, "#000", 1, nil),
	}, "", nil)
	result := renderer.Render(scene)
	if !strings.Contains(result, "\u2500") {
		t.Fatalf("expected horizontal line char, got: %q", result)
	}
}

// ---------------------------------------------------------------------------
// RenderWith integration
// ---------------------------------------------------------------------------

func TestRenderWithIntegration(t *testing.T) {
	scene := drawinstructions.CreateScene(5, 1, []drawinstructions.DrawInstruction{
		textInst(0, 0, "OK", "start"),
	}, "", nil)
	renderer := NewTextRenderer(opts1x1())
	result := drawinstructions.RenderWith[string](scene, renderer)
	if result != "OK" {
		t.Fatalf("expected 'OK', got: %q", result)
	}
}

// ---------------------------------------------------------------------------
// Empty scene
// ---------------------------------------------------------------------------

func TestEmptyScene(t *testing.T) {
	scene := drawinstructions.CreateScene(0, 0, []drawinstructions.DrawInstruction{}, "", nil)
	result := RenderText(scene, opts1x1())
	if result != "" {
		t.Fatalf("expected empty string for empty scene, got: %q", result)
	}
}

// ---------------------------------------------------------------------------
// Transparent rect is not rendered
// ---------------------------------------------------------------------------

func TestTransparentRectNotRendered(t *testing.T) {
	scene := drawinstructions.CreateScene(5, 3, []drawinstructions.DrawInstruction{
		drawinstructions.DrawRect(0, 0, 4, 2, "transparent", nil),
	}, "", nil)
	result := RenderText(scene, opts1x1())
	if result != "" {
		t.Fatalf("expected empty string for transparent rect, got: %q", result)
	}
}

// ---------------------------------------------------------------------------
// Diagonal line (Bresenham)
// ---------------------------------------------------------------------------

func TestDiagonalLine(t *testing.T) {
	scene := drawinstructions.CreateScene(5, 5, []drawinstructions.DrawInstruction{
		drawinstructions.DrawLine(0, 0, 4, 4, "#000", 1, nil),
	}, "", nil)
	result := RenderText(scene, opts1x1())
	// Should contain some characters along the diagonal
	if result == "" {
		t.Fatal("diagonal line should not produce empty result")
	}
	lines := strings.Split(result, "\n")
	if len(lines) < 3 {
		t.Fatalf("expected multiple lines for diagonal, got %d", len(lines))
	}
}

// ---------------------------------------------------------------------------
// Text does not get overwritten by box-drawing
// ---------------------------------------------------------------------------

func TestTextOverridesBoxDrawing(t *testing.T) {
	// First draw a horizontal line, then put text on top -- text wins
	scene := drawinstructions.CreateScene(10, 1, []drawinstructions.DrawInstruction{
		drawinstructions.DrawLine(0, 0, 9, 0, "#000", 1, nil),
		textInst(2, 0, "AB", "start"),
	}, "", nil)
	result := RenderText(scene, opts1x1())
	runes := []rune(result)
	if string(runes[2]) != "A" || string(runes[3]) != "B" {
		t.Fatalf("text should override box-drawing, got: %q", result)
	}
}

// ---------------------------------------------------------------------------
// Box-drawing does not overwrite text
// ---------------------------------------------------------------------------

func TestBoxDrawingDoesNotOverwriteText(t *testing.T) {
	// First place text, then draw a line through same position -- text stays
	scene := drawinstructions.CreateScene(10, 1, []drawinstructions.DrawInstruction{
		textInst(2, 0, "X", "start"),
		drawinstructions.DrawLine(0, 0, 9, 0, "#000", 1, nil),
	}, "", nil)
	result := RenderText(scene, opts1x1())
	runes := []rune(result)
	if string(runes[2]) != "X" {
		t.Fatalf("text should not be overwritten by box-drawing, got: %q", result)
	}
}

// ---------------------------------------------------------------------------
// Filled rect with "none" fill is not rendered
// ---------------------------------------------------------------------------

func TestNoneFillRectNotRendered(t *testing.T) {
	scene := drawinstructions.CreateScene(5, 3, []drawinstructions.DrawInstruction{
		drawinstructions.DrawRect(0, 0, 4, 2, "none", nil),
	}, "", nil)
	result := RenderText(scene, opts1x1())
	if result != "" {
		t.Fatalf("expected empty string for 'none' fill rect, got: %q", result)
	}
}

// ---------------------------------------------------------------------------
// Clip with nested clip
// ---------------------------------------------------------------------------

func TestNestedClip(t *testing.T) {
	// Outer clip: 5 wide, inner clip: 3 wide. Text should be clipped to 3.
	innerClip := drawinstructions.DrawClipRegion(0, 0, 3, 1, []drawinstructions.DrawInstruction{
		textInst(0, 0, "ABCDEFGH", "start"),
	}, nil)
	outerClip := drawinstructions.DrawClipRegion(0, 0, 5, 1, []drawinstructions.DrawInstruction{
		innerClip,
	}, nil)
	scene := drawinstructions.CreateScene(10, 1, []drawinstructions.DrawInstruction{outerClip}, "", nil)
	result := RenderText(scene, opts1x1())
	if result != "ABC" {
		t.Fatalf("expected 'ABC' from nested clip, got: %q", result)
	}
}

// ---------------------------------------------------------------------------
// Reversed line (x2 < x1)
// ---------------------------------------------------------------------------

func TestReversedHorizontalLine(t *testing.T) {
	scene := drawinstructions.CreateScene(5, 1, []drawinstructions.DrawInstruction{
		drawinstructions.DrawLine(4, 0, 0, 0, "#000", 1, nil),
	}, "", nil)
	result := RenderText(scene, opts1x1())
	if result != "\u2500\u2500\u2500\u2500\u2500" {
		t.Fatalf("reversed horizontal line wrong: %q", result)
	}
}

func TestReversedVerticalLine(t *testing.T) {
	scene := drawinstructions.CreateScene(1, 3, []drawinstructions.DrawInstruction{
		drawinstructions.DrawLine(0, 2, 0, 0, "#000", 1, nil),
	}, "", nil)
	result := RenderText(scene, opts1x1())
	if result != "\u2502\n\u2502\n\u2502" {
		t.Fatalf("reversed vertical line wrong: %q", result)
	}
}

// ---------------------------------------------------------------------------
// Single-cell line
// ---------------------------------------------------------------------------

func TestSingleCellHorizontalLine(t *testing.T) {
	scene := drawinstructions.CreateScene(1, 1, []drawinstructions.DrawInstruction{
		drawinstructions.DrawLine(0, 0, 0, 0, "#000", 1, nil),
	}, "", nil)
	result := RenderText(scene, opts1x1())
	// Single-cell vertical line (y1==y2 but also x1==x2, treated as horizontal
	// then vertical path; the first branch catches it)
	if result == "" {
		t.Fatal("single-cell line should produce some output")
	}
}

// ---------------------------------------------------------------------------
// Steep diagonal (more vertical than horizontal)
// ---------------------------------------------------------------------------

func TestSteepDiagonalLine(t *testing.T) {
	scene := drawinstructions.CreateScene(3, 7, []drawinstructions.DrawInstruction{
		drawinstructions.DrawLine(0, 0, 2, 6, "#000", 1, nil),
	}, "", nil)
	result := RenderText(scene, opts1x1())
	if result == "" {
		t.Fatal("steep diagonal should produce output")
	}
	// Since it's more vertical than horizontal, should use vertical chars
	if !strings.Contains(result, "\u2502") {
		t.Fatalf("steep diagonal should use vertical chars, got:\n%s", result)
	}
}
