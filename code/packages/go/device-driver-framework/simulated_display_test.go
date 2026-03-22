package devicedriverframework

import "testing"

func TestSimulatedDisplayDefaults(t *testing.T) {
	disp := NewSimulatedDisplay("display0", 0)
	if disp.Name != "display0" {
		t.Errorf("Name = %q, want display0", disp.Name)
	}
	if disp.Type != DeviceCharacter {
		t.Errorf("Type = %v, want CHARACTER", disp.Type)
	}
	if disp.Major != MajorDisplay {
		t.Errorf("Major = %d, want %d", disp.Major, MajorDisplay)
	}
	if disp.InterruptNumber != -1 {
		t.Errorf("InterruptNumber = %d, want -1", disp.InterruptNumber)
	}
}

func TestSimulatedDisplayFramebufferSize(t *testing.T) {
	disp := NewSimulatedDisplay("display0", 0)
	if len(disp.Framebuffer()) != FramebufferSize {
		t.Errorf("Framebuffer size = %d, want %d", len(disp.Framebuffer()), FramebufferSize)
	}
}

func TestSimulatedDisplayInit(t *testing.T) {
	disp := NewSimulatedDisplay("display0", 0)
	disp.Write([]byte("Hello"))
	disp.Init()
	if !disp.Initialized {
		t.Error("Should be initialized after Init()")
	}
	row, col := disp.CursorPosition()
	if row != 0 || col != 0 {
		t.Errorf("Cursor = (%d,%d), want (0,0)", row, col)
	}
	if disp.CharAt(0, 0) != 0x20 {
		t.Errorf("CharAt(0,0) = 0x%02X, want 0x20", disp.CharAt(0, 0))
	}
}

func TestSimulatedDisplayWriteSingleChar(t *testing.T) {
	disp := NewSimulatedDisplay("display0", 0)
	disp.Init()
	disp.Write([]byte("H"))
	if disp.CharAt(0, 0) != 'H' {
		t.Errorf("CharAt(0,0) = %c, want H", disp.CharAt(0, 0))
	}
	if disp.AttrAt(0, 0) != DefaultAttribute {
		t.Errorf("AttrAt(0,0) = 0x%02X, want 0x%02X", disp.AttrAt(0, 0), DefaultAttribute)
	}
	row, col := disp.CursorPosition()
	if row != 0 || col != 1 {
		t.Errorf("Cursor = (%d,%d), want (0,1)", row, col)
	}
}

func TestSimulatedDisplayWriteMultipleChars(t *testing.T) {
	disp := NewSimulatedDisplay("display0", 0)
	disp.Init()
	n := disp.Write([]byte("Hi"))
	if n != 2 {
		t.Errorf("Write returned %d, want 2", n)
	}
	if disp.CharAt(0, 0) != 'H' {
		t.Errorf("CharAt(0,0) = %c, want H", disp.CharAt(0, 0))
	}
	if disp.CharAt(0, 1) != 'i' {
		t.Errorf("CharAt(0,1) = %c, want i", disp.CharAt(0, 1))
	}
}

func TestSimulatedDisplayNewline(t *testing.T) {
	disp := NewSimulatedDisplay("display0", 0)
	disp.Init()
	disp.Write([]byte("A\nB"))
	if disp.CharAt(0, 0) != 'A' {
		t.Errorf("CharAt(0,0) = %c, want A", disp.CharAt(0, 0))
	}
	if disp.CharAt(1, 0) != 'B' {
		t.Errorf("CharAt(1,0) = %c, want B", disp.CharAt(1, 0))
	}
	row, col := disp.CursorPosition()
	if row != 1 || col != 1 {
		t.Errorf("Cursor = (%d,%d), want (1,1)", row, col)
	}
}

func TestSimulatedDisplayLineWrap(t *testing.T) {
	disp := NewSimulatedDisplay("display0", 0)
	disp.Init()
	// Write 80 X's to fill the first row
	data := make([]byte, DisplayCols)
	for i := range data {
		data[i] = 'X'
	}
	disp.Write(data)
	row, col := disp.CursorPosition()
	if row != 1 || col != 0 {
		t.Errorf("Cursor after full row = (%d,%d), want (1,0)", row, col)
	}
	// Write one more character
	disp.Write([]byte("Y"))
	if disp.CharAt(1, 0) != 'Y' {
		t.Errorf("CharAt(1,0) = %c, want Y", disp.CharAt(1, 0))
	}
}

func TestSimulatedDisplayScroll(t *testing.T) {
	disp := NewSimulatedDisplay("display0", 0)
	disp.Init()
	// Fill all 25 rows with different characters
	for row := 0; row < DisplayRows; row++ {
		data := make([]byte, DisplayCols)
		for i := range data {
			data[i] = byte('A' + row)
		}
		disp.Write(data)
	}
	// After scrolling: row 0 should contain 'B' (was row 1)
	if disp.CharAt(0, 0) != 'B' {
		t.Errorf("CharAt(0,0) after scroll = %c, want B", disp.CharAt(0, 0))
	}
	// Row 23 should contain what was in row 24
	if disp.CharAt(DisplayRows-2, 0) != byte('A'+DisplayRows-1) {
		t.Errorf("CharAt(%d,0) = %c, want %c",
			DisplayRows-2, disp.CharAt(DisplayRows-2, 0), byte('A'+DisplayRows-1))
	}
	// Last row should be cleared
	if disp.CharAt(DisplayRows-1, 0) != 0x20 {
		t.Errorf("Last row should be spaces after scroll, got 0x%02X", disp.CharAt(DisplayRows-1, 0))
	}
}

func TestSimulatedDisplayReadReturnsZero(t *testing.T) {
	disp := NewSimulatedDisplay("display0", 0)
	buf := make([]byte, 10)
	n := disp.Read(buf)
	if n != 0 {
		t.Errorf("Read returned %d, want 0", n)
	}
}

func TestSimulatedDisplayClearScreen(t *testing.T) {
	disp := NewSimulatedDisplay("display0", 0)
	disp.Write([]byte("Hello World"))
	disp.ClearScreen()
	row, col := disp.CursorPosition()
	if row != 0 || col != 0 {
		t.Errorf("Cursor after clear = (%d,%d), want (0,0)", row, col)
	}
	for i := 0; i < 11; i++ {
		if disp.CharAt(0, i) != 0x20 {
			t.Errorf("CharAt(0,%d) = 0x%02X, want 0x20", i, disp.CharAt(0, i))
		}
	}
}

func TestSimulatedDisplayWriteEmpty(t *testing.T) {
	disp := NewSimulatedDisplay("display0", 0)
	disp.Init()
	n := disp.Write([]byte{})
	if n != 0 {
		t.Errorf("Write(empty) returned %d, want 0", n)
	}
}
