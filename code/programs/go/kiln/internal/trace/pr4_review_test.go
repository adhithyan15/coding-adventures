package trace

import (
	"strings"
	"testing"
)

// Regression tests for the adversarial review of PR 4 (writes_complete): what
// the parser hands the classifier must keep the facts the classifier's
// fail-closed rules need.
func TestPOCParser(t *testing.T) {
	head := "100  execve(\"/usr/bin/sh\", [\"sh\", \"-c\", \"make\"], 0x7ffd1c2e3a40 /* 41 vars */) = 0\n"
	parse := func(text string) (*Trace, error) {
		return Parse(strings.NewReader(text), "/work/repo", Limits{})
	}
	// A child that dumps core: the exit line says so (the kernel writes the
	// core file with no system call, so the classifier must see this).
	tr, err := parse(head + "100  clone(child_stack=NULL, flags=CLONE_CHILD_CLEARTID|CLONE_CHILD_SETTID|SIGCHLD, child_tidptr=0x7f0000000010) = 101\n" +
		"101  mkdir(\"out\", 0777) = 0\n101  +++ killed by SIGSEGV (core dumped) +++\n100  +++ exited with 0 +++\n")
	if err != nil {
		t.Fatal(err)
	}
	if ex := tr.Procs[101].Exit; ex == nil || !ex.Killed || !ex.CoreDumped || ex.Signal != "SIGSEGV" {
		t.Errorf("core dump exit = %+v", ex)
	}
	// Refused outright: an O_TMPFILE file, a superseded process, a result "?".
	for name, text := range map[string]string{
		"O_TMPFILE":  head + "100  openat(AT_FDCWD</work/repo>, \".\", O_RDWR|O_TMPFILE, 0600) = 3</work/repo/#1234 (deleted)>\n100  +++ exited with 0 +++\n",
		"superseded": head + "100  +++ superseded by execve in pid 101 +++\n",
		"result ?":   head + "100  mkdir(\"out\", 0777) = ?\n100  +++ exited with 0 +++\n",
	} {
		if _, err := parse(text); err == nil {
			t.Errorf("%s must be refused", name)
		}
	}
	// A trailing slash survives parsing (the kernel follows it, M1).
	tr, err = parse(head + "100  lchown(\"dl/\", 1000, 1000) = 0\n100  utimensat(AT_FDCWD</work/repo>, \"dl/\", NULL, AT_SYMLINK_NOFOLLOW) = 0\n100  +++ exited with 0 +++\n")
	if err != nil {
		t.Fatal(err)
	}
	for _, e := range tr.Events[1:] {
		if e.Path != "dl/" {
			t.Errorf("%s path = %q, want the slash kept", e.Syscall, e.Path)
		}
	}
	// `> /dev/stderr` re-opens a pipe: the -y text is pipe:[...] (M2).
	tr, err = parse(head + "100  openat(AT_FDCWD</work/repo>, \"/dev/stderr\", O_WRONLY|O_CREAT|O_TRUNC, 0666) = 3<pipe:[8123]>\n100  +++ exited with 0 +++\n")
	if err != nil {
		t.Fatal(err)
	}
	if e := tr.Events[1]; e.ReturnFDPath != "pipe:[8123]" {
		t.Errorf("return fd path = %q", e.ReturnFDPath)
	}
}
