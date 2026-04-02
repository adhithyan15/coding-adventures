package devicedriverframework

// =========================================================================
// SimulatedDisplay -- a character device representing a text-mode display
// =========================================================================
//
// Early computers used "text-mode" displays showing a grid of characters --
// typically 80 columns by 25 rows (from IBM PC VGA text mode).
//
// Each cell stores TWO bytes:
//   - Byte 0: the ASCII character (e.g., 0x48 = 'H')
//   - Byte 1: the attribute byte (foreground/background color, blink)
//
// Total framebuffer size: 80 * 25 * 2 = 4000 bytes
//
//   Memory layout:
//   ┌────────────────────────────────────────────────────────────────┐
//   │ Row 0: [char0][attr0] [char1][attr1] ... [char79][attr79]     │
//   │ Row 1: [char0][attr0] [char1][attr1] ... [char79][attr79]     │
//   │ ...                                                            │
//   │ Row 24: [char0][attr0] [char1][attr1] ... [char79][attr79]    │
//   └────────────────────────────────────────────────────────────────┘
//
// The display is WRITE-ONLY: Read() returns 0 (no data available).
// It does NOT generate interrupts (InterruptNumber = -1).

const (
	// DisplayCols is the number of columns (VGA text mode standard).
	DisplayCols = 80
	// DisplayRows is the number of rows.
	DisplayRows = 25
	// BytesPerCell is the number of bytes per character cell (char + attr).
	BytesPerCell = 2
	// FramebufferSize is the total framebuffer size in bytes.
	FramebufferSize = DisplayCols * DisplayRows * BytesPerCell
	// DefaultAttribute is light gray on black (standard VGA).
	DefaultAttribute = 0x07
)

// SimulatedDisplay is a simulated text-mode display with an 80x25 framebuffer.
type SimulatedDisplay struct {
	DeviceBase
	framebuffer []byte
	cursorRow   int
	cursorCol   int
}

// NewSimulatedDisplay creates a new simulated display.
func NewSimulatedDisplay(name string, minor int) *SimulatedDisplay {
	result, _ := StartNew[*SimulatedDisplay]("device-driver-framework.NewSimulatedDisplay", nil,
		func(op *Operation[*SimulatedDisplay], rf *ResultFactory[*SimulatedDisplay]) *OperationResult[*SimulatedDisplay] {
			op.AddProperty("name", name)
			op.AddProperty("minor", minor)
			d := &SimulatedDisplay{
				DeviceBase: DeviceBase{
					Name:            name,
					Type:            DeviceCharacter,
					Major:           MajorDisplay,
					Minor:           minor,
					InterruptNumber: -1, // Display does not generate interrupts
				},
				framebuffer: make([]byte, FramebufferSize),
			}
			d.clearScreen()
			return rf.Generate(true, false, d)
		}).GetResult()
	return result
}

// Init initializes the display by clearing the screen.
func (d *SimulatedDisplay) Init() {
	_, _ = StartNew[struct{}]("device-driver-framework.SimulatedDisplay.Init", struct{}{},
		func(op *Operation[struct{}], rf *ResultFactory[struct{}]) *OperationResult[struct{}] {
			d.clearScreen()
			d.Initialized = true
			return rf.Generate(true, false, struct{}{})
		}).GetResult()
}

// clearScreen fills every cell with a space (0x20) and the default attribute.
func (d *SimulatedDisplay) clearScreen() {
	for i := 0; i < DisplayCols*DisplayRows; i++ {
		offset := i * BytesPerCell
		d.framebuffer[offset] = 0x20 // space
		d.framebuffer[offset+1] = DefaultAttribute
	}
	d.cursorRow = 0
	d.cursorCol = 0
}

// ClearScreen is the public version -- clears the display and resets cursor.
func (d *SimulatedDisplay) ClearScreen() {
	_, _ = StartNew[struct{}]("device-driver-framework.SimulatedDisplay.ClearScreen", struct{}{},
		func(op *Operation[struct{}], rf *ResultFactory[struct{}]) *OperationResult[struct{}] {
			d.clearScreen()
			return rf.Generate(true, false, struct{}{})
		}).GetResult()
}

// Read attempts to read from the display (always returns 0).
// You cannot read from a display -- it is an output-only device.
func (d *SimulatedDisplay) Read(buf []byte) int {
	result, _ := StartNew[int]("device-driver-framework.SimulatedDisplay.Read", 0,
		func(op *Operation[int], rf *ResultFactory[int]) *OperationResult[int] {
			return rf.Generate(true, false, 0)
		}).GetResult()
	return result
}

// Write writes characters to the display at the current cursor position.
// Each byte is treated as an ASCII character. Special handling:
//   - 0x0A (newline): moves cursor to start of next line
//   - All other bytes: written to framebuffer at cursor position
//
// Returns the number of bytes written.
func (d *SimulatedDisplay) Write(data []byte) int {
	result, _ := StartNew[int]("device-driver-framework.SimulatedDisplay.Write", 0,
		func(op *Operation[int], rf *ResultFactory[int]) *OperationResult[int] {
			for _, b := range data {
				if b == 0x0A { // newline
					d.cursorCol = 0
					d.cursorRow++
				} else {
					d.putCharAt(d.cursorRow, d.cursorCol, b)
					d.cursorCol++
				}

				// Wrap to next line if past the right edge
				if d.cursorCol >= DisplayCols {
					d.cursorCol = 0
					d.cursorRow++
				}

				// Scroll if past the bottom
				if d.cursorRow >= DisplayRows {
					d.scrollUp()
					d.cursorRow = DisplayRows - 1
				}
			}
			return rf.Generate(true, false, len(data))
		}).GetResult()
	return result
}

// putCharAt places a character at a specific (row, col) position.
func (d *SimulatedDisplay) putCharAt(row, col int, ch byte) {
	offset := (row*DisplayCols + col) * BytesPerCell
	d.framebuffer[offset] = ch
	d.framebuffer[offset+1] = DefaultAttribute
}

// scrollUp scrolls the display up by one line.
// Row 0 is lost; the last row is filled with spaces.
func (d *SimulatedDisplay) scrollUp() {
	rowBytes := DisplayCols * BytesPerCell
	copy(d.framebuffer[0:rowBytes*(DisplayRows-1)],
		d.framebuffer[rowBytes:rowBytes*DisplayRows])
	// Clear the last row
	lastRowStart := (DisplayRows - 1) * rowBytes
	for i := 0; i < DisplayCols; i++ {
		offset := lastRowStart + i*BytesPerCell
		d.framebuffer[offset] = 0x20
		d.framebuffer[offset+1] = DefaultAttribute
	}
}

// CharAt returns the character byte at a specific (row, col) position.
func (d *SimulatedDisplay) CharAt(row, col int) byte {
	result, _ := StartNew[byte]("device-driver-framework.SimulatedDisplay.CharAt", 0,
		func(op *Operation[byte], rf *ResultFactory[byte]) *OperationResult[byte] {
			op.AddProperty("row", row)
			op.AddProperty("col", col)
			offset := (row*DisplayCols + col) * BytesPerCell
			return rf.Generate(true, false, d.framebuffer[offset])
		}).GetResult()
	return result
}

// AttrAt returns the attribute byte at a specific (row, col) position.
func (d *SimulatedDisplay) AttrAt(row, col int) byte {
	result, _ := StartNew[byte]("device-driver-framework.SimulatedDisplay.AttrAt", 0,
		func(op *Operation[byte], rf *ResultFactory[byte]) *OperationResult[byte] {
			op.AddProperty("row", row)
			op.AddProperty("col", col)
			offset := (row*DisplayCols + col) * BytesPerCell
			return rf.Generate(true, false, d.framebuffer[offset+1])
		}).GetResult()
	return result
}

// CursorPosition returns the current cursor position as (row, col).
func (d *SimulatedDisplay) CursorPosition() (int, int) {
	var col int
	row, _ := StartNew[int]("device-driver-framework.SimulatedDisplay.CursorPosition", 0,
		func(op *Operation[int], rf *ResultFactory[int]) *OperationResult[int] {
			col = d.cursorCol
			return rf.Generate(true, false, d.cursorRow)
		}).GetResult()
	return row, col
}

// Framebuffer returns the backing framebuffer slice (for testing/debugging).
func (d *SimulatedDisplay) Framebuffer() []byte {
	result, _ := StartNew[[]byte]("device-driver-framework.SimulatedDisplay.Framebuffer", nil,
		func(op *Operation[[]byte], rf *ResultFactory[[]byte]) *OperationResult[[]byte] {
			return rf.Generate(true, false, d.framebuffer)
		}).GetResult()
	return result
}
