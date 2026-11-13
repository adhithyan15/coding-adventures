package executor

import (
	"reflect"
	"runtime"
	"strings"
	"testing"
	"time"

	"github.com/adhithyan15/coding-adventures/code/programs/go/build-tool/internal/discovery"
)

func TestWrapCommandPrefixesUnixAndWindowsShells(t *testing.T) {
	wrapper := []string{"kiln", "sandbox-exec", "--"}

	unix := wrapCommand(shellCommandForOS("echo hi", "linux"), wrapper)
	if want := []string{"kiln", "sandbox-exec", "--", "sh", "-c", "echo hi"}; !reflect.DeepEqual(unix.Args, want) {
		t.Fatalf("unix argv = %q, want %q", unix.Args, want)
	}

	win := wrapCommand(shellCommandForOS("echo hi", "windows"), wrapper)
	if want := []string{"kiln", "sandbox-exec", "--", "cmd", "/C", "echo hi"}; !reflect.DeepEqual(win.Args, want) {
		t.Fatalf("windows argv = %q, want %q", win.Args, want)
	}
}

func TestWrapperEnvReplacesCallerKilnVariables(t *testing.T) {
	base := []string{"PATH=/bin", "KILN_STEP=forged", "KILN_ACTION=forged", "KILN_STEP_COUNT=9", "KILN_OTHER=keep"}
	got := wrapperEnv(base, "code/packages/go/x", 2, 5)
	want := []string{"PATH=/bin", "KILN_OTHER=keep",
		"KILN_ACTION=code/packages/go/x", "KILN_STEP=2", "KILN_STEP_COUNT=5"}
	if !reflect.DeepEqual(got, want) {
		t.Fatalf("env = %q, want %q", got, want)
	}
}

func TestExecWrapperActionFallsBackToAbsolutePath(t *testing.T) {
	old := ExecWrapperRepoRoot
	defer func() { ExecWrapperRepoRoot = old }()
	ExecWrapperRepoRoot = ""
	if got := execWrapperAction(discovery.Package{Path: "/r/code/p"}); got != "/r/code/p" {
		t.Fatalf("got %q", got)
	}
}

// Without a wrapper the executor must behave exactly as before: same output,
// no KILN_* variables anywhere, and step counts that do not change what ran.
func TestRunCommandsWithoutWrapperIsUnchanged(t *testing.T) {
	if runtime.GOOS == "windows" {
		t.Skip("POSIX shell commands")
	}
	if ExecWrapper != nil {
		t.Fatal("ExecWrapper must default to nil")
	}
	pkg := discovery.Package{Name: "go/x", Path: t.TempDir()}
	res := runCommands(pkg, []string{"echo a; env | grep -c '^KILN_' || true", "echo b"}, time.Now())
	if res.Status != "built" || res.Stdout != "a\n0\nb\n" || res.StepsRun != 2 || res.StepsTotal != 2 {
		t.Fatalf("result = %+v", res)
	}

	res = runCommands(pkg, []string{"true", "exit 3", "echo never"}, time.Now())
	if res.Status != "failed" || res.ReturnCode != 3 || res.StepsRun != 2 || res.StepsTotal != 3 || strings.Contains(res.Stdout, "never") {
		t.Fatalf("failure result = %+v", res)
	}
}
