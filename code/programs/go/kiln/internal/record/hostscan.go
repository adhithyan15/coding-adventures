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
//   - No mount boundary is crossed. A subdirectory whose st_dev differs from
//     the root's is another file system; the sandbox binds a root
//     NON-recursively, so nothing mounted beneath it comes along, and walking
//     it would only report things the step cannot reach.
//   - One goroutine per root, because /usr is large and the roots are few.
//   - An entry we are not allowed to read is skipped, not an error: a step runs
//     as the same user and cannot read it either (the sandbox has no more
//     privilege than the recorder). Any other error fails the scan, because a
//     scan that quietly stopped is a pass nobody earned.

import (
	"errors"
	"fmt"
	"os"
	"path/filepath"
	"sort"
	"sync"
)

// devOf reads st_dev; a variable so a test can pretend a directory is a mount.
var devOf = deviceOf

// HostScan walks every root and returns an error naming at most MaxReported
// sockets or FIFOs.
func HostScan(roots []string) error {
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
			found, e := scanRoot(r)
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

// scanRoot returns the sockets and FIFOs under one root, sorted.
func scanRoot(root string) ([]string, error) {
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
			if errors.Is(err, os.ErrPermission) || errors.Is(err, os.ErrNotExist) {
				return nil
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
			case mode.IsDir():
				if dev, ok := devOf(info); haveDev && ok && dev != rootDev {
					continue // another file system: not part of a non-recursive bind
				}
				if err := walk(p); err != nil {
					return err
				}
			}
		}
		return nil
	}
	if err := walk(root); err != nil {
		return nil, fmt.Errorf("scan of %q: %w", printable(root), err)
	}
	sort.Strings(found)
	return found, nil
}

// hazard: a pathname socket or a FIFO.
func hazard(m os.FileMode) bool { return m&(os.ModeSocket|os.ModeNamedPipe) != 0 }
