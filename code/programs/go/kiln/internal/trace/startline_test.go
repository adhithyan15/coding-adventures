package trace

import "testing"

func TestStartLineAndDescriptorWrites(t *testing.T) {
	// A split call starts at its <unfinished> line and completes at the resume.
	text := "1  clone(child_stack=NULL, flags=SIGCHLD) = 2\n" +
		"1  openat(AT_FDCWD, \"a\", O_RDONLY <unfinished ...>\n" +
		"2  stat(\"b\", 0x1) = 0\n" +
		"1  <... openat resumed>) = 3</r/a>\n" +
		"2  +++ exited with 0 +++\n" +
		"1  +++ exited with 0 +++\n"
	tr := mustParse(t, text)
	var st, op *Event
	for i := range tr.Events {
		switch tr.Events[i].Syscall {
		case "stat":
			st = &tr.Events[i]
		case "openat":
			op = &tr.Events[i]
		}
	}
	if st == nil || op == nil || st.StartLine != 3 || st.Line != 3 || op.StartLine != 2 || op.Line != 4 {
		t.Fatalf("%+v %+v", st, op)
	}
	// Metadata writes through a descriptor use the -y path.
	args := map[string]string{"fchmod": "3</r/a>, 0755", "fchown": "3</r/a>, 1, 2", "fsetxattr": `3</r/a>, "u.x", "v", 1, 0`,
		"fremovexattr": `3</r/a>, "u.x"`, "utimensat": "3</r/a>, NULL, NULL, 0"}
	// futimens is not a system call on x86-64: glibc makes it utimensat on a
	// descriptor with a NULL path, which is what the trace shows.
	for name, a := range args {
		got := mustParse(t, wrap(name+"("+a+") = 0"))
		if e := got.Events[0]; e.Kind != KindWrite || e.Path != "/r/a" {
			t.Errorf("%s: %+v", name, e)
		}
	}
	// Network events carry the socket's -y name.
	n := mustParse(t, wrap(`socket(AF_INET, SOCK_STREAM, 0) = 3<socket:[11]>`,
		`bind(3<socket:[11]>, {sa_family=AF_INET, sin_port=htons(1), sin_addr=inet_addr("127.0.0.1")}, 16) = 0`,
		`listen(3<socket:[11]>, 1) = 0`))
	for _, e := range n.Events {
		if e.SockPath != "socket:[11]" {
			t.Errorf("%s: %q", e.Syscall, e.SockPath)
		}
	}
}
