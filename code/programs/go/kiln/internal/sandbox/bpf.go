package sandbox

import (
	"errors"
	"fmt"

	"github.com/adhithyan15/coding-adventures/code/programs/go/kiln/internal/syscallceil"
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
	RetUserNotif   uint32 = 0x7fc00000
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
	offArg1 = 24
)

// Classic BPF opcodes (linux/filter.h).
const (
	bpfLdWAbs uint16 = 0x20 // BPF_LD  | BPF_W   | BPF_ABS
	bpfJeqK   uint16 = 0x15 // BPF_JMP | BPF_JEQ | BPF_K
	bpfJgtK   uint16 = 0x25 // BPF_JMP | BPF_JGT | BPF_K (unsigned)
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

// MaxKnownSyscall is the highest x86-64 system call number the pinned strace
// (6.8, Linux 6.8's table) and the parser's table know: lsm_list_modules, 461.
// Every number ABOVE it gets ENOSYS (KILN04 section 4.7, L7).  A call newer than
// the pinned strace is neither printed (the trace set cannot name it) nor
// refused by the parser, so a step could use a path-taking one (getxattrat,
// listxattrat, setxattrat, removexattrat 463-466, open_tree_attr 467,
// file_getattr and file_setattr 468 and 469) without a line in the trace.
// Failing closed with ENOSYS also makes a libc that probes for them fall back
// to the older call.  Nothing the Go runtime, sh, coreutils or the toolchains
// use today is above 461 (glibc 2.39 stops at fchmodat2 452 and clone3 435; the
// kernel's 462 is mseal, which none of them call).  WHEN THE PINNED STRACE IS
// BUMPED, RAISE THIS to the highest number its table knows, and add the new
// path-taking calls to the parser's table first (KILN04 section 14).
//
// The number itself lives in internal/syscallceil, so the trace parser's D47
// rule (a dying call above the ceiling did nothing) reads the very constant the
// filter's jgt uses.  A bump is made there.
const MaxKnownSyscall = syscallceil.Max

// Syscall numbers the filters treat specially.
const (
	sysClone       uint32 = 56
	sysClone3      uint32 = 435
	sysPrctl       uint32 = 157
	sysExecve      uint32 = 59
	sysExecat      uint32 = 322 // execveat
	sysSeccompCall uint32 = 317
)

// SeccompFlagNewListener is SECCOMP_FILTER_FLAG_NEW_LISTENER, the flag of
// seccomp(2) that makes a filter's listener.
const SeccompFlagNewListener = 1 << 3

// PrSetMM is prctl's option 35, PR_SET_MM: it moves the process's own memory
// map description, including `/proc/<pid>/exe` (PR_SET_MM_EXE_FILE and
// PR_SET_MM_MAP with exe_fd).  The exec-head supervisor confirms an exec by
// watching that link change (KILN04 D48), so a step that could re-point it could
// make a confirm lie.  The kernel wants CAP_SYS_RESOURCE, which the sandbox's
// init has dropped, so this refusal is defence in depth: the hole is closed by
// the filter whatever the capability check does.  The filter refuses EVERY
// sub-option, unconditionally.  No build step has a reason to call it.
const PrSetMM uint32 = 35

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
//	jgt MaxKnownSyscall    then -> ENOSYS
//	jeq <each refused nr>  then -> EPERM
//	jeq clone              then -> CLONE
//	jeq clone3             then -> ENOSYS
//	jeq prctl              then -> PRCTL
//	ret ALLOW
//	CLONE: ld args[0] (low 32 bits); jset <refused flags> then -> EPERM; ret ALLOW
//	PRCTL: ld args[0] (low 32 bits); jeq PR_SET_MM then -> EPERM; ret ALLOW
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
//
// prctl's OPTION is likewise the first argument and an `int`: the kernel's
// prctl wrapper takes `int option`, so only the low 32 bits of args[0] choose
// the option.  The PR_SET_MM test therefore loads and compares the LOW word
// only.  A comparison of all 64 bits would let `35 | 1<<32` through the filter
// and straight into PR_SET_MM (KILN04 D48).
func BuildFilter() ([]Instruction, error) {
	steps := []step{
		{code: bpfLdWAbs, k: offArch},
		{code: bpfJeqK, k: AuditArchX86_64, jf: "kill"},
		{code: bpfLdWAbs, k: offNr},
		{code: bpfJsetK, k: X32SyscallBit, jt: "kill"},
		{code: bpfJgtK, k: MaxKnownSyscall, jt: "enosys"},
	}
	for _, s := range RefusedSyscalls {
		steps = append(steps, step{code: bpfJeqK, k: s.Nr, jt: "eperm"})
	}
	steps = append(steps,
		step{code: bpfJeqK, k: sysClone, jt: "clone"},
		step{code: bpfJeqK, k: sysClone3, jt: "enosys"},
		step{code: bpfJeqK, k: sysPrctl, jt: "prctl"},
		step{code: bpfRetK, k: RetAllow},
		step{label: "clone", code: bpfLdWAbs, k: offArg0},
		step{code: bpfJsetK, k: CloneRefusedSet, jt: "eperm"},
		step{code: bpfRetK, k: RetAllow},
		step{label: "prctl", code: bpfLdWAbs, k: offArg0},
		step{code: bpfJeqK, k: PrSetMM, jt: "eperm"},
		step{code: bpfRetK, k: RetAllow},
		step{label: "eperm", code: bpfRetK, k: RetErrno | Eperm},
		step{label: "enosys", code: bpfRetK, k: RetErrno | Enosys},
		step{label: "kill", code: bpfRetK, k: RetKillProcess},
	)
	return assemble(steps)
}

// BuildExecNotifyFilter assembles the SECOND filter, the one the init installs
// with SECCOMP_FILTER_FLAG_NEW_LISTENER when --exec-notify-fd is given (KILN04
// D48).  It is self-contained and says one thing: tell the supervisor about every
// exec.
//
//	ld  arch
//	jeq AUDIT_ARCH_X86_64  else -> KILL
//	ld  nr
//	jset __X32_SYSCALL_BIT then -> KILL
//	jeq seccomp            then -> SECCOMP
//	jeq execve             then -> NOTIFY
//	jeq execveat           then -> NOTIFY
//	ret ALLOW
//	SECCOMP: ld args[1] (low 32 bits); jset NEW_LISTENER then -> EPERM; ret ALLOW
//	NOTIFY: ret USER_NOTIF
//	EPERM: ret ERRNO|EPERM
//	KILL: ret KILL_PROCESS
//
// It has its own architecture and x32 tests, exactly the first filter's, so a
// number of another ABI is never read as x86-64's execve (59 is another call on
// i386 and aarch64).
//
// HOW IT COMBINES WITH THE FIRST.  Every filter of a task runs on every call and
// the most restrictive verdict wins: KILL, TRAP, ERRNO, USER_NOTIF, TRACE, LOG,
// ALLOW, in that order.  So the first filter's EPERM and ENOSYS refusals and its
// KILLs all beat USER_NOTIF (a refused call is never reported), and an execve,
// which the first filter allows, is notified.  The same order is why a step's
// own filter that returns an ERRNO for execve simply loses the step its execs
// (the notification never happens), while one that returns ALLOW cannot turn a
// notification off.
//
// THE EQUAL-ACTION TIE (a step stealing the notifications).  A step could
// install a filter of its own that also answers USER_NOTIF for execve.  When two
// filters give the same action the kernel keeps the NEWEST one's verdict, so the
// notification would go to the step's listener, not the supervisor's.  The step
// cannot get there:
//
//   - To get a listener it needs seccomp(SET_MODE_FILTER, NEW_LISTENER).  A task
//     may have only ONE listener in its whole filter chain: the kernel's
//     has_duplicate_listener() walks every filter of the task and fails the call
//     with EBUSY if any has one, and the init's second filter does.
//   - A step filter that returns USER_NOTIF WITHOUT NEW_LISTENER has no listener
//     of its own, and a USER_NOTIF verdict for a filter with no listener fails the
//     call with ENOSYS.  The exec is lost; nothing runs unseen.  That is
//     fail-closed.
//
// Defence in depth, so the refusal is ours and not only the kernel's: this filter
// itself answers EPERM to seccomp(2) when the low 32 bits of its flags argument
// (args[1]; an unsigned int) have SECCOMP_FILTER_FLAG_NEW_LISTENER set.  It sits
// BEFORE the execve compares.  seccomp(2) without that flag, and
// prctl(PR_SET_SECCOMP), stay allowed: steps that sandbox their own tests need
// them, and a filter without a listener cannot take a notification.
func BuildExecNotifyFilter() ([]Instruction, error) {
	return assemble([]step{
		{code: bpfLdWAbs, k: offArch},
		{code: bpfJeqK, k: AuditArchX86_64, jf: "kill"},
		{code: bpfLdWAbs, k: offNr},
		{code: bpfJsetK, k: X32SyscallBit, jt: "kill"},
		{code: bpfJeqK, k: sysSeccompCall, jt: "seccomp"},
		{code: bpfJeqK, k: sysExecve, jt: "notify"},
		{code: bpfJeqK, k: sysExecat, jt: "notify"},
		{code: bpfRetK, k: RetAllow},
		{label: "seccomp", code: bpfLdWAbs, k: offArg1},
		{code: bpfJsetK, k: SeccompFlagNewListener, jt: "eperm"},
		{code: bpfRetK, k: RetAllow},
		{label: "notify", code: bpfRetK, k: RetUserNotif},
		{label: "eperm", code: bpfRetK, k: RetErrno | Eperm},
		{label: "kill", code: bpfRetK, k: RetKillProcess},
	})
}

// assemble resolves the labels of steps into jump offsets.
func assemble(steps []step) ([]Instruction, error) {
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
