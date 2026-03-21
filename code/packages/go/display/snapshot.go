package display

import (
	"strings"
)

// ============================================================
// DisplaySnapshot — a read-friendly view of the framebuffer
// ============================================================
//
// The snapshot converts raw framebuffer bytes into human-readable strings.
// It is the primary interface for tests and for the boot trace in the
// SystemBoard (S06). When you want to check "what is currently on screen,"
// you take a snapshot and inspect its Lines or use Contains().
//
// Snapshots are immutable — they capture the display state at one moment
// in time. Subsequent writes to the framebuffer do not affect an existing
// snapshot.

// DisplaySnapshot holds a frozen view of the display's text content.
type DisplaySnapshot struct {
	Lines   []string       // Text content of each row (trailing spaces trimmed)
	Cursor  CursorPosition // Cursor position at snapshot time
	Rows    int            // Number of rows
	Columns int            // Number of columns
}

// Snapshot reads the framebuffer and returns a DisplaySnapshot with the
// current text content. Each row is extracted as a string with trailing
// spaces trimmed — so a row containing "Hello" followed by 75 spaces
// becomes just "Hello".
func (d *DisplayDriver) Snapshot() DisplaySnapshot {
	lines := make([]string, d.Config.Rows)
	for row := 0; row < d.Config.Rows; row++ {
		var buf strings.Builder
		for col := 0; col < d.Config.Columns; col++ {
			offset := (row*d.Config.Columns + col) * BytesPerCell
			buf.WriteByte(d.Memory[offset])
		}
		// Trim trailing spaces. A row that is all spaces becomes "".
		lines[row] = strings.TrimRight(buf.String(), " ")
	}
	return DisplaySnapshot{
		Lines:   lines,
		Cursor:  d.Cursor,
		Rows:    d.Config.Rows,
		Columns: d.Config.Columns,
	}
}

// String returns the full display as a multi-line string.
// Each line is padded to the full column width. Lines are joined with
// newlines. This produces a faithful text rendering of the entire screen.
func (s *DisplaySnapshot) String() string {
	var buf strings.Builder
	for i, line := range s.Lines {
		// Pad each line to full width.
		padded := line + strings.Repeat(" ", s.Columns-len(line))
		buf.WriteString(padded)
		if i < len(s.Lines)-1 {
			buf.WriteByte('\n')
		}
	}
	return buf.String()
}

// Contains returns true if the given text appears anywhere in the display.
// It searches each line independently — the text must fit within a single
// row (it does not span across line boundaries).
//
// Examples:
//
//	snap.Contains("Hello World")  // true if "Hello World" is on any row
//	snap.Contains("Goodbye")     // false if not present
//	snap.Contains("World")       // true if "World" appears as a substring
func (s *DisplaySnapshot) Contains(text string) bool {
	for _, line := range s.Lines {
		if strings.Contains(line, text) {
			return true
		}
	}
	return false
}

// LineAt returns the text content of a specific row (trailing spaces trimmed).
// Returns "" if the row is out of bounds.
func (s *DisplaySnapshot) LineAt(row int) string {
	if row < 0 || row >= len(s.Lines) {
		return ""
	}
	return s.Lines[row]
}
