package display

// ============================================================
// DisplayDriver — the main interface to the framebuffer
// ============================================================
//
// The display driver manages writing characters to the framebuffer,
// tracking the cursor position, and handling special operations like
// scrolling and clearing. It is the software layer between the OS
// kernel (which calls PutChar for each character of output) and the
// raw framebuffer memory.
//
// Usage:
//
//	config := DefaultDisplayConfig()
//	memory := make([]byte, config.Columns*config.Rows*BytesPerCell)
//	driver := NewDisplayDriver(config, memory)
//	driver.Puts("Hello World\n")
//	snap := driver.Snapshot()
//	fmt.Println(snap.Lines[0])  // "Hello World"

// DisplayDriver manages the framebuffer and cursor state.
type DisplayDriver struct {
	Config DisplayConfig  // Display dimensions and settings
	Memory []byte         // The framebuffer memory (Columns * Rows * 2 bytes)
	Cursor CursorPosition // Current cursor position
}

// NewDisplayDriver creates a display driver backed by the given memory region.
// The memory slice must be at least Columns * Rows * 2 bytes long.
// All cells are initialized to space + default attribute (a cleared screen).
//
// This mimics what happens when a real VGA card powers on: the screen shows
// blank space with the default color, and the cursor sits at the top-left.
func NewDisplayDriver(config DisplayConfig, memory []byte) *DisplayDriver {
	result, _ := StartNew[*DisplayDriver]("display.NewDisplayDriver", nil,
		func(op *Operation[*DisplayDriver], rf *ResultFactory[*DisplayDriver]) *OperationResult[*DisplayDriver] {
			d := &DisplayDriver{
				Config: config,
				Memory: memory,
				Cursor: CursorPosition{Row: 0, Col: 0},
			}
			d.Clear()
			return rf.Generate(true, false, d)
		}).GetResult()
	return result
}

// ============================================================
// Writing characters
// ============================================================

// PutChar writes a single character at the current cursor position using
// the default attribute, then advances the cursor to the right.
//
// Special characters are handled as control codes, not written to the screen:
//
//	'\n' (newline):        move to column 0 of the next row
//	'\r' (carriage return): move to column 0 of the current row
//	'\t' (tab):            advance to the next multiple of 8
//	'\b' (backspace):      move cursor left by 1 (does not erase)
//
// If the cursor moves past column 79 (end of row), it wraps to column 0
// of the next row. If it moves past row 24 (bottom of screen), the display
// scrolls up by one line.
func (d *DisplayDriver) PutChar(ch byte) {
	_, _ = StartNew[struct{}]("display.PutChar", struct{}{},
		func(op *Operation[struct{}], rf *ResultFactory[struct{}]) *OperationResult[struct{}] {
			switch ch {
			case '\n':
				// Newline: move to the beginning of the next row.
				// This is the most common control character — every "print line"
				// ends with one.
				d.Cursor.Col = 0
				d.Cursor.Row++
			case '\r':
				// Carriage return: move to column 0 of the current row.
				// Named after the typewriter carriage that physically returned
				// to the left margin.
				d.Cursor.Col = 0
			case '\t':
				// Tab: advance to the next tab stop (every 8 columns).
				// Tab stops at columns 0, 8, 16, 24, 32, 40, 48, 56, 64, 72.
				d.Cursor.Col = (d.Cursor.Col/8 + 1) * 8
				if d.Cursor.Col >= d.Config.Columns {
					d.Cursor.Col = 0
					d.Cursor.Row++
				}
			case '\b':
				// Backspace: move cursor left by one. Does not erase the character
				// under the cursor — that requires writing a space separately.
				if d.Cursor.Col > 0 {
					d.Cursor.Col--
				}
			default:
				// Regular character: write it to the framebuffer at the cursor
				// position, then advance the cursor.
				offset := (d.Cursor.Row*d.Config.Columns + d.Cursor.Col) * BytesPerCell
				if offset >= 0 && offset+1 < len(d.Memory) {
					d.Memory[offset] = ch
					d.Memory[offset+1] = d.Config.DefaultAttribute
				}
				d.Cursor.Col++

				// Line wrap: if we've gone past the last column, wrap to the
				// next row. This happens automatically — no explicit newline
				// needed. Real terminals do this too.
				if d.Cursor.Col >= d.Config.Columns {
					d.Cursor.Col = 0
					d.Cursor.Row++
				}
			}

			// Scroll check: if the cursor has moved past the last row, the
			// entire screen needs to scroll up. This is the "terminal scroll"
			// behavior — old content slides up and disappears off the top.
			if d.Cursor.Row >= d.Config.Rows {
				d.Scroll()
			}
			return rf.Generate(true, false, struct{}{})
		}).GetResult()
}

// PutCharAt writes a character with a specific attribute at the given
// position. Unlike PutChar, this does NOT move the cursor and does NOT
// handle special characters. It's a raw framebuffer write — useful for
// drawing UI elements or colored text at specific positions.
//
// Out-of-bounds positions are silently ignored (no crash, no wrap).
func (d *DisplayDriver) PutCharAt(row, col int, ch byte, attr byte) {
	_, _ = StartNew[struct{}]("display.PutCharAt", struct{}{},
		func(op *Operation[struct{}], rf *ResultFactory[struct{}]) *OperationResult[struct{}] {
			if row < 0 || row >= d.Config.Rows || col < 0 || col >= d.Config.Columns {
				return rf.Generate(true, false, struct{}{})
			}
			offset := (row*d.Config.Columns + col) * BytesPerCell
			d.Memory[offset] = ch
			d.Memory[offset+1] = attr
			return rf.Generate(true, false, struct{}{})
		}).GetResult()
}

// Puts writes a string to the display, one character at a time via PutChar.
// Each character goes through the same cursor-advance and special-character
// handling as PutChar.
//
// Example:
//
//	driver.Puts("Hello\nWorld")
//	// Row 0: "Hello"
//	// Row 1: "World"
func (d *DisplayDriver) Puts(s string) {
	_, _ = StartNew[struct{}]("display.Puts", struct{}{},
		func(op *Operation[struct{}], rf *ResultFactory[struct{}]) *OperationResult[struct{}] {
			for i := 0; i < len(s); i++ {
				d.PutChar(s[i])
			}
			return rf.Generate(true, false, struct{}{})
		}).GetResult()
}

// ============================================================
// Screen management
// ============================================================

// Clear resets the entire display: every cell becomes space (0x20) with the
// default attribute, and the cursor returns to (0, 0).
//
// This is the equivalent of the "cls" command on DOS or "clear" on Unix.
func (d *DisplayDriver) Clear() {
	_, _ = StartNew[struct{}]("display.Clear", struct{}{},
		func(op *Operation[struct{}], rf *ResultFactory[struct{}]) *OperationResult[struct{}] {
			totalBytes := d.Config.Columns * d.Config.Rows * BytesPerCell
			for i := 0; i < totalBytes && i+1 < len(d.Memory); i += BytesPerCell {
				d.Memory[i] = ' '
				d.Memory[i+1] = d.Config.DefaultAttribute
			}
			d.Cursor.Row = 0
			d.Cursor.Col = 0
			return rf.Generate(true, false, struct{}{})
		}).GetResult()
}

// Scroll shifts all rows up by one line. Row 1 becomes row 0, row 2 becomes
// row 1, and so on. The last row is cleared (filled with spaces).
// The cursor is placed at (lastRow, 0).
//
// This is how terminals handle output that exceeds the screen height:
// old content scrolls off the top and is lost forever (unless you have
// a scroll-back buffer, which we don't simulate).
//
// Implementation: a single memory copy moves rows 1-24 into rows 0-23,
// then we clear row 24. This is an O(n) operation where n is the
// framebuffer size minus one row.
func (d *DisplayDriver) Scroll() {
	_, _ = StartNew[struct{}]("display.Scroll", struct{}{},
		func(op *Operation[struct{}], rf *ResultFactory[struct{}]) *OperationResult[struct{}] {
			bytesPerRow := d.Config.Columns * BytesPerCell
			totalBytes := d.Config.Rows * bytesPerRow

			// Copy rows 1..N-1 into rows 0..N-2.
			// In memory terms: shift everything left by one row's worth of bytes.
			copy(d.Memory[0:], d.Memory[bytesPerRow:totalBytes])

			// Clear the last row: fill with space + default attribute.
			lastRowStart := (d.Config.Rows - 1) * bytesPerRow
			for i := lastRowStart; i < totalBytes; i += BytesPerCell {
				d.Memory[i] = ' '
				d.Memory[i+1] = d.Config.DefaultAttribute
			}

			// Place cursor at the beginning of the last row.
			d.Cursor.Row = d.Config.Rows - 1
			d.Cursor.Col = 0
			return rf.Generate(true, false, struct{}{})
		}).GetResult()
}

// ============================================================
// Cursor management
// ============================================================

// SetCursor moves the cursor to the given position.
// Row and column are clamped to valid bounds:
//   - row is clamped to [0, Rows-1]
//   - col is clamped to [0, Columns-1]
func (d *DisplayDriver) SetCursor(row, col int) {
	_, _ = StartNew[struct{}]("display.SetCursor", struct{}{},
		func(op *Operation[struct{}], rf *ResultFactory[struct{}]) *OperationResult[struct{}] {
			if row < 0 {
				row = 0
			} else if row >= d.Config.Rows {
				row = d.Config.Rows - 1
			}
			if col < 0 {
				col = 0
			} else if col >= d.Config.Columns {
				col = d.Config.Columns - 1
			}
			d.Cursor.Row = row
			d.Cursor.Col = col
			return rf.Generate(true, false, struct{}{})
		}).GetResult()
}

// GetCursor returns the current cursor position.
func (d *DisplayDriver) GetCursor() CursorPosition {
	result, _ := StartNew[CursorPosition]("display.GetCursor", CursorPosition{},
		func(op *Operation[CursorPosition], rf *ResultFactory[CursorPosition]) *OperationResult[CursorPosition] {
			return rf.Generate(true, false, d.Cursor)
		}).GetResult()
	return result
}

// ============================================================
// Reading cells
// ============================================================

// GetCell returns the character and attribute at the given position.
// If the position is out of bounds, returns Cell{' ', DefaultAttribute}.
func (d *DisplayDriver) GetCell(row, col int) Cell {
	result, _ := StartNew[Cell]("display.GetCell", Cell{},
		func(op *Operation[Cell], rf *ResultFactory[Cell]) *OperationResult[Cell] {
			if row < 0 || row >= d.Config.Rows || col < 0 || col >= d.Config.Columns {
				return rf.Generate(true, false, Cell{Character: ' ', Attribute: d.Config.DefaultAttribute})
			}
			offset := (row*d.Config.Columns + col) * BytesPerCell
			return rf.Generate(true, false, Cell{
				Character: d.Memory[offset],
				Attribute: d.Memory[offset+1],
			})
		}).GetResult()
	return result
}
