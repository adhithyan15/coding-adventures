package trace

import (
	"reflect"
	"strings"
	"testing"
)

// KILN04 D48 step 3: a sandbox started with --exec-notify-fd makes a few more
// calls in the helper (the root checks the socket and closes its copy; the init
// installs the second filter, sends the listener and closes both).  None of
// them may refuse the trace or leak into the events, and the new argument must
// reach the argv the recorder compares.  The lines follow strace's -y output.

const (
	notifySock   = "6<socket:[4242]>"
	notifyFilter = "seccomp(SECCOMP_SET_MODE_FILTER, SECCOMP_FILTER_FLAG_NEW_LISTENER, {len=9, filter=0xc000012340}) = 5<anon_inode:seccomp notify>"
	notifySend   = `sendmsg(4<socket:[4242]>, {msg_name=NULL, msg_namelen=0, msg_iov=[{iov_base="L", iov_len=1}], msg_iovlen=1, ` +
		`msg_control=[{cmsg_len=20, cmsg_level=SOL_SOCKET, cmsg_type=SCM_RIGHTS, cmsg_data=[5<anon_inode:seccomp notify>]}], ` +
		`msg_controllen=24, msg_flags=0}, MSG_NOSIGNAL) = 1`
)

// TestExecNotifyHelperCallsAreTolerated: every call the option adds, in the
// root before and after the spawn and in the init's window.
func TestExecNotifyHelperCallsAreTolerated(t *testing.T) {
	rootBefore := []string{
		`10  fstat(` + notifySock + `, {st_mode=S_IFSOCK|0777, st_size=0, ...}) = 0`,
		`10  getsockopt(` + notifySock + `, SOL_SOCKET, SO_DOMAIN, [AF_UNIX], [4]) = 0`,
		`10  getsockopt(` + notifySock + `, SOL_SOCKET, SO_TYPE, [SOCK_STREAM], [4]) = 0`,
		`10  getpeername(` + notifySock + `, {sa_family=AF_UNIX, sun_path=""}, [124 => 2]) = 0`,
		`10  getpeername(` + notifySock + `, {sa_family=AF_UNIX}, [112 => 2]) = 0`,
	}
	for _, l := range rootBefore {
		text := helperRootExec + l + "\n" + helperRootSetup[len(helperRootExec):] + helperInitSetup + helperReExec + helperExec + helperEnd
		if _, err := parseAt(t, "/start", text, helperOpts); err != nil {
			t.Errorf("root, before the spawn: %s\n   %v", l, err)
		}
	}
	// The root's close of its copy comes after the spawn: an ignored call.
	closeLine := `10  close(` + notifySock + `) = 0` + "\n"
	text := helperRootSetup + closeLine + helperInitSetup + helperReExec + helperExec + helperEnd
	if _, err := parseAt(t, "/start", text, helperOpts); err != nil {
		t.Errorf("root, close after the spawn: %v", err)
	}

	initLines := []string{
		notifyFilter,
		// A seccomp call strace prints without decoding the flag.
		"seccomp(SECCOMP_SET_MODE_FILTER, 0x8, 0xc000012340) = 5",
		notifySend,
		`sendmsg(4, {msg_name=NULL, msg_namelen=0, msg_iov=[{iov_base="L", iov_len=1}], msg_iovlen=1, msg_control=[{cmsg_len=20, cmsg_level=SOL_SOCKET, cmsg_type=SCM_RIGHTS, cmsg_data=[5]}], msg_controllen=24, msg_flags=0}, MSG_NOSIGNAL) = 1`,
		`sendmsg(4<socket:[4242]>, {msg_name=NULL, msg_namelen=0, msg_iov=[{iov_base="L", iov_len=1}], msg_iovlen=1, msg_control=[{cmsg_len=20, cmsg_level=SOL_SOCKET, cmsg_type=SCM_RIGHTS, cmsg_data=[5<anon_inode:seccomp notify>]}], msg_controllen=24, msg_flags=0}, MSG_NOSIGNAL) = -1 EPIPE (Broken pipe)`,
		`close(5<anon_inode:seccomp notify>) = 0`,
		`close(4<socket:[4242]>) = 0`,
		`fstat(4<socket:[4242]>, {st_mode=S_IFSOCK|0777, st_size=0, ...}) = 0`,
		`getsockopt(4<socket:[4242]>, SOL_SOCKET, SO_DOMAIN, [AF_UNIX], [4]) = 0`,
		`getpeername(4<socket:[4242]>, {sa_family=AF_UNIX, sun_path=""}, [124 => 2]) = 0`,
		`fcntl(5<anon_inode:seccomp notify>, F_SETFD, FD_CLOEXEC) = 0`,
	}
	for _, l := range initLines {
		text := helperRootSetup + helperInitSetup + helperReExec + "11  " + l + "\n" + helperExec + helperEnd
		tr, err := parseAt(t, "/start", text, helperOpts)
		if err != nil {
			t.Errorf("init: %s\n   %v", l, err)
			continue
		}
		for _, e := range tr.Events {
			if e.Syscall != "execve" {
				t.Errorf("%s leaked into the events: %+v", l, e)
			}
		}
	}
}

// TestExecNotifyFullRun: the whole shape of a run with the option, root and
// init argv carrying the new argument, parses to exactly the step's events.
func TestExecNotifyFullRun(t *testing.T) {
	rootExec := "10  execve(\"" + kilnExe + "\", [\"kiln\", \"sandbox-exec\", \"--config\", \"/c\", \"--config-sha256\", \"aa\", " +
		"\"--stdout-fd\", \"3\", \"--stderr-fd\", \"4\", \"--exec-notify-fd\", \"6\", \"--\", \"sh\", \"-c\", \"make\"], 0x0 /* 1 vars */) = 0\n"
	reExec := "11  execve(\"/proc/self/exe\", [\"kiln\", \"sandbox-init\", \"--config\", \"/c\", \"--sha256\", \"bb\", \"--action\", \"p\", " +
		"\"--uid\", \"1\", \"--gid\", \"1\", \"--exec-notify-fd\", \"4\", \"--\", \"sh\", \"-c\", \"make\"], 0x0 /* 1 vars */) = 0\n"
	text := rootExec +
		"10  getsockopt(" + notifySock + ", SOL_SOCKET, SO_DOMAIN, [AF_UNIX], [4]) = 0\n" +
		helperRootSetup[len(helperRootExec):] +
		"10  close(" + notifySock + ") = 0\n" +
		helperInitSetup + reExec +
		"11  " + notifyFilter + "\n" +
		"11  " + notifySend + "\n" +
		"11  close(5<anon_inode:seccomp notify>) = 0\n" +
		"11  close(4<socket:[4242]>) = 0\n" +
		helperExec + helperEnd
	tr, err := parseAt(t, "/start", text, helperOpts)
	if err != nil {
		t.Fatal(err)
	}
	if len(tr.Events) != 1 || tr.Events[0].Path != "/bin/sh" {
		t.Fatalf("%+v", tr.Events)
	}
	// The argv of both execs is handed over whole, option included, and the
	// digest is still found by name, exactly once, before the first "--".
	if !reflect.DeepEqual(tr.RootExecArgv[10:12], []string{"--exec-notify-fd", "6"}) {
		t.Errorf("root argv %q", tr.RootExecArgv)
	}
	if !reflect.DeepEqual(tr.InitExecArgv[12:14], []string{"--exec-notify-fd", "4"}) {
		t.Errorf("init argv %q", tr.InitExecArgv)
	}
	if strings.Count(strings.Join(tr.RootExecArgv, " "), "--config-sha256") != 1 {
		t.Errorf("%q", tr.RootExecArgv)
	}
}

// TestExecNotifyDeadListenerRun: with the listener gone the init's execve fails
// with ENOSYS and the init exits 125, with no step exec: the parser says
// "helper-no-exec", which the recorder reads (with exit status 125) as the
// sandbox refusing to start the step.
func TestExecNotifyDeadListenerRun(t *testing.T) {
	text := helperRootSetup + helperInitSetup + helperReExec +
		"11  " + notifyFilter + "\n" + "11  " + notifySend + "\n" +
		"11  close(5<anon_inode:seccomp notify>) = 0\n11  close(4<socket:[4242]>) = 0\n" +
		"11  execve(\"/bin/sh\", [\"sh\", \"-c\", \"make\"], 0x0 /* 1 vars */) = -1 ENOSYS (Function not implemented)\n" +
		"11  +++ exited with 125 +++\n10  wait4(11, [{WIFEXITED(s) && WEXITSTATUS(s) == 125}], 0, NULL) = 11\n10  +++ exited with 125 +++\n"
	_, err := parseAt(t, "/start", text, helperOpts)
	te, ok := err.(*Error)
	if !ok || te.Reason != "helper-no-exec" {
		t.Fatalf("got %v, want helper-no-exec", err)
	}
}
