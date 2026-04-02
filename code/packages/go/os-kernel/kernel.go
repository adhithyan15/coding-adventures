package oskernel

import (
	"github.com/adhithyan15/coding-adventures/code/packages/go/display"
	interrupthandler "github.com/adhithyan15/coding-adventures/code/packages/go/interrupt-handler"
)

// =========================================================================
// Well-known addresses for the kernel
// =========================================================================

const (
	// DefaultKernelBase is where the kernel code and data live in memory.
	DefaultKernelBase uint32 = 0x00020000

	// DefaultKernelSize is the size of the kernel memory region (64 KB).
	DefaultKernelSize uint32 = 0x00010000

	// DefaultIdleProcessBase is the memory region for PID 0 (idle).
	DefaultIdleProcessBase uint32 = 0x00030000

	// DefaultIdleProcessSize is the size of the idle process region (64 KB).
	DefaultIdleProcessSize uint32 = 0x00010000

	// DefaultUserProcessBase is the memory region for PID 1 (hello-world).
	DefaultUserProcessBase uint32 = 0x00040000

	// DefaultUserProcessSize is the size of user process regions (64 KB).
	DefaultUserProcessSize uint32 = 0x00010000

	// DefaultKernelStackTop is the kernel stack pointer (grows downward).
	DefaultKernelStackTop uint32 = 0x0006FFF0

	// DefaultKernelStackBase is the start of the kernel stack region.
	DefaultKernelStackBase uint32 = 0x00060000

	// DefaultKernelStackSize is the size of the kernel stack region (64 KB).
	DefaultKernelStackSize uint32 = 0x00010000

	// Interrupt numbers for hardware events.
	InterruptTimer    = 32
	InterruptKeyboard = 33
	InterruptSyscall  = 128
)

// =========================================================================
// KernelConfig
// =========================================================================

// KernelConfig holds tunable parameters for the kernel.
type KernelConfig struct {
	// TimerInterval is the number of CPU cycles between timer interrupts.
	// The scheduler uses timer interrupts for preemptive multitasking.
	TimerInterval int

	// MaxProcesses is the maximum number of processes in the process table.
	MaxProcesses int

	// MemoryLayout defines the pre-allocated memory regions.
	MemoryLayout []MemoryRegion
}

// DefaultKernelConfig returns a configuration suitable for the hello-world demo.
func DefaultKernelConfig() KernelConfig {
	result, _ := StartNew[KernelConfig]("os-kernel.DefaultKernelConfig", KernelConfig{},
		func(op *Operation[KernelConfig], rf *ResultFactory[KernelConfig]) *OperationResult[KernelConfig] {
			return rf.Generate(true, false, KernelConfig{
				TimerInterval: 100,
				MaxProcesses:  16,
				MemoryLayout: []MemoryRegion{
					{Base: 0x00000000, Size: 0x00001000, Permissions: PermRead, Owner: -1, Name: "IDT"},
					{Base: 0x00001000, Size: 0x00001000, Permissions: PermRead | PermWrite, Owner: -1, Name: "Boot Protocol"},
					{Base: DefaultKernelBase, Size: DefaultKernelSize, Permissions: PermRead | PermWrite | PermExecute, Owner: -1, Name: "Kernel Code"},
					{Base: DefaultIdleProcessBase, Size: DefaultIdleProcessSize, Permissions: PermRead | PermWrite | PermExecute, Owner: 0, Name: "Idle Process"},
					{Base: DefaultUserProcessBase, Size: DefaultUserProcessSize, Permissions: PermRead | PermWrite | PermExecute, Owner: 1, Name: "User Process"},
					{Base: DefaultKernelStackBase, Size: DefaultKernelStackSize, Permissions: PermRead | PermWrite, Owner: -1, Name: "Kernel Stack"},
				},
			})
		}).GetResult()
	return result
}

// =========================================================================
// Kernel
// =========================================================================

// Kernel is the central component of the operating system. It manages
// processes, handles system calls, and coordinates scheduling.
//
// The kernel operates at the Go level -- syscall handlers, the scheduler,
// and memory management are Go functions. The hello-world and idle programs
// are real RISC-V machine code that triggers ecall instructions, which the
// SystemBoard intercepts and dispatches to the kernel.
type Kernel struct {
	// Config holds the kernel's tunable parameters.
	Config KernelConfig

	// ProcessTable holds all process control blocks, indexed by PID.
	ProcessTable []*ProcessControlBlock

	// CurrentProcess is the PID of the currently running process.
	CurrentProcess int

	// Scheduler manages round-robin process selection.
	Scheduler *Scheduler

	// MemoryManager tracks memory regions and permissions.
	MemoryManager *MemoryManager

	// InterruptCtrl is the interrupt controller from S03.
	InterruptCtrl *interrupthandler.InterruptController

	// Display is the display driver from S05, used by sys_write.
	Display *display.DisplayDriver

	// KeyboardBuffer accumulates keystrokes waiting to be read by sys_read.
	KeyboardBuffer []byte

	// SyscallTable maps syscall numbers to handler functions.
	SyscallTable map[int]SyscallHandler

	// Booted is true after Boot() completes.
	Booted bool

	// nextPID is the next PID to assign.
	nextPID int
}

// NewKernel creates a kernel with the given configuration and hardware references.
func NewKernel(
	config KernelConfig,
	interruptCtrl *interrupthandler.InterruptController,
	displayDriver *display.DisplayDriver,
) *Kernel {
	result, _ := StartNew[*Kernel]("os-kernel.NewKernel", nil,
		func(op *Operation[*Kernel], rf *ResultFactory[*Kernel]) *OperationResult[*Kernel] {
			k := &Kernel{
				Config:        config,
				InterruptCtrl: interruptCtrl,
				Display:       displayDriver,
				SyscallTable:  DefaultSyscallTable(),
				ProcessTable:  make([]*ProcessControlBlock, 0, config.MaxProcesses),
				nextPID:       0,
			}
			return rf.Generate(true, false, k)
		}).GetResult()
	return result
}

// Boot initializes all subsystems, creates processes, and starts the scheduler.
//
// Boot sequence:
//   1. Initialize memory manager with pre-defined regions
//   2. Register ISRs with the interrupt controller
//   3. Create idle process (PID 0)
//   4. Create hello-world process (PID 1)
//   5. Start scheduler with PID 1 as the first running process
func (k *Kernel) Boot() {
	_, _ = StartNew[struct{}]("os-kernel.Kernel.Boot", struct{}{},
		func(op *Operation[struct{}], rf *ResultFactory[struct{}]) *OperationResult[struct{}] {
			// Step 1: Initialize memory manager.
			k.MemoryManager = NewMemoryManager(k.Config.MemoryLayout)

			// Step 2: Register ISR handlers.
			if k.InterruptCtrl != nil {
				k.InterruptCtrl.Registry.Register(InterruptTimer, func(frame *interrupthandler.InterruptFrame, kernel interface{}) {
					k.HandleTimer(frame)
				})
				k.InterruptCtrl.Registry.Register(InterruptKeyboard, func(frame *interrupthandler.InterruptFrame, kernel interface{}) {
					k.HandleKeyboard(frame)
				})
				k.InterruptCtrl.Registry.Register(InterruptSyscall, func(frame *interrupthandler.InterruptFrame, kernel interface{}) {
					k.HandleSyscallFrame(frame)
				})
			}

			// Step 3: Create idle process (PID 0).
			idleBinary := GenerateIdleProgram()
			k.CreateProcess("idle", idleBinary, DefaultIdleProcessBase, DefaultIdleProcessSize)

			// Step 4: Create hello-world process (PID 1).
			hwBinary := GenerateHelloWorldProgram(DefaultUserProcessBase)
			k.CreateProcess("hello-world", hwBinary, DefaultUserProcessBase, DefaultUserProcessSize)

			// Step 5: Start scheduler.
			k.Scheduler = NewScheduler(k.ProcessTable)

			// Start hello-world (PID 1) as the first running process.
			if len(k.ProcessTable) > 1 {
				k.ProcessTable[1].State = ProcessRunning
				k.CurrentProcess = 1
				k.Scheduler.Current = 1
			}

			k.Booted = true
			return rf.Generate(true, false, struct{}{})
		}).GetResult()
}

// CreateProcess creates a new process with the given binary.
// Returns the PID assigned to the new process.
func (k *Kernel) CreateProcess(name string, binary []byte, memBase, memSize uint32) int {
	result, _ := StartNew[int]("os-kernel.Kernel.CreateProcess", -1,
		func(op *Operation[int], rf *ResultFactory[int]) *OperationResult[int] {
			if len(k.ProcessTable) >= k.Config.MaxProcesses {
				return rf.Generate(true, false, -1) // Process table full
			}

			pid := k.nextPID
			k.nextPID++

			pcb := &ProcessControlBlock{
				PID:          pid,
				State:        ProcessReady,
				SavedPC:      memBase, // Start executing at the beginning of the binary
				StackPointer: memBase + memSize - 16, // Stack at end of region
				MemoryBase:   memBase,
				MemorySize:   memSize,
				Name:         name,
			}

			// Set the stack pointer in saved registers (x2 = sp).
			pcb.SavedRegisters[RegSP] = pcb.StackPointer

			k.ProcessTable = append(k.ProcessTable, pcb)
			return rf.Generate(true, false, pid)
		}).GetResult()
	return result
}

// HandleSyscall dispatches a syscall based on the a7 register value.
// This is called by the SystemBoard when an ecall trap is detected.
func (k *Kernel) HandleSyscall(syscallNum int, regs RegisterAccess, mem MemoryAccess) bool {
	result, _ := StartNew[bool]("os-kernel.Kernel.HandleSyscall", false,
		func(op *Operation[bool], rf *ResultFactory[bool]) *OperationResult[bool] {
			handler, ok := k.SyscallTable[syscallNum]
			if !ok {
				// Unknown syscall -- terminate the process.
				pid := k.CurrentProcess
				if pid >= 0 && pid < len(k.ProcessTable) {
					k.ProcessTable[pid].State = ProcessTerminated
					k.ProcessTable[pid].ExitCode = -1
				}
				return rf.Generate(true, false, false)
			}
			return rf.Generate(true, false, handler(k, regs, mem))
		}).GetResult()
	return result
}

// HandleSyscallFrame is the ISR handler for interrupt 128 (ecall).
// It reads the syscall number from the frame's a7 register.
func (k *Kernel) HandleSyscallFrame(frame *interrupthandler.InterruptFrame) {
	_, _ = StartNew[struct{}]("os-kernel.Kernel.HandleSyscallFrame", struct{}{},
		func(op *Operation[struct{}], rf *ResultFactory[struct{}]) *OperationResult[struct{}] {
			// This is a simplified handler used when the interrupt controller
			// dispatches directly. In the full SystemBoard integration, the
			// board intercepts ecall and calls HandleSyscall directly.
			return rf.Generate(true, false, struct{}{})
		}).GetResult()
}

// HandleTimer is the ISR for interrupt 32 (timer tick).
// It triggers a context switch to the next ready process.
func (k *Kernel) HandleTimer(frame *interrupthandler.InterruptFrame) {
	_, _ = StartNew[struct{}]("os-kernel.Kernel.HandleTimer", struct{}{},
		func(op *Operation[struct{}], rf *ResultFactory[struct{}]) *OperationResult[struct{}] {
			if k.Scheduler == nil {
				return rf.Generate(true, false, struct{}{})
			}
			// Save current process state.
			pid := k.CurrentProcess
			if pid >= 0 && pid < len(k.ProcessTable) {
				pcb := k.ProcessTable[pid]
				if pcb.State == ProcessRunning {
					pcb.State = ProcessReady
					pcb.SavedRegisters = frame.Registers
					pcb.SavedPC = frame.PC
				}
			}

			// Schedule next process.
			next := k.Scheduler.Schedule()
			k.Scheduler.ContextSwitch(pid, next)
			k.CurrentProcess = next

			// Load next process state into the frame.
			if next >= 0 && next < len(k.ProcessTable) {
				nextPCB := k.ProcessTable[next]
				frame.Registers = nextPCB.SavedRegisters
				frame.PC = nextPCB.SavedPC
			}
			return rf.Generate(true, false, struct{}{})
		}).GetResult()
}

// HandleKeyboard is the ISR for interrupt 33 (keyboard).
// It appends the keystroke to the keyboard buffer.
func (k *Kernel) HandleKeyboard(frame *interrupthandler.InterruptFrame) {
	_, _ = StartNew[struct{}]("os-kernel.Kernel.HandleKeyboard", struct{}{},
		func(op *Operation[struct{}], rf *ResultFactory[struct{}]) *OperationResult[struct{}] {
			// The keystroke byte is passed via the frame's register or memory.
			// In our simplified model, the SystemBoard calls InjectKeystroke
			// which adds directly to the keyboard buffer.
			return rf.Generate(true, false, struct{}{})
		}).GetResult()
}

// IsIdle returns true when only the idle process (PID 0) is Ready and all
// other processes are Terminated.
func (k *Kernel) IsIdle() bool {
	result, _ := StartNew[bool]("os-kernel.Kernel.IsIdle", false,
		func(op *Operation[bool], rf *ResultFactory[bool]) *OperationResult[bool] {
			for _, pcb := range k.ProcessTable {
				if pcb.PID == 0 {
					continue // Skip idle
				}
				if pcb.State != ProcessTerminated {
					return rf.Generate(true, false, false)
				}
			}
			return rf.Generate(true, false, true)
		}).GetResult()
	return result
}

// ProcessInfo returns a summary of a process for snapshots and debugging.
func (k *Kernel) ProcessInfo(pid int) ProcessInfo {
	result, _ := StartNew[ProcessInfo]("os-kernel.Kernel.ProcessInfo", ProcessInfo{},
		func(op *Operation[ProcessInfo], rf *ResultFactory[ProcessInfo]) *OperationResult[ProcessInfo] {
			if pid < 0 || pid >= len(k.ProcessTable) {
				return rf.Generate(true, false, ProcessInfo{})
			}
			pcb := k.ProcessTable[pid]
			return rf.Generate(true, false, ProcessInfo{
				PID:   pcb.PID,
				Name:  pcb.Name,
				State: pcb.State,
				PC:    pcb.SavedPC,
			})
		}).GetResult()
	return result
}

// ProcessCount returns the number of processes in the table.
func (k *Kernel) ProcessCount() int {
	result, _ := StartNew[int]("os-kernel.Kernel.ProcessCount", 0,
		func(op *Operation[int], rf *ResultFactory[int]) *OperationResult[int] {
			return rf.Generate(true, false, len(k.ProcessTable))
		}).GetResult()
	return result
}

// GetCurrentPCB returns the PCB of the currently running process.
func (k *Kernel) GetCurrentPCB() *ProcessControlBlock {
	result, _ := StartNew[*ProcessControlBlock]("os-kernel.Kernel.GetCurrentPCB", nil,
		func(op *Operation[*ProcessControlBlock], rf *ResultFactory[*ProcessControlBlock]) *OperationResult[*ProcessControlBlock] {
			if k.CurrentProcess >= 0 && k.CurrentProcess < len(k.ProcessTable) {
				return rf.Generate(true, false, k.ProcessTable[k.CurrentProcess])
			}
			return rf.Generate(true, false, nil)
		}).GetResult()
	return result
}

// AddKeystroke appends a character to the keyboard buffer.
func (k *Kernel) AddKeystroke(ch byte) {
	_, _ = StartNew[struct{}]("os-kernel.Kernel.AddKeystroke", struct{}{},
		func(op *Operation[struct{}], rf *ResultFactory[struct{}]) *OperationResult[struct{}] {
			k.KeyboardBuffer = append(k.KeyboardBuffer, ch)
			return rf.Generate(true, false, struct{}{})
		}).GetResult()
}
