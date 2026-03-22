package processmanager

import "testing"

// =============================================================================
// CreateProcess Tests
// =============================================================================

func TestCreateProcessFirstPID(t *testing.T) {
	pm := NewProcessManager()
	pcb := pm.CreateProcess("init", -1, 0, 0, 0)
	if pcb.PID != 0 {
		t.Errorf("First PID = %d, want 0", pcb.PID)
	}
}

func TestCreateProcessSequentialPIDs(t *testing.T) {
	pm := NewProcessManager()
	p0 := pm.CreateProcess("init", -1, 0, 0, 0)
	p1 := pm.CreateProcess("shell", 0, 20, 0, 0)
	if p0.PID != 0 || p1.PID != 1 {
		t.Errorf("PIDs = (%d, %d), want (0, 1)", p0.PID, p1.PID)
	}
}

func TestCreateProcessWithParent(t *testing.T) {
	pm := NewProcessManager()
	parent := pm.CreateProcess("init", -1, 0, 0, 0)
	child := pm.CreateProcess("shell", parent.PID, 20, 0, 0)

	if child.ParentPID != parent.PID {
		t.Errorf("ParentPID = %d, want %d", child.ParentPID, parent.PID)
	}
	found := false
	for _, c := range parent.Children {
		if c == child.PID {
			found = true
		}
	}
	if !found {
		t.Error("Child not in parent's children list")
	}
}

func TestCreateProcessState(t *testing.T) {
	pm := NewProcessManager()
	pcb := pm.CreateProcess("test", -1, 20, 0, 0)
	if pcb.State != Ready {
		t.Errorf("State = %d, want Ready", pcb.State)
	}
}

func TestProcessCount(t *testing.T) {
	pm := NewProcessManager()
	if pm.ProcessCount() != 0 {
		t.Error("Empty manager should have 0 processes")
	}
	pm.CreateProcess("a", -1, 20, 0, 0)
	if pm.ProcessCount() != 1 {
		t.Error("Should have 1 process")
	}
	pm.CreateProcess("b", -1, 20, 0, 0)
	if pm.ProcessCount() != 2 {
		t.Error("Should have 2 processes")
	}
}

// =============================================================================
// Fork Tests
// =============================================================================

func TestForkReturns(t *testing.T) {
	pm := NewProcessManager()
	init := pm.CreateProcess("init", -1, 0, 0, 0)

	childPID, childRet, ok := pm.Fork(init.PID)
	if !ok {
		t.Fatal("Fork failed")
	}
	if childPID <= 0 {
		t.Errorf("childPID = %d, want > 0", childPID)
	}
	if childRet != 0 {
		t.Errorf("childReturn = %d, want 0", childRet)
	}
}

func TestForkChildPIDDifferent(t *testing.T) {
	pm := NewProcessManager()
	init := pm.CreateProcess("init", -1, 0, 0, 0)

	childPID, _, _ := pm.Fork(init.PID)
	if childPID == init.PID {
		t.Error("Child PID should differ from parent PID")
	}
}

func TestForkChildParentPID(t *testing.T) {
	pm := NewProcessManager()
	init := pm.CreateProcess("init", -1, 0, 0, 0)

	childPID, _, _ := pm.Fork(init.PID)
	child := pm.GetProcess(childPID)
	if child.ParentPID != init.PID {
		t.Errorf("Child ParentPID = %d, want %d", child.ParentPID, init.PID)
	}
}

func TestForkChildInParentChildren(t *testing.T) {
	pm := NewProcessManager()
	init := pm.CreateProcess("init", -1, 0, 0, 0)

	childPID, _, _ := pm.Fork(init.PID)

	found := false
	for _, c := range init.Children {
		if c == childPID {
			found = true
		}
	}
	if !found {
		t.Error("Child should appear in parent's children list")
	}
}

func TestForkChildStateReady(t *testing.T) {
	pm := NewProcessManager()
	init := pm.CreateProcess("init", -1, 0, 0, 0)

	childPID, _, _ := pm.Fork(init.PID)
	child := pm.GetProcess(childPID)
	if child.State != Ready {
		t.Errorf("Child state = %d, want Ready", child.State)
	}
}

func TestForkInheritsRegisters(t *testing.T) {
	pm := NewProcessManager()
	init := pm.CreateProcess("init", -1, 0, 0, 0)
	init.Registers[10] = 42
	init.Registers[15] = 99

	childPID, _, _ := pm.Fork(init.PID)
	child := pm.GetProcess(childPID)

	if child.Registers[10] != 42 || child.Registers[15] != 99 {
		t.Error("Child should inherit parent's registers")
	}
}

func TestForkRegistersIndependent(t *testing.T) {
	pm := NewProcessManager()
	init := pm.CreateProcess("init", -1, 0, 0, 0)
	init.Registers[10] = 42

	childPID, _, _ := pm.Fork(init.PID)
	child := pm.GetProcess(childPID)
	child.Registers[10] = 999

	if init.Registers[10] != 42 {
		t.Error("Modifying child registers should not affect parent")
	}
}

func TestForkInheritsPCAndPriority(t *testing.T) {
	pm := NewProcessManager()
	init := pm.CreateProcess("init", -1, 5, 0, 0)
	init.PC = 0x10000

	childPID, _, _ := pm.Fork(init.PID)
	child := pm.GetProcess(childPID)

	if child.PC != 0x10000 {
		t.Errorf("Child PC = %d, want 0x10000", child.PC)
	}
	if child.Priority != 5 {
		t.Errorf("Child Priority = %d, want 5", child.Priority)
	}
}

func TestForkCPUTimeReset(t *testing.T) {
	pm := NewProcessManager()
	init := pm.CreateProcess("init", -1, 0, 0, 0)
	init.CPUTime = 500

	childPID, _, _ := pm.Fork(init.PID)
	child := pm.GetProcess(childPID)

	if child.CPUTime != 0 {
		t.Errorf("Child CPUTime = %d, want 0", child.CPUTime)
	}
}

func TestForkEmptyChildren(t *testing.T) {
	pm := NewProcessManager()
	init := pm.CreateProcess("init", -1, 0, 0, 0)

	childPID, _, _ := pm.Fork(init.PID)
	child := pm.GetProcess(childPID)

	if len(child.Children) != 0 {
		t.Error("Child should start with empty children list")
	}
}

func TestForkNoPendingSignals(t *testing.T) {
	pm := NewProcessManager()
	init := pm.CreateProcess("init", -1, 0, 0, 0)
	init.PendingSignals = append(init.PendingSignals, SIGTERM)

	childPID, _, _ := pm.Fork(init.PID)
	child := pm.GetProcess(childPID)

	if len(child.PendingSignals) != 0 {
		t.Error("Child should start with no pending signals")
	}
}

func TestForkInheritsSignalHandlers(t *testing.T) {
	pm := NewProcessManager()
	init := pm.CreateProcess("init", -1, 0, 0, 0)
	init.SignalHandlers[SIGTERM] = 0x40000

	childPID, _, _ := pm.Fork(init.PID)
	child := pm.GetProcess(childPID)

	if child.SignalHandlers[SIGTERM] != 0x40000 {
		t.Error("Child should inherit signal handlers")
	}
}

func TestForkNonexistentParent(t *testing.T) {
	pm := NewProcessManager()
	_, _, ok := pm.Fork(999)
	if ok {
		t.Error("Fork with nonexistent parent should fail")
	}
}

func TestForkInheritsMemory(t *testing.T) {
	pm := NewProcessManager()
	init := pm.CreateProcess("init", -1, 0, 0x20000, 8192)

	childPID, _, _ := pm.Fork(init.PID)
	child := pm.GetProcess(childPID)

	if child.MemoryBase != 0x20000 || child.MemorySize != 8192 {
		t.Error("Child should inherit parent's memory region")
	}
}

// =============================================================================
// Exec Tests
// =============================================================================

func TestExecSuccess(t *testing.T) {
	pm := NewProcessManager()
	proc := pm.CreateProcess("shell", -1, 10, 0, 0)
	proc.Registers[10] = 42

	ok := pm.Exec(proc.PID, 0x10000, 0x7FFFF000, 0, 0)
	if !ok {
		t.Fatal("Exec failed")
	}
}

func TestExecSetsPC(t *testing.T) {
	pm := NewProcessManager()
	proc := pm.CreateProcess("shell", -1, 20, 0, 0)

	pm.Exec(proc.PID, 0x10000, 0x7FFFF000, 0, 0)
	if proc.PC != 0x10000 {
		t.Errorf("PC = %d, want 0x10000", proc.PC)
	}
}

func TestExecSetsSP(t *testing.T) {
	pm := NewProcessManager()
	proc := pm.CreateProcess("shell", -1, 20, 0, 0)

	pm.Exec(proc.PID, 0x10000, 0x7FFFF000, 0, 0)
	if proc.SP != 0x7FFFF000 {
		t.Errorf("SP = %d, want 0x7FFFF000", proc.SP)
	}
}

func TestExecZeroesRegisters(t *testing.T) {
	pm := NewProcessManager()
	proc := pm.CreateProcess("shell", -1, 20, 0, 0)
	proc.Registers[10] = 42

	pm.Exec(proc.PID, 0x10000, 0x7FFFF000, 0, 0)
	for i, v := range proc.Registers {
		if v != 0 {
			t.Errorf("Registers[%d] = %d, want 0", i, v)
		}
	}
}

func TestExecClearsSignalHandlers(t *testing.T) {
	pm := NewProcessManager()
	proc := pm.CreateProcess("shell", -1, 20, 0, 0)
	proc.SignalHandlers[SIGTERM] = 0x40000

	pm.Exec(proc.PID, 0x10000, 0x7FFFF000, 0, 0)
	if len(proc.SignalHandlers) != 0 {
		t.Error("Signal handlers should be cleared after exec")
	}
}

func TestExecClearsPendingSignals(t *testing.T) {
	pm := NewProcessManager()
	proc := pm.CreateProcess("shell", -1, 20, 0, 0)
	proc.PendingSignals = append(proc.PendingSignals, SIGINT)

	pm.Exec(proc.PID, 0x10000, 0x7FFFF000, 0, 0)
	if len(proc.PendingSignals) != 0 {
		t.Error("Pending signals should be cleared after exec")
	}
}

func TestExecKeepsPID(t *testing.T) {
	pm := NewProcessManager()
	proc := pm.CreateProcess("shell", -1, 20, 0, 0)
	oldPID := proc.PID

	pm.Exec(proc.PID, 0x10000, 0x7FFFF000, 0, 0)
	if proc.PID != oldPID {
		t.Error("PID should not change after exec")
	}
}

func TestExecKeepsPriority(t *testing.T) {
	pm := NewProcessManager()
	proc := pm.CreateProcess("shell", -1, 10, 0, 0)

	pm.Exec(proc.PID, 0x10000, 0x7FFFF000, 0, 0)
	if proc.Priority != 10 {
		t.Errorf("Priority = %d, want 10", proc.Priority)
	}
}

func TestExecNonexistentPID(t *testing.T) {
	pm := NewProcessManager()
	ok := pm.Exec(999, 0x10000, 0x7FFFF000, 0, 0)
	if ok {
		t.Error("Exec with nonexistent PID should fail")
	}
}

func TestExecUpdatesMemory(t *testing.T) {
	pm := NewProcessManager()
	proc := pm.CreateProcess("shell", -1, 20, 0, 0)

	pm.Exec(proc.PID, 0x10000, 0x7FFFF000, 0x20000, 8192)
	if proc.MemoryBase != 0x20000 || proc.MemorySize != 8192 {
		t.Error("Memory should be updated by exec")
	}
}

// =============================================================================
// Wait Tests
// =============================================================================

func TestWaitNoZombie(t *testing.T) {
	pm := NewProcessManager()
	parent := pm.CreateProcess("shell", -1, 20, 0, 0)
	pm.Fork(parent.PID)

	_, _, ok := pm.Wait(parent.PID, -1)
	if ok {
		t.Error("Wait should fail when no zombie children")
	}
}

func TestWaitReapsZombie(t *testing.T) {
	pm := NewProcessManager()
	parent := pm.CreateProcess("shell", -1, 20, 0, 0)
	childPID, _, _ := pm.Fork(parent.PID)
	pm.ExitProcess(childPID, 42)

	reapedPID, exitCode, ok := pm.Wait(parent.PID, childPID)
	if !ok {
		t.Fatal("Wait should succeed")
	}
	if reapedPID != childPID || exitCode != 42 {
		t.Errorf("Got (%d, %d), want (%d, 42)", reapedPID, exitCode, childPID)
	}
}

func TestWaitRemovesFromProcessTable(t *testing.T) {
	pm := NewProcessManager()
	parent := pm.CreateProcess("shell", -1, 20, 0, 0)
	childPID, _, _ := pm.Fork(parent.PID)
	pm.ExitProcess(childPID, 0)
	pm.Wait(parent.PID, childPID)

	if pm.GetProcess(childPID) != nil {
		t.Error("Reaped child should be removed from process table")
	}
}

func TestWaitRemovesFromChildrenList(t *testing.T) {
	pm := NewProcessManager()
	parent := pm.CreateProcess("shell", -1, 20, 0, 0)
	childPID, _, _ := pm.Fork(parent.PID)
	pm.ExitProcess(childPID, 0)
	pm.Wait(parent.PID, childPID)

	for _, c := range parent.Children {
		if c == childPID {
			t.Error("Reaped child should be removed from parent's children")
		}
	}
}

func TestWaitAnyChild(t *testing.T) {
	pm := NewProcessManager()
	parent := pm.CreateProcess("shell", -1, 20, 0, 0)
	pm.Fork(parent.PID)                // child1
	child2PID, _, _ := pm.Fork(parent.PID) // child2
	pm.ExitProcess(child2PID, 7)

	reapedPID, exitCode, ok := pm.Wait(parent.PID, -1)
	if !ok {
		t.Fatal("Wait -1 should succeed")
	}
	if reapedPID != child2PID || exitCode != 7 {
		t.Errorf("Got (%d, %d), want (%d, 7)", reapedPID, exitCode, child2PID)
	}
}

func TestWaitNonexistentParent(t *testing.T) {
	pm := NewProcessManager()
	_, _, ok := pm.Wait(999, -1)
	if ok {
		t.Error("Wait with nonexistent parent should fail")
	}
}

// =============================================================================
// Kill Tests
// =============================================================================

func TestKillSigterm(t *testing.T) {
	pm := NewProcessManager()
	proc := pm.CreateProcess("daemon", -1, 20, 0, 0)

	ok := pm.Kill(proc.PID, SIGTERM)
	if !ok {
		t.Fatal("Kill should succeed")
	}

	found := false
	for _, sig := range proc.PendingSignals {
		if sig == SIGTERM {
			found = true
		}
	}
	if !found {
		t.Error("SIGTERM should be in pending signals")
	}
}

func TestKillSigkill(t *testing.T) {
	pm := NewProcessManager()
	proc := pm.CreateProcess("daemon", -1, 20, 0, 0)
	proc.State = Running

	pm.Kill(proc.PID, SIGKILL)
	if proc.State != Zombie {
		t.Errorf("State = %d, want Zombie", proc.State)
	}
}

func TestKillNonexistentPID(t *testing.T) {
	pm := NewProcessManager()
	ok := pm.Kill(999, SIGTERM)
	if ok {
		t.Error("Kill with nonexistent PID should fail")
	}
}

// =============================================================================
// ExitProcess Tests
// =============================================================================

func TestExitSetsZombie(t *testing.T) {
	pm := NewProcessManager()
	init := pm.CreateProcess("init", -1, 0, 0, 0)
	childPID, _, _ := pm.Fork(init.PID)

	pm.ExitProcess(childPID, 0)

	child := pm.GetProcess(childPID)
	if child.State != Zombie {
		t.Errorf("State = %d, want Zombie", child.State)
	}
}

func TestExitRecordsExitCode(t *testing.T) {
	pm := NewProcessManager()
	init := pm.CreateProcess("init", -1, 0, 0, 0)
	childPID, _, _ := pm.Fork(init.PID)

	pm.ExitProcess(childPID, 42)

	child := pm.GetProcess(childPID)
	if child.ExitCode != 42 {
		t.Errorf("ExitCode = %d, want 42", child.ExitCode)
	}
}

func TestExitReparentsChildren(t *testing.T) {
	pm := NewProcessManager()
	init := pm.CreateProcess("init", -1, 0, 0, 0)
	parentPID, _, _ := pm.Fork(init.PID)
	grandchildPID, _, _ := pm.Fork(parentPID)

	pm.ExitProcess(parentPID, 0)

	grandchild := pm.GetProcess(grandchildPID)
	if grandchild.ParentPID != 0 {
		t.Errorf("Grandchild ParentPID = %d, want 0", grandchild.ParentPID)
	}
}

func TestExitSendsSigchld(t *testing.T) {
	pm := NewProcessManager()
	init := pm.CreateProcess("init", -1, 0, 0, 0)
	childPID, _, _ := pm.Fork(init.PID)

	pm.ExitProcess(childPID, 0)

	found := false
	for _, sig := range init.PendingSignals {
		if sig == SIGCHLD {
			found = true
		}
	}
	if !found {
		t.Error("Parent should receive SIGCHLD when child exits")
	}
}

func TestExitNonexistentPID(t *testing.T) {
	pm := NewProcessManager()
	// Should not panic.
	pm.ExitProcess(999, 0)
}

func TestExitClearsChildrenList(t *testing.T) {
	pm := NewProcessManager()
	init := pm.CreateProcess("init", -1, 0, 0, 0)
	parentPID, _, _ := pm.Fork(init.PID)
	pm.Fork(parentPID) // grandchild

	pm.ExitProcess(parentPID, 0)

	parent := pm.GetProcess(parentPID)
	if len(parent.Children) != 0 {
		t.Error("Exited process should have empty children list")
	}
}

// =============================================================================
// Query Method Tests
// =============================================================================

func TestGetProcess(t *testing.T) {
	pm := NewProcessManager()
	init := pm.CreateProcess("init", -1, 0, 0, 0)

	pcb := pm.GetProcess(init.PID)
	if pcb == nil || pcb.PID != init.PID {
		t.Error("GetProcess should return the correct PCB")
	}
}

func TestGetProcessNonexistent(t *testing.T) {
	pm := NewProcessManager()
	if pm.GetProcess(999) != nil {
		t.Error("GetProcess should return nil for nonexistent PID")
	}
}

func TestGetChildren(t *testing.T) {
	pm := NewProcessManager()
	init := pm.CreateProcess("init", -1, 0, 0, 0)
	childPID, _, _ := pm.Fork(init.PID)

	children := pm.GetChildren(init.PID)
	if len(children) == 0 {
		t.Fatal("Should have children")
	}
	found := false
	for _, c := range children {
		if c == childPID {
			found = true
		}
	}
	if !found {
		t.Error("Child PID not in children list")
	}
}

func TestGetChildrenNonexistent(t *testing.T) {
	pm := NewProcessManager()
	children := pm.GetChildren(999)
	if children != nil {
		t.Error("Should return nil for nonexistent PID")
	}
}

func TestGetParent(t *testing.T) {
	pm := NewProcessManager()
	init := pm.CreateProcess("init", -1, 0, 0, 0)
	childPID, _, _ := pm.Fork(init.PID)

	if pm.GetParent(childPID) != init.PID {
		t.Error("GetParent should return the parent PID")
	}
}

func TestGetParentNonexistent(t *testing.T) {
	pm := NewProcessManager()
	if pm.GetParent(999) != -1 {
		t.Error("GetParent should return -1 for nonexistent PID")
	}
}

func TestSignalManagerProperty(t *testing.T) {
	pm := NewProcessManager()
	if pm.SignalManager() == nil {
		t.Error("SignalManager should not be nil")
	}
}

// =============================================================================
// Integration: Fork + Exec + Wait Lifecycle
// =============================================================================

func TestShellRunsCommand(t *testing.T) {
	// Simulate: shell forks, child execs "ls", child exits, parent waits.
	pm := NewProcessManager()
	shell := pm.CreateProcess("shell", -1, 20, 0, 0)

	// Fork.
	childPID, childRet, ok := pm.Fork(shell.PID)
	if !ok || childRet != 0 {
		t.Fatal("Fork failed")
	}

	// Exec.
	pm.Exec(childPID, 0x10000, 0x7FFFF000, 0, 0)
	child := pm.GetProcess(childPID)
	if child.PC != 0x10000 {
		t.Error("PC should be set to entry point after exec")
	}

	// Exit.
	pm.ExitProcess(childPID, 0)
	if child.State != Zombie {
		t.Error("Child should be zombie after exit")
	}

	// Wait.
	reapedPID, exitCode, ok := pm.Wait(shell.PID, childPID)
	if !ok || reapedPID != childPID || exitCode != 0 {
		t.Error("Wait should reap the zombie child")
	}

	// Verify zombie is gone.
	if pm.GetProcess(childPID) != nil {
		t.Error("Reaped child should be removed from process table")
	}
}

func TestSignalChain(t *testing.T) {
	// Send SIGTERM (caught), then SIGKILL (uncatchable).
	pm := NewProcessManager()
	parent := pm.CreateProcess("shell", -1, 20, 0, 0)
	childPID, _, _ := pm.Fork(parent.PID)
	child := pm.GetProcess(childPID)

	// Register SIGTERM handler.
	pm.SignalManager().RegisterHandler(child, SIGTERM, 0x40000)

	// Send SIGTERM — goes to pending (handler exists).
	pm.Kill(childPID, SIGTERM)
	if child.State == Zombie {
		t.Error("SIGTERM with handler should not terminate")
	}

	// Send SIGKILL — uncatchable.
	pm.Kill(childPID, SIGKILL)
	if child.State != Zombie {
		t.Error("SIGKILL should terminate the process")
	}
}
