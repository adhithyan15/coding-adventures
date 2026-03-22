package fpga

import (
	"testing"
)

// =========================================================================
// IOBlock Tests
// =========================================================================

func TestIOBlock_InputMode(t *testing.T) {
	io := NewIOBlock("sensor_in", IOInput)

	io.DrivePad(1)
	if io.ReadInternal() != 1 {
		t.Errorf("Input mode: ReadInternal() = %d, want 1", io.ReadInternal())
	}

	pad := io.ReadPad()
	if pad == nil || *pad != 1 {
		t.Errorf("Input mode: ReadPad() = %v, want &1", pad)
	}
}

func TestIOBlock_OutputMode(t *testing.T) {
	io := NewIOBlock("led_0", IOOutput)

	io.DriveInternal(1)
	pad := io.ReadPad()
	if pad == nil || *pad != 1 {
		t.Errorf("Output mode: ReadPad() = %v, want &1", pad)
	}

	// ReadInternal in output mode returns the internal value
	if io.ReadInternal() != 1 {
		t.Errorf("Output mode: ReadInternal() = %d, want 1", io.ReadInternal())
	}
}

func TestIOBlock_TristateMode(t *testing.T) {
	io := NewIOBlock("bus_0", IOTristate)

	io.DriveInternal(1)
	pad := io.ReadPad()
	if pad != nil {
		t.Errorf("Tristate mode: ReadPad() = %v, want nil", pad)
	}

	// ReadInternal in tristate returns the internal value
	if io.ReadInternal() != 1 {
		t.Errorf("Tristate mode: ReadInternal() = %d, want 1", io.ReadInternal())
	}
}

func TestIOBlock_Configure(t *testing.T) {
	io := NewIOBlock("pin", IOInput)
	if io.Mode() != IOInput {
		t.Errorf("Mode() = %v, want IOInput", io.Mode())
	}

	io.Configure(IOOutput)
	if io.Mode() != IOOutput {
		t.Errorf("After configure: Mode() = %v, want IOOutput", io.Mode())
	}

	io.Configure(IOTristate)
	if io.Mode() != IOTristate {
		t.Errorf("After configure: Mode() = %v, want IOTristate", io.Mode())
	}
}

func TestIOBlock_Properties(t *testing.T) {
	io := NewIOBlock("my_pin", IOInput)
	if io.Name() != "my_pin" {
		t.Errorf("Name() = %q, want 'my_pin'", io.Name())
	}
}

func TestIOBlock_Invalid(t *testing.T) {
	assertPanics(t, "empty name", func() { NewIOBlock("", IOInput) })
	io := NewIOBlock("pin", IOInput)
	assertPanics(t, "bad pad value", func() { io.DrivePad(2) })
	assertPanics(t, "bad internal value", func() { io.DriveInternal(2) })
}

func TestIOMode_String(t *testing.T) {
	if IOInput.String() != "input" {
		t.Errorf("IOInput.String() = %q, want 'input'", IOInput.String())
	}
	if IOOutput.String() != "output" {
		t.Errorf("IOOutput.String() = %q, want 'output'", IOOutput.String())
	}
	if IOTristate.String() != "tristate" {
		t.Errorf("IOTristate.String() = %q, want 'tristate'", IOTristate.String())
	}
	// Unknown mode
	unknown := IOMode(99)
	if unknown.String() == "" {
		t.Error("Unknown IOMode should have a non-empty string")
	}
}
