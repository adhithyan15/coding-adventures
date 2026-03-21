# S05 — Display Driver

## Overview

The display driver simulates a text-mode framebuffer, modeled after the
classic VGA text mode that dominated personal computing from the 1980s
through the early 2000s. The display is a memory-mapped region where each
cell is 2 bytes: one for the ASCII character and one for the attribute
(foreground/background color). The driver provides `PutChar`, `Puts`,
`Scroll`, and cursor management.

This is the final visible output of the entire computing stack. When
"Hello World" appears on the display, it means every layer worked: logic
gates computed ALU operations, the pipeline fetched and executed instructions,
the cache served memory accesses, the bootloader loaded the kernel, the
kernel dispatched the sys_write syscall, and the display driver wrote
characters to the framebuffer. The display is proof that everything works.

**Analogy:** The framebuffer is like a grid of Post-it notes on a wall. The
wall is 80 notes wide and 25 notes tall (2,000 notes total). Each note has
one character written on it and a color. To display text, you write
characters one by one onto the notes, moving left to right, top to bottom.
When you reach the bottom of the wall and need more space, you peel off the
top row and shift everything up by one row (scroll). The cursor is your
finger pointing at the next note to write on.

## Layer Position

```
User Program
│
├── sys_write(1, "Hello World\n", 12)
│     └── ecall → kernel → syscall handler
│
├── OS Kernel (S04)
│     └── for each byte: display.PutChar(byte)
│
├── Display Driver (S05) ← YOU ARE HERE
│     └── write character + attribute to framebuffer memory
│
└── Framebuffer Memory Region (0xFFFB0000)
      └── read by host program to render output
```

**Depends on:** Memory subsystem (the framebuffer is a region of the address
space)

**Used by:** S04 Kernel (calls PutChar/Puts during sys_write), S06
SystemBoard (reads DisplaySnapshot for the boot trace)

## Key Concepts

### VGA Text Mode: A Brief History

In 1981, IBM's original PC shipped with a Monochrome Display Adapter (MDA)
that displayed 80 columns by 25 rows of text. The Color Graphics Adapter
(CGA) added color. The Video Graphics Array (VGA), introduced in 1987,
became the universal standard. Every x86 PC — even today — boots in VGA
text mode before switching to a graphical framebuffer.

VGA text mode is brilliantly simple: a region of memory (starting at
`0xB8000` on real x86 hardware) is directly mapped to the display. Write
a byte to that memory region, and the corresponding character appears on
screen. No GPU, no graphics driver, no compositor — just raw memory writes.

We simulate this exact model, placing our framebuffer at `0xFFFB0000`
(a high address to avoid conflicting with program memory).

### Framebuffer Layout

The framebuffer is a contiguous block of memory. Each cell (character
position) occupies exactly 2 bytes:

```
Framebuffer Memory Layout:
┌──────────────────────────────────────────────────────┐
│ Row 0, Col 0  │ Row 0, Col 1  │ ... │ Row 0, Col 79 │
│ [char][attr]  │ [char][attr]  │     │ [char][attr]  │
├───────────────┼───────────────┤     ├───────────────┤
│ Row 1, Col 0  │ Row 1, Col 1  │ ... │ Row 1, Col 79 │
│ [char][attr]  │ [char][attr]  │     │ [char][attr]  │
├───────────────┼───────────────┤     ├───────────────┤
│ ...           │ ...           │     │ ...           │
├───────────────┼───────────────┤     ├───────────────┤
│ Row 24, Col 0 │ Row 24, Col 1 │ ... │ Row 24, Col 79│
│ [char][attr]  │ [char][attr]  │     │ [char][attr]  │
└───────────────┴───────────────┴─────┴───────────────┘

Total size: 80 columns x 25 rows x 2 bytes/cell = 4,000 bytes
```

### Address Calculation

To find the memory address of any cell:

```
address = FramebufferBase + (row * Columns + col) * 2

Example: cell at row 3, col 10 with base 0xFFFB0000:
  address = 0xFFFB0000 + (3 * 80 + 10) * 2
          = 0xFFFB0000 + 250 * 2
          = 0xFFFB0000 + 500
          = 0xFFFB01F4

  Byte at 0xFFFB01F4: the character (ASCII code)
  Byte at 0xFFFB01F5: the attribute (color info)
```

### The Attribute Byte

Each character has an associated attribute byte that controls its color.
This is a simplified version of the VGA attribute byte:

```
Attribute Byte:
┌───┬───┬───┬───┬───┬───┬───┬───┐
│ 7 │ 6 │ 5 │ 4 │ 3 │ 2 │ 1 │ 0 │
└───┴───┴───┴───┴───┴───┴───┴───┘
  │   └──┬──┘   └──────┬──────┘
  │   Background (0-7) Foreground (0-15)
  └── Unused (always 0)

Foreground Colors (4 bits):         Background Colors (3 bits):
  0  = Black                          0 = Black
  1  = Blue                           1 = Blue
  2  = Green                          2 = Green
  3  = Cyan                           3 = Cyan
  4  = Red                            4 = Red
  5  = Magenta                        5 = Magenta
  6  = Brown                          6 = Brown
  7  = Light Gray                     7 = Light Gray
  8  = Dark Gray (bright black)
  9  = Light Blue
  10 = Light Green
  11 = Light Cyan
  12 = Light Red
  13 = Light Magenta
  14 = Yellow (bright brown)
  15 = White (bright light gray)
```

The default attribute is `0x07`: light gray foreground on black background.
This matches the classic terminal appearance — gray text on a black screen.

```
Common attribute values:
  0x07 = light gray on black   (default terminal)
  0x0F = white on black         (bright text)
  0x1F = white on blue          (classic BSOD look)
  0x4F = white on red           (error highlight)
  0x0A = light green on black   (Matrix-style)
  0x0E = yellow on black        (warning text)
```

### Cursor Management

The cursor tracks where the next character will be written. It is a logical
position (row, col) maintained by the driver — not a hardware register in
our simulation.

```
Cursor Behavior:

PutChar('A'):  Write 'A' at cursor, advance cursor to the right
               ┌─┬─┬─┬─┬─┐        ┌─┬─┬─┬─┬─┐
               │ │ │ │ │ │   →     │A│ │ │ │ │
               └─┴─┴─┴─┴─┘        └─┴─┴─┴─┴─┘
                ^cursor              ^cursor

PutChar('\n'): Move cursor to column 0 of the next row
               Row 0: [Hello___...]        Row 0: [Hello___...]
               Row 1: [________...]  →     Row 1: [________...]
                       ^cursor                     ^cursor

Line wrap: When col >= 80, move to col 0 of the next row
               Col 79: [x]           →     Col 0 of next row
               (automatic, no explicit newline needed)

Scroll trigger: When row >= 25, scroll the display
```

### Scrolling

When the cursor moves past the last row (row 24), the entire display
scrolls up by one line:

```
Before scroll (cursor at row 25, which does not exist):
  Row 0:  [Line 1 text          ]
  Row 1:  [Line 2 text          ]
  Row 2:  [Line 3 text          ]
  ...
  Row 23: [Line 24 text         ]
  Row 24: [Line 25 text         ]
  Row 25: ← cursor here (off-screen!)

Scroll operation:
  1. Copy row 1 → row 0
  2. Copy row 2 → row 1
  3. ...
  4. Copy row 24 → row 23
  5. Clear row 24 (fill with spaces + default attribute)
  6. Set cursor to (row 24, col 0)

After scroll:
  Row 0:  [Line 2 text          ]  ← was row 1
  Row 1:  [Line 3 text          ]  ← was row 2
  ...
  Row 23: [Line 25 text         ]  ← was row 24
  Row 24: [                     ]  ← cleared, cursor here

Line 1 is gone — scrolled off the top. Just like a real terminal.
```

In the framebuffer, scrolling is a memory copy operation:

```go
// Scroll: copy rows 1-24 to rows 0-23
bytesPerRow := columns * 2  // 80 * 2 = 160 bytes
copy(memory[0:], memory[bytesPerRow:bytesPerRow*rows])

// Clear the last row
for i := bytesPerRow * (rows - 1); i < bytesPerRow*rows; i += 2 {
    memory[i] = ' '           // space character
    memory[i+1] = defaultAttr // default attribute
}
```

### Display Snapshot

The `DisplaySnapshot` provides a read-friendly view of the current display
state. It converts the raw framebuffer bytes into strings (one per row,
trailing spaces trimmed) and supports text search. This is the primary
interface for tests and the boot trace.

```
Example:
  Framebuffer contains "Hello World" at row 0, cursor at (0, 11)

  snapshot := display.Snapshot()
  snapshot.Lines[0]  == "Hello World"
  snapshot.Lines[1]  == ""              (empty — no text on row 1)
  snapshot.Cursor    == {Row: 0, Col: 11}
  snapshot.Contains("Hello World") == true
  snapshot.Contains("Goodbye")     == false
  snapshot.String()  ==
    "Hello World                                                                     \n"
    "                                                                                \n"
    ... (25 lines total)
```

## Public API

```go
// --- Configuration ---

type DisplayConfig struct {
    Columns          int     // Number of character columns (default: 80)
    Rows             int     // Number of character rows (default: 25)
    FramebufferBase  uint32  // Memory-mapped base address (default: 0xFFFB0000)
    DefaultAttribute byte    // Default color attribute (default: 0x07)
}

// DefaultDisplayConfig returns the standard 80x25 VGA text mode configuration.
func DefaultDisplayConfig() DisplayConfig

// --- Cell ---

// Cell represents a single character position in the framebuffer.
type Cell struct {
    Character byte  // ASCII character code
    Attribute byte  // Color attribute byte
}

// --- Cursor ---

type CursorPosition struct {
    Row int
    Col int
}

// --- Display Driver ---

type DisplayDriver struct {
    Config  DisplayConfig
    Memory  []byte          // Reference to the framebuffer memory region
    Cursor  CursorPosition  // Current cursor position
}

// NewDisplayDriver creates a display driver backed by the given memory region.
// The memory slice must be at least Columns * Rows * 2 bytes.
// Initializes all cells to space + default attribute (cleared screen).
func NewDisplayDriver(config DisplayConfig, memory []byte) *DisplayDriver

// PutChar writes a single character at the current cursor position using
// the default attribute, then advances the cursor.
// Handles special characters:
//   '\n' (newline): move cursor to column 0 of next row
//   '\r' (carriage return): move cursor to column 0 of current row
//   '\t' (tab): advance cursor to next multiple of 8
//   '\b' (backspace): move cursor left by 1 (does not erase)
// Triggers scroll if cursor moves past the last row.
func (d *DisplayDriver) PutChar(ch byte)

// PutCharAt writes a character with a specific attribute at the given position.
// Does NOT move the cursor. Does NOT handle special characters.
// No-op if row/col is out of bounds.
func (d *DisplayDriver) PutCharAt(row, col int, ch byte, attr byte)

// Puts writes a string to the display, one character at a time via PutChar.
func (d *DisplayDriver) Puts(s string)

// Clear resets the entire display: fill all cells with space + default
// attribute, reset cursor to (0, 0).
func (d *DisplayDriver) Clear()

// Scroll shifts all rows up by one: row 1 becomes row 0, row 2 becomes
// row 1, etc. The last row is cleared. Cursor moves to (lastRow, 0).
func (d *DisplayDriver) Scroll()

// SetCursor moves the cursor to the given position.
// Clamps to valid range: row in [0, Rows-1], col in [0, Columns-1].
func (d *DisplayDriver) SetCursor(row, col int)

// GetCursor returns the current cursor position.
func (d *DisplayDriver) GetCursor() CursorPosition

// GetCell returns the character and attribute at the given position.
// Returns Cell{' ', DefaultAttribute} if out of bounds.
func (d *DisplayDriver) GetCell(row, col int) Cell

// Snapshot returns a read-friendly view of the current display state.
func (d *DisplayDriver) Snapshot() DisplaySnapshot

// --- Display Snapshot ---

type DisplaySnapshot struct {
    Lines   []string        // Text content of each row (trailing spaces trimmed)
    Cursor  CursorPosition  // Cursor position at snapshot time
    Rows    int             // Number of rows
    Columns int             // Number of columns
}

// String returns the full display as a multi-line string.
// Each line is padded to Columns width. Lines are joined with newlines.
func (s *DisplaySnapshot) String() string

// Contains returns true if the given text appears anywhere in the display.
// Searches across all lines (does not span across line boundaries).
func (s *DisplaySnapshot) Contains(text string) bool

// LineAt returns the text content of a specific row (trailing spaces trimmed).
// Returns "" if row is out of bounds.
func (s *DisplaySnapshot) LineAt(row int) string
```

## Data Structures

### Framebuffer Memory Format

```go
// Each cell is 2 bytes in memory:
//   byte 0: ASCII character code (0x00-0xFF)
//   byte 1: attribute byte (foreground + background color)
//
// Cells are stored in row-major order:
//   offset = (row * Columns + col) * 2
//
// Total framebuffer size: Columns * Rows * 2 bytes
//   For 80x25: 4,000 bytes

const BytesPerCell = 2
const DefaultColumns = 80
const DefaultRows = 25
const DefaultFramebufferBase = 0xFFFB0000
const DefaultAttribute = 0x07  // Light gray on black
```

### Color Constants

```go
// Foreground colors (bits 0-3 of attribute byte)
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

// MakeAttribute combines foreground and background colors into an attribute byte.
func MakeAttribute(fg, bg byte) byte {
    return (bg << 4) | (fg & 0x0F)
}
```

### Predefined Configurations

```go
// Standard VGA text mode (the default)
var VGA80x25 = DisplayConfig{
    Columns:          80,
    Rows:             25,
    FramebufferBase:  0xFFFB0000,
    DefaultAttribute: 0x07,
}

// Compact mode for testing (faster to fill, easier to verify)
var Compact40x10 = DisplayConfig{
    Columns:          40,
    Rows:             10,
    FramebufferBase:  0xFFFB0000,
    DefaultAttribute: 0x07,
}
```

## Test Strategy

### PutChar Tests

- **Basic write**: PutChar('A'), verify cell at (0,0) has character 'A'
  with default attribute
- **Cursor advance**: PutChar('A'), verify cursor moved to (0,1)
- **Multiple characters**: PutChar('H'), PutChar('i'), verify cells at
  (0,0)='H' and (0,1)='i', cursor at (0,2)
- **Newline**: PutChar('A'), PutChar('\n'), verify cursor at (1,0)
- **Carriage return**: write to col 5, PutChar('\r'), verify cursor at
  (row, 0)
- **Tab**: PutChar('\t'), verify cursor at (0,8); PutChar('x'),
  PutChar('\t'), verify cursor at (0,8)
- **Backspace**: write "AB", PutChar('\b'), verify cursor at (0,1)

### PutCharAt Tests

- **Write at position**: PutCharAt(5, 10, 'X', 0x0F), verify cell at
  (5,10) has 'X' with attribute 0x0F
- **Does not move cursor**: set cursor to (0,0), PutCharAt(5, 10, 'X', 0x07),
  verify cursor still at (0,0)
- **Out of bounds**: PutCharAt(30, 0, 'X', 0x07) with 25 rows — no crash,
  no effect

### Puts Tests

- **Simple string**: Puts("Hello"), verify 5 cells contain H, e, l, l, o
- **String with newline**: Puts("Hi\nBye"), verify "Hi" on row 0, "Bye"
  on row 1
- **Empty string**: Puts(""), verify no change to display or cursor

### Line Wrap Tests

- **Wrap at end of row**: write 80 characters, verify cursor moves to
  (1, 0); write one more, verify it appears at (1, 0)
- **Multi-line wrap**: write 161 characters (2 full rows + 1), verify
  text wraps correctly across 3 rows

### Scroll Tests

- **Trigger scroll**: fill all 25 rows, then PutChar('\n'), verify row 0
  now contains what was row 1
- **Last row cleared**: after scroll, verify row 24 is all spaces with
  default attribute
- **Cursor after scroll**: verify cursor is at (24, 0) after scroll
- **Multiple scrolls**: write 30 lines of text, verify the last 25 are
  visible and the first 5 are gone
- **Scroll preserves attributes**: write colored text, scroll, verify
  attributes shifted correctly

### Clear Tests

- **Clear display**: write text, call Clear(), verify all cells are space
  with default attribute
- **Cursor reset**: write text (cursor not at 0,0), Clear(), verify
  cursor at (0,0)

### Snapshot Tests

- **Basic snapshot**: Puts("Hello World"), snapshot, verify
  Lines[0]=="Hello World"
- **Trailing spaces trimmed**: write "Hi" at row 0, snapshot, verify
  Lines[0]=="Hi" (not "Hi" + 78 spaces)
- **Empty lines**: write nothing, snapshot, verify all Lines are ""
- **Contains**: Puts("Hello World"), verify Snapshot().Contains("Hello World")==true
- **Contains negative**: verify Snapshot().Contains("Goodbye")==false
- **Contains partial**: Puts("Hello World"), verify
  Snapshot().Contains("World")==true
- **String output**: verify Snapshot().String() produces 25 lines of text
- **Cursor in snapshot**: set cursor to (5,10), verify
  Snapshot().Cursor=={5,10}

### Attribute Tests

- **Default attribute**: PutChar('A'), verify attribute byte is 0x07
- **Custom attribute**: PutCharAt(0, 0, 'A', 0x1F), verify attribute is 0x1F
- **MakeAttribute**: verify MakeAttribute(ColorWhite, ColorBlue)==0x1F

### Edge Case Tests

- **Full framebuffer**: write 2000 characters, verify all cells filled
- **Rapid scrolling**: write 100 lines, verify no memory corruption
- **Null character**: PutChar(0x00), verify cell contains 0x00 (not crash)
- **All ASCII values**: write every byte 0x00-0xFF, verify all stored correctly

## Future Extensions

- **Cursor blinking**: track cursor blink state for visual display
- **Hardware cursor register**: simulate VGA CRTC cursor position registers
- **Graphics mode**: pixel-level framebuffer (not just text cells)
- **Double buffering**: write to a back buffer, then flip (reduces flicker)
- **ANSI escape codes**: support `\033[` sequences for cursor movement,
  color changes, and screen clearing (like a modern terminal)
- **Unicode support**: extend cell to 4 bytes for UTF-32 characters
- **Font rendering**: map character codes to bitmap fonts for graphical output
