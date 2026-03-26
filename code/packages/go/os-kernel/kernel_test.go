package oskernel

import (
	"testing"

	"github.com/adhithyan15/coding-adventures/code/packages/go/display"
	interrupthandler "github.com/adhithyan15/coding-adventures/code/packages/go/interrupt-handler"
)

// =========================================================================
// Process Management Tests
// =========================================================================

func TestCreateProcess(t *testing.T) {
	k := newTestKernel()
	binary := []byte{0x01, 0x02, 0x03, 0x04}
	pid := k.CreateProcess("test", binary, 0x00040000, 0x10000)

	if pid != 0 {
		t.Fatalf("First PID should be 0, got %d", pid)
	}
	if len(k.ProcessTable) != 1 {
		t.Fatalf("Process table should have 1 entry, got %d", len(k.ProcessTable))
	}

	pcb := k.ProcessTable[0]
	if pcb.Name != "test" {
		t.Errorf("Name = %q, expected %q", pcb.Name, "test")
	}
	if pcb.State != ProcessReady {
		t.Errorf("State = %v, expected Ready", pcb.State)
	}
	if pcb.MemoryBase != 0x00040000 {
		t.Errorf("MemoryBase = 0x%08X, expected 0x00040000", pcb.MemoryBase)
	}
	if pcb.SavedPC != 0x00040000 {
		t.Errorf("SavedPC = 0x%08X, expected 0x00040000", pcb.SavedPC)
	}
}

func TestMultipleProcesses(t *testing.T) {
	k := newTestKernel()
	pid0 := k.CreateProcess("p0", []byte{}, 0x30000, 0x10000)
	pid1 := k.CreateProcess("p1", []byte{}, 0x40000, 0x10000)

	if pid0 == pid1 {
		t.Fatalf("PIDs should be distinct: %d vs %d", pid0, pid1)
	}
	if k.ProcessCount() != 2 {
		t.Fatalf("Process count = %d, expected 2", k.ProcessCount())
	}
}

func TestMaxProcesses(t *testing.T) {
	config := DefaultKernelConfig()
	config.MaxProcesses = 2
	k := NewKernel(config, nil, nil)
	k.CreateProcess("p0", []byte{}, 0x30000, 0x10000)
	k.CreateProcess("p1", []byte{}, 0x40000, 0x10000)
	pid := k.CreateProcess("p2", []byte{}, 0x50000, 0x10000)
	if pid != -1 {
		t.Fatalf("Expected -1 for full table, got %d", pid)
	}
}

func TestProcessStateTransitions(t *testing.T) {
	k := newTestKernel()
	k.CreateProcess("test", []byte{}, 0x40000, 0x10000)

	pcb := k.ProcessTable[0]
	if pcb.State != ProcessReady {
		t.Fatalf("Initial state should be Ready")
	}

	pcb.State = ProcessRunning
	if pcb.State != ProcessRunning {
		t.Fatalf("Should be Running")
	}

	pcb.State = ProcessTerminated
	pcb.ExitCode = 42
	if pcb.ExitCode != 42 {
		t.Fatalf("Exit code should be 42")
	}
}

func TestProcessStateString(t *testing.T) {
	tests := []struct {
		state ProcessState
		want  string
	}{
		{ProcessReady, "Ready"},
		{ProcessRunning, "Running"},
		{ProcessBlocked, "Blocked"},
		{ProcessTerminated, "Terminated"},
		{ProcessState(99), "Unknown"},
	}
	for _, tt := range tests {
		if got := tt.state.String(); got != tt.want {
			t.Errorf("ProcessState(%d).String() = %q, want %q", tt.state, got, tt.want)
		}
	}
}

// =========================================================================
// Scheduler Tests
// =========================================================================

func TestSchedulerRoundRobin(t *testing.T) {
	procs := []*ProcessControlBlock{
		{PID: 0, State: ProcessReady, Name: "idle"},
		{PID: 1, State: ProcessReady, Name: "hello"},
	}
	sched := NewScheduler(procs)
	sched.Current = 0

	// From PID 0, should schedule PID 1.
	next := sched.Schedule()
	if next != 1 {
		t.Fatalf("From PID 0, next = %d, expected 1", next)
	}

	// From PID 1, should schedule PID 0.
	sched.Current = 1
	next = sched.Schedule()
	if next != 0 {
		t.Fatalf("From PID 1, next = %d, expected 0", next)
	}
}

func TestSchedulerSkipTerminated(t *testing.T) {
	procs := []*ProcessControlBlock{
		{PID: 0, State: ProcessReady, Name: "idle"},
		{PID: 1, State: ProcessTerminated, Name: "hello"},
	}
	sched := NewScheduler(procs)
	sched.Current = 0

	next := sched.Schedule()
	if next != 0 {
		t.Fatalf("With PID 1 terminated, next = %d, expected 0", next)
	}
}

func TestSchedulerContextSwitch(t *testing.T) {
	procs := []*ProcessControlBlock{
		{PID: 0, State: ProcessRunning, Name: "idle"},
		{PID: 1, State: ProcessReady, Name: "hello"},
	}
	sched := NewScheduler(procs)

	sched.ContextSwitch(0, 1)

	if procs[0].State != ProcessReady {
		t.Errorf("PID 0 should be Ready after switch, got %v", procs[0].State)
	}
	if procs[1].State != ProcessRunning {
		t.Errorf("PID 1 should be Running after switch, got %v", procs[1].State)
	}
	if sched.Current != 1 {
		t.Errorf("Current should be 1, got %d", sched.Current)
	}
}

func TestSchedulerEmptyTable(t *testing.T) {
	sched := NewScheduler([]*ProcessControlBlock{})
	next := sched.Schedule()
	if next != 0 {
		t.Fatalf("Empty table should return 0, got %d", next)
	}
}

// =========================================================================
// Memory Manager Tests
// =========================================================================

func TestMemoryManagerFindRegion(t *testing.T) {
	regions := []MemoryRegion{
		{Base: 0x1000, Size: 0x1000, Permissions: PermRead, Owner: -1, Name: "A"},
		{Base: 0x3000, Size: 0x2000, Permissions: PermRead | PermWrite, Owner: 1, Name: "B"},
	}
	mm := NewMemoryManager(regions)

	r := mm.FindRegion(0x1500)
	if r == nil || r.Name != "A" {
		t.Fatal("Should find region A for address 0x1500")
	}

	r = mm.FindRegion(0x4000)
	if r == nil || r.Name != "B" {
		t.Fatal("Should find region B for address 0x4000")
	}

	r = mm.FindRegion(0x6000)
	if r != nil {
		t.Fatal("Should not find region for address 0x6000")
	}
}

func TestMemoryManagerCheckAccess(t *testing.T) {
	regions := []MemoryRegion{
		{Base: 0x1000, Size: 0x1000, Permissions: PermRead | PermWrite, Owner: -1, Name: "Kernel"},
		{Base: 0x3000, Size: 0x1000, Permissions: PermRead | PermWrite | PermExecute, Owner: 1, Name: "P1"},
	}
	mm := NewMemoryManager(regions)

	// Kernel region is accessible by all.
	if !mm.CheckAccess(0, 0x1000, PermRead) {
		t.Error("PID 0 should be able to read kernel region")
	}

	// Process 1 can access its own region.
	if !mm.CheckAccess(1, 0x3000, PermRead|PermWrite) {
		t.Error("PID 1 should be able to read/write its region")
	}

	// Process 0 should not access PID 1's region.
	if mm.CheckAccess(0, 0x3000, PermRead) {
		t.Error("PID 0 should NOT access PID 1's region")
	}

	// Unmapped address.
	if mm.CheckAccess(0, 0x9000, PermRead) {
		t.Error("Unmapped address should deny access")
	}
}

func TestMemoryManagerAllocateRegion(t *testing.T) {
	mm := NewMemoryManager(nil)
	if mm.RegionCount() != 0 {
		t.Fatalf("Initial region count = %d, expected 0", mm.RegionCount())
	}

	mm.AllocateRegion(1, 0x5000, 0x1000, PermRead|PermWrite, "new")
	if mm.RegionCount() != 1 {
		t.Fatalf("After allocate, region count = %d, expected 1", mm.RegionCount())
	}

	r := mm.FindRegion(0x5500)
	if r == nil || r.Name != "new" {
		t.Fatal("Should find newly allocated region")
	}
}

// =========================================================================
// Syscall Tests
// =========================================================================

func TestSysExit(t *testing.T) {
	k := newBootedKernel()
	regs := &mockRegAccess{regs: make(map[int]uint32)}
	regs.regs[RegA0] = 42 // exit code

	k.CurrentProcess = 1
	k.ProcessTable[1].State = ProcessRunning

	k.HandleSyscall(SysExit, regs, &mockMemAccess{})

	if k.ProcessTable[1].State != ProcessTerminated {
		t.Fatalf("PID 1 should be Terminated, got %v", k.ProcessTable[1].State)
	}
	if k.ProcessTable[1].ExitCode != 42 {
		t.Fatalf("Exit code = %d, expected 42", k.ProcessTable[1].ExitCode)
	}
}

func TestSysWrite(t *testing.T) {
	displayMem := make([]byte, 80*25*2)
	driver := display.NewDisplayDriver(display.DefaultDisplayConfig(), displayMem)

	k := newBootedKernelWithDisplay(driver)

	// Set up memory with "Hi" at address 0x00040100.
	mem := &mockMemAccess{data: map[uint32]byte{
		0x00040100: 'H',
		0x00040101: 'i',
	}}

	regs := &mockRegAccess{regs: map[int]uint32{
		RegA0: 1,          // fd = stdout
		RegA1: 0x00040100, // buffer address
		RegA2: 2,          // length
	}}

	k.CurrentProcess = 1
	k.HandleSyscall(SysWrite, regs, mem)

	// Check return value (bytes written).
	if regs.regs[RegA0] != 2 {
		t.Fatalf("sys_write return = %d, expected 2", regs.regs[RegA0])
	}

	// Check display.
	snap := driver.Snapshot()
	if !snap.Contains("Hi") {
		t.Fatalf("Display should contain 'Hi', got: %q", snap.LineAt(0))
	}
}

func TestSysWriteWrongFD(t *testing.T) {
	k := newBootedKernel()
	regs := &mockRegAccess{regs: map[int]uint32{
		RegA0: 2, // fd = stderr (unsupported)
		RegA1: 0x40100,
		RegA2: 5,
	}}
	k.CurrentProcess = 1
	k.HandleSyscall(SysWrite, regs, &mockMemAccess{})
	if regs.regs[RegA0] != 0 {
		t.Fatalf("sys_write with wrong fd should return 0, got %d", regs.regs[RegA0])
	}
}

func TestSysRead(t *testing.T) {
	k := newBootedKernel()
	k.KeyboardBuffer = []byte{'A', 'B'}

	regs := &mockRegAccess{regs: map[int]uint32{
		RegA0: 0,  // fd = stdin
		RegA2: 10, // max length
	}}

	k.CurrentProcess = 1
	k.HandleSyscall(SysRead, regs, &mockMemAccess{})

	if regs.regs[RegA0] != 2 {
		t.Fatalf("sys_read return = %d, expected 2", regs.regs[RegA0])
	}
	if len(k.KeyboardBuffer) != 0 {
		t.Fatalf("Keyboard buffer should be empty after read, len = %d", len(k.KeyboardBuffer))
	}
}

func TestSysReadEmptyBuffer(t *testing.T) {
	k := newBootedKernel()
	regs := &mockRegAccess{regs: map[int]uint32{
		RegA0: 0,
		RegA2: 10,
	}}
	k.CurrentProcess = 1
	k.HandleSyscall(SysRead, regs, &mockMemAccess{})
	if regs.regs[RegA0] != 0 {
		t.Fatalf("sys_read empty should return 0, got %d", regs.regs[RegA0])
	}
}

func TestSysYield(t *testing.T) {
	k := newBootedKernel()
	k.CurrentProcess = 1
	k.ProcessTable[1].State = ProcessRunning

	regs := &mockRegAccess{regs: make(map[int]uint32)}
	k.HandleSyscall(SysYield, regs, &mockMemAccess{})

	if k.ProcessTable[1].State != ProcessReady {
		t.Fatalf("PID 1 should be Ready after yield, got %v", k.ProcessTable[1].State)
	}
}

func TestUnknownSyscall(t *testing.T) {
	k := newBootedKernel()
	k.CurrentProcess = 1
	k.ProcessTable[1].State = ProcessRunning

	regs := &mockRegAccess{regs: make(map[int]uint32)}
	ok := k.HandleSyscall(99, regs, &mockMemAccess{})

	if ok {
		t.Fatal("Unknown syscall should return false")
	}
	if k.ProcessTable[1].State != ProcessTerminated {
		t.Fatal("Unknown syscall should terminate process")
	}
}

// =========================================================================
// Kernel Boot Tests
// =========================================================================

func TestKernelBoot(t *testing.T) {
	ic := interrupthandler.NewInterruptController()
	displayMem := make([]byte, 80*25*2)
	driver := display.NewDisplayDriver(display.DefaultDisplayConfig(), displayMem)

	k := NewKernel(DefaultKernelConfig(), ic, driver)
	k.Boot()

	if !k.Booted {
		t.Fatal("Kernel should be booted")
	}
	if k.ProcessCount() != 2 {
		t.Fatalf("Process count = %d, expected 2", k.ProcessCount())
	}
	if k.ProcessTable[0].Name != "idle" {
		t.Errorf("PID 0 name = %q, expected 'idle'", k.ProcessTable[0].Name)
	}
	if k.ProcessTable[1].Name != "hello-world" {
		t.Errorf("PID 1 name = %q, expected 'hello-world'", k.ProcessTable[1].Name)
	}
	if k.CurrentProcess != 1 {
		t.Errorf("Current process = %d, expected 1", k.CurrentProcess)
	}
	if k.ProcessTable[1].State != ProcessRunning {
		t.Errorf("PID 1 should be Running, got %v", k.ProcessTable[1].State)
	}

	// Verify ISRs are registered.
	if !ic.Registry.HasHandler(InterruptTimer) {
		t.Error("Timer ISR should be registered")
	}
	if !ic.Registry.HasHandler(InterruptKeyboard) {
		t.Error("Keyboard ISR should be registered")
	}
	if !ic.Registry.HasHandler(InterruptSyscall) {
		t.Error("Syscall ISR should be registered")
	}
}

func TestIsIdle(t *testing.T) {
	k := newBootedKernel()

	if k.IsIdle() {
		t.Fatal("Should not be idle when hello-world is Ready/Running")
	}

	k.ProcessTable[1].State = ProcessTerminated
	if !k.IsIdle() {
		t.Fatal("Should be idle when only idle process remains")
	}
}

func TestProcessInfo(t *testing.T) {
	k := newBootedKernel()
	info := k.ProcessInfo(1)
	if info.PID != 1 {
		t.Errorf("PID = %d, expected 1", info.PID)
	}
	if info.Name != "hello-world" {
		t.Errorf("Name = %q, expected 'hello-world'", info.Name)
	}
}

func TestAddKeystroke(t *testing.T) {
	k := newBootedKernel()
	k.AddKeystroke('H')
	k.AddKeystroke('i')
	if string(k.KeyboardBuffer) != "Hi" {
		t.Fatalf("Keyboard buffer = %q, expected 'Hi'", string(k.KeyboardBuffer))
	}
}

// =========================================================================
// Timer Handler Tests
// =========================================================================

func TestTimerHandlerContextSwitch(t *testing.T) {
	k := newBootedKernel()
	k.CurrentProcess = 1
	k.ProcessTable[1].State = ProcessRunning
	k.ProcessTable[0].State = ProcessReady

	frame := &interrupthandler.InterruptFrame{
		PC:        0x00040010,
		Registers: [32]uint32{},
	}
	frame.Registers[RegA0] = 42 // Some register value

	k.HandleTimer(frame)

	// PID 1 should be Ready, PID 0 should be Running.
	if k.ProcessTable[1].State != ProcessReady {
		t.Errorf("PID 1 should be Ready after timer, got %v", k.ProcessTable[1].State)
	}
	if k.ProcessTable[1].SavedPC != 0x00040010 {
		t.Errorf("PID 1 saved PC should be 0x00040010, got 0x%08X", k.ProcessTable[1].SavedPC)
	}
}

// =========================================================================
// Program Generation Tests
// =========================================================================

func TestGenerateIdleProgram(t *testing.T) {
	binary := GenerateIdleProgram()
	if len(binary) == 0 {
		t.Fatal("Idle program should not be empty")
	}
	if len(binary)%4 != 0 {
		t.Fatalf("Idle program length %d is not a multiple of 4", len(binary))
	}
}

func TestGenerateHelloWorldProgram(t *testing.T) {
	binary := GenerateHelloWorldProgram(0x00040000)
	if len(binary) == 0 {
		t.Fatal("Hello world program should not be empty")
	}

	// The binary should contain "Hello World\n" at offset 0x100.
	message := "Hello World\n"
	dataOffset := 0x100
	if len(binary) < dataOffset+len(message) {
		t.Fatalf("Binary too short: %d bytes, need at least %d", len(binary), dataOffset+len(message))
	}

	found := string(binary[dataOffset : dataOffset+len(message)])
	if found != message {
		t.Fatalf("Data at offset 0x100 = %q, expected %q", found, message)
	}
}

func TestGenerateHelloWorldBinary(t *testing.T) {
	binary := GenerateHelloWorldBinary()
	if len(binary) == 0 {
		t.Fatal("GenerateHelloWorldBinary should not return empty")
	}
}

// =========================================================================
// Default Config Tests
// =========================================================================

func TestDefaultKernelConfig(t *testing.T) {
	config := DefaultKernelConfig()
	if config.TimerInterval != 100 {
		t.Errorf("TimerInterval = %d, expected 100", config.TimerInterval)
	}
	if config.MaxProcesses != 16 {
		t.Errorf("MaxProcesses = %d, expected 16", config.MaxProcesses)
	}
	if len(config.MemoryLayout) == 0 {
		t.Error("MemoryLayout should not be empty")
	}
}

// =========================================================================
// Helpers
// =========================================================================

func newTestKernel() *Kernel {
	return NewKernel(DefaultKernelConfig(), nil, nil)
}

func newBootedKernel() *Kernel {
	ic := interrupthandler.NewInterruptController()
	k := NewKernel(DefaultKernelConfig(), ic, nil)
	k.Boot()
	return k
}

func newBootedKernelWithDisplay(driver *display.DisplayDriver) *Kernel {
	ic := interrupthandler.NewInterruptController()
	k := NewKernel(DefaultKernelConfig(), ic, driver)
	k.Boot()
	return k
}

// mockRegAccess implements RegisterAccess for testing.
type mockRegAccess struct {
	regs map[int]uint32
}

func (m *mockRegAccess) ReadRegister(index int) uint32 {
	return m.regs[index]
}

func (m *mockRegAccess) WriteRegister(index int, value uint32) {
	m.regs[index] = value
}

// mockMemAccess implements MemoryAccess for testing.
type mockMemAccess struct {
	data map[uint32]byte
}

func (m *mockMemAccess) ReadMemoryByte(address uint32) byte {
	if m.data == nil {
		return 0
	}
	return m.data[address]
}
