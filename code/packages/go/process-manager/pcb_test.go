package processmanager

import "testing"

// =============================================================================
// ProcessState Tests
// =============================================================================

func TestProcessStateValues(t *testing.T) {
	// Verify that each state has the expected integer value.
	// These values are used for comparisons and must remain stable.
	tests := []struct {
		name  string
		state ProcessState
		want  int
	}{
		{"Ready", Ready, 0},
		{"Running", Running, 1},
		{"Blocked", Blocked, 2},
		{"Terminated", Terminated, 3},
		{"Zombie", Zombie, 4},
	}
	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			if int(tt.state) != tt.want {
				t.Errorf("ProcessState.%s = %d, want %d", tt.name, tt.state, tt.want)
			}
		})
	}
}

// =============================================================================
// ProcessControlBlock Tests
// =============================================================================

func TestNewPCBDefaults(t *testing.T) {
	pcb := NewPCB(42, "test_proc")

	if pcb.PID != 42 {
		t.Errorf("PID = %d, want 42", pcb.PID)
	}
	if pcb.Name != "test_proc" {
		t.Errorf("Name = %q, want %q", pcb.Name, "test_proc")
	}
	if pcb.State != Ready {
		t.Errorf("State = %d, want Ready (%d)", pcb.State, Ready)
	}
	if pcb.ParentPID != -1 {
		t.Errorf("ParentPID = %d, want -1", pcb.ParentPID)
	}
	if pcb.Priority != DefaultPriority {
		t.Errorf("Priority = %d, want %d", pcb.Priority, DefaultPriority)
	}
	if pcb.CPUTime != 0 {
		t.Errorf("CPUTime = %d, want 0", pcb.CPUTime)
	}
	if pcb.ExitCode != 0 {
		t.Errorf("ExitCode = %d, want 0", pcb.ExitCode)
	}
	if pcb.PC != 0 {
		t.Errorf("PC = %d, want 0", pcb.PC)
	}
	if pcb.SP != 0 {
		t.Errorf("SP = %d, want 0", pcb.SP)
	}
	if pcb.MemoryBase != 0 {
		t.Errorf("MemoryBase = %d, want 0", pcb.MemoryBase)
	}
	if pcb.MemorySize != 0 {
		t.Errorf("MemorySize = %d, want 0", pcb.MemorySize)
	}
}

func TestNewPCBRegistersZeroed(t *testing.T) {
	// All 32 RISC-V registers should be initialized to 0.
	pcb := NewPCB(0, "")
	for i, v := range pcb.Registers {
		if v != 0 {
			t.Errorf("Registers[%d] = %d, want 0", i, v)
		}
	}
}

func TestNewPCBEmptyCollections(t *testing.T) {
	pcb := NewPCB(0, "")

	if len(pcb.Children) != 0 {
		t.Errorf("Children should be empty, got %d", len(pcb.Children))
	}
	if len(pcb.PendingSignals) != 0 {
		t.Errorf("PendingSignals should be empty, got %d", len(pcb.PendingSignals))
	}
	if len(pcb.SignalHandlers) != 0 {
		t.Errorf("SignalHandlers should be empty, got %d", len(pcb.SignalHandlers))
	}
	if len(pcb.SignalMask) != 0 {
		t.Errorf("SignalMask should be empty, got %d", len(pcb.SignalMask))
	}
}

func TestPCBRegisterIndependence(t *testing.T) {
	// Each PCB should have its own independent register set.
	pcb1 := NewPCB(0, "a")
	pcb2 := NewPCB(1, "b")

	pcb1.Registers[10] = 42
	if pcb2.Registers[10] != 0 {
		t.Error("Modifying pcb1 registers affected pcb2")
	}
}

func TestPCBStateTransition(t *testing.T) {
	pcb := NewPCB(0, "")
	if pcb.State != Ready {
		t.Fatalf("Initial state should be Ready")
	}

	pcb.State = Running
	if pcb.State != Running {
		t.Error("State transition to Running failed")
	}

	pcb.State = Zombie
	if pcb.State != Zombie {
		t.Error("State transition to Zombie failed")
	}
}
