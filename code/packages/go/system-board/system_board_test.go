package systemboard

import (
	"testing"
)

// =========================================================================
// THE CRITICAL TEST: Boot to Hello World
// =========================================================================
//
// This is the proof that the entire stack works: from BIOS initialization
// through bootloader kernel loading, kernel initialization, hello-world
// execution, sys_write to display, and sys_exit to idle.

func TestBootToHelloWorld(t *testing.T) {
	board := NewSystemBoard(DefaultSystemConfig())
	board.PowerOn()
	board.Run(100000)

	snap := board.DisplaySnapshot()
	if snap == nil {
		t.Fatal("Display snapshot is nil")
	}
	if !snap.Contains("Hello World") {
		t.Fatalf("Expected 'Hello World' on display, got: %q", snap.String())
	}
}

func TestBootToHelloWorldIdleAfter(t *testing.T) {
	board := NewSystemBoard(DefaultSystemConfig())
	board.PowerOn()
	board.Run(100000)

	if !board.IsIdle() {
		t.Fatal("System should be idle after hello-world terminates")
	}
}

// =========================================================================
// Power-On Tests
// =========================================================================

func TestNewSystemBoard(t *testing.T) {
	config := DefaultSystemConfig()
	board := NewSystemBoard(config)
	if board == nil {
		t.Fatal("NewSystemBoard returned nil")
	}
	if board.Powered {
		t.Fatal("Board should not be powered before PowerOn()")
	}
}

func TestPowerOn(t *testing.T) {
	board := NewSystemBoard(DefaultSystemConfig())
	board.PowerOn()

	if !board.Powered {
		t.Fatal("Board should be powered after PowerOn()")
	}
	if board.CPU == nil {
		t.Fatal("CPU should be initialized")
	}
	if board.Display == nil {
		t.Fatal("Display should be initialized")
	}
	if board.InterruptCtrl == nil {
		t.Fatal("InterruptCtrl should be initialized")
	}
	if board.Kernel == nil {
		t.Fatal("Kernel should be initialized")
	}
	if board.DiskImage == nil {
		t.Fatal("DiskImage should be initialized")
	}
}

func TestDoublePowerOn(t *testing.T) {
	board := NewSystemBoard(DefaultSystemConfig())
	board.PowerOn()
	board.PowerOn() // Should not crash (idempotent)
	if !board.Powered {
		t.Fatal("Board should still be powered")
	}
}

// =========================================================================
// Phase Transition Tests
// =========================================================================

func TestBootPhases(t *testing.T) {
	board := NewSystemBoard(DefaultSystemConfig())
	board.PowerOn()
	board.Run(100000)

	phases := board.Trace.Phases()
	if len(phases) == 0 {
		t.Fatal("Trace should have phases")
	}

	// Should at least have PowerOn and BIOS.
	foundPowerOn := false
	foundBIOS := false
	for _, p := range phases {
		if p == PhasePowerOn {
			foundPowerOn = true
		}
		if p == PhaseBIOS {
			foundBIOS = true
		}
	}
	if !foundPowerOn {
		t.Error("Missing PowerOn phase")
	}
	if !foundBIOS {
		t.Error("Missing BIOS phase")
	}
}

func TestBootPhaseOrder(t *testing.T) {
	board := NewSystemBoard(DefaultSystemConfig())
	board.PowerOn()
	board.Run(100000)

	phases := board.Trace.Phases()
	// Verify ordering: each phase should start at a later cycle than the previous.
	for i := 1; i < len(phases); i++ {
		prevStart := board.Trace.PhaseStartCycle(phases[i-1])
		currStart := board.Trace.PhaseStartCycle(phases[i])
		if currStart < prevStart {
			t.Errorf("Phase %v (cycle %d) started before %v (cycle %d)",
				phases[i], currStart, phases[i-1], prevStart)
		}
	}
}

// =========================================================================
// Boot Trace Tests
// =========================================================================

func TestBootTraceHasEvents(t *testing.T) {
	board := NewSystemBoard(DefaultSystemConfig())
	board.PowerOn()
	board.Run(100000)

	if len(board.Trace.Events) == 0 {
		t.Fatal("Boot trace should have events")
	}
}

func TestBootTraceEventsHaveDescriptions(t *testing.T) {
	board := NewSystemBoard(DefaultSystemConfig())
	board.PowerOn()
	board.Run(100000)

	for i, e := range board.Trace.Events {
		if e.Description == "" {
			t.Errorf("Event %d has empty description", i)
		}
	}
}

func TestBootTraceTotalCycles(t *testing.T) {
	board := NewSystemBoard(DefaultSystemConfig())
	board.PowerOn()
	board.Run(100000)

	total := board.Trace.TotalCycles()
	if total <= 0 {
		t.Fatalf("TotalCycles = %d, expected positive", total)
	}
}

func TestBootTracePhaseStartCycle(t *testing.T) {
	board := NewSystemBoard(DefaultSystemConfig())
	board.PowerOn()
	board.Run(100000)

	powerOnCycle := board.Trace.PhaseStartCycle(PhasePowerOn)
	if powerOnCycle != 0 {
		t.Errorf("PowerOn phase should start at cycle 0, got %d", powerOnCycle)
	}

	// Non-existent phase should return -1.
	missing := board.Trace.PhaseStartCycle(BootPhase(99))
	if missing != -1 {
		t.Errorf("Missing phase should return -1, got %d", missing)
	}
}

func TestBootTraceEventsInPhase(t *testing.T) {
	board := NewSystemBoard(DefaultSystemConfig())
	board.PowerOn()
	board.Run(100000)

	events := board.Trace.EventsInPhase(PhasePowerOn)
	if len(events) == 0 {
		t.Error("Should have PowerOn events")
	}
}

// =========================================================================
// Display Tests
// =========================================================================

func TestDisplayAfterBoot(t *testing.T) {
	board := NewSystemBoard(DefaultSystemConfig())
	board.PowerOn()
	board.Run(100000)

	snap := board.DisplaySnapshot()
	if snap == nil {
		t.Fatal("Display snapshot should not be nil")
	}

	// Should contain Hello World.
	if !snap.Contains("Hello World") {
		t.Fatalf("Display should contain 'Hello World', got: %q", snap.LineAt(0))
	}
}

// =========================================================================
// Keystroke Injection Tests
// =========================================================================

func TestInjectKeystroke(t *testing.T) {
	board := NewSystemBoard(DefaultSystemConfig())
	board.PowerOn()
	board.Run(100000)

	board.InjectKeystroke('A')
	if len(board.Kernel.KeyboardBuffer) != 1 {
		t.Fatalf("Keyboard buffer length = %d, expected 1", len(board.Kernel.KeyboardBuffer))
	}
	if board.Kernel.KeyboardBuffer[0] != 'A' {
		t.Fatalf("Keyboard buffer = %q, expected 'A'", string(board.Kernel.KeyboardBuffer))
	}
}

func TestInjectMultipleKeystrokes(t *testing.T) {
	board := NewSystemBoard(DefaultSystemConfig())
	board.PowerOn()
	board.Run(100000)

	board.InjectKeystroke('H')
	board.InjectKeystroke('i')
	if string(board.Kernel.KeyboardBuffer) != "Hi" {
		t.Fatalf("Keyboard buffer = %q, expected 'Hi'", string(board.Kernel.KeyboardBuffer))
	}
}

// =========================================================================
// Cycle Count Tests
// =========================================================================

func TestCycleCountPositive(t *testing.T) {
	board := NewSystemBoard(DefaultSystemConfig())
	board.PowerOn()
	board.Run(100000)

	cycles := board.GetCycleCount()
	if cycles <= 0 {
		t.Fatalf("Cycle count = %d, expected positive", cycles)
	}
}

func TestCycleBudgetRespected(t *testing.T) {
	board := NewSystemBoard(DefaultSystemConfig())
	board.PowerOn()
	board.Run(100000)

	if board.GetCycleCount() > 100000 {
		t.Fatalf("Exceeded cycle budget: %d > 100000", board.GetCycleCount())
	}
}

// =========================================================================
// Error Handling Tests
// =========================================================================

func TestStepBeforePowerOn(t *testing.T) {
	board := NewSystemBoard(DefaultSystemConfig())
	board.Step() // Should not crash
	if board.Cycle != 0 {
		t.Fatal("Step before PowerOn should not advance cycle")
	}
}

func TestRunBeforePowerOn(t *testing.T) {
	board := NewSystemBoard(DefaultSystemConfig())
	trace := board.Run(100)
	if trace == nil {
		t.Fatal("Run should return non-nil trace even before PowerOn")
	}
}

func TestRunZeroCycles(t *testing.T) {
	board := NewSystemBoard(DefaultSystemConfig())
	board.PowerOn()
	board.Run(0)
	if board.GetCycleCount() != 0 {
		t.Fatalf("Run(0) should execute 0 cycles, got %d", board.GetCycleCount())
	}
}

// =========================================================================
// Config Tests
// =========================================================================

func TestDefaultSystemConfig(t *testing.T) {
	config := DefaultSystemConfig()
	if config.MemorySize != 1024*1024 {
		t.Errorf("MemorySize = %d, expected 1MB", config.MemorySize)
	}
	if config.DisplayConfig.Columns != 80 {
		t.Errorf("Display columns = %d, expected 80", config.DisplayConfig.Columns)
	}
	if config.DisplayConfig.Rows != 25 {
		t.Errorf("Display rows = %d, expected 25", config.DisplayConfig.Rows)
	}
}

// =========================================================================
// Boot Phase String Tests
// =========================================================================

func TestBootPhaseString(t *testing.T) {
	tests := []struct {
		phase BootPhase
		want  string
	}{
		{PhasePowerOn, "PowerOn"},
		{PhaseBIOS, "BIOS"},
		{PhaseBootloader, "Bootloader"},
		{PhaseKernelInit, "KernelInit"},
		{PhaseUserProgram, "UserProgram"},
		{PhaseIdle, "Idle"},
		{BootPhase(99), "Unknown"},
	}
	for _, tt := range tests {
		if got := tt.phase.String(); got != tt.want {
			t.Errorf("BootPhase(%d).String() = %q, want %q", tt.phase, got, tt.want)
		}
	}
}
