package trace

import (
	"fmt"
	"strings"
	"testing"

	"github.com/adhithyan15/coding-adventures/code/programs/go/kiln/internal/syscallceil"
)

// KILN04 D47: a dying call strace could not name is dropped only when the
// sandbox's filter provably stopped it (its number is above the filter's
// ceiling) or the number is in a closed set of path-free calls (456).  Every
// Parse here runs both readers under the cross-check hook, so each case is also
// a parity test between them.

// The dying call is pid 55, a thread of the helper's init (pid 11), as in the
// Go-action traces that motivated D47.  Unknown names are refused at their first
// line outside helper mode (classifyName), so these traces are helper traces:
// the dying-call policy is what judges them.
const (
	d47Pre = helperRootSetup + helperInitSetup + helperReExec + helperExec +
		"11  clone(child_stack=0x7f0000001000, " + dyThread + ") = 2 /* 55 in strace's PID NS */\n"
	d47End = "11  exit_group(5) = ?\n55  +++ exited with 0 +++\n" + helperEnd
)

// d47Trace wraps the body of pid 55 into a complete helper trace.
func d47Trace(body string) string { return d47Pre + body + d47End }

// d47Shapes returns the three dying shapes of a call, with nargs arguments, as
// pid 55 prints them.
func d47Shapes(name string, nargs int) map[string]string {
	args := make([]string, nargs)
	for i := range args {
		args[i] = fmt.Sprintf("0x%x", i+1)
	}
	entry := "55  " + name + "(" + strings.Join(args, ", ") + " <unfinished ...>\n"
	return map[string]string{
		"entry":               entry,
		"resumed unavailable": entry + "55  <... " + name + " resumed>) = ? <unavailable>\n",
		"resumed unfinished":  entry + "55  <... " + name + " resumed> <unfinished ...>) = ?\n",
	}
}

func TestUnknownNumberDyingAccepted(t *testing.T) {
	ceil := uint64(syscallceil.Max)
	for _, n := range []uint64{ceil + 1, ceil + 2, 0x1f4, 456, 0xffffffff} {
		name := fmt.Sprintf("syscall_%#x", n)
		for nargs := 0; nargs <= 6; nargs++ {
			for shape, body := range d47Shapes(name, nargs) {
				t.Run(fmt.Sprintf("%s/%d args/%s", name, nargs, shape), func(t *testing.T) {
					tr, err := parseAt(t, "/start", d47Trace(body), helperOpts)
					if err != nil {
						t.Fatalf("Parse: %v", err)
					}
					for _, e := range tr.Events {
						if e.Pid == 55 {
							t.Errorf("a dropped call left an event: %+v", e)
						}
					}
				})
			}
		}
	}
}

func TestUnknownNumberDyingRefused(t *testing.T) {
	for _, name := range []string{
		fmt.Sprintf("syscall_%#x", syscallceil.Max), // equal to the ceiling: reaches the kernel
		"syscall_0x1c7", // 455 futex_wait: below the ceiling, not in the set
		"syscall_0x150", // 336
		"syscall_0x1",
		"syscall_0",
		"syscall_0x1000001c8", // above 32 bits: the filter sees nr 456, not this
		"syscall_0xffffffffffffffff",
		"syscall_0X1c8", "syscall_01c8", "syscall_", "syscall_0x", "syscall_0x1C8", "syscall_0x01f4",
		"syscall_0x11111111111111111", // 17 hex digits
		"syscall_0x1f4x", "syscall_1f4", "syscall_500",
		"openat", "stat", "futex", // known names
	} {
		for nargs := 0; nargs <= 6; nargs += 3 {
			for shape, body := range d47Shapes(name, nargs) {
				t.Run(fmt.Sprintf("%s/%d args/%s", name, nargs, shape), func(t *testing.T) {
					e := refusal(t, d47Trace(body), true)
					// A well-spelled unknown number is refused by the dying-call policy
					// itself.  A near miss or a known name is refused by whatever
					// else judges it (the name, the arity, the lexer): what matters
					// is that it is not accepted, which refusal() already checked.
					if isUnknownNumberName(name) && e.Reason != reasonUnresumed {
						t.Errorf("refused %s, want %s", e.Reason, reasonUnresumed)
					}
				})
			}
		}
	}
}

// The rule is about DYING calls only: a completed call with an unknown-number
// name, even one above the ceiling, is judged as before (the closed table does
// not know it).
func TestUnknownNumberCompletedCallUnchanged(t *testing.T) {
	for _, name := range []string{"syscall_0x1c8", "syscall_0x1f4", "syscall_0x1c7"} {
		text := d47Pre + "55  " + name + "(0x1, 0x2) = -1 ENOSYS (Function not implemented)\n" + d47End
		if e := refusal(t, text, true); e.Reason != "unknown-syscall:"+name {
			t.Errorf("%s: refused %s, want unknown-syscall", name, e.Reason)
		}
	}
}

// D47 drops no spawn: it must not account for an orphan pid (D44, D44a).
func TestUnknownNumberDyingAccountsForNoOrphan(t *testing.T) {
	text := d47Pre + "55  syscall_0x1c8(0x1 <unfinished ...>\n" + "11  exit_group(5) = ?\n77  +++ exited with 0 +++\n55  +++ exited with 0 +++\n" + helperEnd
	if e := refusal(t, text, true); e.Reason != reasonOrphanPid {
		t.Errorf("refused %s, want %s", e.Reason, reasonOrphanPid)
	}
}

func TestUnknownNumberParse(t *testing.T) {
	for name, want := range map[string]struct {
		n  uint64
		ok bool
	}{
		"syscall_0x1c8": {456, true}, "syscall_0": {0, true},
		"syscall_0xffffffffffffffff": {^uint64(0), true},
		"syscall_0x01c8":             {0, false}, "write": {0, false},
	} {
		if n, ok := unknownNumber(name); n != want.n || ok != want.ok {
			t.Errorf("unknownNumber(%q) = %d, %v", name, n, ok)
		}
	}
	if len(pathFreeUnknown) != 1 || !pathFreeUnknown[456] {
		t.Errorf("pathFreeUnknown changed: %v (an addition needs a kernel-table citation, D47)", pathFreeUnknown)
	}
}
