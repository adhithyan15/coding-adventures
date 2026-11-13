package trace

import (
	"strings"
	"testing"
)

// A killed process hides no write (M5): a path call it never finished is
// refused by the parser, so every call up to its death is in the trace.
func TestR3Parser(t *testing.T) {
	head := "100  execve(\"/usr/bin/sh\", [\"sh\", \"-c\", \"make\"], 0x7ffd1c2e3a40 /* 41 vars */) = 0\n" +
		"100  clone(child_stack=NULL, flags=CLONE_CHILD_CLEARTID|CLONE_CHILD_SETTID|SIGCHLD, child_tidptr=0x7f0000000010) = 101\n"
	tail := "100  +++ exited with 0 +++\n"
	refused := map[string]string{
		"rename unfinished then killed": "101  rename(\"a\", \"b\" <unfinished ...>\n101  +++ killed by SIGKILL +++\n",
		"openat unfinished then killed": "101  openat(AT_FDCWD</work/repo>, \"o\", O_WRONLY|O_CREAT, 0666 <unfinished ...>\n101  +++ killed by SIGTERM +++\n",
		"bind unfinished then killed":   "101  bind(3<socket:[9]>, {sa_family=AF_UNIX, sun_path=\"s\"}, 110 <unfinished ...>\n101  +++ killed by SIGKILL +++\n",
		"fchmod unfinished then killed": "101  fchmod(3</work/repo/x>, 0644 <unfinished ...>\n101  +++ killed by SIGKILL +++\n",
		"mkdir = ? then killed":         "101  mkdir(\"o\", 0777) = ?\n101  +++ killed by SIGKILL +++\n",
		"resumed with ?":                "101  rename(\"a\", \"b\" <unfinished ...>\n101  <... rename resumed>) = ?\n101  +++ killed by SIGKILL +++\n",
	}
	for name, body := range refused {
		if _, err := Parse(strings.NewReader(head+body+tail), "/work/repo", Limits{}); err == nil {
			t.Errorf("%s must be refused", name)
		}
	}
	// A call that touches no path may be cut off.
	body := "101  wait4(-1,  <unfinished ...>\n101  +++ killed by SIGKILL +++\n"
	if _, err := Parse(strings.NewReader(head+body+tail), "/work/repo", Limits{}); err != nil {
		t.Errorf("an unfinished wait4 is harmless: %v", err)
	}
}
