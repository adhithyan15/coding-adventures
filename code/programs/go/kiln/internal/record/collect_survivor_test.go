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
}

type fakeEntry struct {
	owner   uint32
	ownErr  error
	cmdline string // NUL-separated
	cmdErr  error
	ns      string
	nsErr   error
	mounts  string // mountinfo text
	miErr   error
}

func (f *fakeProc) Pids() ([]int, error) {
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
	}
	return nil, errors.New("unexpected file " + name)
}

const (
	surRoot = "/work/repo"
	surCfg  = "/work/record/pre/sandbox.json"
	hostNS  = "mnt:[4026531841]"
	boxNS   = "mnt:[4026532999]"
)

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
	got, err := scanSurvivors(f, surRoot, surCfg)
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
	if _, err := scanSurvivors(f, surRoot, surCfg); err == nil {
		t.Fatal("an unlistable /proc was a clean scan")
	}
	f = host()
	f.selfNSErr = errors.New("denied")
	if _, err := scanSurvivors(f, surRoot, surCfg); err == nil {
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
	got := waitSurvivors(func() ([]string, error) { calls++; return []string{"pid 1: x"}, nil }, -1, c.now, c.sleep)
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
	}, 5*time.Second, c.now, c.sleep)
	if len(got) != 0 || calls != 4 || c.sleeps != 3 {
		t.Fatalf("%v calls %d sleeps %d", got, calls, c.sleeps)
	}
}

func TestWaitSurvivorsStillThereAfterTheWait(t *testing.T) {
	c := &clock{t: time.Unix(0, 0)}
	calls := 0
	got := waitSurvivors(func() ([]string, error) { calls++; return []string{"pid 1: x"}, nil }, time.Second, c.now, c.sleep)
	// 200 ms apart over one second: the scans at 0, 200, ... 1000 ms.
	if len(got) != 1 || calls != 6 || c.sleeps != 5 {
		t.Fatalf("%v calls %d sleeps %d", got, calls, c.sleeps)
	}
}

func TestWaitSurvivorsDefaultIsTenSecondsAndAFailedScanIsASurvivor(t *testing.T) {
	c := &clock{t: time.Unix(0, 0)}
	got := waitSurvivors(func() ([]string, error) { return nil, errors.New("boom") }, 0, c.now, c.sleep)
	if len(got) != 1 || !strings.Contains(got[0], "boom") {
		t.Fatalf("%v", got)
	}
	if c.t.Sub(time.Unix(0, 0)) != DefaultSurvivorWait {
		t.Fatalf("waited %v", c.t.Sub(time.Unix(0, 0)))
	}
}
