package display

import (
	"strings"
	"testing"
)

// ============================================================
// Test helpers
// ============================================================

// newTestDriver creates a display driver with the given config and a
// fresh memory buffer. Uses the compact 40x10 config by default for
// faster tests.
func newTestDriver() *DisplayDriver {
	config := Compact40x10
	mem := make([]byte, config.Columns*config.Rows*BytesPerCell)
	return NewDisplayDriver(config, mem)
}

// newStandardDriver creates a driver with the full 80x25 VGA config.
func newStandardDriver() *DisplayDriver {
	config := DefaultDisplayConfig()
	mem := make([]byte, config.Columns*config.Rows*BytesPerCell)
	return NewDisplayDriver(config, mem)
}

// ============================================================
// Framebuffer / Config tests
// ============================================================

func TestDefaultDisplayConfig(t *testing.T) {
	config := DefaultDisplayConfig()
	if config.Columns != 80 {
		t.Errorf("expected 80 columns, got %d", config.Columns)
	}
	if config.Rows != 25 {
		t.Errorf("expected 25 rows, got %d", config.Rows)
	}
	if config.FramebufferBase != 0xFFFB0000 {
		t.Errorf("expected base 0xFFFB0000, got 0x%X", config.FramebufferBase)
	}
	if config.DefaultAttribute != 0x07 {
		t.Errorf("expected attribute 0x07, got 0x%02X", config.DefaultAttribute)
	}
}

func TestMakeAttribute(t *testing.T) {
	// White on blue = 0x1F
	attr := MakeAttribute(ColorWhite, ColorBlue)
	if attr != 0x1F {
		t.Errorf("MakeAttribute(White, Blue) = 0x%02X, want 0x1F", attr)
	}

	// Light gray on black = 0x07 (default terminal)
	attr = MakeAttribute(ColorLightGray, ColorBlack)
	if attr != 0x07 {
		t.Errorf("MakeAttribute(LightGray, Black) = 0x%02X, want 0x07", attr)
	}

	// White on red = 0x4F (error highlight)
	attr = MakeAttribute(ColorWhite, ColorRed)
	if attr != 0x4F {
		t.Errorf("MakeAttribute(White, Red) = 0x%02X, want 0x4F", attr)
	}

	// Light green on black = 0x0A (Matrix style)
	attr = MakeAttribute(ColorLightGreen, ColorBlack)
	if attr != 0x0A {
		t.Errorf("MakeAttribute(LightGreen, Black) = 0x%02X, want 0x0A", attr)
	}
}

// ============================================================
// Constructor tests
// ============================================================

func TestNewDisplayDriverClearsScreen(t *testing.T) {
	d := newTestDriver()

	// Every cell should be space + default attribute after construction.
	for row := 0; row < d.Config.Rows; row++ {
		for col := 0; col < d.Config.Columns; col++ {
			cell := d.GetCell(row, col)
			if cell.Character != ' ' {
				t.Errorf("cell(%d,%d) char = 0x%02X, want 0x20 (space)", row, col, cell.Character)
			}
			if cell.Attribute != DefaultAttribute {
				t.Errorf("cell(%d,%d) attr = 0x%02X, want 0x07", row, col, cell.Attribute)
			}
		}
	}

	// Cursor should be at origin.
	pos := d.GetCursor()
	if pos.Row != 0 || pos.Col != 0 {
		t.Errorf("cursor = (%d,%d), want (0,0)", pos.Row, pos.Col)
	}
}

// ============================================================
// PutChar tests
// ============================================================

func TestPutCharBasic(t *testing.T) {
	d := newTestDriver()
	d.PutChar('A')

	cell := d.GetCell(0, 0)
	if cell.Character != 'A' {
		t.Errorf("cell(0,0) = %q, want 'A'", cell.Character)
	}
	if cell.Attribute != DefaultAttribute {
		t.Errorf("attribute = 0x%02X, want 0x07", cell.Attribute)
	}
}

func TestPutCharCursorAdvance(t *testing.T) {
	d := newTestDriver()
	d.PutChar('A')

	pos := d.GetCursor()
	if pos.Row != 0 || pos.Col != 1 {
		t.Errorf("cursor = (%d,%d), want (0,1)", pos.Row, pos.Col)
	}
}

func TestPutCharMultiple(t *testing.T) {
	d := newTestDriver()
	d.PutChar('H')
	d.PutChar('i')

	if c := d.GetCell(0, 0); c.Character != 'H' {
		t.Errorf("cell(0,0) = %q, want 'H'", c.Character)
	}
	if c := d.GetCell(0, 1); c.Character != 'i' {
		t.Errorf("cell(0,1) = %q, want 'i'", c.Character)
	}
	pos := d.GetCursor()
	if pos.Row != 0 || pos.Col != 2 {
		t.Errorf("cursor = (%d,%d), want (0,2)", pos.Row, pos.Col)
	}
}

func TestPutCharNewline(t *testing.T) {
	d := newTestDriver()
	d.PutChar('A')
	d.PutChar('\n')

	pos := d.GetCursor()
	if pos.Row != 1 || pos.Col != 0 {
		t.Errorf("cursor = (%d,%d), want (1,0)", pos.Row, pos.Col)
	}
}

func TestPutCharCarriageReturn(t *testing.T) {
	d := newTestDriver()
	// Write to col 5, then CR
	for i := 0; i < 5; i++ {
		d.PutChar('x')
	}
	d.PutChar('\r')

	pos := d.GetCursor()
	if pos.Row != 0 || pos.Col != 0 {
		t.Errorf("cursor = (%d,%d), want (0,0)", pos.Row, pos.Col)
	}
}

func TestPutCharTab(t *testing.T) {
	d := newTestDriver()
	d.PutChar('\t')

	pos := d.GetCursor()
	if pos.Row != 0 || pos.Col != 8 {
		t.Errorf("cursor after tab from 0 = (%d,%d), want (0,8)", pos.Row, pos.Col)
	}

	// Tab from col 1 should advance to col 8
	d.Clear()
	d.PutChar('x')
	d.PutChar('\t')
	pos = d.GetCursor()
	if pos.Col != 8 {
		t.Errorf("cursor col after tab from 1 = %d, want 8", pos.Col)
	}
}

func TestPutCharBackspace(t *testing.T) {
	d := newTestDriver()
	d.PutChar('A')
	d.PutChar('B')
	d.PutChar('\b')

	pos := d.GetCursor()
	if pos.Row != 0 || pos.Col != 1 {
		t.Errorf("cursor = (%d,%d), want (0,1)", pos.Row, pos.Col)
	}

	// Backspace at col 0 should not go negative.
	d.Clear()
	d.PutChar('\b')
	pos = d.GetCursor()
	if pos.Col != 0 {
		t.Errorf("backspace at col 0: cursor col = %d, want 0", pos.Col)
	}
}

// ============================================================
// PutCharAt tests
// ============================================================

func TestPutCharAtBasic(t *testing.T) {
	d := newTestDriver()
	d.PutCharAt(5, 10, 'X', 0x0F)

	cell := d.GetCell(5, 10)
	if cell.Character != 'X' {
		t.Errorf("cell(5,10) = %q, want 'X'", cell.Character)
	}
	if cell.Attribute != 0x0F {
		t.Errorf("attribute = 0x%02X, want 0x0F", cell.Attribute)
	}
}

func TestPutCharAtDoesNotMoveCursor(t *testing.T) {
	d := newTestDriver()
	d.SetCursor(0, 0)
	d.PutCharAt(5, 10, 'X', 0x07)

	pos := d.GetCursor()
	if pos.Row != 0 || pos.Col != 0 {
		t.Errorf("cursor = (%d,%d), want (0,0)", pos.Row, pos.Col)
	}
}

func TestPutCharAtOutOfBounds(t *testing.T) {
	d := newTestDriver()
	// Should not panic.
	d.PutCharAt(30, 0, 'X', 0x07)
	d.PutCharAt(-1, 0, 'X', 0x07)
	d.PutCharAt(0, -1, 'X', 0x07)
	d.PutCharAt(0, 100, 'X', 0x07)
}

// ============================================================
// Puts tests
// ============================================================

func TestPutsSimple(t *testing.T) {
	d := newTestDriver()
	d.Puts("Hello")

	expected := "Hello"
	for i, ch := range expected {
		cell := d.GetCell(0, i)
		if cell.Character != byte(ch) {
			t.Errorf("cell(0,%d) = %q, want %q", i, cell.Character, ch)
		}
	}

	pos := d.GetCursor()
	if pos.Col != 5 {
		t.Errorf("cursor col = %d, want 5", pos.Col)
	}
}

func TestPutsWithNewline(t *testing.T) {
	d := newTestDriver()
	d.Puts("Hi\nBye")

	snap := d.Snapshot()
	if snap.Lines[0] != "Hi" {
		t.Errorf("line 0 = %q, want %q", snap.Lines[0], "Hi")
	}
	if snap.Lines[1] != "Bye" {
		t.Errorf("line 1 = %q, want %q", snap.Lines[1], "Bye")
	}
}

func TestPutsEmpty(t *testing.T) {
	d := newTestDriver()
	d.Puts("")

	pos := d.GetCursor()
	if pos.Row != 0 || pos.Col != 0 {
		t.Errorf("cursor = (%d,%d), want (0,0)", pos.Row, pos.Col)
	}
}

// ============================================================
// Line wrap tests
// ============================================================

func TestLineWrapAtEndOfRow(t *testing.T) {
	d := newTestDriver()
	// Write exactly 40 characters (compact config has 40 columns)
	for i := 0; i < d.Config.Columns; i++ {
		d.PutChar('A')
	}

	pos := d.GetCursor()
	if pos.Row != 1 || pos.Col != 0 {
		t.Errorf("cursor after %d chars = (%d,%d), want (1,0)", d.Config.Columns, pos.Row, pos.Col)
	}

	// Write one more, should appear at (1, 0)
	d.PutChar('B')
	cell := d.GetCell(1, 0)
	if cell.Character != 'B' {
		t.Errorf("cell(1,0) = %q, want 'B'", cell.Character)
	}
}

func TestMultiLineWrap(t *testing.T) {
	d := newTestDriver()
	cols := d.Config.Columns
	// Write 2 full rows + 1 character
	total := cols*2 + 1
	for i := 0; i < total; i++ {
		d.PutChar('x')
	}

	pos := d.GetCursor()
	if pos.Row != 2 || pos.Col != 1 {
		t.Errorf("cursor = (%d,%d), want (2,1)", pos.Row, pos.Col)
	}
}

// ============================================================
// Scroll tests
// ============================================================

func TestScrollTrigger(t *testing.T) {
	d := newTestDriver()
	rows := d.Config.Rows

	// Write a unique character on each row.
	for row := 0; row < rows; row++ {
		d.PutCharAt(row, 0, byte('A'+row), DefaultAttribute)
	}

	// Fill row 0 with 'A', and remember what was on row 1
	row1Char := d.GetCell(1, 0).Character

	// Move cursor to last row, newline triggers scroll
	d.SetCursor(rows-1, 0)
	d.PutChar('\n')

	// After scroll, row 0 should have what was row 1
	cell := d.GetCell(0, 0)
	if cell.Character != row1Char {
		t.Errorf("row 0 after scroll = %q, want %q", cell.Character, row1Char)
	}
}

func TestScrollLastRowCleared(t *testing.T) {
	d := newTestDriver()
	rows := d.Config.Rows

	// Fill every cell with 'X'
	for row := 0; row < rows; row++ {
		for col := 0; col < d.Config.Columns; col++ {
			d.PutCharAt(row, col, 'X', DefaultAttribute)
		}
	}

	// Trigger scroll
	d.SetCursor(rows-1, 0)
	d.PutChar('\n')

	// Last row should be cleared
	for col := 0; col < d.Config.Columns; col++ {
		cell := d.GetCell(rows-1, col)
		if cell.Character != ' ' {
			t.Errorf("cell(%d,%d) = %q after scroll, want space", rows-1, col, cell.Character)
		}
	}
}

func TestScrollCursorPosition(t *testing.T) {
	d := newTestDriver()
	d.SetCursor(d.Config.Rows-1, 0)
	d.PutChar('\n')

	pos := d.GetCursor()
	if pos.Row != d.Config.Rows-1 || pos.Col != 0 {
		t.Errorf("cursor after scroll = (%d,%d), want (%d,0)", pos.Row, pos.Col, d.Config.Rows-1)
	}
}

func TestMultipleScrolls(t *testing.T) {
	d := newTestDriver()

	// Write 30 lines (more than the 10-row compact display).
	for i := 0; i < 30; i++ {
		d.Puts(strings.Repeat(string(rune('A'+i%26)), 5))
		d.PutChar('\n')
	}

	// The last `rows` lines should be visible. Line 30 is blank (just
	// written a newline). Lines 21-29 wrote characters.
	snap := d.Snapshot()

	// The last written content line should be on row rows-2 or rows-1
	// depending on cursor position. Just verify no crashes and content exists.
	found := false
	for _, line := range snap.Lines {
		if len(line) > 0 {
			found = true
			break
		}
	}
	if !found {
		t.Error("after 30 lines of output, expected some content on screen")
	}
}

func TestScrollPreservesAttributes(t *testing.T) {
	d := newTestDriver()
	rows := d.Config.Rows

	// Write colored text on row 1
	customAttr := MakeAttribute(ColorWhite, ColorBlue)
	d.PutCharAt(1, 0, 'Z', customAttr)

	// Trigger scroll
	d.SetCursor(rows-1, 0)
	d.PutChar('\n')

	// Row 1's content should now be on row 0
	cell := d.GetCell(0, 0)
	if cell.Character != 'Z' {
		t.Errorf("cell(0,0) char = %q, want 'Z'", cell.Character)
	}
	if cell.Attribute != customAttr {
		t.Errorf("cell(0,0) attr = 0x%02X, want 0x%02X", cell.Attribute, customAttr)
	}
}

// ============================================================
// Clear tests
// ============================================================

func TestClearDisplay(t *testing.T) {
	d := newTestDriver()
	d.Puts("Hello World")
	d.Clear()

	for row := 0; row < d.Config.Rows; row++ {
		for col := 0; col < d.Config.Columns; col++ {
			cell := d.GetCell(row, col)
			if cell.Character != ' ' {
				t.Errorf("cell(%d,%d) = %q after clear, want space", row, col, cell.Character)
			}
			if cell.Attribute != DefaultAttribute {
				t.Errorf("cell(%d,%d) attr = 0x%02X after clear, want 0x07", row, col, cell.Attribute)
			}
		}
	}
}

func TestClearResetsCursor(t *testing.T) {
	d := newTestDriver()
	d.Puts("Hello")
	d.Clear()

	pos := d.GetCursor()
	if pos.Row != 0 || pos.Col != 0 {
		t.Errorf("cursor after clear = (%d,%d), want (0,0)", pos.Row, pos.Col)
	}
}

// ============================================================
// Snapshot tests
// ============================================================

func TestSnapshotBasic(t *testing.T) {
	d := newTestDriver()
	d.Puts("Hello World")

	snap := d.Snapshot()
	if snap.Lines[0] != "Hello World" {
		t.Errorf("line 0 = %q, want %q", snap.Lines[0], "Hello World")
	}
}

func TestSnapshotTrailingSpacesTrimmed(t *testing.T) {
	d := newTestDriver()
	d.Puts("Hi")

	snap := d.Snapshot()
	if snap.Lines[0] != "Hi" {
		t.Errorf("line 0 = %q, want %q", snap.Lines[0], "Hi")
	}
}

func TestSnapshotEmptyLines(t *testing.T) {
	d := newTestDriver()
	snap := d.Snapshot()

	for i, line := range snap.Lines {
		if line != "" {
			t.Errorf("line %d = %q, want empty", i, line)
		}
	}
}

func TestSnapshotContains(t *testing.T) {
	d := newTestDriver()
	d.Puts("Hello World")

	snap := d.Snapshot()
	if !snap.Contains("Hello World") {
		t.Error("Contains(\"Hello World\") = false, want true")
	}
}

func TestSnapshotContainsNegative(t *testing.T) {
	d := newTestDriver()
	d.Puts("Hello World")

	snap := d.Snapshot()
	if snap.Contains("Goodbye") {
		t.Error("Contains(\"Goodbye\") = true, want false")
	}
}

func TestSnapshotContainsPartial(t *testing.T) {
	d := newTestDriver()
	d.Puts("Hello World")

	snap := d.Snapshot()
	if !snap.Contains("World") {
		t.Error("Contains(\"World\") = false, want true")
	}
}

func TestSnapshotString(t *testing.T) {
	d := newTestDriver()
	d.Puts("Hello")

	snap := d.Snapshot()
	str := snap.String()
	lines := strings.Split(str, "\n")
	if len(lines) != d.Config.Rows {
		t.Errorf("String() has %d lines, want %d", len(lines), d.Config.Rows)
	}
	// Each line should be padded to full width.
	for i, line := range lines {
		if len(line) != d.Config.Columns {
			t.Errorf("line %d length = %d, want %d", i, len(line), d.Config.Columns)
		}
	}
}

func TestSnapshotCursor(t *testing.T) {
	d := newTestDriver()
	d.SetCursor(5, 10)

	snap := d.Snapshot()
	if snap.Cursor.Row != 5 || snap.Cursor.Col != 10 {
		t.Errorf("snapshot cursor = (%d,%d), want (5,10)", snap.Cursor.Row, snap.Cursor.Col)
	}
}

func TestSnapshotLineAt(t *testing.T) {
	d := newTestDriver()
	d.Puts("Line 0")
	d.PutChar('\n')
	d.Puts("Line 1")

	snap := d.Snapshot()
	if snap.LineAt(0) != "Line 0" {
		t.Errorf("LineAt(0) = %q, want %q", snap.LineAt(0), "Line 0")
	}
	if snap.LineAt(1) != "Line 1" {
		t.Errorf("LineAt(1) = %q, want %q", snap.LineAt(1), "Line 1")
	}
	if snap.LineAt(-1) != "" {
		t.Errorf("LineAt(-1) = %q, want empty", snap.LineAt(-1))
	}
	if snap.LineAt(100) != "" {
		t.Errorf("LineAt(100) = %q, want empty", snap.LineAt(100))
	}
}

// ============================================================
// Attribute tests
// ============================================================

func TestDefaultAttribute(t *testing.T) {
	d := newTestDriver()
	d.PutChar('A')

	cell := d.GetCell(0, 0)
	if cell.Attribute != 0x07 {
		t.Errorf("default attribute = 0x%02X, want 0x07", cell.Attribute)
	}
}

func TestCustomAttribute(t *testing.T) {
	d := newTestDriver()
	d.PutCharAt(0, 0, 'A', 0x1F)

	cell := d.GetCell(0, 0)
	if cell.Attribute != 0x1F {
		t.Errorf("custom attribute = 0x%02X, want 0x1F", cell.Attribute)
	}
}

// ============================================================
// Cursor management tests
// ============================================================

func TestSetCursorClamps(t *testing.T) {
	d := newTestDriver()

	d.SetCursor(-5, -5)
	pos := d.GetCursor()
	if pos.Row != 0 || pos.Col != 0 {
		t.Errorf("clamped cursor = (%d,%d), want (0,0)", pos.Row, pos.Col)
	}

	d.SetCursor(100, 100)
	pos = d.GetCursor()
	if pos.Row != d.Config.Rows-1 || pos.Col != d.Config.Columns-1 {
		t.Errorf("clamped cursor = (%d,%d), want (%d,%d)", pos.Row, pos.Col, d.Config.Rows-1, d.Config.Columns-1)
	}
}

// ============================================================
// Edge case tests
// ============================================================

func TestFullFramebuffer(t *testing.T) {
	d := newTestDriver()
	totalCells := d.Config.Columns * d.Config.Rows

	// Fill every cell with 'X' via PutChar.
	for i := 0; i < totalCells; i++ {
		d.PutChar('X')
	}

	// After writing exactly totalCells characters, a scroll should have
	// been triggered (cursor went past last row). Verify no crash.
	snap := d.Snapshot()
	if !snap.Contains("X") {
		t.Error("expected 'X' on screen after filling framebuffer")
	}
}

func TestRapidScrolling(t *testing.T) {
	d := newTestDriver()

	// Write 100 lines. Should trigger many scrolls without corruption.
	for i := 0; i < 100; i++ {
		d.Puts("Line")
		d.PutChar('\n')
	}

	snap := d.Snapshot()
	if !snap.Contains("Line") {
		t.Error("expected 'Line' on screen after 100 lines")
	}
}

func TestNullCharacter(t *testing.T) {
	d := newTestDriver()
	d.PutChar(0x00)

	cell := d.GetCell(0, 0)
	if cell.Character != 0x00 {
		t.Errorf("null char = 0x%02X, want 0x00", cell.Character)
	}
}

func TestAllASCIIValues(t *testing.T) {
	d := newStandardDriver() // Need 80x25 = 2000 cells for 256 chars

	// Write every byte value 0-255 using PutCharAt (to avoid control char handling).
	for i := 0; i < 256; i++ {
		row := i / d.Config.Columns
		col := i % d.Config.Columns
		d.PutCharAt(row, col, byte(i), DefaultAttribute)
	}

	// Verify all stored correctly.
	for i := 0; i < 256; i++ {
		row := i / d.Config.Columns
		col := i % d.Config.Columns
		cell := d.GetCell(row, col)
		if cell.Character != byte(i) {
			t.Errorf("cell(%d,%d) = 0x%02X, want 0x%02X", row, col, cell.Character, byte(i))
		}
	}
}

func TestGetCellOutOfBounds(t *testing.T) {
	d := newTestDriver()

	cell := d.GetCell(-1, 0)
	if cell.Character != ' ' || cell.Attribute != DefaultAttribute {
		t.Errorf("out-of-bounds cell = {%q, 0x%02X}, want {' ', 0x07}", cell.Character, cell.Attribute)
	}

	cell = d.GetCell(0, 100)
	if cell.Character != ' ' || cell.Attribute != DefaultAttribute {
		t.Errorf("out-of-bounds cell = {%q, 0x%02X}, want {' ', 0x07}", cell.Character, cell.Attribute)
	}
}

// ============================================================
// Standard 80x25 specific tests
// ============================================================

func TestStandard80x25PutChar(t *testing.T) {
	d := newStandardDriver()
	d.PutChar('A')

	cell := d.GetCell(0, 0)
	if cell.Character != 'A' {
		t.Errorf("cell(0,0) = %q, want 'A'", cell.Character)
	}
	if cell.Attribute != 0x07 {
		t.Errorf("attribute = 0x%02X, want 0x07", cell.Attribute)
	}

	pos := d.GetCursor()
	if pos.Col != 1 {
		t.Errorf("cursor col = %d, want 1", pos.Col)
	}
}

func TestStandard80x25LineWrap(t *testing.T) {
	d := newStandardDriver()

	// Write 81 characters — should wrap to row 1.
	for i := 0; i < 81; i++ {
		d.PutChar('A')
	}

	pos := d.GetCursor()
	if pos.Row != 1 || pos.Col != 1 {
		t.Errorf("cursor = (%d,%d), want (1,1)", pos.Row, pos.Col)
	}
}

func TestStandard80x25Scroll(t *testing.T) {
	d := newStandardDriver()

	// Fill all 25 rows with unique content.
	for row := 0; row < 25; row++ {
		ch := byte('A' + row%26)
		d.PutCharAt(row, 0, ch, DefaultAttribute)
	}

	// Trigger scroll.
	d.SetCursor(24, 0)
	d.PutChar('\n')

	// Row 0 should now contain what was on row 1.
	cell := d.GetCell(0, 0)
	if cell.Character != 'B' {
		t.Errorf("after scroll, row 0 = %q, want 'B'", cell.Character)
	}
}

func TestSnapshotRowsAndColumns(t *testing.T) {
	d := newTestDriver()
	snap := d.Snapshot()

	if snap.Rows != d.Config.Rows {
		t.Errorf("snapshot Rows = %d, want %d", snap.Rows, d.Config.Rows)
	}
	if snap.Columns != d.Config.Columns {
		t.Errorf("snapshot Columns = %d, want %d", snap.Columns, d.Config.Columns)
	}
}

func TestTabWrapToNextRow(t *testing.T) {
	d := newTestDriver()
	// Position cursor near end of row, tab should wrap
	d.SetCursor(0, 39) // Last column in compact 40-col display
	d.PutChar('\t')

	pos := d.GetCursor()
	if pos.Row != 1 || pos.Col != 0 {
		t.Errorf("cursor after tab at col 39 = (%d,%d), want (1,0)", pos.Row, pos.Col)
	}
}

func TestPredefinedConfigs(t *testing.T) {
	if VGA80x25.Columns != 80 || VGA80x25.Rows != 25 {
		t.Errorf("VGA80x25 = %dx%d, want 80x25", VGA80x25.Columns, VGA80x25.Rows)
	}
	if Compact40x10.Columns != 40 || Compact40x10.Rows != 10 {
		t.Errorf("Compact40x10 = %dx%d, want 40x10", Compact40x10.Columns, Compact40x10.Rows)
	}
}
