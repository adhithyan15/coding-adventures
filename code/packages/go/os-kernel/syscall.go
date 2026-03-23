package oskernel

// =========================================================================
// System Call Interface
// =========================================================================
//
// System calls are how user programs request services from the kernel.
// On RISC-V, the convention is:
//
//	a7 (x17): syscall number
//	a0 (x10): first argument / return value
//	a1 (x11): second argument
//	a2 (x12): third argument
//
// The ecall instruction triggers interrupt 128, which the SystemBoard
// intercepts and dispatches to the kernel's HandleSyscall method.

// Syscall numbers -- the interface between user programs and the kernel.
const (
	// SysExit terminates the current process.
	// a0 = exit code.
	SysExit = 0

	// SysWrite writes bytes to a file descriptor.
	// a0 = fd (1 = stdout), a1 = buffer address, a2 = length.
	// Returns bytes written in a0.
	SysWrite = 1

	// SysRead reads bytes from a file descriptor.
	// a0 = fd (0 = stdin), a1 = buffer address, a2 = max length.
	// Returns bytes read in a0.
	SysRead = 2

	// SysYield voluntarily gives up the CPU to the next process.
	SysYield = 3
)

// RISC-V register numbers used in the syscall convention.
const (
	RegA0 = 10 // x10 = a0 (first arg / return value)
	RegA1 = 11 // x11 = a1 (second arg)
	RegA2 = 12 // x12 = a2 (third arg)
	RegA7 = 17 // x17 = a7 (syscall number)
	RegSP = 2  // x2 = sp (stack pointer)
)

// SyscallHandler is a function that handles a specific syscall.
// It receives the kernel, the register reader/writer, and the memory reader.
// It returns true if the syscall was handled successfully.
type SyscallHandler func(k *Kernel, regs RegisterAccess, mem MemoryAccess) bool

// RegisterAccess provides read/write access to CPU registers.
// This interface decouples the kernel from the specific CPU implementation.
type RegisterAccess interface {
	ReadRegister(index int) uint32
	WriteRegister(index int, value uint32)
}

// MemoryAccess provides read access to the CPU's memory.
// Used by sys_write to read the user's buffer.
type MemoryAccess interface {
	ReadMemoryByte(address uint32) byte
}

// DefaultSyscallTable returns the standard syscall dispatch table.
func DefaultSyscallTable() map[int]SyscallHandler {
	return map[int]SyscallHandler{
		SysExit:  handleSysExit,
		SysWrite: handleSysWrite,
		SysRead:  handleSysRead,
		SysYield: handleSysYield,
	}
}

// handleSysExit terminates the current process.
// a0 = exit code.
func handleSysExit(k *Kernel, regs RegisterAccess, mem MemoryAccess) bool {
	exitCode := int(regs.ReadRegister(RegA0))
	pid := k.CurrentProcess
	if pid >= 0 && pid < len(k.ProcessTable) {
		k.ProcessTable[pid].State = ProcessTerminated
		k.ProcessTable[pid].ExitCode = exitCode
	}
	// Schedule the next process.
	next := k.Scheduler.Schedule()
	k.Scheduler.ContextSwitch(pid, next)
	k.CurrentProcess = next
	return true
}

// handleSysWrite writes bytes to the display (stdout).
// a0 = fd (must be 1), a1 = buffer address, a2 = length.
// Returns bytes written in a0.
func handleSysWrite(k *Kernel, regs RegisterAccess, mem MemoryAccess) bool {
	fd := regs.ReadRegister(RegA0)
	bufAddr := regs.ReadRegister(RegA1)
	length := regs.ReadRegister(RegA2)

	if fd != 1 {
		// Only stdout (fd=1) is supported.
		regs.WriteRegister(RegA0, 0)
		return true
	}

	if k.Display == nil {
		regs.WriteRegister(RegA0, 0)
		return true
	}

	// Read bytes from process memory and write to display.
	var written uint32
	for i := uint32(0); i < length; i++ {
		ch := mem.ReadMemoryByte(bufAddr + i)
		k.Display.PutChar(ch)
		written++
	}

	regs.WriteRegister(RegA0, written)
	return true
}

// handleSysRead reads bytes from the keyboard buffer (stdin).
// a0 = fd (must be 0), a1 = buffer address, a2 = max length.
// Returns bytes read in a0.
func handleSysRead(k *Kernel, regs RegisterAccess, mem MemoryAccess) bool {
	fd := regs.ReadRegister(RegA0)
	length := regs.ReadRegister(RegA2)

	if fd != 0 {
		regs.WriteRegister(RegA0, 0)
		return true
	}

	// Read from keyboard buffer.
	available := uint32(len(k.KeyboardBuffer))
	toRead := length
	if toRead > available {
		toRead = available
	}

	// We don't write to process memory here -- that would need a MemoryWriter.
	// Instead we just report how many bytes are available and consume them.
	// The SystemBoard is responsible for the actual memory write.
	regs.WriteRegister(RegA0, toRead)

	// Consume the bytes from the keyboard buffer.
	if toRead > 0 {
		k.KeyboardBuffer = k.KeyboardBuffer[toRead:]
	}

	return true
}

// handleSysYield voluntarily gives up the CPU.
func handleSysYield(k *Kernel, regs RegisterAccess, mem MemoryAccess) bool {
	pid := k.CurrentProcess
	if pid >= 0 && pid < len(k.ProcessTable) {
		if k.ProcessTable[pid].State == ProcessRunning {
			k.ProcessTable[pid].State = ProcessReady
		}
	}
	next := k.Scheduler.Schedule()
	k.Scheduler.ContextSwitch(pid, next)
	k.CurrentProcess = next
	return true
}
