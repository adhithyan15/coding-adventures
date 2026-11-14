package record

// The host scan of the read-only roots (KILN04 section 4.8, stage 4b-1,
// decision P8; D15)
// =======================================================================
//
// The sandbox's new root is an allowlist: it holds the checkout, the state
// roots and the `ro_roots`, and nothing else of the host. A pathname Unix
// socket or a FIFO that lies inside one of those is reachable whatever the
// mount flags say, because `connect` and `open` on it reach the process on the
// other end, and that process is outside the sandbox:
//
//	ro root /usr        /usr/lib/foo/daemon.sock   <- a step connects to it,
//	                                                  and asks the daemon to do
//	                                                  what the sandbox forbids
//
// So before any BUILD, every `ro_roots` entry is walked ON THE HOST, and the
// shard is void if it holds a socket or a FIFO.
//
// How the walk is kept honest:
//
//   - `lstat` only. A symlink is never followed: it is recreated, not entered,
//     in the sandbox, and following one would walk the host outside the root.
//   - A root that is itself a symlink (`/bin` on most images) is not entered.
//     Its target is another root or is absent from the sandbox.
//   - A real mount point is not crossed. The sandbox binds a root
//     NON-recursively, so nothing mounted beneath it comes along, and walking
//     it would only report things the step cannot reach. A different st_dev is
//     only a hint: it is believed when /proc/self/mountinfo lists the
//     directory as a mount point. If mountinfo cannot be read (or does not list
//     it), the directory is scanned like any other, which can only add
//     findings: the scan fails closed. (What a host mount shadows beneath it is
//     covered by the in-sandbox `shadow` probe: probe.go and ShadowedMounts.)
//   - One goroutine per root, because /usr is large and the roots are few.
//   - A directory we cannot list is skipped ONLY when we cannot traverse it
//     either (no search permission): then a step, which runs as the same user,
//     cannot reach anything in it. A directory with search permission but no
//     read permission (`--x`) hides its names from a listing but not from a
//     `connect` to a known name, so that fails the scan. Any other error fails
//     it too, because a scan that quietly stopped is a pass nobody earned.
//   - The walk of a root stops after MaxReported findings.

import (
	"errors"
	"fmt"
	"os"
	"path/filepath"
	"sort"
	"strconv"
	"strings"
	"sync"

	"github.com/adhithyan15/coding-adventures/code/programs/go/kiln/internal/sandbox"
)

// devOf reads st_dev; a variable so a test can pretend a directory is a mount.
var devOf = deviceOf

// mountPoints returns the mount points of this mount namespace, or nil when
// they cannot be known. A variable so a test can supply them.
var mountPoints = readMountPoints

// readMountPoints parses the kernel's mountinfo for this process (field 5 of
// each line is the mount point, with space, tab, newline and backslash written
// as octal escapes).
func readMountPoints() map[string]bool {
	data, err := os.ReadFile("/proc/self/mountinfo")
	if err != nil {
		return nil
	}
	out := map[string]bool{}
	for _, line := range strings.Split(string(data), "\n") {
		f := strings.Fields(line)
		if len(f) >= 5 {
			out[unescapeMount(f[4])] = true
		}
	}
	return out
}

// unescapeMount decodes the kernel's \040-style escapes.
func unescapeMount(s string) string {
	if !strings.Contains(s, "\\") {
		return s
	}
	var b strings.Builder
	for i := 0; i < len(s); i++ {
		if s[i] == '\\' && i+3 < len(s) {
			if n, err := strconv.ParseUint(s[i+1:i+4], 8, 8); err == nil {
				b.WriteByte(byte(n))
				i += 3
				continue
			}
		}
		b.WriteByte(s[i])
	}
	return b.String()
}

// errEnough stops a walk that has found MaxReported hazards.
var errEnough = errors.New("enough findings")

// HostScan walks every root and returns an error naming at most MaxReported
// sockets or FIFOs.
func HostScan(roots []string) error {
	mounts := mountPoints()
	var (
		mu  sync.Mutex
		bad problems
		wg  sync.WaitGroup
		err error
	)
	for _, r := range roots {
		wg.Add(1)
		go func() {
			defer wg.Done()
			found, e := scanRoot(r, mounts)
			mu.Lock()
			defer mu.Unlock()
			for _, p := range found {
				bad.add(p, "a socket or FIFO inside a read-only root: a step could reach a process outside the sandbox through it")
			}
			if e != nil && err == nil {
				err = e
			}
		}()
	}
	wg.Wait()
	if err != nil {
		return err
	}
	return bad.err("host scan")
}

// scanRoot returns the sockets and FIFOs under one root, sorted (at most
// MaxReported of them).
func scanRoot(root string, mounts map[string]bool) ([]string, error) {
	st, err := os.Lstat(root)
	if err != nil {
		return nil, err
	}
	if st.Mode()&os.ModeSymlink != 0 {
		return nil, nil // recreated, never entered
	}
	if hazard(st.Mode()) {
		return []string{root}, nil
	}
	if !st.IsDir() {
		return nil, nil
	}
	rootDev, haveDev := devOf(st)
	var found []string
	var walk func(dir string) error
	walk = func(dir string) error {
		entries, err := os.ReadDir(dir)
		if err != nil {
			if errors.Is(err, os.ErrNotExist) {
				return nil
			}
			if errors.Is(err, os.ErrPermission) && !searchable(dir) {
				return nil // unreachable by this user, so by a step
			}
			return err
		}
		for _, e := range entries {
			p := filepath.Join(dir, e.Name())
			info, err := e.Info() // lstat
			if err != nil {
				if errors.Is(err, os.ErrNotExist) {
					continue
				}
				return err
			}
			switch mode := info.Mode(); {
			case hazard(mode):
				found = append(found, p)
				if len(found) >= MaxReported {
					return errEnough
				}
			case mode.IsDir():
				if dev, ok := devOf(info); haveDev && ok && dev != rootDev && mounts[p] {
					continue // a real mount point: not part of a non-recursive bind
				}
				if err := walk(p); err != nil {
					return err
				}
			}
		}
		return nil
	}
	if err := walk(root); err != nil && !errors.Is(err, errEnough) {
		return nil, fmt.Errorf("scan of %q: %w", printable(root), err)
	}
	sort.Strings(found)
	return found, nil
}

// hazard: a pathname socket or a FIFO.
func hazard(m os.FileMode) bool { return m&(os.ModeSocket|os.ModeNamedPipe) != 0 }

// ShadowedMounts are the host mount points that lie strictly inside a place the
// sandbox binds NON-recursively: a read-only root (that is not a symlink), the
// checkout, or a state root. The sandbox shows, at such a path, the directory
// UNDER the mount, which the host scan (it does not cross mount points) never
// looked at. The in-sandbox probe `shadow` (probe.go) does, once for all of them.
//
// The result is sorted, has no duplicate, and holds no entry inside another. A mountinfo that cannot be read is
// an error (the answer would be "none", which a failed read must never give), and
// so is a list longer than MaxShadowed.
func ShadowedMounts(cfg *sandbox.Config) ([]string, error) {
	mounts := mountPoints()
	if mounts == nil {
		return nil, fmt.Errorf("the mount points of this host cannot be read (/proc/self/mountinfo): the shadowed directories cannot be listed")
	}
	var roots []string
	for _, r := range cfg.RoRoots {
		st, err := os.Lstat(r)
		if errors.Is(err, os.ErrNotExist) {
			continue
		}
		if err != nil {
			return nil, fmt.Errorf("shadowed mounts: ro root %q: %v", printable(r), err)
		}
		if st.Mode()&os.ModeSymlink != 0 {
			continue // recreated, not bound: nothing under it is the host's
		}
		roots = append(roots, r)
	}
	roots = append(roots, cfg.RepoRoot)
	roots = append(roots, cfg.StateRoots...)
	seen := map[string]bool{}
	out := []string{}
	for m := range mounts {
		for _, r := range roots {
			if strings.HasPrefix(m, strings.TrimSuffix(r, "/")+"/") && !seen[m] {
				seen[m] = true
				out = append(out, m)
			}
		}
	}
	sort.Strings(out)
	// Only the outermost mount point is visible in the sandbox: beneath a
	// shadowed directory the nested mount point may not exist at all (find would
	// fail on it), and everything under it is covered by walking the outer one.
	kept := out[:0]
	for _, m := range out {
		if n := len(kept); n > 0 && strings.HasPrefix(m, kept[n-1]+"/") {
			continue
		}
		kept = append(kept, m)
	}
	out = kept
	if len(out) > MaxShadowed {
		return nil, fmt.Errorf("%d host mount points lie inside the read-only roots, the checkout or the state roots (at most %d are probed)", len(out), MaxShadowed)
	}
	return out, nil
}
