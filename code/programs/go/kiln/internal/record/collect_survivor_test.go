package record

import (
	"errors"
	"os"
	"strconv"
	"strings"
	"syscall"
	"testing"
	"time"
)

// fakeProc is a /proc made of maps: no test here reads the real one or waits.
type fakeProc struct {
	self  int
	uid   uint32
	procs map[int]*fakeEntry
	// listErr and selfNSErr make the two scan-wide reads fail.
	listErr, selfNSErr error
	// block, when set, makes Pids wait for it (a /proc read stuck for good).
	block chan struct{}
}

// fakeBoot is the boot time of the fake host; ticks count from it at 100 Hz.
var fakeBoot = time.Unix(1_000_000, 0)

func (f *fakeProc) Boot() (time.Time, int, error) { return fakeBoot, 100, nil }

type fakeEntry struct {
	owner   uint32
	ownErr  error
	cmdline string // NUL-separated
	cmdErr  error
	ns      string
	nsErr   error
	mounts  string // mountinfo text
	miErr   error
	// status and stat are the raw files; empty means "make one": a status whose
	// Uid is owner, and a stat of a process started at boot (old).
	status, stat       string
	statusErr, statErr error
}

// statLine is a /proc/<pid>/stat with the given comm and starttime (in ticks).
func statLine(comm string, ticks int) string {
	// fields 3..21 are 19 values; field 22 is the start time.
	return "123 (" + comm + ") S 1 1 1 0 -1 4194560 0 0 0 0 0 0 0 0 20 0 1 0 " + strconv.Itoa(ticks) + " 1000 100 18446744073709551615 0 0 0\n"
}

func (f *fakeProc) Pids() ([]int, error) {
	if f.block != nil {
		<-f.block
	}
	if f.listErr != nil {
		return nil, f.listErr
	}
	var out []int
	for p := range f.procs {
		out = append(out, p)
	}
	return out, nil
}
func (f *fakeProc) Self() int   { return f.self }
func (f *fakeProc) UID() uint32 { return f.uid }
func (f *fakeProc) Owner(pid int) (uint32, error) {
	e := f.procs[pid]
	return e.owner, e.ownErr
}
func (f *fakeProc) ReadLink(pid int, name string) (string, error) {
	if pid == f.self && f.selfNSErr != nil {
		return "", f.selfNSErr
	}
	e := f.procs[pid]
	if name != "ns/mnt" {
		return "", errors.New("unexpected link " + name)
	}
	return e.ns, e.nsErr
}
func (f *fakeProc) ReadFile(pid int, name string) ([]byte, error) {
	e := f.procs[pid]
	switch name {
	case "cmdline":
		return []byte(e.cmdline), e.cmdErr
	case "mountinfo":
		return []byte(e.mounts), e.miErr
	case "status":
		if e.statusErr != nil || e.status != "" {
			return []byte(e.status), e.statusErr
		}
		u := strconv.Itoa(int(e.owner))
		return []byte("Name:\tx\nUid:\t" + u + "\t" + u + "\t" + u + "\t" + u + "\n"), nil
	case "stat":
		if e.statErr != nil || e.stat != "" {
			return []byte(e.stat), e.statErr
		}
		return []byte(statLine("x", 0)), nil
	}
	return nil, errors.New("unexpected file " + name)
}

const (
	surRoot  = "/work/repo"
	surCfg   = "/work/record/pre/sandbox.json"
	surProbe = "/work/record/probe/toolchains-sandbox.json"
	hostNS   = "mnt:[4026531841]"
	boxNS    = "mnt:[4026532999]"
)

// surQuery has no start time: every process counts for S3.
var surQuery = SurvivorQuery{RepoRoot: surRoot, Configs: []string{surCfg, surProbe}}

// mountinfoWith is a mountinfo with the given mount points (field 5).
func mountinfoWith(points ...string) string {
	var b strings.Builder
	for i, p := range points {
		b.WriteString(strconv.Itoa(30+i) + " 1 0:25 / " + p + " rw - tmpfs tmpfs rw\n")
	}
	return b.String()
}

// host is the common world: this process (pid 10), in the host namespace.
func host() *fakeProc {
	return &fakeProc{self: 10, uid: 1000, procs: map[int]*fakeEntry{
		10: {owner: 1000, ns: hostNS, cmdline: "collect\x00"},
		1:  {owner: 0, ns: hostNS, cmdline: "init\x00"},
		20: {owner: 1000, ns: hostNS, cmdline: "bash\x00"},
	}}
}

func scanOne(t *testing.T, f *fakeProc) []string {
	t.Helper()
	got, err := scanSurvivors(f, surQuery)
	if err != nil {
		t.Fatal(err)
	}
	return got
}

func TestSurvivorScanFindsNothingOnAQuietHost(t *testing.T) {
	if got := scanOne(t, host()); len(got) != 0 {
		t.Fatalf("%v", got)
	}
}

func TestSurvivorS1AnotherNamespaceWithTheCheckoutMounted(t *testing.T) {
	f := host()
	f.procs[30] = &fakeEntry{owner: 1000, ns: boxNS, cmdline: "sleep\x0060\x00", mounts: mountinfoWith("/", surRoot, "/tmp")}
	got := scanOne(t, f)
	if len(got) != 1 || !strings.HasPrefix(got[0], "pid 30: S1") {
		t.Fatalf("%v", got)
	}
}

func TestSurvivorS1NeedsTheCheckoutToBeAMountPoint(t *testing.T) {
	f := host()
	// A container, or any other namespace, that does not have the checkout.
	f.procs[30] = &fakeEntry{owner: 1000, ns: boxNS, cmdline: "app\x00", mounts: mountinfoWith("/", "/work/other", surRoot+"/sub")}
	// A mount point with a space: the kernel writes \040.
	f.procs[31] = &fakeEntry{owner: 1000, ns: boxNS, cmdline: "app\x00", mounts: mountinfoWith("/", "/work/repo\\040two")}
	if got := scanOne(t, f); len(got) != 0 {
		t.Fatalf("%v", got)
	}
	// The same namespace as collect's, even when the checkout is a mount point.
	f.procs[32] = &fakeEntry{owner: 1000, ns: hostNS, cmdline: "app\x00", mounts: mountinfoWith("/", surRoot)}
	if got := scanOne(t, f); len(got) != 0 {
		t.Fatalf("%v", got)
	}
}

func TestSurvivorS2TheSealedConfigurationInACommandLine(t *testing.T) {
	f := host()
	f.procs[30] = &fakeEntry{owner: 1000, ns: hostNS, cmdline: "kiln\x00sandbox-exec\x00--config\x00" + surCfg + "\x00--\x00sh\x00"}
	// Somebody else's process names it too: S2 holds whoever owns the process.
	f.procs[31] = &fakeEntry{owner: 0, ns: hostNS, cmdline: "kiln\x00--config\x00" + surCfg + "\x00"}
	// Only a whole element counts.
	f.procs[32] = &fakeEntry{owner: 1000, ns: hostNS, cmdline: "grep\x00" + surCfg + ".bak\x00"}
	f.procs[33] = &fakeEntry{owner: 1000, ns: hostNS, cmdline: "grep\x00--config=" + surCfg + "\x00"}
	got := scanOne(t, f)
	if len(got) != 2 || !strings.HasPrefix(got[0], "pid ") || !strings.Contains(strings.Join(got, "|"), "S2") {
		t.Fatalf("%v", got)
	}
	for _, g := range got {
		if !strings.Contains(g, "pid 30:") && !strings.Contains(g, "pid 31:") {
			t.Fatalf("%v", got)
		}
	}
}

func TestSurvivorS3AProcessOfOursThatCannotBeRead(t *testing.T) {
	denied := &os.PathError{Op: "open", Path: "x", Err: syscall.EACCES}
	for name, e := range map[string]*fakeEntry{
		"cmdline":   {owner: 1000, ns: hostNS, cmdErr: denied},
		"ns":        {owner: 1000, cmdline: "x\x00", nsErr: denied},
		"mountinfo": {owner: 1000, ns: boxNS, cmdline: "x\x00", miErr: denied},
		"empty":     {owner: 1000, ns: boxNS, cmdline: "x\x00"},
		"owner":     {ownErr: denied},
	} {
		f := host()
		f.procs[30] = e
		got := scanOne(t, f)
		if len(got) != 1 || !strings.Contains(got[0], "S3") {
			t.Fatalf("%s: %v", name, got)
		}
	}
}

func TestSurvivorS3IsOnlyForOurOwnProcesses(t *testing.T) {
	denied := &os.PathError{Op: "readlink", Path: "x", Err: syscall.EACCES}
	f := host()
	// root's process, whose namespace we may not read: not ours, not counted.
	f.procs[30] = &fakeEntry{owner: 0, cmdline: "daemon\x00", nsErr: denied}
	f.procs[31] = &fakeEntry{owner: 0, ns: boxNS, cmdline: "daemon\x00", miErr: denied}
	f.procs[32] = &fakeEntry{owner: 0, cmdErr: denied, ns: hostNS}
	if got := scanOne(t, f); len(got) != 0 {
		t.Fatalf("%v", got)
	}
}

func TestSurvivorAProcessThatIsGoneIsNotOne(t *testing.T) {
	f := host()
	f.procs[30] = &fakeEntry{ownErr: &os.PathError{Op: "lstat", Path: "x", Err: syscall.ENOENT}}
	f.procs[31] = &fakeEntry{owner: 1000, cmdErr: syscall.ESRCH}
	f.procs[32] = &fakeEntry{owner: 1000, cmdline: "x\x00", nsErr: &os.PathError{Op: "readlink", Path: "x", Err: syscall.ENOENT}}
	f.procs[33] = &fakeEntry{owner: 1000, cmdline: "x\x00", ns: boxNS, miErr: syscall.ESRCH}
	if got := scanOne(t, f); len(got) != 0 {
		t.Fatalf("%v", got)
	}
}

func TestSurvivorExcludesThisProcessEvenWhenItLooksLikeOne(t *testing.T) {
	f := host()
	// collect itself, in a namespace that lists the checkout, with the config on
	// its command line: it must not find itself.
	f.procs[10] = &fakeEntry{owner: 1000, ns: boxNS, cmdline: surCfg + "\x00", mounts: mountinfoWith(surRoot)}
	f.procs[1].ns, f.procs[20].ns = boxNS, boxNS // the same namespace as this process
	if got := scanOne(t, f); len(got) != 0 {
		t.Fatalf("%v", got)
	}
}

func TestSurvivorScanFailsClosedWhenItCannotBeMade(t *testing.T) {
	f := host()
	f.listErr = errors.New("no /proc")
	if _, err := scanSurvivors(f, surQuery); err == nil {
		t.Fatal("an unlistable /proc was a clean scan")
	}
	f = host()
	f.selfNSErr = errors.New("denied")
	if _, err := scanSurvivors(f, surQuery); err == nil {
		t.Fatal("an unreadable own namespace was a clean scan")
	}
}

func TestSurvivorListIsBounded(t *testing.T) {
	f := host()
	for i := 0; i < 30; i++ {
		f.procs[100+i] = &fakeEntry{owner: 1000, ns: hostNS, cmdline: surCfg + "\x00"}
	}
	got := scanOne(t, f)
	if len(got) != maxSurvivorsListed+1 || !strings.Contains(got[len(got)-1], "22 more") {
		t.Fatalf("%d entries, last %q", len(got), got[len(got)-1])
	}
}

// ---------------------------------------------------------------------------
// The wait.

// clock is a fake clock: sleep advances it and counts.
type clock struct {
	t      time.Time
	sleeps int
}

func (c *clock) now() time.Time        { return c.t }
func (c *clock) sleep(d time.Duration) { c.t = c.t.Add(d); c.sleeps++ }

func TestWaitSurvivorsNegativeWaitLooksOnce(t *testing.T) {
	c := &clock{t: time.Unix(0, 0)}
	calls := 0
	got := waitSurvivors(func() ([]string, error) { calls++; return []string{"pid 1: x"}, nil }, -1, c.now, c.sleep, 0)
	if len(got) != 1 || calls != 1 || c.sleeps != 0 {
		t.Fatalf("%v %d %d", got, calls, c.sleeps)
	}
}

func TestWaitSurvivorsGoneWithinTheWait(t *testing.T) {
	c := &clock{t: time.Unix(0, 0)}
	calls := 0
	got := waitSurvivors(func() ([]string, error) {
		calls++
		if calls < 4 {
			return []string{"pid 1: x"}, nil
		}
		return nil, nil
	}, 5*time.Second, c.now, c.sleep, 0)
	if len(got) != 0 || calls != 4 || c.sleeps != 3 {
		t.Fatalf("%v calls %d sleeps %d", got, calls, c.sleeps)
	}
}

func TestWaitSurvivorsStillThereAfterTheWait(t *testing.T) {
	c := &clock{t: time.Unix(0, 0)}
	calls := 0
	got := waitSurvivors(func() ([]string, error) { calls++; return []string{"pid 1: x"}, nil }, time.Second, c.now, c.sleep, 0)
	// 200 ms apart over one second: the scans at 0, 200, ... 1000 ms.
	if len(got) != 1 || calls != 6 || c.sleeps != 5 {
		t.Fatalf("%v calls %d sleeps %d", got, calls, c.sleeps)
	}
}

func TestWaitSurvivorsDefaultIsTenSecondsAndAFailedScanIsASurvivor(t *testing.T) {
	c := &clock{t: time.Unix(0, 0)}
	got := waitSurvivors(func() ([]string, error) { return nil, errors.New("boom") }, 0, c.now, c.sleep, 0)
	if len(got) != 1 || !strings.Contains(got[0], "boom") {
		t.Fatalf("%v", got)
	}
	if c.t.Sub(time.Unix(0, 0)) != DefaultSurvivorWait {
		t.Fatalf("waited %v", c.t.Sub(time.Unix(0, 0)))
	}
}

// ---------------------------------------------------------------------------
// S3 after the review: ownership from status, start time, S2 for the probe
// configuration, truncated reads, a scan that blocks.

// prepared is the moment prepare ran in these tests: 1000 s after boot.
var prepared = fakeBoot.Add(1000 * time.Second)

func sinceQuery() SurvivorQuery {
	q := surQuery
	q.Since = prepared.Add(-2 * time.Second)
	return q
}

func TestSurvivorS3OldUninspectableProcessIsIgnored(t *testing.T) {
	denied := &os.PathError{Op: "readlink", Path: "x", Err: syscall.EACCES}
	f := host()
	// A systemd --user manager, started at boot: ours, non-dumpable, old.
	f.procs[30] = &fakeEntry{owner: 1000, cmdline: "(sd-pam)\x00", nsErr: denied, stat: statLine("(sd-pam)", 500)}
	got, err := scanSurvivors(f, sinceQuery())
	if err != nil || len(got) != 0 {
		t.Fatalf("%v %v", got, err)
	}
}

func TestSurvivorS3NewUninspectableProcessIsFlagged(t *testing.T) {
	denied := &os.PathError{Op: "readlink", Path: "x", Err: syscall.EACCES}
	f := host()
	f.procs[30] = &fakeEntry{owner: 1000, cmdline: "x\x00", nsErr: denied, stat: statLine("x", 100*1000+50)}
	got, _ := scanSurvivors(f, sinceQuery())
	if len(got) != 1 || !strings.Contains(got[0], "S3") {
		t.Fatalf("%v", got)
	}
	// One that started within the slack before prepare counts too.
	f.procs[30].stat = statLine("x", 100*(1000-1))
	if got, _ = scanSurvivors(f, sinceQuery()); len(got) != 1 {
		t.Fatalf("%v", got)
	}
}

func TestSurvivorS3UnparsableStatCountsAsNew(t *testing.T) {
	denied := &os.PathError{Op: "readlink", Path: "x", Err: syscall.EACCES}
	for name, e := range map[string]*fakeEntry{
		"garbage": {owner: 1000, cmdline: "x\x00", nsErr: denied, stat: "nonsense"},
		"short":   {owner: 1000, cmdline: "x\x00", nsErr: denied, stat: "1 (x) S 1 2 3\n"},
		"nan":     {owner: 1000, cmdline: "x\x00", nsErr: denied, stat: strings.Replace(statLine("x", 1), " 1 1000", " zz 1000", 1)},
		"unread":  {owner: 1000, cmdline: "x\x00", nsErr: denied, statErr: denied},
		"no-comm": {owner: 1000, cmdline: "x\x00", nsErr: denied, stat: "1 x S 1\n"},
	} {
		f := host()
		f.procs[30] = e
		got, _ := scanSurvivors(f, sinceQuery())
		if len(got) != 1 || !strings.Contains(got[0], "S3") {
			t.Fatalf("%s: %v", name, got)
		}
	}
}

func TestSurvivorS3HostileOwnerRootButStatusUidOurs(t *testing.T) {
	// The bypass: a non-dumpable task's /proc/<pid> is owned by root, so the
	// owner says "not ours"; its status says it is, and ns/mnt is unreadable.
	denied := &os.PathError{Op: "readlink", Path: "x", Err: syscall.EACCES}
	f := host()
	f.procs[30] = &fakeEntry{owner: 0, status: "Name:\tx\nUid:\t1000\t1000\t1000\t1000\n", cmdline: "x\x00", nsErr: denied, stat: statLine("x", 100*2000)}
	got, _ := scanSurvivors(f, sinceQuery())
	if len(got) != 1 || !strings.Contains(got[0], "S3") {
		t.Fatalf("%v", got)
	}
	// Effective uid ours, real not: ours as well.
	f.procs[30].status = "Uid:\t0\t1000\t0\t0\n"
	if got, _ = scanSurvivors(f, sinceQuery()); len(got) != 1 {
		t.Fatalf("%v", got)
	}
	// Truly somebody else's, with an owner that is not ours either: skipped.
	f.procs[30].status = "Uid:\t0\t0\t0\t0\n"
	if got, _ = scanSurvivors(f, sinceQuery()); len(got) != 0 {
		t.Fatalf("%v", got)
	}
	// An unreadable or unparsable status is ours.
	for _, st := range []*fakeEntry{
		{owner: 0, statusErr: denied, cmdline: "x\x00", nsErr: denied, stat: statLine("x", 100*2000)},
		{owner: 0, status: "Name:\tx\n", cmdline: "x\x00", nsErr: denied, stat: statLine("x", 100*2000)},
		{owner: 0, status: "Uid:\tfoo\n", cmdline: "x\x00", nsErr: denied, stat: statLine("x", 100*2000)},
	} {
		f.procs[30] = st
		if got, _ = scanSurvivors(f, sinceQuery()); len(got) != 1 {
			t.Fatalf("%+v: %v", st, got)
		}
	}
}

func TestSurvivorStatParsesACommWithParentheses(t *testing.T) {
	st := []byte(statLine("a ) (b c", 12345))
	got, err := startedAt(st, fakeBoot, 100)
	if err != nil || !got.Equal(fakeBoot.Add(123450*time.Millisecond)) {
		t.Fatalf("%v %v", got, err)
	}
	// ... end to end: a comm like that, old, is ignored.
	denied := &os.PathError{Op: "readlink", Path: "x", Err: syscall.EACCES}
	f := host()
	f.procs[30] = &fakeEntry{owner: 1000, cmdline: "x\x00", nsErr: denied, stat: statLine(") (", 10)}
	if got, _ := scanSurvivors(f, sinceQuery()); len(got) != 0 {
		t.Fatalf("%v", got)
	}
}

func TestSurvivorS2AlsoNamesTheProbeConfiguration(t *testing.T) {
	f := host()
	f.procs[30] = &fakeEntry{owner: 1000, ns: hostNS, cmdline: "kiln\x00sandbox-exec\x00--config\x00" + surProbe + "\x00"}
	// Old, too: S2 applies whatever the start time.
	f.procs[30].stat = statLine("kiln", 5)
	got, _ := scanSurvivors(f, sinceQuery())
	if len(got) != 1 || !strings.Contains(got[0], "S2") || !strings.Contains(got[0], "toolchains-sandbox") {
		t.Fatalf("%v", got)
	}
}

func TestSurvivorTruncatedMountinfoFailsClosedWhateverTheOwner(t *testing.T) {
	// Another user's process in another namespace whose mountinfo is too long to
	// read: not parsed, counted (when new).
	f := host()
	f.procs[30] = &fakeEntry{owner: 0, status: "Uid:\t0\t0\t0\t0\n", ns: boxNS, cmdline: "x\x00", miErr: errTruncated, stat: statLine("x", 100*2000)}
	got, _ := scanSurvivors(f, sinceQuery())
	if len(got) != 1 || !strings.Contains(got[0], "S3") || !strings.Contains(got[0], "mountinfo") {
		t.Fatalf("%v", got)
	}
	// Started before prepare: the start-time rule still applies.
	f.procs[30].stat = statLine("x", 10)
	if got, _ = scanSurvivors(f, sinceQuery()); len(got) != 0 {
		t.Fatalf("%v", got)
	}
}

func TestSurvivorScanThatBlocksCountsAsASurvivor(t *testing.T) {
	f := host()
	f.block = make(chan struct{})
	defer close(f.block)
	scan := func() ([]string, error) { return scanSurvivors(f, surQuery) }
	c := &clock{t: time.Unix(0, 0)}
	got := waitSurvivors(scan, -1, c.now, c.sleep, 20*time.Millisecond)
	if len(got) != 1 || !strings.Contains(got[0], "took longer") {
		t.Fatalf("%v", got)
	}
}
