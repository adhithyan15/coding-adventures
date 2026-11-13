package sandbox

import (
	"reflect"
	"sort"
	"strings"
	"testing"

	"github.com/adhithyan15/coding-adventures/code/programs/go/kiln/internal/trace"
)

func TestStraceArgsShape(t *testing.T) {
	args, err := StraceArgs("/tmp/k/trace.txt")
	if err != nil {
		t.Fatal(err)
	}
	wantHead := []string{"-f", "--quiet=attach", "-y", "-s", "4096", "--kill-on-exit", "--pidns-translation", "-e", "signal=none", "-e"}
	if !reflect.DeepEqual(args[:len(wantHead)], wantHead) {
		t.Errorf("head %v", args[:len(wantHead)])
	}
	tail := args[len(wantHead):]
	if len(tail) != 4 || !strings.HasPrefix(tail[0], "trace=%file,%process,%network,") ||
		tail[1] != "-o" || tail[2] != "/tmp/k/trace.txt" || tail[3] != "--" {
		t.Errorf("tail %v", tail)
	}
	// What the caller appends is the command, not more options: the last
	// element must be the separator.
	if args[len(args)-1] != "--" {
		t.Error("the argv does not end with --")
	}
}

func TestStraceArgsRefusesAnUnsafeFile(t *testing.T) {
	for _, bad := range []string{"", "trace.txt", "-o", "/a/../b", "/a/b/", "/with,comma", "/with:colon", "/", "/x\n"} {
		if a, err := StraceArgs(bad); err == nil {
			t.Errorf("%q accepted: %v", bad, a)
		}
	}
}

func TestStraceArgsIsACopy(t *testing.T) {
	a, _ := StraceArgs("/t")
	a[0] = "changed"
	if b, _ := StraceArgs("/t"); b[0] != "-f" {
		t.Error("a caller changed the shared argv")
	}
	n := TraceNames()
	n[0] = "changed"
	if TraceNames()[0] == "changed" {
		t.Error("a caller changed the shared name list")
	}
}

// TestTraceSetAgreesWithTheParser: the strace set and the parser's closed table
// must say the same thing (KILN04 sections 4.3 and 4.4).
func TestTraceSetAgreesWithTheParser(t *testing.T) {
	names := TraceNames()
	seen := map[string]bool{}
	for _, n := range names {
		if seen[n] {
			t.Errorf("%s is listed twice", n)
		}
		seen[n] = true
		// Every traced name is known to the parser: one it does not know would
		// be reported as an unknown system call at the first trace that shows it.
		if trace.SyscallClass(n) == "" {
			t.Errorf("%s is traced but unknown to the parser's table", n)
		}
	}
	// The set is exactly the classes plus those names, in that order.
	if got := TraceSet(); got != "%file,%process,%network,"+strings.Join(names, ",") {
		t.Errorf("TraceSet %q", got)
	}

	// Every refused call is traced, so it shows up: the filter's list, clone3
	// (which the filter answers with ENOSYS), and the parser's refused class.
	refused := map[string]bool{"clone3": true}
	for _, s := range RefusedSyscalls {
		refused[s.Name] = true
	}
	for n := range refused {
		if !seen[n] {
			t.Errorf("%s is refused by the filter but not traced, so it would leave no line", n)
		}
		if trace.SyscallClass(n) != "refused" {
			t.Errorf("%s is refused by the filter but the parser's class is %q", n, trace.SyscallClass(n))
		}
	}
	parserRefused := trace.RefusedNames()
	for _, n := range parserRefused {
		if !refused[n] {
			t.Errorf("%s is in the parser's refused class but the filter does not refuse it", n)
		}
		if !seen[n] {
			t.Errorf("%s is refused by the parser but not traced", n)
		}
	}
	// Exact agreement of the two lists (a number check, not just inclusion).
	var want []string
	for n := range refused {
		want = append(want, n)
	}
	sort.Strings(want)
	if !reflect.DeepEqual(parserRefused, want) {
		t.Errorf("parser refused %v\nfilter refused %v", parserRefused, want)
	}
	if len(want) != 31 {
		t.Errorf("%d refused calls; KILN04 section 4.4 names thirty and clone3 makes 31", len(want))
	}
}

// TestSyscallClasses covers the parser accessors this agreement rests on.
func TestSyscallClasses(t *testing.T) {
	for name, want := range map[string]string{
		"openat": "handled", "close": "ignored", "mount": "refused", "no_such_call": "",
	} {
		if got := trace.SyscallClass(name); got != want {
			t.Errorf("%s: %q, want %q", name, got, want)
		}
	}
}
