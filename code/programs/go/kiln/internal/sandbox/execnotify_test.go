package sandbox

// KILN04 D48 step 3, the parts that are pure and so run on every OS: the
// --exec-notify-fd arguments of both helper commands, the socket decision, the
// PR_SET_MM refusal of the first filter and the second (listener) filter.  The
// real-kernel tests are in the kiln package (sandbox_execnotify_*_test.go).

import (
	"reflect"
	"strings"
	"testing"
)

func TestParseExecArgsExecNotifyFD(t *testing.T) {
	sh := []string{"sh", "-c", "make"}
	for name, tc := range map[string]struct {
		args   []string
		notify int
	}{
		"absent":            {[]string{"--config", "/c", "--", "sh", "-c", "make"}, NoFD},
		"alone":             {[]string{"--config", "/c", "--exec-notify-fd", "6", "--", "sh", "-c", "make"}, 6},
		"after outputs":     {[]string{"--config", "/c", "--stdout-fd", "3", "--stderr-fd", "4", "--exec-notify-fd", "6", "--", "sh", "-c", "make"}, 6},
		"after sha+outputs": {[]string{"--config", "/c", "--config-sha256", strings.Repeat("a", 64), "--stdout-fd", "3", "--stderr-fd", "4", "--exec-notify-fd", "5", "--", "sh", "-c", "make"}, 5},
	} {
		a, err := ParseExecArgs(tc.args)
		if err != nil {
			t.Errorf("%s: %v", name, err)
			continue
		}
		if a.ExecNotifyFD != tc.notify || !reflect.DeepEqual(a.Argv, sh) {
			t.Errorf("%s: %+v", name, a)
		}
	}
	bad := map[string][]string{
		"before the outputs": {"--config", "/c", "--exec-notify-fd", "6", "--stdout-fd", "3", "--stderr-fd", "4", "--", "sh"},
		"twice":              {"--config", "/c", "--exec-notify-fd", "6", "--exec-notify-fd", "7", "--", "sh"},
		"no number":          {"--config", "/c", "--exec-notify-fd", "--", "sh"},
		"signed":             {"--config", "/c", "--exec-notify-fd", "-6", "--", "sh"},
		"hex":                {"--config", "/c", "--exec-notify-fd", "0x6", "--", "sh"},
		"a standard stream":  {"--config", "/c", "--exec-notify-fd", "2", "--", "sh"},
		"too big":            {"--config", "/c", "--exec-notify-fd", "1048577", "--", "sh"},
		"is the stdout fd":   {"--config", "/c", "--stdout-fd", "3", "--stderr-fd", "4", "--exec-notify-fd", "3", "--", "sh"},
		"is the stderr fd":   {"--config", "/c", "--stdout-fd", "3", "--stderr-fd", "4", "--exec-notify-fd", "4", "--", "sh"},
		"no argv":            {"--config", "/c", "--exec-notify-fd", "6", "--"},
		"no separator":       {"--config", "/c", "--exec-notify-fd", "6", "sh", "x"},
	}
	for name, args := range bad {
		if a, err := ParseExecArgs(args); err == nil {
			t.Errorf("%s: accepted: %+v", name, a)
		}
	}
	// After the `--` everything is the step's argv, whatever it looks like.
	a, err := ParseExecArgs([]string{"--config", "/c", "--", "--exec-notify-fd", "6"})
	if err != nil || a.ExecNotifyFD != NoFD || !reflect.DeepEqual(a.Argv, []string{"--exec-notify-fd", "6"}) {
		t.Errorf("a flag in the step's argv: %+v, %v", a, err)
	}
}

func TestParseInitArgsExecNotifyFD(t *testing.T) {
	sum := strings.Repeat("a", 64)
	base := []string{"--config", "/c.json", "--sha256", sum, "--action", "pkg", "--uid", "501", "--gid", "20"}
	with := func(rest ...string) []string { return append(append([]string{}, base...), rest...) }

	a, err := ParseInitArgs(with("--", "sh", "-c", "x"))
	if err != nil || a.ExecNotifyFD != NoFD || !reflect.DeepEqual(a.Argv, []string{"sh", "-c", "x"}) {
		t.Fatalf("without the option: %+v, %v", a, err)
	}
	a, err = ParseInitArgs(with("--exec-notify-fd", "4", "--", "sh", "-c", "x"))
	if err != nil || a.ExecNotifyFD != 4 || a.Config != "/c.json" || a.UID != 501 || a.GID != 20 || !reflect.DeepEqual(a.Argv, []string{"sh", "-c", "x"}) {
		t.Fatalf("with the option: %+v, %v", a, err)
	}
	for name, args := range map[string][]string{
		"no number":      with("--exec-notify-fd", "--", "sh"),
		"bad number":     with("--exec-notify-fd", "x", "--", "sh", "y"),
		"low number":     with("--exec-notify-fd", "2", "--", "sh", "y"),
		"no separator":   with("--exec-notify-fd", "4", "sh", "y"),
		"no argv":        with("--exec-notify-fd", "4", "--"),
		"twice":          with("--exec-notify-fd", "4", "--exec-notify-fd", "5", "--", "sh"),
		"truncated flag": with("--exec-notify-fd", "4"),
	} {
		if a, err := ParseInitArgs(args); err == nil {
			t.Errorf("%s: accepted: %+v", name, a)
		}
	}
	// The option is a flag of the helper, never of the step: the same text
	// after the `--` is argv.
	a, err = ParseInitArgs(with("--", "--exec-notify-fd", "4"))
	if err != nil || a.ExecNotifyFD != NoFD || !reflect.DeepEqual(a.Argv, []string{"--exec-notify-fd", "4"}) {
		t.Errorf("a flag in the step's argv: %+v, %v", a, err)
	}
}

func TestCheckExecNotifySocket(t *testing.T) {
	const (
		sock = 0o140000 | 0o755
		fifo = 0o010000 | 0o600
		reg  = 0o100000 | 0o644
	)
	if err := CheckExecNotifySocket("fd 6", sock, 1, 1, true); err != nil {
		t.Errorf("a connected AF_UNIX stream socket: %v", err)
	}
	for name, tc := range map[string]struct {
		mode         uint32
		domain, kind int
		connected    bool
		want         string
	}{
		"a pipe":        {fifo, 0, 0, false, "not a socket"},
		"a file":        {reg, 0, 0, false, "not a socket"},
		"no type bits":  {0, 1, 1, true, "not a socket"},
		"AF_INET":       {sock, 2, 1, true, "AF_UNIX"},
		"AF_NETLINK":    {sock, 16, 1, true, "AF_UNIX"},
		"unknown":       {sock, 0, 1, true, "AF_UNIX"},
		"datagram":      {sock, 1, 2, true, "SOCK_STREAM"},
		"seqpacket":     {sock, 1, 5, true, "SOCK_STREAM"},
		"not connected": {sock, 1, 1, false, "not a connected socket"},
	} {
		err := CheckExecNotifySocket("--exec-notify-fd 6", tc.mode, tc.domain, tc.kind, tc.connected)
		if err == nil || !strings.Contains(err.Error(), tc.want) || !strings.Contains(err.Error(), "--exec-notify-fd 6") {
			t.Errorf("%s: %v, want an error containing %q", name, err, tc.want)
		}
	}
	// The notification socket is a SOCKET: the output check refuses it by
	// name, so the two checks cannot be mixed up.
	if err := CheckOutputFD("fd", sock, oWronly); err == nil {
		t.Error("CheckOutputFD accepted a socket")
	}
	if err := CheckExecNotifySocket("fd", fifo|oWronly, 1, 1, true); err == nil {
		t.Error("CheckExecNotifySocket accepted a pipe")
	}
}

// TestHandshakeConstants pins what PR 4's supervisor reads: one byte, a
// message prefix, and the refusal text.
func TestHandshakeConstants(t *testing.T) {
	if ExecNotifyByte != 'L' || ExecNotifyByteKillable != 'K' {
		t.Errorf("handshake bytes %q %q", ExecNotifyByte, ExecNotifyByteKillable)
	}
	if !strings.HasPrefix(ExecNotifyRefused, "exec-notify") || !strings.Contains(ExecNotifyListenerGone, "did not run") {
		t.Errorf("%q %q", ExecNotifyRefused, ExecNotifyListenerGone)
	}
	if ExitFailure != 125 {
		t.Errorf("a refused step exits %d", ExitFailure)
	}
}

// ---------------------------------------------------------------- filters

func prctlCall(opt uint64) seccompData {
	d := native(sysPrctl)
	d.args[0] = opt
	return d
}

// TestFilterRefusesPrSetMM: prctl(PR_SET_MM, ...) is EPERM, whatever the
// sub-option and the other arguments, and with any bits in the high word of the
// option (the kernel's option is an int: it reads only the low 32 bits).  Other
// prctl options are untouched.
func TestFilterRefusesPrSetMM(t *testing.T) {
	p := filter(t)
	want := RetErrno | Eperm
	if PrSetMM != 35 {
		t.Fatalf("PR_SET_MM is %d", PrSetMM)
	}
	for _, opt := range []uint64{
		35,
		35 | 1<<32,
		35 | 0xffffffff<<32,
		35 | 1<<63,
		35 | 0x80000000<<32,
		0xdeadbeef00000023,
	} {
		if got := evalBPF(t, p, prctlCall(opt)); got != want {
			t.Errorf("prctl(%#x): %#x, want EPERM", opt, got)
		}
		// The sub-option and the rest are irrelevant: refused unconditionally.
		for sub := uint64(0); sub < 16; sub++ {
			d := prctlCall(opt)
			d.args[1], d.args[2], d.args[4] = sub, sub<<40, 1
			if got := evalBPF(t, p, d); got != want {
				t.Errorf("prctl(%#x, sub %d): %#x, want EPERM", opt, sub, got)
			}
		}
	}
	// Everything else: allowed, including the neighbours of 35, options that
	// have 35 only in the HIGH word, and the options a build really uses.
	for _, opt := range []uint64{
		0, 1, 15 /* PR_SET_NAME */, 16 /* PR_GET_NAME */, 22 /* PR_SET_SECCOMP */, 34, 36, 37, 38, 39,
		35 << 32, 35<<32 | 39, 35<<32 | 15, 0x59616d61 /* PR_SET_PTRACER */, 0xffffffff,
		34 | 1<<32, 36 | 1<<32,
	} {
		if got := evalBPF(t, p, prctlCall(opt)); got != RetAllow {
			t.Errorf("prctl(%#x): %#x, want ALLOW", opt, got)
		}
	}
	// Only prctl is looked at: 35 as the first argument of another call is not
	// PR_SET_MM.
	for _, nr := range []uint32{0, 1, 59, 157 - 1, 157 + 1, 317} {
		d := native(nr)
		d.args[0] = 35
		if got := evalBPF(t, p, d); got != RetAllow {
			t.Errorf("nr %d with args[0]=35: %#x, want ALLOW", nr, got)
		}
	}
	// A wrong architecture still dies first.
	d := prctlCall(35)
	d.arch = 0x40000003
	if got := evalBPF(t, p, d); got != RetKillProcess {
		t.Errorf("prctl on another ABI: %#x", got)
	}
}

func TestExecNotifyFilterVerdicts(t *testing.T) {
	p, err := BuildExecNotifyFilter()
	if err != nil {
		t.Fatal(err)
	}
	// Its own architecture test and x32 test, as the first filter's.
	for _, arch := range []uint32{0x40000003, 0xc00000b7, 0, 0xffffffff} {
		for _, nr := range []uint32{0, 59, 322} {
			if got := evalBPF(t, p, seccompData{nr: nr, arch: arch}); got != RetKillProcess {
				t.Errorf("arch %#x nr %d: %#x, want KILL_PROCESS", arch, nr, got)
			}
		}
	}
	for _, nr := range []uint32{0, 59, 322, 0} {
		if got := evalBPF(t, p, native(nr|X32SyscallBit)); got != RetKillProcess {
			t.Errorf("x32 nr %#x: %#x, want KILL_PROCESS", nr|X32SyscallBit, got)
		}
	}
	// Exactly execve and execveat are notified; everything else is allowed,
	// including the numbers the FIRST filter refuses (its verdict is the one
	// that counts for them), and whatever the arguments are.
	for nr := uint32(0); nr < 1024; nr++ {
		want := RetAllow
		if nr == 59 || nr == 322 {
			want = RetUserNotif
		}
		for _, a0 := range []uint64{0, 35, 1 << 32, ^uint64(0)} {
			d := native(nr)
			d.args[0] = a0
			if got := evalBPF(t, p, d); got != want {
				t.Errorf("nr %d args[0]=%#x: %#x, want %#x", nr, a0, got, want)
			}
		}
	}
	for _, nr := range []uint32{0x3fffffff, 0x80000000, MaxKnownSyscall + 1} {
		if got := evalBPF(t, p, native(nr)); got != RetAllow {
			t.Errorf("nr %#x: %#x, want ALLOW (the first filter answers these)", nr, got)
		}
	}
	if RetUserNotif != 0x7fc00000 {
		t.Errorf("USER_NOTIF is %#x", RetUserNotif)
	}
	if last := p[len(p)-1]; last.Code != 0x06 || last.K != RetKillProcess {
		t.Errorf("last instruction %+v", last)
	}
	for i, in := range p {
		if in.Code == 0x15 || in.Code == 0x45 {
			if i+1+int(in.Jt) >= len(p) || i+1+int(in.Jf) >= len(p) {
				t.Errorf("instruction %d jumps out of the program", i)
			}
		}
	}
	// A distinct program from the first: the two never share state.
	first := filter(t)
	if reflect.DeepEqual(first, p) {
		t.Error("the second filter is the first")
	}
}

// TestExecNotifyFilterRefusesNewListener: seccomp(2) with
// SECCOMP_FILTER_FLAG_NEW_LISTENER among its flags is EPERM in the second
// filter, whatever junk is in the high bits of the (unsigned int) flags
// argument; other flags and other operations are allowed; and the check does not
// touch other calls with that bit in args[1].
func TestExecNotifyFilterRefusesNewListener(t *testing.T) {
	p, err := BuildExecNotifyFilter()
	if err != nil {
		t.Fatal(err)
	}
	call := func(op, flags uint64) seccompData {
		d := native(317)
		d.args[0], d.args[1] = op, flags
		return d
	}
	want := RetErrno | Eperm
	for _, flags := range []uint64{
		SeccompFlagNewListener, SeccompFlagNewListener | 1, SeccompFlagNewListener | 1<<4,
		SeccompFlagNewListener | 1<<32, SeccompFlagNewListener | 0xffffffff<<32, 0xffffffff,
	} {
		for _, op := range []uint64{0, 1, 2, 3} {
			if got := evalBPF(t, p, call(op, flags)); got != want {
				t.Errorf("seccomp(%d, %#x): %#x, want EPERM", op, flags, got)
			}
		}
	}
	for _, flags := range []uint64{0, 1, 2, 4, 1 << 4, 1 << 5, 1 << 32, 8 << 32, 0xfffffff7} {
		if got := evalBPF(t, p, call(1, flags)); got != RetAllow {
			t.Errorf("seccomp(1, %#x): %#x, want ALLOW", flags, got)
		}
	}
	// Another call with the bit in args[1] is untouched; execve still notifies.
	for _, nr := range []uint32{0, 157, 316, 318} {
		d := native(nr)
		d.args[1] = SeccompFlagNewListener
		if got := evalBPF(t, p, d); got != RetAllow {
			t.Errorf("nr %d: %#x, want ALLOW", nr, got)
		}
	}
	d := native(59)
	d.args[1] = SeccompFlagNewListener
	if got := evalBPF(t, p, d); got != RetUserNotif {
		t.Errorf("execve: %#x", got)
	}
	// Combined with the first filter, EPERM still beats everything but KILL.
	first := filter(t)
	got := combined(evalBPF(t, first, call(1, SeccompFlagNewListener)), evalBPF(t, p, call(1, SeccompFlagNewListener)))
	if got != want {
		t.Errorf("combined: %#x", got)
	}
	// prctl(PR_SET_SECCOMP) and seccomp without the flag stay allowed in the first.
	if evalBPF(t, first, call(1, 0)) != RetAllow || evalBPF(t, first, prctlCall(22)) != RetAllow {
		t.Error("the first filter refuses seccomp or PR_SET_SECCOMP")
	}
}

// restrictiveness orders seccomp verdicts as the kernel does: the action is
// compared as a signed integer and the lowest wins (KILL_PROCESS 0x80000000 is
// the most negative, then TRAP, ERRNO, USER_NOTIF, TRACE, LOG, ALLOW).
func restrictiveness(v uint32) int32 { return int32(v & 0xffff0000) }

func combined(vs ...uint32) uint32 {
	best := vs[0]
	for _, v := range vs[1:] {
		if restrictiveness(v) < restrictiveness(best) {
			best = v
		}
	}
	return best
}

// TestFiltersCombine: with both filters installed the first one's refusals
// still win, and an exec is notified (KILN04 D48, "the first filter's
// refusals take precedence").
func TestFiltersCombine(t *testing.T) {
	first := filter(t)
	second, err := BuildExecNotifyFilter()
	if err != nil {
		t.Fatal(err)
	}
	if !(restrictiveness(RetKillProcess) < restrictiveness(RetErrno|Eperm) &&
		restrictiveness(RetErrno|Eperm) < restrictiveness(RetUserNotif) &&
		restrictiveness(RetUserNotif) < restrictiveness(RetAllow)) {
		t.Fatal("the precedence model is wrong")
	}
	for nr := uint32(0); nr < 1024; nr++ {
		for _, a0 := range []uint64{0, 35, 35 | 1<<32, 0x00800000, 0x10000000} {
			d := native(nr)
			d.args[0] = a0
			f := evalBPF(t, first, d)
			s := evalBPF(t, second, d)
			got := combined(f, s)
			switch {
			case f != RetAllow:
				if got != f {
					t.Errorf("nr %d args[0]=%#x: first says %#x, combined %#x", nr, a0, f, got)
				}
			case nr == 59 || nr == 322:
				if got != RetUserNotif {
					t.Errorf("nr %d: %#x, want USER_NOTIF", nr, got)
				}
			default:
				if got != RetAllow {
					t.Errorf("nr %d args[0]=%#x: %#x, want ALLOW", nr, a0, got)
				}
			}
		}
	}
	// Other ABIs and x32 stay KILL in the combination.
	if got := combined(evalBPF(t, first, seccompData{nr: 59, arch: 0x40000003}), evalBPF(t, second, seccompData{nr: 59, arch: 0x40000003})); got != RetKillProcess {
		t.Errorf("i386 execve: %#x", got)
	}
	// And the step's own filter, if it installs one, cannot undo a
	// notification: ALLOW loses to USER_NOTIF, and an ERRNO for execve wins
	// over it (the step merely loses its execs).
	if got := combined(RetAllow, RetAllow, RetUserNotif); got != RetUserNotif {
		t.Errorf("a step's ALLOW: %#x", got)
	}
	if got := combined(RetAllow, RetUserNotif, RetErrno|13); got != RetErrno|13 {
		t.Errorf("a step's ERRNO for execve: %#x", got)
	}
}
