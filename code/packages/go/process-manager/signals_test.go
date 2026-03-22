package processmanager

import "testing"

// =============================================================================
// Signal Constant Tests
// =============================================================================

func TestSignalNumbers(t *testing.T) {
	// Verify POSIX signal numbers are correct.
	tests := []struct {
		name   string
		signal int
		want   int
	}{
		{"SIGINT", SIGINT, 2},
		{"SIGKILL", SIGKILL, 9},
		{"SIGTERM", SIGTERM, 15},
		{"SIGCHLD", SIGCHLD, 17},
		{"SIGCONT", SIGCONT, 18},
		{"SIGSTOP", SIGSTOP, 19},
	}
	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			if tt.signal != tt.want {
				t.Errorf("%s = %d, want %d", tt.name, tt.signal, tt.want)
			}
		})
	}
}

// =============================================================================
// SignalManager Tests
// =============================================================================

func TestSendSignalAddsToPending(t *testing.T) {
	sm := NewSignalManager()
	pcb := NewPCB(1, "test")

	sm.SendSignal(pcb, SIGTERM)

	if len(pcb.PendingSignals) != 1 || pcb.PendingSignals[0] != SIGTERM {
		t.Errorf("Expected SIGTERM in pending, got %v", pcb.PendingSignals)
	}
}

func TestSendMultipleSignals(t *testing.T) {
	sm := NewSignalManager()
	pcb := NewPCB(1, "test")

	sm.SendSignal(pcb, SIGTERM)
	sm.SendSignal(pcb, SIGINT)

	if len(pcb.PendingSignals) != 2 {
		t.Errorf("Expected 2 pending signals, got %d", len(pcb.PendingSignals))
	}
}

func TestSendToTerminatedFails(t *testing.T) {
	sm := NewSignalManager()
	pcb := NewPCB(1, "test")
	pcb.State = Terminated

	ok := sm.SendSignal(pcb, SIGTERM)
	if ok {
		t.Error("Expected false for sending to terminated process")
	}
}

func TestSigkillImmediatelyTerminates(t *testing.T) {
	sm := NewSignalManager()
	pcb := NewPCB(1, "test")
	pcb.State = Running

	sm.SendSignal(pcb, SIGKILL)

	if pcb.State != Zombie {
		t.Errorf("State = %d, want Zombie (%d)", pcb.State, Zombie)
	}
	if len(pcb.PendingSignals) != 0 {
		t.Error("SIGKILL should not be added to pending")
	}
}

func TestSigstopImmediatelyBlocks(t *testing.T) {
	sm := NewSignalManager()
	pcb := NewPCB(1, "test")
	pcb.State = Running

	sm.SendSignal(pcb, SIGSTOP)

	if pcb.State != Blocked {
		t.Errorf("State = %d, want Blocked (%d)", pcb.State, Blocked)
	}
	if len(pcb.PendingSignals) != 0 {
		t.Error("SIGSTOP should not be added to pending")
	}
}

func TestSigcontResumesBlockedProcess(t *testing.T) {
	sm := NewSignalManager()
	pcb := NewPCB(1, "test")
	pcb.State = Blocked

	sm.SendSignal(pcb, SIGCONT)

	if pcb.State != Ready {
		t.Errorf("State = %d, want Ready (%d)", pcb.State, Ready)
	}
}

func TestSigcontAddedToPending(t *testing.T) {
	sm := NewSignalManager()
	pcb := NewPCB(1, "test")

	sm.SendSignal(pcb, SIGCONT)

	found := false
	for _, sig := range pcb.PendingSignals {
		if sig == SIGCONT {
			found = true
		}
	}
	if !found {
		t.Error("SIGCONT should be added to pending for handler delivery")
	}
}

func TestDeliverPendingWithHandler(t *testing.T) {
	sm := NewSignalManager()
	pcb := NewPCB(1, "test")
	pcb.SignalHandlers[SIGTERM] = 0x40000
	pcb.PendingSignals = append(pcb.PendingSignals, SIGTERM)

	sig, addr, delivered := sm.DeliverPending(pcb)

	if !delivered {
		t.Error("Expected delivery")
	}
	if sig != SIGTERM {
		t.Errorf("Signal = %d, want SIGTERM (%d)", sig, SIGTERM)
	}
	if addr != 0x40000 {
		t.Errorf("Handler addr = %d, want 0x40000", addr)
	}
}

func TestDeliverRemovesFromPending(t *testing.T) {
	sm := NewSignalManager()
	pcb := NewPCB(1, "test")
	pcb.PendingSignals = append(pcb.PendingSignals, SIGTERM)

	sm.DeliverPending(pcb)

	for _, sig := range pcb.PendingSignals {
		if sig == SIGTERM {
			t.Error("SIGTERM should be removed from pending after delivery")
		}
	}
}

func TestDeliverFatalWithoutHandler(t *testing.T) {
	sm := NewSignalManager()
	pcb := NewPCB(1, "test")
	pcb.PendingSignals = append(pcb.PendingSignals, SIGTERM)

	sig, _, delivered := sm.DeliverPending(pcb)

	if !delivered {
		t.Error("Expected delivery for fatal signal")
	}
	if sig != SIGTERM {
		t.Errorf("Signal = %d, want SIGTERM", sig)
	}
	if pcb.State != Zombie {
		t.Errorf("State = %d, want Zombie", pcb.State)
	}
}

func TestDeliverNonfatalWithoutHandler(t *testing.T) {
	sm := NewSignalManager()
	pcb := NewPCB(1, "test")
	pcb.PendingSignals = append(pcb.PendingSignals, SIGCHLD)

	_, _, delivered := sm.DeliverPending(pcb)

	if delivered {
		t.Error("Non-fatal signal without handler should not count as delivered")
	}
	if pcb.State != Ready {
		t.Errorf("State should remain Ready, got %d", pcb.State)
	}
}

func TestDeliverNoPending(t *testing.T) {
	sm := NewSignalManager()
	pcb := NewPCB(1, "test")

	_, _, delivered := sm.DeliverPending(pcb)

	if delivered {
		t.Error("Expected no delivery when no signals pending")
	}
}

func TestMaskedSignalNotDelivered(t *testing.T) {
	sm := NewSignalManager()
	pcb := NewPCB(1, "test")
	sm.MaskSignal(pcb, SIGTERM)
	pcb.PendingSignals = append(pcb.PendingSignals, SIGTERM)

	_, _, delivered := sm.DeliverPending(pcb)

	if delivered {
		t.Error("Masked signal should not be delivered")
	}
	if len(pcb.PendingSignals) != 1 {
		t.Error("Masked signal should remain in pending")
	}
}

func TestUnmaskAllowsDelivery(t *testing.T) {
	sm := NewSignalManager()
	pcb := NewPCB(1, "test")
	sm.MaskSignal(pcb, SIGTERM)
	pcb.PendingSignals = append(pcb.PendingSignals, SIGTERM)

	// While masked: no delivery.
	_, _, delivered := sm.DeliverPending(pcb)
	if delivered {
		t.Error("Should not deliver while masked")
	}

	// Unmask and deliver.
	sm.UnmaskSignal(pcb, SIGTERM)
	_, _, delivered = sm.DeliverPending(pcb)

	// SIGTERM without handler is fatal — process becomes zombie.
	if pcb.State != Zombie {
		t.Error("After unmasking, SIGTERM should terminate the process")
	}
}

func TestSigkillCannotBeMasked(t *testing.T) {
	sm := NewSignalManager()
	pcb := NewPCB(1, "test")
	sm.MaskSignal(pcb, SIGKILL)

	if pcb.SignalMask[SIGKILL] {
		t.Error("SIGKILL should not be maskable")
	}
}

func TestSigstopCannotBeMasked(t *testing.T) {
	sm := NewSignalManager()
	pcb := NewPCB(1, "test")
	sm.MaskSignal(pcb, SIGSTOP)

	if pcb.SignalMask[SIGSTOP] {
		t.Error("SIGSTOP should not be maskable")
	}
}

func TestRegisterHandler(t *testing.T) {
	sm := NewSignalManager()
	pcb := NewPCB(1, "test")

	sm.RegisterHandler(pcb, SIGTERM, 0x40000)

	if pcb.SignalHandlers[SIGTERM] != 0x40000 {
		t.Errorf("Handler = %d, want 0x40000", pcb.SignalHandlers[SIGTERM])
	}
}

func TestRegisterHandlerSigkillIgnored(t *testing.T) {
	sm := NewSignalManager()
	pcb := NewPCB(1, "test")

	sm.RegisterHandler(pcb, SIGKILL, 0x40000)

	if _, exists := pcb.SignalHandlers[SIGKILL]; exists {
		t.Error("Should not be able to register handler for SIGKILL")
	}
}

func TestRegisterHandlerSigstopIgnored(t *testing.T) {
	sm := NewSignalManager()
	pcb := NewPCB(1, "test")

	sm.RegisterHandler(pcb, SIGSTOP, 0x40000)

	if _, exists := pcb.SignalHandlers[SIGSTOP]; exists {
		t.Error("Should not be able to register handler for SIGSTOP")
	}
}

func TestIsFatal(t *testing.T) {
	sm := NewSignalManager()

	fatals := []int{SIGINT, SIGKILL, SIGTERM}
	for _, sig := range fatals {
		if !sm.IsFatal(sig) {
			t.Errorf("Signal %d should be fatal", sig)
		}
	}

	nonFatals := []int{SIGCHLD, SIGCONT, SIGSTOP}
	for _, sig := range nonFatals {
		if sm.IsFatal(sig) {
			t.Errorf("Signal %d should not be fatal", sig)
		}
	}
}
