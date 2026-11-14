package trace

import (
	"reflect"
	"testing"
)

// The recorder compares the digests in the two execs of the Kiln binary that
// setup makes (KILN04 4.8, G7), so the parser must hand their argument vectors
// over although it drops the events themselves.
func TestHelperTraceKeepsTheRootAndInitExecArgv(t *testing.T) {
	rootExec := "10  execve(\"" + kilnExe + "\", [\"kiln\", \"sandbox-exec\", \"--config-sha256\", \"aa\", \"--\", \"sh\"], 0x0 /* 1 vars */) = 0\n"
	reExec := "11  execve(\"/proc/self/exe\", [\"kiln\", \"sandbox-init\", \"--sha256\", \"bb\", \"--\", \"sh\"], 0x0 /* 1 vars */) = 0\n"
	text := rootExec + helperRootSetup[len(helperRootExec):] + helperInitSetup + reExec + helperExec + helperEnd
	tr, err := parseAt(t, "/start", text, helperOpts)
	if err != nil {
		t.Fatal(err)
	}
	if want := []string{"kiln", "sandbox-exec", "--config-sha256", "aa", "--", "sh"}; !reflect.DeepEqual(tr.RootExecArgv, want) {
		t.Fatalf("root argv %q", tr.RootExecArgv)
	}
	if want := []string{"kiln", "sandbox-init", "--sha256", "bb", "--", "sh"}; !reflect.DeepEqual(tr.InitExecArgv, want) {
		t.Fatalf("init argv %q", tr.InitExecArgv)
	}
	// Neither exec became an event: only the step's own exec did.
	if len(tr.Events) != 1 || tr.Events[0].Path != "/bin/sh" {
		t.Fatalf("%+v", tr.Events)
	}
	// An argv that cannot be decoded exactly is nil, never a guess; a trace
	// without the init's re-exec has none.
	trunc := "10  execve(\"" + kilnExe + "\", [\"kiln\", \"sandbox-exec\", ...], 0x0 /* 1 vars */) = 0\n"
	tr, err = parseAt(t, "/start", trunc+helperRootSetup[len(helperRootExec):]+helperInitSetup+helperExec+helperEnd, helperOpts)
	if err != nil {
		t.Fatal(err)
	}
	if tr.RootExecArgv != nil || tr.InitExecArgv != nil {
		t.Fatalf("%q %q", tr.RootExecArgv, tr.InitExecArgv)
	}
	// Only the init's FIRST exec of the binary counts.
	second := "11  execve(\"" + kilnExe + "\", [\"kiln\", \"sandbox-init\", \"--sha256\", \"cc\"], 0x0) = 0\n"
	tr, err = parseAt(t, "/start", helperRootSetup+helperInitSetup+reExec+second+helperExec+helperEnd, helperOpts)
	if err != nil {
		t.Fatal(err)
	}
	if len(tr.InitExecArgv) < 4 || tr.InitExecArgv[3] != "bb" {
		t.Fatalf("%q", tr.InitExecArgv)
	}
	// Outside helper mode there is nothing to keep.
	plain := mustParse(t, wrap(`stat("a", 0x1) = 0`))
	if plain.RootExecArgv != nil || plain.InitExecArgv != nil {
		t.Fatal("argv without helper mode")
	}
}
