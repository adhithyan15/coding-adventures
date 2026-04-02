// Package display simulates a VGA text-mode framebuffer display.
//
// # What is a framebuffer?
//
// A framebuffer is a region of memory that directly maps to what appears on
// screen. In VGA text mode, the framebuffer is an array of cells, where each
// cell is 2 bytes: one byte for the ASCII character and one byte for the
// color attribute. Writing a byte into the framebuffer instantly changes what
// appears on screen — no GPU, no graphics driver, just raw memory writes.
//
// Think of it like a wall of Post-it notes: 80 columns wide and 25 rows tall.
// Each note holds one character and has a color. To display text, you write
// characters one by one onto the notes, moving left to right, top to bottom.
//
// # Memory layout
//
// Cells are stored in row-major order. The address of any cell is:
//
//	address = FramebufferBase + (row * Columns + col) * 2
//
// At each cell's address:
//   - byte 0: ASCII character code (0x00-0xFF)
//   - byte 1: attribute byte (foreground + background color)
//
// For the standard 80x25 configuration, the framebuffer is 4,000 bytes.
//
// # The attribute byte
//
// The attribute byte packs foreground and background colors into a single byte:
//
//	Bit 7:   unused (always 0)
//	Bits 6-4: background color (0-7)
//	Bits 3-0: foreground color (0-15)
//
// The default attribute 0x07 gives light gray text on a black background —
// the classic terminal look.
package display

// ============================================================
// Constants — the fundamental parameters of VGA text mode
// ============================================================

const (
	// BytesPerCell is the number of bytes each character cell occupies.
	// Byte 0 = character, byte 1 = attribute.
	BytesPerCell = 2

	// DefaultColumns is the standard VGA text mode width.
	DefaultColumns = 80

	// DefaultRows is the standard VGA text mode height.
	DefaultRows = 25

	// DefaultFramebufferBase is the memory-mapped address where the
	// framebuffer begins. We use 0xFFFB0000 (a high address) to avoid
	// conflicts with program memory. On real x86 hardware, VGA text
	// mode lives at 0xB8000.
	DefaultFramebufferBase uint32 = 0xFFFB0000

	// DefaultAttribute is light gray on black (0x07). This matches
	// the classic terminal appearance: gray text on a dark screen.
	//
	//   Foreground: 7 (light gray)  = bits 3-0
	//   Background: 0 (black)       = bits 6-4
	//   Attribute:  0000_0111       = 0x07
	DefaultAttribute byte = 0x07
)

// ============================================================
// Color constants — the VGA color palette
// ============================================================
//
// Foreground colors use 4 bits (0-15), allowing 16 colors.
// Background colors use 3 bits (0-7), allowing 8 colors.
// The bright variants (8-15) are only available as foreground.
//
// Color truth table:
//   Value  Name           As FG?  As BG?
//   0      Black          yes     yes
//   1      Blue           yes     yes
//   2      Green          yes     yes
//   3      Cyan           yes     yes
//   4      Red            yes     yes
//   5      Magenta        yes     yes
//   6      Brown          yes     yes
//   7      Light Gray     yes     yes
//   8      Dark Gray      yes     no
//   9      Light Blue     yes     no
//   10     Light Green    yes     no
//   11     Light Cyan     yes     no
//   12     Light Red      yes     no
//   13     Light Magenta  yes     no
//   14     Yellow         yes     no
//   15     White          yes     no

const (
	ColorBlack        byte = 0
	ColorBlue         byte = 1
	ColorGreen        byte = 2
	ColorCyan         byte = 3
	ColorRed          byte = 4
	ColorMagenta      byte = 5
	ColorBrown        byte = 6
	ColorLightGray    byte = 7
	ColorDarkGray     byte = 8
	ColorLightBlue    byte = 9
	ColorLightGreen   byte = 10
	ColorLightCyan    byte = 11
	ColorLightRed     byte = 12
	ColorLightMagenta byte = 13
	ColorYellow       byte = 14
	ColorWhite        byte = 15
)

// MakeAttribute combines a foreground and background color into a single
// attribute byte. The foreground occupies the low 4 bits, the background
// occupies bits 4-6.
//
// Examples:
//
//	MakeAttribute(ColorWhite, ColorBlue) = 0x1F  (white on blue, the BSOD look)
//	MakeAttribute(ColorLightGray, ColorBlack) = 0x07  (default terminal)
//	MakeAttribute(ColorWhite, ColorRed) = 0x4F  (error highlight)
func MakeAttribute(fg, bg byte) byte {
	result, _ := StartNew[byte]("display.MakeAttribute", 0,
		func(op *Operation[byte], rf *ResultFactory[byte]) *OperationResult[byte] {
			return rf.Generate(true, false, (bg<<4)|(fg&0x0F))
		}).GetResult()
	return result
}

// ============================================================
// DisplayConfig — parameters for the display
// ============================================================

// DisplayConfig holds the dimensions and memory mapping of the display.
// The default configuration matches VGA text mode: 80 columns, 25 rows,
// framebuffer at 0xFFFB0000, light gray on black.
type DisplayConfig struct {
	Columns          int    // Number of character columns (default: 80)
	Rows             int    // Number of character rows (default: 25)
	FramebufferBase  uint32 // Memory-mapped base address (default: 0xFFFB0000)
	DefaultAttribute byte   // Default color attribute (default: 0x07)
}

// DefaultDisplayConfig returns the standard 80x25 VGA text mode configuration.
func DefaultDisplayConfig() DisplayConfig {
	result, _ := StartNew[DisplayConfig]("display.DefaultDisplayConfig", DisplayConfig{},
		func(op *Operation[DisplayConfig], rf *ResultFactory[DisplayConfig]) *OperationResult[DisplayConfig] {
			return rf.Generate(true, false, DisplayConfig{
				Columns:          DefaultColumns,
				Rows:             DefaultRows,
				FramebufferBase:  DefaultFramebufferBase,
				DefaultAttribute: DefaultAttribute,
			})
		}).GetResult()
	return result
}

// Predefined configurations for common use cases.
var (
	// VGA80x25 is the standard VGA text mode configuration.
	VGA80x25 = DisplayConfig{
		Columns:          80,
		Rows:             25,
		FramebufferBase:  DefaultFramebufferBase,
		DefaultAttribute: DefaultAttribute,
	}

	// Compact40x10 is a smaller configuration useful for testing.
	// Fewer cells means faster to fill and easier to verify.
	Compact40x10 = DisplayConfig{
		Columns:          40,
		Rows:             10,
		FramebufferBase:  DefaultFramebufferBase,
		DefaultAttribute: DefaultAttribute,
	}
)

// ============================================================
// Cell — a single character position
// ============================================================

// Cell represents one character position in the framebuffer.
// It stores both the visible character and its color attribute.
type Cell struct {
	Character byte // ASCII character code
	Attribute byte // Color attribute byte
}

// ============================================================
// CursorPosition — where the next character will be written
// ============================================================

// CursorPosition tracks the row and column of the cursor.
// Row 0 is the top of the screen, column 0 is the left edge.
type CursorPosition struct {
	Row int
	Col int
}
