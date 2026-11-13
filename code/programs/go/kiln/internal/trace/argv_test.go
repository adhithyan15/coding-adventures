package trace

import (
	"reflect"
	"testing"
)

func TestParseArgv(t *testing.T) {
	cases := []struct {
		in   string
		want []string
	}{
		{`["sh", "-c", "make"]`, []string{"sh", "-c", "make"}},
		{`[]`, []string{}},
		{`["a\"b", "c d"]`, []string{`a"b`, "c d"}},
		{`["x"...]`, nil},    // a truncated string
		{`["a", ...]`, nil},  // strace cut the vector short
		{`["a", "b",]`, nil}, // dangling comma
		{`["a" "b"]`, nil},   // missing separator
		{`0x7ffd1c2e3a40`, nil},
		{`["\377"]`, nil}, // not UTF-8
		{`[3]`, nil},      // not a string
		{`["a", "b"`, nil},
		{`["a", "b", "\n"]`, []string{"a", "b", "\n"}},
	}
	for _, c := range cases {
		if got := parseArgv(c.in); !reflect.DeepEqual(got, c.want) {
			t.Errorf("parseArgv(%s) = %#v, want %#v", c.in, got, c.want)
		}
	}
}

func TestStepEventAndArgv(t *testing.T) {
	// A shell that probes PATH (failed execs) and then runs a program.
	text := "1  execve(\"/usr/bin/sh\", [\"sh\", \"-c\", \"make all\"], 0x0 /* 1 vars */) = 0\n" +
		"1  execve(\"/nope/cc\", [\"cc\"], 0x0 /* 1 vars */) = -1 ENOENT (No such file or directory)\n" +
		"1  execve(\"/usr/bin/cc\", [\"cc\", \"-c\"], 0x0 /* 1 vars */) = 0\n" +
		"1  +++ exited with 0 +++\n"
	tr := mustParse(t, text)
	st := tr.StepEvent()
	if st == nil || st.Path != "/usr/bin/sh" || !reflect.DeepEqual(st.Argv, []string{"sh", "-c", "make all"}) {
		t.Fatalf("step event %+v", st)
	}
	if tr.Events[1].Result.OK || !reflect.DeepEqual(tr.Events[1].Argv, []string{"cc"}) {
		t.Fatalf("failed exec %+v", tr.Events[1])
	}
	// No exec at all: no step event.
	if mustParse(t, wrap(`stat("a", 0x1) = 0`)).StepEvent() != nil {
		t.Fatal("step event without an exec")
	}
	// execveat carries argv in its own position.
	at := mustParse(t, wrap(`execveat(AT_FDCWD, "/bin/sh", ["sh", "-c", "x"], 0x0 /* 1 vars */, 0) = 0`))
	if got := at.StepEvent(); got == nil || !reflect.DeepEqual(got.Argv, []string{"sh", "-c", "x"}) {
		t.Fatalf("%+v", got)
	}
	// Under the helper, the step is the init child's first successful exec.
	h, err := parseAt(t, "/start", helperRootSetup+helperInitSetup+helperReExec+helperExec+helperEnd, helperOpts)
	if err != nil {
		t.Fatal(err)
	}
	if got := h.StepEvent(); got == nil || got.Pid != 11 || !reflect.DeepEqual(got.Argv, []string{"sh", "-c", "make"}) {
		t.Fatalf("%+v", got)
	}
}
