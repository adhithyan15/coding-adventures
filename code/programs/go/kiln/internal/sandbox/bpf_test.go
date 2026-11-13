package sandbox

import (
	"testing"
)

// seccompData is the input of the filter: the fields of struct seccomp_data.
type seccompData struct {
	nr   uint32
	arch uint32
	args [6]uint64
}

// word reads the 32-bit word at a seccomp_data offset (little endian, as on
// x86-64).
func (d seccompData) word(off uint32) uint32 {
	switch {
	case off == 0:
		return d.nr
	case off == 4:
		return d.arch
	case off >= 16 && off < 64 && off%4 == 0:
		a := d.args[(off-16)/8]
		if (off-16)%8 == 0 {
			return uint32(a)
		}
		return uint32(a >> 32)
	}
	panic("load outside seccomp_data")
}

// evalBPF is a minimal interpreter for the classic-BPF subset BuildFilter
// uses, so the program is tested by running it, not by comparing it with
// itself.  It fails the test on an unknown opcode or a jump off the end.
func evalBPF(t *testing.T, prog []Instruction, d seccompData) uint32 {
	t.Helper()
	var a uint32
	for pc := 0; pc < len(prog); {
		in := prog[pc]
		pc++
		switch in.Code {
		case 0x20: // ld abs
			a = d.word(in.K)
		case 0x15: // jeq k
			if a == in.K {
				pc += int(in.Jt)
			} else {
				pc += int(in.Jf)
			}
		case 0x45: // jset k
			if a&in.K != 0 {
				pc += int(in.Jt)
			} else {
				pc += int(in.Jf)
			}
		case 0x06: // ret k
			return in.K
		default:
			t.Fatalf("unknown opcode %#x at %d", in.Code, pc-1)
		}
	}
	t.Fatal("program fell off the end")
	return 0
}

func filter(t *testing.T) []Instruction {
	t.Helper()
	p, err := BuildFilter()
	if err != nil {
		t.Fatal(err)
	}
	return p
}

func native(nr uint32) seccompData { return seccompData{nr: nr, arch: AuditArchX86_64} }

func TestFilterWrongArchIsKilled(t *testing.T) {
	p := filter(t)
	for _, arch := range []uint32{0x40000003 /* i386 */, 0xc00000b7 /* aarch64 */, 0, 0xffffffff} {
		// Even a call number that is allowed natively.
		if got := evalBPF(t, p, seccompData{nr: 0, arch: arch}); got != RetKillProcess {
			t.Errorf("arch %#x: %#x, want KILL_PROCESS", arch, got)
		}
		// And one that is refused natively.
		if got := evalBPF(t, p, seccompData{nr: 165, arch: arch}); got != RetKillProcess {
			t.Errorf("arch %#x mount: %#x, want KILL_PROCESS", arch, got)
		}
	}
}

func TestFilterX32IsKilled(t *testing.T) {
	p := filter(t)
	for _, nr := range []uint32{0, 56, 165, 435, X32SyscallBit | 56, X32SyscallBit} {
		got := evalBPF(t, p, native(nr|X32SyscallBit))
		if got != RetKillProcess {
			t.Errorf("x32 nr %#x: %#x, want KILL_PROCESS", nr|X32SyscallBit, got)
		}
	}
}

func TestFilterRefusedCallsGetEPERM(t *testing.T) {
	p := filter(t)
	if len(RefusedSyscalls) != 20 {
		t.Fatalf("the refused table has %d calls; KILN04 section 4.4 names 20", len(RefusedSyscalls))
	}
	want := RetErrno | Eperm
	seen := map[uint32]bool{}
	for _, s := range RefusedSyscalls {
		if seen[s.Nr] {
			t.Errorf("%s: number %d listed twice", s.Name, s.Nr)
		}
		seen[s.Nr] = true
		if got := evalBPF(t, p, native(s.Nr)); got != want {
			t.Errorf("%s (%d): %#x, want EPERM %#x", s.Name, s.Nr, got, want)
		}
		// The arguments are irrelevant to these.
		d := native(s.Nr)
		d.args[0] = 0
		if got := evalBPF(t, p, d); got != want {
			t.Errorf("%s with zero args: %#x", s.Name, got)
		}
	}
	if want != 0x00050001 {
		t.Fatalf("EPERM verdict %#x", want)
	}
}

func TestFilterClone(t *testing.T) {
	p := filter(t)
	clone := func(flags uint64) uint32 {
		d := native(56)
		d.args[0] = flags
		return evalBPF(t, p, d)
	}
	const (
		vm, fs, files, sighand, thread, sysvsem = 0x100, 0x200, 0x400, 0x800, 0x10000, 0x40000
		settls, parentSettid, childCleartid     = 0x80000, 0x100000, 0x200000
		sigchld                                 = 17
	)
	goThread := uint64(vm | fs | files | sighand | thread | sysvsem)
	for name, flags := range map[string]uint64{
		"go thread":         goThread,
		"pthread":           goThread | settls | parentSettid | childCleartid,
		"plain fork":        sigchld,
		"fork with a vfork": sigchld | 0x4000 | vm,
		"zero":              0,
	} {
		if got := clone(flags); got != RetAllow {
			t.Errorf("clone %s: %#x, want ALLOW", name, got)
		}
	}
	refused := map[string]uint32{
		"CLONE_UNTRACED": CloneUntraced, "CLONE_NEWUSER": CloneNewUser, "CLONE_NEWNS": CloneNewNS,
		"CLONE_NEWPID": CloneNewPID, "CLONE_NEWNET": CloneNewNet, "CLONE_NEWCGROUP": CloneNewCgroup,
		"CLONE_NEWIPC": CloneNewIPC, "CLONE_NEWUTS": CloneNewUTS,
	}
	for name, bit := range refused {
		for _, base := range []uint64{0, goThread, sigchld} {
			if got := clone(base | uint64(bit)); got != RetErrno|Eperm {
				t.Errorf("clone %s over %#x: %#x, want EPERM", name, base, got)
			}
		}
	}
	// Only the low 32 bits are flags; the kernel ignores the rest, and so does
	// the filter (an allowed call with junk above is still allowed).
	if got := clone(goThread | 1<<40); got != RetAllow {
		t.Errorf("junk in the high word: %#x", got)
	}
	// The flags are args[0] on x86-64: nonzero args[1] means nothing.
	d := native(56)
	d.args[1] = uint64(CloneUntraced)
	if got := evalBPF(t, p, d); got != RetAllow {
		t.Errorf("args[1] mistaken for flags: %#x", got)
	}
}

func TestFilterClone3IsENOSYS(t *testing.T) {
	p := filter(t)
	d := native(435)
	d.args[0] = 1 << 20 // a pointer, as clone3's first argument is
	if got := evalBPF(t, p, d); got != 0x00050026 {
		t.Errorf("clone3: %#x, want ENOSYS", got)
	}
}

func TestFilterAllowsEverythingElse(t *testing.T) {
	p := filter(t)
	refused := map[uint32]bool{56: true, 435: true}
	for _, s := range RefusedSyscalls {
		refused[s.Nr] = true
	}
	// read, write, open, execve, exit_group, openat, getdents64, seccomp,
	// prctl, futex, execveat, io_uring_enter (useless without a setup).
	for _, nr := range []uint32{0, 1, 2, 59, 231, 257, 217, 317, 157, 202, 322, 426, 0x3ff} {
		if refused[nr] {
			t.Fatalf("test bug: %d is refused", nr)
		}
		if got := evalBPF(t, p, native(nr)); got != RetAllow {
			t.Errorf("nr %d: %#x, want ALLOW", nr, got)
		}
	}
	// Exhaustive: nothing but the listed numbers changes the verdict below 1024.
	for nr := uint32(0); nr < 1024; nr++ {
		got := evalBPF(t, p, native(nr))
		switch {
		case refused[nr] && nr != 56 && nr != 435:
			if got != RetErrno|Eperm {
				t.Errorf("nr %d: %#x", nr, got)
			}
		case nr == 435:
			if got != RetErrno|Enosys {
				t.Errorf("nr %d: %#x", nr, got)
			}
		default:
			if got != RetAllow {
				t.Errorf("nr %d: %#x, want ALLOW", nr, got)
			}
		}
	}
}

func TestFilterShape(t *testing.T) {
	p := filter(t)
	// The first instructions are the architecture test, then the x32 test.
	if p[0] != (Instruction{Code: 0x20, K: offArch}) {
		t.Errorf("first instruction %+v loads the architecture", p[0])
	}
	if p[1].Code != 0x15 || p[1].K != AuditArchX86_64 || p[1].Jt != 0 {
		t.Errorf("second instruction %+v", p[1])
	}
	if p[2] != (Instruction{Code: 0x20, K: offNr}) || p[3].Code != 0x45 || p[3].K != X32SyscallBit {
		t.Errorf("third and fourth: %+v %+v", p[2], p[3])
	}
	// The last instruction is the KILL return that the early jumps land on.
	last := p[len(p)-1]
	if last.Code != 0x06 || last.K != RetKillProcess {
		t.Errorf("last instruction %+v", last)
	}
	// No jump leaves the program.
	for i, in := range p {
		if in.Code == 0x15 || in.Code == 0x45 {
			if i+1+int(in.Jt) >= len(p) || i+1+int(in.Jf) >= len(p) {
				t.Errorf("instruction %d jumps out of the program", i)
			}
		}
	}
	if len(p) > 4096 {
		t.Errorf("%d instructions exceed BPF_MAXINSNS", len(p))
	}
}

func TestFilterKnownNumbers(t *testing.T) {
	// A few x86-64 numbers, spelled out so a typo in the table is caught on
	// every OS (the linux/amd64 test compares against package syscall).
	want := map[string]uint32{
		"mount": 165, "umount2": 166, "pivot_root": 155, "chroot": 161, "ptrace": 101,
		"unshare": 272, "setns": 308, "bpf": 321, "io_uring_setup": 425, "mount_setattr": 442,
		"open_by_handle_at": 304, "name_to_handle_at": 303, "pidfd_getfd": 438, "process_vm_readv": 310,
		"fsopen": 430, "fsconfig": 431, "fsmount": 432, "fspick": 433, "open_tree": 428, "move_mount": 429,
	}
	for _, s := range RefusedSyscalls {
		if want[s.Name] != s.Nr {
			t.Errorf("%s = %d, want %d", s.Name, s.Nr, want[s.Name])
		}
		delete(want, s.Name)
	}
	if len(want) != 0 {
		t.Errorf("missing from the table: %v", want)
	}
}

func TestEvalHarnessRejectsBadPrograms(t *testing.T) {
	// The interpreter must not silently accept what the kernel would refuse.
	// (This covers the harness's own failure paths.)
	defer func() {
		if recover() == nil {
			t.Error("a load outside seccomp_data did not panic")
		}
	}()
	seccompData{}.word(3)
}

func TestSeccompDataHighWord(t *testing.T) {
	d := seccompData{args: [6]uint64{0x1122334455667788}}
	if d.word(16) != 0x55667788 || d.word(20) != 0x11223344 {
		t.Errorf("%#x %#x", d.word(16), d.word(20))
	}
}
