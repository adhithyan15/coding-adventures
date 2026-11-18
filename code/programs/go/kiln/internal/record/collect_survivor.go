package record

// The survivor check (KILN04 section 4.8, decision D6, run-void token `survivor`)
// ==============================================================================
//
// A build tool that is killed, or a step that is, can leave a process behind:
// a `sleep 60` the BUILD backgrounded, a compiler server, an orphaned
// supervisor. Such a process is still inside the sandbox, still holds a writable
// bind of the checkout, and may write to it AFTER `collect` has looked at the
// tree. Nothing `collect` reads afterwards could say who wrote what, so the
// run, not the shard, is void: header token `survivor` (merge rule M3), and
// every record of the shard is an unknown write set.
//
// `collect` runs this scan right after it holds the exclusive lock and has
// verified pre/: a supervisor that held the shared lock is gone by then, so
// what the scan finds is a process that is NOT one of the lock holders.
//
// What a survivor is. Every /proc/<pid> other than this process is looked at,
// and is one when ANY of these holds:
//
//	S1  its mount namespace is not collect's, and its mountinfo lists the
//	    checkout (RepoRoot) as a mount point. This is how a process of the
//	    sandbox looks from outside: the sandbox bind-mounts the checkout in a
//	    namespace of its own. (A namespace test alone would flag every
//	    container on the host.)
//	S2  an element of its command line is exactly `<dir>/pre/sandbox.json` (or
//	    the toolchain probes' `<dir>/probe/toolchains-sandbox.json`): `kiln
//	    sandbox-exec --config <that>` is the sandbox's own supervisor.
//	S3  it is ours (by the Uid line of its status), it started after `prepare`,
//	    and reading it fails with anything but "it is gone" (ENOENT, ESRCH). A
//	    process of ours that cannot be inspected cannot be cleared, so it is
//	    counted: the scan fails closed. See scanSurvivors for the details.
//
// There is NO test on the working directory (D6): a shell somebody left in the
// checkout is not a writer, and a cwd test would void every run done from a
// terminal sitting in the repository.
//
// A process that vanishes between listing and reading is not a survivor; a
// process that is still there after `SurvivorWait` (default 10 s, polled every
// 200 ms: a dying tree needs a moment) is. A scan that cannot be made at all
// (no /proc, unreadable) is reported as a survivor: "nobody is there" is a claim
// the scan must earn.

import (
	"errors"
	"fmt"
	"os"
	"strconv"
	"strings"
	"syscall"
	"time"

	"github.com/adhithyan15/coding-adventures/code/programs/go/kiln/internal/recfmt"
)

// DefaultSurvivorWait and survivorPoll are the wait for a dying process tree and
// how often the scan is repeated within it.
const (
	DefaultSurvivorWait = 10 * time.Second
	survivorPoll        = 200 * time.Millisecond
	maxSurvivorsListed  = 8
)

// ClockTicksPerSecond is USER_HZ; see recfmt.ClockTicksPerSecond.
const ClockTicksPerSecond = recfmt.ClockTicksPerSecond

// SurvivorSlackTicks is how much earlier than `prepare` a process may have
// started and still count as new: 200 ticks, two seconds. A process's starttime
// is truncated to a tick and `prepare` reads /proc/uptime at a one-tick
// resolution, so a process forked a moment before the read must not look old.
const SurvivorSlackTicks = 2 * ClockTicksPerSecond

// SurvivorQuery is what a scan is told: the checkout, the sealed sandbox
// configurations whose command-line mention marks a sandbox supervisor (S2), and
// the boot-relative moment `prepare` started (S3 looks only at processes that
// started after it).
//
// SinceTicks is in clock ticks since boot, the same clock as a process's
// starttime, so the comparison involves no wall-clock time at all and a step of
// the wall clock (NTP, a manual `date -s`) between `prepare` and `collect`
// cannot move a process across the line (decision D26). It already has the slack
// taken off. Zero or less: every process counts.
type SurvivorQuery struct {
	RepoRoot   string
	Configs    []string
	SinceTicks int64
}

// ReadSelfStartTicks is this process's own start time: field 22 of
// /proc/self/stat, in clock ticks since boot. It is the default of
// Options.SelfStartTicks (prepare) and CollectOptions.SelfStartTicks, and it uses
// startTicks, the parser the S3 gate applies to every other process, so the
// sealed value and the comparands are one field of one file, in one unit, on one
// epoch. It fails anywhere without /proc.
func ReadSelfStartTicks() (int64, error) {
	data, err := os.ReadFile("/proc/self/stat")
	if err != nil {
		return 0, err
	}
	t, err := startTicks(data)
	if err != nil {
		return 0, err
	}
	if t < 1 {
		return 0, fmt.Errorf("/proc/self/stat says this process started at tick %d", t)
	}
	return t, nil
}

// errTruncated is what a read of a /proc file says when it hit the read bound:
// a partial mountinfo or command line is not parsed, it is an inspection that
// failed.
var errTruncated = errors.New("the /proc file is longer than the read bound")

// procFS is every read the scan makes of /proc, so a test can supply a fake one.
type procFS interface {
	// Pids lists the numeric entries of /proc.
	Pids() ([]int, error)
	// Self is this process's id; UID the (effective) user it runs as.
	Self() int
	UID() uint32
	// Owner is the user that owns /proc/<pid>. It is only ever an ADDITIONAL
	// signal: for a non-dumpable task the kernel makes it the root of the task's
	// user namespace, which can be global root.
	Owner(pid int) (uint32, error)
	// ReadLink reads /proc/<pid>/<name> (a symlink, such as ns/mnt).
	ReadLink(pid int, name string) (string, error)
	// ReadFile reads /proc/<pid>/<name>, bounded (errTruncated past the bound).
	ReadFile(pid int, name string) ([]byte, error)
}

// gone says an error means the process no longer exists.
func gone(err error) bool {
	return errors.Is(err, os.ErrNotExist) || errors.Is(err, syscall.ESRCH)
}

// mountPointsOf parses a mountinfo (field 5 of each line is the mount point).
func mountPointsOf(data []byte) map[string]bool {
	out := map[string]bool{}
	for _, line := range strings.Split(string(data), "\n") {
		f := strings.Fields(line)
		if len(f) >= 5 {
			out[unescapeMount(f[4])] = true
		}
	}
	return out
}

// statusOurs reads the real and effective user ids out of /proc/<pid>/status
// (world-readable, whatever the process's dumpable flag) and says whether either
// is ours. A status that cannot be parsed is ours: fail closed.
func statusOurs(status []byte, me uint32) bool {
	for _, line := range strings.Split(string(status), "\n") {
		rest, ok := strings.CutPrefix(line, "Uid:")
		if !ok {
			continue
		}
		f := strings.Fields(rest)
		if len(f) < 2 {
			return true
		}
		real, e1 := strconv.ParseUint(f[0], 10, 32)
		eff, e2 := strconv.ParseUint(f[1], 10, 32)
		if e1 != nil || e2 != nil {
			return true
		}
		return uint32(real) == me || uint32(eff) == me
	}
	return true
}

// startTicks is when a process started: field 22 of /proc/<pid>/stat, in clock
// ticks since boot (ClockTicksPerSecond). It stays in ticks: the gate compares
// it with the ticks `prepare` sealed, and never converts through btime, which is
// derived from the wall clock (D26). comm, field 2, may hold spaces and
// parentheses, so the fields are counted from after the LAST ")".
func startTicks(stat []byte) (int64, error) {
	i := strings.LastIndexByte(string(stat), ')')
	if i < 0 {
		return 0, errors.New("stat has no comm")
	}
	f := strings.Fields(string(stat[i+1:])) // f[0] is field 3
	if len(f) < 20 {
		return 0, errors.New("stat has too few fields")
	}
	ticks, err := strconv.ParseUint(f[19], 10, 63)
	if err != nil {
		return 0, err
	}
	// A count of more than 100 years of ticks is not a real one: unknown, which
	// the caller counts as "started after prepare" (fail closed).
	if ticks > recfmt.MaxClockTicks {
		return 0, errors.New("stat starttime is absurd")
	}
	return int64(ticks), nil
}

// scanSurvivors returns what it found, one line per survivor (at most
// maxSurvivorsListed, then a count), or an error when the scan itself could not
// be made.
//
// S3 in detail. A real survivor of the sandbox is ALWAYS inspectable from here:
// it lives in a user namespace our user owns, so we hold ptrace access to it
// even when it made itself non-dumpable. What we cannot inspect is therefore
// either somebody else's, or a process of ours that is hiding. To tell, without
// voiding every run on a host that has a `systemd --user` manager, an
// ssh-agent or a keyring daemon (non-dumpable, ours, uninspectable, and OLD):
//
//   - "ours" is decided by the Uid line of /proc/<pid>/status, not by who owns
//     /proc/<pid> (which a non-dumpable task makes the root of its user
//     namespace: a hostile step could use that to look like root's). A status
//     that cannot be read or parsed is ours; the owner of /proc/<pid> is only
//     one more reason to say ours, never a reason to skip;
//   - S3 applies only to a process that started at or after `prepare` (minus
//     slack): nothing of this build is older. Both sides are ticks since boot
//     (starttime against binding.prepared_ticks), so a wall-clock step cannot
//     misplace a process (D26). A stat that cannot be read or parsed counts as
//     started after.
//
// S1 and S2 apply to every process.
func scanSurvivors(fs procFS, q SurvivorQuery) ([]string, error) {
	self := fs.Self()
	selfNS, err := fs.ReadLink(self, "ns/mnt")
	if err != nil {
		return nil, fmt.Errorf("this process's mount namespace cannot be read: %v", err)
	}
	pids, err := fs.Pids()
	if err != nil {
		return nil, fmt.Errorf("/proc cannot be listed: %v", err)
	}
	me := fs.UID()
	var found []string
	n := 0
	add := func(pid int, why string) {
		n++
		if len(found) < maxSurvivorsListed {
			found = append(found, fmt.Sprintf("pid %d: %s", pid, why))
		}
	}
	for _, pid := range pids {
		if pid == self {
			continue
		}
		status, serr := fs.ReadFile(pid, "status")
		if serr != nil && gone(serr) {
			continue
		}
		ours := serr != nil || statusOurs(status, me)
		if owner, err := fs.Owner(pid); err != nil {
			if gone(err) {
				continue
			}
			ours = true
		} else if owner == me {
			ours = true
		}
		// after: did it start at or after prepare? Computed once, lazily.
		after, known := true, false
		started := func() bool {
			if known {
				return after
			}
			known = true
			if q.SinceTicks <= 0 {
				return after
			}
			st, err := fs.ReadFile(pid, "stat")
			if err != nil {
				return after
			}
			t, err := startTicks(st)
			if err != nil {
				return after
			}
			after = t >= q.SinceTicks
			return after
		}
		// fail records an inspection that failed: gone is nothing; otherwise it is
		// S3 for a process of ours (or any, when ownership does not matter: a
		// truncated read) that started after prepare.
		fail := func(what string, err error) {
			if gone(err) {
				return
			}
			if (ours || errors.Is(err, errTruncated)) && started() {
				add(pid, "S3: "+what+" cannot be inspected: "+err.Error())
			}
		}
		// S2: a sealed configuration named in the command line.
		cmd, err := fs.ReadFile(pid, "cmdline")
		if err != nil {
			if gone(err) {
				continue
			}
			fail("cmdline", err)
		} else {
			hit := ""
			for _, el := range strings.Split(string(cmd), "\x00") {
				for _, c := range q.Configs {
					if el == c {
						hit = c
					}
				}
			}
			if hit != "" {
				add(pid, "S2: its command line names "+printable(hit))
				continue
			}
		}
		// S1: another mount namespace that has the checkout mounted.
		ns, err := fs.ReadLink(pid, "ns/mnt")
		if err != nil {
			fail("ns/mnt", err)
			continue
		}
		if ns == selfNS {
			continue
		}
		mi, err := fs.ReadFile(pid, "mountinfo")
		if err != nil {
			fail("mountinfo", err)
			continue
		}
		mounts := mountPointsOf(mi)
		if len(mounts) == 0 {
			// A namespace with no mounts at all is not a thing a live process has.
			if ours && started() {
				add(pid, "S3: its mountinfo lists nothing")
			}
			continue
		}
		if mounts[q.RepoRoot] {
			add(pid, "S1: it is in another mount namespace that has the checkout mounted")
		}
	}
	if n > len(found) {
		found = append(found, fmt.Sprintf("... and %d more", n-len(found)))
	}
	return found, nil
}

// DefaultSurvivorScanTimeout bounds ONE scan: a /proc read can block for good on
// a task stuck in uninterruptible sleep.
const DefaultSurvivorScanTimeout = 5 * time.Second

// scanWithin runs scan and gives up after d: a scan that does not finish counts
// as a survivor (the stuck goroutine is abandoned; collect is about to end).
func scanWithin(scan func() ([]string, error), d time.Duration) ([]string, error) {
	if d <= 0 {
		d = DefaultSurvivorScanTimeout
	}
	type result struct {
		found []string
		err   error
	}
	ch := make(chan result, 1)
	go func() {
		f, e := scan()
		ch <- result{f, e}
	}()
	t := time.NewTimer(d)
	defer t.Stop()
	select {
	case r := <-ch:
		return r.found, r.err
	case <-t.C:
		return nil, fmt.Errorf("a scan of /proc took longer than %s", d)
	}
}

// waitSurvivors runs the scan until it finds nothing or the wait is spent. wait
// below zero means one scan and no waiting (a test's); zero is the default. A
// scan that fails is a survivor, and is retried like one. It returns what the
// last scan found.
func waitSurvivors(scan func() ([]string, error), wait time.Duration, now func() time.Time, sleep func(time.Duration), timeout time.Duration) []string {
	if wait == 0 {
		wait = DefaultSurvivorWait
	}
	deadline := now().Add(wait)
	for {
		found, err := scanWithin(scan, timeout)
		if err != nil {
			found = []string{"the survivor scan failed: " + printable(err.Error())}
		}
		if len(found) == 0 || wait < 0 || !now().Before(deadline) {
			return found
		}
		sleep(survivorPoll)
	}
}
