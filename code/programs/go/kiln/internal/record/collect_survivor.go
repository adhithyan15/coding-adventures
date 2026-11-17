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
//	S2  an element of its command line is exactly `<dir>/pre/sandbox.json`, the
//	    sealed configuration: `kiln sandbox-exec --config <that>` is the
//	    sandbox's own supervisor.
//	S3  it belongs to our user and reading it fails with anything but "it is
//	    gone" (ENOENT, ESRCH). A process of ours that cannot be inspected cannot
//	    be cleared, so it is counted: the scan fails closed.
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
	"strings"
	"syscall"
	"time"
)

// DefaultSurvivorWait and survivorPoll are the wait for a dying process tree and
// how often the scan is repeated within it.
const (
	DefaultSurvivorWait = 10 * time.Second
	survivorPoll        = 200 * time.Millisecond
	maxSurvivorsListed  = 8
)

// procFS is every read the scan makes of /proc, so a test can supply a fake one.
type procFS interface {
	// Pids lists the numeric entries of /proc.
	Pids() ([]int, error)
	// Self is this process's id; UID the user it runs as.
	Self() int
	UID() uint32
	// Owner is the user that owns /proc/<pid>.
	Owner(pid int) (uint32, error)
	// ReadLink reads /proc/<pid>/<name> (a symlink, such as ns/mnt).
	ReadLink(pid int, name string) (string, error)
	// ReadFile reads /proc/<pid>/<name>, bounded.
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

// scanSurvivors returns what it found, one line per survivor (at most
// maxSurvivorsListed, then a count), or an error when the scan itself could not
// be made. repoRoot is the checkout, cfgPath the sealed sandbox configuration.
func scanSurvivors(fs procFS, repoRoot, cfgPath string) ([]string, error) {
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
		owner, err := fs.Owner(pid)
		if err != nil {
			if !gone(err) {
				// Not even the owner is known: it cannot be shown not to be ours.
				add(pid, "S3: /proc/<pid> cannot be examined: "+err.Error())
			}
			continue
		}
		ours := owner == me
		// fail records a read that failed: gone is nothing, an error on a process
		// of ours is S3, an error on somebody else's is not our concern (a sandbox
		// runs as the user that started it).
		fail := func(what string, err error) {
			if !gone(err) && ours {
				add(pid, "S3: "+what+" cannot be read: "+err.Error())
			}
		}
		// S2: the sealed configuration named in the command line.
		cmd, err := fs.ReadFile(pid, "cmdline")
		if err != nil {
			if gone(err) {
				continue
			}
			fail("cmdline", err)
		} else {
			hit := false
			for _, el := range strings.Split(string(cmd), "\x00") {
				if el == cfgPath {
					hit = true
				}
			}
			if hit {
				add(pid, "S2: its command line names "+printable(cfgPath))
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
			if ours {
				add(pid, "S3: its mountinfo lists nothing")
			}
			continue
		}
		if mounts[repoRoot] {
			add(pid, "S1: it is in another mount namespace that has the checkout mounted")
		}
	}
	if n > len(found) {
		found = append(found, fmt.Sprintf("... and %d more", n-len(found)))
	}
	return found, nil
}

// waitSurvivors runs the scan until it finds nothing or the wait is spent. wait
// below zero means one scan and no waiting (a test's); zero is the default. A
// scan that fails is a survivor, and is retried like one. It returns what the
// last scan found.
func waitSurvivors(scan func() ([]string, error), wait time.Duration, now func() time.Time, sleep func(time.Duration)) []string {
	if wait == 0 {
		wait = DefaultSurvivorWait
	}
	deadline := now().Add(wait)
	for {
		found, err := scan()
		if err != nil {
			found = []string{"the survivor scan failed: " + printable(err.Error())}
		}
		if len(found) == 0 || wait < 0 || !now().Before(deadline) {
			return found
		}
		sleep(survivorPoll)
	}
}
