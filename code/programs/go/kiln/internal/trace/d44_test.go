package trace

import (
	"strings"
	"testing"
)

// KILN04 D44: orphan exits, NULL-path dying calls, short mknod, and the census's
// unknown-number code.  Every Parse here runs both readers under the
// cross-check hook (crosscheck_init_test.go), so each case is also a parity
// test between the old scanner and the grammar parser.

const dySpawnAtEntry = "  clone(child_stack=0x7f0000002000, " + dyThread + " <unfinished ...>\n"

// dyOrphanTrace: pid 1 makes `threads` threads (pids 2, 3, 4), each of which
// dies inside a clone (shape 3, a dropped dying spawn), then the group ends, and
// then orphanExits are the lines of pids that have no other line.
func dyOrphanTrace(threads int, orphanExits string) string {
	var b strings.Builder
	pids := []string{"2", "3", "4"}[:threads]
	for _, p := range pids {
		b.WriteString("1  clone(child_stack=0x7f0000001000, " + dyThread + ") = " + p + "\n")
	}
	for _, p := range pids {
		b.WriteString(p + dySpawnAtEntry)
	}
	b.WriteString(dyGroupEnd + orphanExits)
	for _, p := range pids {
		b.WriteString(p + "  +++ exited with 0 +++\n")
	}
	b.WriteString(dyExit1)
	return b.String()
}

func TestOrphanExitAccountedByDroppedSpawn(t *testing.T) {
	tr := mustParse(t, dyOrphanTrace(1, "7  +++ exited with 0 +++\n"))
	if len(tr.Procs) != 2 || tr.Procs[7] != nil {
		t.Errorf("the orphan must not become a process: %+v", tr.Procs)
	}
	found := false
	for _, ex := range tr.Exits {
		found = found || ex.Pid == 7 // kept, so DaemonLeft still sees it
	}
	if !found {
		t.Errorf("the orphan's exit is missing from Exits: %+v", tr.Exits)
	}
	mustParse(t, dyOrphanTrace(1, "7  +++ killed by SIGKILL +++\n"))
}

// More orphans than dropped spawns: all refuse, as before.
func TestOrphanExitsBeyondDroppedSpawnsRefuse(t *testing.T) {
	wantReason(t, dyOrphanTrace(1, "7  +++ exited with 0 +++\n8  +++ exited with 0 +++\n"), reasonOrphanPid)
	mustParse(t, dyOrphanTrace(2, "7  +++ exited with 0 +++\n8  +++ exited with 0 +++\n"))
	wantReason(t, dyOrphanTrace(2, "7  +++ exited with 0 +++\n8  +++ exited with 0 +++\n9  +++ exited with 0 +++\n"), reasonOrphanPid)
}

// An orphan with no dropped spawn refuses.
func TestOrphanExitWithoutDyingSpawnRefuses(t *testing.T) {
	wantReason(t, "1  stat(\"a\", 0x1) = 0\n7  +++ exited with 0 +++\n1  +++ exited with 0 +++\n", reasonOrphanPid)
	// A dropped call that is not a spawn (tgkill) accounts for nothing.
	text := dyClone + "2  tgkill(1, 1, SIGURG <unfinished ...>\n" + dyGroupEnd + "7  +++ exited with 0 +++\n" + dyExit2 + dyExit1
	wantReason(t, text, reasonOrphanPid)
}

// A pid with any other record is not an orphan, even with a dropped spawn.
func TestOrphanWithOtherRecordsRefuses(t *testing.T) {
	wantReason(t, dyOrphanTrace(1, "7  stat(\"a\", 0x1) = 0\n7  +++ exited with 0 +++\n"), reasonOrphanPid)
	wantReason(t, dyOrphanTrace(1, "7  +++ exited with 0 +++\n7  +++ exited with 0 +++\n"), reasonOrphanPid)
}

func TestNullPathDyingCalls(t *testing.T) {
	for _, name := range []string{"stat", "lstat", "access", "readlink"} {
		for shape, body := range map[string]string{
			"entry":               "2  " + name + "(NULL, <unfinished ...>\n" + dyGroupEnd,
			"resumed unavailable": "2  " + name + "(NULL, 0x7f3a6c001230 <unfinished ...>\n" + dyGroupEnd + "2  <... " + name + " resumed>) = ? <unavailable>\n",
			"resumed unfinished":  "2  " + name + "(NULL, <unfinished ...>\n" + dyGroupEnd + "2  <... " + name + " resumed> <unfinished ...>) = ?\n",
		} {
			t.Run(name+" "+shape, func(t *testing.T) { mustParse(t, dyTrace(body)) })
		}
	}
	// Negative: a real path, an address, a different call.
	for _, body := range []string{
		"2  stat(\"/etc/passwd\", <unfinished ...>\n",
		"2  stat(0x7f3a6c001230, <unfinished ...>\n",
		"2  lstat(\"NULL\", <unfinished ...>\n",
		"2  newfstatat(AT_FDCWD</w>, NULL, <unfinished ...>\n",
		"2  statx(AT_FDCWD</w>, NULL, <unfinished ...>\n",
		"2  readlinkat(AT_FDCWD</w>, NULL, <unfinished ...>\n",
		"2  open(NULL, O_RDONLY <unfinished ...>\n",
	} {
		e := refusal(t, dyTrace(body+dyGroupEnd), false)
		if e.Reason != reasonUnresumed {
			t.Errorf("%q: refused %s, want %s", body, e.Reason, reasonUnresumed)
		}
	}
	p := func(name string, args ...string) *pending { return &pending{name: name, args: args} }
	for _, c := range []struct {
		p    *pending
		drop bool
	}{
		{p("stat", "NULL", "0x1"), true},
		{p("access", "NULL", "R_OK"), true},
		{p("stat", "\"NULL\"", "0x1"), false},
		{p("stat"), false},
		{p("newfstatat", "AT_FDCWD</w>", "NULL", "0x1", "AT_EMPTY_PATH"), false},
		{p("newfstatat", "NULL", "NULL"), false},
	} {
		if got := dyingPolicy(c.p); got != c.drop {
			t.Errorf("dyingPolicy(%s %v) = %v, want %v", c.p.name, c.p.args, got, c.drop)
		}
	}
}

// strace omits dev unless the type is S_IFCHR or S_IFBLK (its mknod.c), so the
// short form of every other type is accepted, and a short device node is not a
// call we know.
func TestMknodShortForm(t *testing.T) {
	for _, line := range []string{
		`mknod("a", S_IFIFO|0600) = 0`,
		`mknod("a", S_IFSOCK|0600) = 0`,
		`mknod("a", S_IFREG|0644) = 0`,
		`mknod("a", 0644) = 0`,
		`mknodat(AT_FDCWD, "a", S_IFIFO|0600) = 0`,
		`mknodat(AT_FDCWD, "a", S_IFSOCK|0600) = 0`,
		`mknodat(AT_FDCWD, "a", S_IFREG|0644) = 0`,
		`mknodat(AT_FDCWD, "a", S_IFIFO|0600, 0) = 0`, // the older long spelling stays accepted
		`mknodat(AT_FDCWD, "a", S_IFCHR|0600, makedev(0x1, 0x3)) = 0`,
	} {
		tr := mustParse(t, "1  "+line+"\n1  +++ exited with 0 +++\n")
		if len(tr.Events) != 1 || tr.Events[0].Kind != KindWrite {
			t.Errorf("%s: %+v", line, tr.Events)
		}
	}
	for _, line := range []string{
		`mknod("a", S_IFCHR|0600) = 0`,
		`mknod("a", S_IFBLK|0600) = 0`,
		`mknodat(AT_FDCWD, "a", S_IFCHR|0600) = 0`,
		`mknodat(AT_FDCWD, "a", S_IFBLK|0600) = 0`,
		`mknodat(AT_FDCWD, "a", 0x1234) = 0`, // an unreadable mode is not judged a non-device
		`mknodat(AT_FDCWD, "a", 020600) = 0`, // octal with S_IFCHR bits: not strace's spelling of a non-device
		`mknod("a", 060600) = 0`,             // octal with S_IFBLK bits
		`mknod("a") = 0`,
		`mknodat(AT_FDCWD, "a") = 0`,
	} {
		wantReason(t, "1  "+line+"\n1  +++ exited with 0 +++\n", reasonArity)
	}
}

func TestUnknownNumberSpelling(t *testing.T) {
	for name, want := range map[string]bool{
		"syscall_0x1c8": true, "syscall_0x1": true, "syscall_0": true, "syscall_0xffffffffffffffff": true,
		"syscall_0x01c8": false, "syscall_0x1C8": false, "syscall_1c8": false, "syscall_": false,
		"syscall_0x": false, "syscall_0x1c8x": false, "syscall_0x11111111111111111": false,
		"syscall_00": false, "syscall_0X1": false, "frobnicate": false, "SYSCALL_0x1": false,
		"syscall_0x1c8\n": false, "???": false, "": false,
	} {
		if got := isUnknownNumberName(name); got != want {
			t.Errorf("isUnknownNumberName(%q) = %v, want %v", name, got, want)
		}
	}
}

// A dying call outside the table is still refused (no drop rule yet), and the
// error says only a fixed code about its name, never the name.
func TestUnresumedUnknownCallCarriesOnlyACode(t *testing.T) {
	const pre = helperRootSetup + helperInitSetup + helperReExec + helperExec +
		"11  clone(child_stack=0x7f0000001000, " + dyThread + ") = 2 /* 55 in strace's PID NS */\n"
	const end = "11  exit_group(5) = ?\n55  +++ exited with 0 +++\n" + helperEnd
	for name, want := range map[string]string{
		"syscall_0x1c8":    "unknown-number:yes",
		"syscall_0x01c8":   "unknown-number:no",
		"evil_name_xyz123": "unknown-number:no",
	} {
		body := "55  " + name + "(0x1, 0x2, 0x3, 0x4, 0x5, 0x6 <unfinished ...>\n"
		for shape, text := range map[string]string{
			"entry":   pre + body + end,
			"resumed": pre + body + "55  <... " + name + " resumed>) = ? <unavailable>\n" + end,
		} {
			e := refusal(t, text, true)
			if e.Reason != reasonUnresumed || e.UnknownNumber != want {
				t.Errorf("%s %s: %s / %q, want unresumed-call / %q", name, shape, e.Reason, e.UnknownNumber, want)
			}
			if strings.Contains(e.Error()+e.Reason+e.Call+e.UnknownNumber, "evil_name") {
				t.Errorf("%s %s: the name leaked: %+v", name, shape, e)
			}
		}
	}
	e := refusal(t, pre+"55  openat(AT_FDCWD</start>, 0x7f3a6c001230, O_RDONLY <unfinished ...>\n"+end, true)
	if e.UnknownNumber != "" {
		t.Errorf("a table name got %q", e.UnknownNumber)
	}
}
