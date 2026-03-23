package devicedriverframework

// =========================================================================
// SimulatedKeyboard -- a character device representing a keyboard
// =========================================================================
//
// When you press a key on a physical keyboard:
//   1. The keyboard controller detects the key press.
//   2. It generates a "scan code" (a number identifying the key).
//   3. It raises interrupt 33 (IRQ 1 on the original IBM PC).
//   4. The CPU runs the keyboard ISR (Interrupt Service Routine).
//   5. The ISR reads the scan code and translates it to ASCII.
//   6. The ISR deposits the character into the keyboard buffer.
//   7. The CPU resumes what it was doing.
//
// Our simulation skips the physical hardware. Test code directly pushes
// characters into the buffer using InjectKeystrokes(). Reading from the
// buffer works identically to a real keyboard.
//
// The keyboard is READ-ONLY: Write() always returns -1. You cannot force
// a key to be pressed by writing bytes to the keyboard!

// SimulatedKeyboard is a simulated keyboard backed by an in-memory buffer.
type SimulatedKeyboard struct {
	DeviceBase
	buffer []byte // FIFO queue of keystroke bytes
}

// NewSimulatedKeyboard creates a new simulated keyboard.
func NewSimulatedKeyboard(name string, minor int) *SimulatedKeyboard {
	return &SimulatedKeyboard{
		DeviceBase: DeviceBase{
			Name:            name,
			Type:            DeviceCharacter,
			Major:           MajorKeyboard,
			Minor:           minor,
			InterruptNumber: IntKeyboard,
		},
		buffer: nil,
	}
}

// Init initializes the keyboard by clearing the input buffer.
func (k *SimulatedKeyboard) Init() {
	k.buffer = nil
	k.Initialized = true
}

// InjectKeystrokes simulates key presses by pushing bytes into the buffer.
// This replaces the physical keyboard + ISR pipeline.
func (k *SimulatedKeyboard) InjectKeystrokes(data []byte) {
	k.buffer = append(k.buffer, data...)
}

// Read reads up to len(buf) keystrokes from the buffer.
// Returns the number of bytes actually read. If the buffer is empty,
// returns 0 (non-blocking).
func (k *SimulatedKeyboard) Read(buf []byte) int {
	count := len(buf)
	if count > len(k.buffer) {
		count = len(k.buffer)
	}
	copy(buf[:count], k.buffer[:count])
	k.buffer = k.buffer[count:]
	return count
}

// Write attempts to write to the keyboard (always fails).
// Returns -1 because keyboards are input-only devices.
func (k *SimulatedKeyboard) Write(data []byte) int {
	return -1
}

// BufferSize returns the number of keystrokes currently in the buffer.
func (k *SimulatedKeyboard) BufferSize() int {
	return len(k.buffer)
}
