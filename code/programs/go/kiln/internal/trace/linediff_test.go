package trace

import (
	"math/rand"
	"os"
	"reflect"
	"strings"
	"testing"
)

// The line differential of D24 (KILN04 section 4.9): ONE line, an empty state,
// the old handleLine against the grammar handleLineG.  The rule is the spec's:
// new accepting where the old refused is a failure; both accepting must give the
// same record; a stricter-only divergence must be on the spec's list.

// lineVerdict compares the two on one line.  It returns "" when they agree (or
// differ as the spec allows) and a description otherwise; row names the allowed
// divergence, if one was used.
func lineVerdict(line string) (problem, row string) {
	so := &lineState{pend: map[int]*pending{}}
	sn := &lineState{pend: map[int]*pending{}}
	ro, eo := handleLine(line, 1, so)
	before := grammarPanics.Load()
	rn, en := handleLineG(line, 1, sn)
	if grammarPanics.Load() != before {
		return "recovered panic (never an allowed divergence)", ""
	}
	max := DefaultLimits().MaxLineBytes
	switch {
	case eo != nil && en == nil:
		return "FAIL-OPEN old=" + eo.Reason, ""
	case eo == nil && en != nil:
		if r := stricterRow(line, "", en.Reason, []byte(line), 1, max); r != "" {
			return "", r
		}
		return "stricter outside the list: new=" + en.Reason, ""
	case eo != nil:
		if eo.Reason == en.Reason {
			return "", ""
		}
		if r := stricterRow(line, eo.Reason, en.Reason, []byte(line), 1, max); r != "" {
			return "", r
		}
		return "reasons differ outside the list: old=" + eo.Reason + " new=" + en.Reason, ""
	}
	if !reflect.DeepEqual(ro, rn) || !reflect.DeepEqual(so.pend, sn.pend) {
		return "records or pending state differ", ""
	}
	return "", ""
}

var lineAtoms = []string{"a", "3", "_", "-", "<", ">", "\"", "\\", "(", ")", "[", "]", "{", "}", ",", "=", " ", " ", ".", "...", "?", "+", "|", "/", "*", "@", "x", "0", "7", "4", "\xc3\xa9", "\xff", "\xc2\xa0", "f(", "A(", "<unfinished ...>", " <unfinished ...>", "<... ", " resumed>", "+++", "---", "--- ", " ---", "???(", "???", "\\<", "\\>", "\\\"", "\\\\", "\\x41", "\\777", "\\101", "\\q", "3</x>", "AT_FDCWD</w>", "\"s\"", "\"s\"...", " = 0", " = ?", "exited with ", "killed by SIG", " (core dumped)", "[pid ", "] ", "'", "/*", "*/", "\t", "\x7f", "\x00", "A", "Z", "resumed", "<...", "unfinished", "NAME=", "\n"}

func genLine(r *rand.Rand, corpus []string) string {
	switch r.Intn(4) {
	case 0:
		l := []byte(corpus[r.Intn(len(corpus))])
		for k := r.Intn(4); k >= 0 && len(l) > 0; k-- {
			i := r.Intn(len(l) + 1)
			a := lineAtoms[r.Intn(len(lineAtoms))]
			switch r.Intn(3) {
			case 0:
				l = append(l[:i:i], append([]byte(a), l[i:]...)...)
			case 1:
				if i < len(l) {
					j := min(i+r.Intn(4), len(l))
					l = append(l[:i:i], l[j:]...)
				}
			case 2:
				if i < len(l) {
					l = append(l[:i:i], append([]byte(a), l[i+1:]...)...)
				}
			}
		}
		return string(l)
	case 1:
		var b strings.Builder
		b.WriteString([]string{"1  ", "12 ", "[pid 7] ", "[pid 7]", "1", "", "01  ", "[pid +7] "}[r.Intn(8)])
		for n := r.Intn(14); n >= 0; n-- {
			b.WriteString(lineAtoms[r.Intn(len(lineAtoms))])
		}
		return b.String()
	default:
		var b strings.Builder
		b.WriteString("1  ")
		b.WriteString([]string{"f(", "stat(", "openat(", "<... f resumed>", "--- ", "+++ ", "???(", "f("}[r.Intn(8)])
		for n := r.Intn(12); n >= 0; n-- {
			b.WriteString(lineAtoms[r.Intn(len(lineAtoms))])
		}
		b.WriteString([]string{") = 0", ")", "", " <unfinished ...>", " +++", " ---", ") = ?", ") = 3</x>"}[r.Intn(8)])
		return b.String()
	}
}

// fixtureLines reads every line of the four fixture traces.
func fixtureLines(tb testing.TB) []string {
	var out []string
	for _, n := range []string{"build.strace", "cwd.strace", "paths.strace", "net.strace"} {
		b, err := os.ReadFile("testdata/" + n)
		if err != nil {
			tb.Fatal(err)
		}
		out = append(out, strings.Split(strings.TrimRight(string(b), "\n"), "\n")...)
	}
	return out
}

func TestLineDiffSeeded(t *testing.T) {
	corpus := fixtureLines(t)
	n := 300000
	if testing.Short() {
		n = 30000
	}
	r := rand.New(rand.NewSource(42))
	bad := 0
	counts := map[string]int{}
	check := func(l string) {
		if d, row := lineVerdict(l); d != "" {
			if bad++; bad <= 10 {
				t.Errorf("%s: %q", d, l)
			}
		} else if row != "" {
			counts[row]++
		}
	}
	for _, l := range corpus {
		check(l)
	}
	for i := 0; i < n; i++ {
		check(genLine(r, corpus))
	}
	if g := grammarPanics.Load(); g != 0 {
		t.Fatalf("%d recovered panics in the grammar path", g)
	}
	t.Logf("%d lines (%d fixture lines); allowed stricter-only divergences by row: %v", n+len(corpus), len(corpus), counts)
}

// FuzzLineDiff: one line, an empty state, old against new.
func FuzzLineDiff(f *testing.F) {
	for _, l := range fixtureLines(f) {
		f.Add(l)
	}
	for _, l := range []string{
		"1  ???( <unfinished ...>", "1  ???()  = ?", "1  <... ??? resumed>)  = ?", "1  <... f resumed>) = 0",
		"1  stat(a) = 0 <unfinished ...>", "[pid +5] stat(a) = 0", "1  +++ exited with +5 +++", "1  stat(a) = 0 ",
		"1  f(\"a\\qb\") = 0", "1  f(\"a\"\"b\") = 0", "1  openat(3</a>x</b>, \"x\", O_RDONLY O_CLOEXEC) = 3</x>",
		"1  stat(\"a\\0b\", {}) = 0", "1  f(\xc2\xa0) = 0", "1  --- SIGCHLD {si_signo=SIGCHLD} ---",
		"1  <... f resumed></x>) = 0", "1  wait4(-1,  <unfinished ...>",
		"1  execve(\"/usr/bin/sh\", [\"sh\"], 0x1 /* 5 vars */ <unfinished ...>",
		// The filter's own answers (clone3_enosys_test.go): tolerated, and
		// the near misses that are not.
		"1  clone3({flags=CLONE_VM|CLONE_VFORK|CLONE_CLEAR_SIGHAND, exit_signal=SIGCHLD, stack=0x7f0a4b1ff000, stack_size=0x9000}, 88) = -1 ENOSYS (Function not implemented)",
		"1  clone3({flags=CLONE_VM|CLONE_THREAD, child_tid=0x1, parent_tid=0x1, exit_signal=0, stack=0x1, stack_size=0x1, tls=0x1} <unfinished ...>",
		"1  <... clone3 resumed>, 88) = -1 ENOSYS (Function not implemented)",
		"1  <... clone3 resumed>, 88) = -1 EPERM (Operation not permitted)", "1  clone3({flags=CLONE_VM}, 88) = ?",
		"1  mount(0, 0) = -1 EPERM (Operation not permitted)", "1  mount(0, 0) = 0", "1  <... mount resumed>0) = -1 ENOENT (No such file or directory)",
		"1  clone(child_stack=NULL, flags=CLONE_NEWUSER|SIGCHLD) = -1 EPERM (Operation not permitted)",
	} {
		f.Add(l)
	}
	f.Fuzz(func(t *testing.T, line string) {
		if d, _ := lineVerdict(line); d != "" {
			t.Fatalf("%s: %q", d, line)
		}
		if g := grammarPanics.Load(); g != 0 {
			t.Fatalf("recovered panic on %q", line)
		}
	})
}

// FuzzParseDiff: whole traces.  Every Parse call is cross-checked by the hook
// (crosscheck_init_test.go), so a disagreement panics.
func FuzzParseDiff(f *testing.F) {
	for _, n := range []string{"build.strace", "cwd.strace", "paths.strace", "net.strace"} {
		b, err := os.ReadFile("testdata/" + n)
		if err != nil {
			f.Fatal(err)
		}
		f.Add(string(b))
	}
	for _, s := range killedEntrySeeds {
		f.Add(s)
	}
	for _, s := range phantomExecSeeds {
		f.Add(s)
	}
	for _, s := range signalSeeds {
		f.Add(s)
	}
	for _, s := range []string{
		"1  stat( <unfinished ...>\n1  <... stat resumed>\"a\", 0x1) = 0\n1  +++ exited with 0 +++\n",
		"1  wait4(-1,  <unfinished ...>\n2  +++ exited with 0 +++\n1  <... wait4 resumed>[{WIFEXITED(s) && WEXITSTATUS(s) == 0}], 0, NULL) = 2\n1  +++ exited with 0 +++\n",
		"1  ???( <unfinished ...>\n1  <... ??? resumed>) = ?\n1  +++ exited with 0 +++\n",
		"1  ???( <unfinished ...>\n1  --- SIGKILL {si_signo=SIGKILL} ---\n1  <... ??? resumed>) = ?\n",
		"1  stat(\"a\\q\", 0x1) = 0\n", "[pid +1] stat(\"a\", 0x1) = 0\n",
		"1  clone3({flags=CLONE_VM|CLONE_VFORK|CLONE_CLEAR_SIGHAND, exit_signal=SIGCHLD}, 88) = -1 ENOSYS (Function not implemented)\n1  +++ exited with 0 +++\n",
		"1  clone3({flags=CLONE_VM|CLONE_VFORK, exit_signal=SIGCHLD} <unfinished ...>\n1  <... clone3 resumed>, 88) = -1 ENOSYS (Function not implemented)\n1  +++ exited with 0 +++\n",
		"1  clone3({flags=CLONE_VM, exit_signal=SIGCHLD} <unfinished ...>\n1  +++ exited with 0 +++\n",
		"1  mount(0,  <unfinished ...>\n1  <... mount resumed>0) = -1 EPERM (Operation not permitted)\n1  +++ exited with 0 +++\n",
		"1  clone(child_stack=NULL, flags=CLONE_NEWNS|SIGCHLD) = -1 EPERM (Operation not permitted)\n1  +++ exited with 0 +++\n",
		helperRootSetup + "11  clone3({flags=CLONE_VM}, 88) = -1 ENOSYS (Function not implemented)\n" + helperInitSetup + helperReExec + helperExec + helperEnd,
	} {
		f.Add(s)
	}
	f.Fuzz(func(t *testing.T, in string) {
		_, _ = ParseOptions(strings.NewReader(in), "/w", Options{
			HelperRoot: len(in)%2 == 0,
			HelperExe:  "/usr/bin/kiln",
			Limits:     Limits{MaxLineBytes: 1 << 16, MaxTotalBytes: 1 << 20, MaxEvents: 1 << 12},
		})
		if g := grammarPanics.Load(); g != 0 {
			t.Fatal("recovered panic in the grammar path")
		}
	})
}
