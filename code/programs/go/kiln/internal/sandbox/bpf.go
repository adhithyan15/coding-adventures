package sandbox

import (
	"errors"
	"fmt"
)

// The seccomp filter (KILN04 section 4.3 step 5, section 4.7).  It is a
// CLASSIC BPF program, assembled by hand here, so the repository depends on no
// seccomp library and every instruction can be read.
//
// A classic BPF program for seccomp reads one `struct seccomp_data` the kernel
// builds for each system call:
//
//	offset  0  u32  nr                  the system-call number
//	offset  4  u32  arch                AUDIT_ARCH_*
//	offset  8  u64  instruction_pointer
//	offset 16  u64  args[6]             args[i] at 16+8*i
//
// and ends in a RET whose value is the verdict.

// Seccomp verdicts and the values the program tests.
const (
	RetKillProcess uint32 = 0x80000000
	RetErrno       uint32 = 0x00050000 // OR in the errno
	RetAllow       uint32 = 0x7fff0000

	AuditArchX86_64 uint32 = 0xc000003e
	X32SyscallBit   uint32 = 0x40000000

	Eperm  uint32 = 1
	Enosys uint32 = 38
)

// seccomp_data offsets.
const (
	offNr   = 0
	offArch = 4
	offArg0 = 16
)

// Classic BPF opcodes (linux/filter.h).
const (
	bpfLdWAbs uint16 = 0x20 // BPF_LD  | BPF_W   | BPF_ABS
	bpfJeqK   uint16 = 0x15 // BPF_JMP | BPF_JEQ | BPF_K
	bpfJsetK  uint16 = 0x45 // BPF_JMP | BPF_JSET| BPF_K
	bpfRetK   uint16 = 0x06 // BPF_RET | BPF_K
)

// Instruction is one classic BPF instruction.  Its layout is the kernel's
// `struct sock_filter`, so a []Instruction can be handed to the kernel as is.
type Instruction struct {
	Code uint16
	Jt   uint8
	Jf   uint8
	K    uint32
}

// Syscall is an x86-64 system call by name and number.
type Syscall struct {
	Name string
	Nr   uint32
}

// RefusedSyscalls are the calls the filter answers with EPERM: the ones that
// read files without a path the tracer sees, or change what paths mean
// (KILN04 section 4.4).  The numbers are x86-64's; a linux/amd64 test checks
// the ones package syscall knows.
var RefusedSyscalls = []Syscall{
	{"io_uring_setup", 425},
	{"open_by_handle_at", 304},
	{"name_to_handle_at", 303},
	{"mount", 165},
	{"umount2", 166},
	{"fsopen", 430},
	{"fsmount", 432},
	{"fsconfig", 431},
	{"fspick", 433},
	{"open_tree", 428},
	{"move_mount", 429},
	{"mount_setattr", 442},
	{"pivot_root", 155},
	{"chroot", 161},
	{"unshare", 272},
	{"setns", 308},
	{"ptrace", 101},
	{"process_vm_readv", 310},
	{"pidfd_getfd", 438},
	{"bpf", 321},
	// Added by the stage 3b review: more ways to reach the host or to read
	// without a path.  io_uring_enter and _register are useless without a
	// setup, but a descriptor could in principle be inherited, and a refused
	// call costs nothing.
	{"io_uring_enter", 426},
	{"io_uring_register", 427},
	{"process_vm_writev", 311},
	{"userfaultfd", 323},
	{"kexec_load", 246},
	{"kexec_file_load", 320},
	{"perf_event_open", 298},
	{"add_key", 248},
	{"request_key", 249},
	{"keyctl", 250},
}

// Syscall numbers the filter treats specially.
const (
	sysClone  uint32 = 56
	sysClone3 uint32 = 435
)

// Clone flags the filter refuses in clone's first argument: CLONE_UNTRACED, a
// child strace -f would never attach to, and every namespace flag.
const (
	CloneNewNS      uint32 = 0x00020000
	CloneUntraced   uint32 = 0x00800000
	CloneNewCgroup  uint32 = 0x02000000
	CloneNewUTS     uint32 = 0x04000000
	CloneNewIPC     uint32 = 0x08000000
	CloneNewUser    uint32 = 0x10000000
	CloneNewPID     uint32 = 0x20000000
	CloneNewNet     uint32 = 0x40000000
	CloneRefusedSet        = CloneUntraced | CloneNewNS | CloneNewCgroup | CloneNewUTS | CloneNewIPC | CloneNewUser | CloneNewPID | CloneNewNet
)

// step is one instruction before jumps are resolved: a jump names the label of
// its target rather than an offset.
type step struct {
	code   uint16
	k      uint32
	jt, jf string // labels; "" means fall through to the next instruction
	label  string // this instruction's own label
}

// BuildFilter assembles the program.  Its layout, in order:
//
//	ld  arch
//	jeq AUDIT_ARCH_X86_64  else -> KILL
//	ld  nr
//	jset __X32_SYSCALL_BIT then -> KILL
//	jeq <each refused nr>  then -> EPERM
//	jeq clone              then -> CLONE
//	jeq clone3             then -> ENOSYS
//	ret ALLOW
//	CLONE: ld args[0] (low 32 bits); jset <refused flags> then -> EPERM; ret ALLOW
//	EPERM: ret ERRNO|EPERM
//	ENOSYS: ret ERRNO|ENOSYS
//	KILL: ret KILL_PROCESS
//
// The architecture test comes first.  Without it a 32-bit or x32 call would
// reach the number tests with numbers that mean other calls, and a `clone`
// there would match no x86-64 rule at all.  Any other ABI is KILLed, so a
// non-native ABI is impossible, not merely detected.
//
// clone's flags are the FIRST argument on x86-64 (the order differs on other
// architectures), and the kernel uses only their low 32 bits.
func BuildFilter() ([]Instruction, error) {
	steps := []step{
		{code: bpfLdWAbs, k: offArch},
		{code: bpfJeqK, k: AuditArchX86_64, jf: "kill"},
		{code: bpfLdWAbs, k: offNr},
		{code: bpfJsetK, k: X32SyscallBit, jt: "kill"},
	}
	for _, s := range RefusedSyscalls {
		steps = append(steps, step{code: bpfJeqK, k: s.Nr, jt: "eperm"})
	}
	steps = append(steps,
		step{code: bpfJeqK, k: sysClone, jt: "clone"},
		step{code: bpfJeqK, k: sysClone3, jt: "enosys"},
		step{code: bpfRetK, k: RetAllow},
		step{label: "clone", code: bpfLdWAbs, k: offArg0},
		step{code: bpfJsetK, k: CloneRefusedSet, jt: "eperm"},
		step{code: bpfRetK, k: RetAllow},
		step{label: "eperm", code: bpfRetK, k: RetErrno | Eperm},
		step{label: "enosys", code: bpfRetK, k: RetErrno | Enosys},
		step{label: "kill", code: bpfRetK, k: RetKillProcess},
	)
	at := map[string]int{}
	for i, s := range steps {
		if s.label != "" {
			at[s.label] = i
		}
	}
	prog := make([]Instruction, len(steps))
	for i, s := range steps {
		in := Instruction{Code: s.code, K: s.k}
		for _, j := range []struct {
			label string
			dst   *uint8
		}{{s.jt, &in.Jt}, {s.jf, &in.Jf}} {
			if j.label == "" {
				continue
			}
			target, ok := at[j.label]
			if !ok {
				return nil, fmt.Errorf("seccomp: unknown label %q", j.label)
			}
			off := target - (i + 1)
			if off < 0 || off > 255 {
				return nil, errors.New("seccomp: a jump does not fit in 8 bits")
			}
			*j.dst = uint8(off)
		}
		prog[i] = in
	}
	return prog, nil
}
