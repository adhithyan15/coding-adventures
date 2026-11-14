package record

// The pre-BUILD disk scan (KILN04 section 4.8, stage 4b-1, decision P5)
// ======================================================================
//
// VerifyTree proves every TRACKED path is what the commit says. It says nothing
// about what else is on the disk, and two kinds of stray file would still change
// what the shard records:
//
//	an untracked BUILD file   the build tool would run it: a `BUILD_linux`
//	                          dropped beside a package's BUILD changes the lines
//	                          that package executes, and a whole new package can
//	                          appear as `code/packages/x/new/BUILD` inside an
//	                          untracked directory
//	a hard link               a regular file with more than one name. A BUILD
//	                          that writes through one name changes the content
//	                          at every other name, and a trace only names the
//	                          path that was opened (KILN04 5.5, "Aliases"). A
//	                          link made BEFORE the run is the one alias no trace
//	                          can show, so none may exist.
//
// Why a walk of the whole disk and not just the inventory. The inventory
// collapses a directory that holds no tracked file to one entry
// (`code/packages/x/new`), which is the right granularity for "was it there
// before" and the wrong one for "is there a BUILD in it". So the walk below
// looks at every name, one `lstat` each, and follows nothing:
//
//	for every entry under root, except the root's own .git:
//	    a name starting with BUILD that the index does not list   -> problem
//	    a regular file whose link count is above one              -> problem
//	    a directory                                               -> descend
//	    anything else (symlink, FIFO, socket)                     -> nothing
//
// A symlink is not descended into and its link count is not asked: a symlink's
// own count says nothing about its target. A tracked file that is a hard link
// is a problem like any other; the committed tree decides nothing about it.

import (
	"errors"
	"fmt"
	"os"
	"path"
	"path/filepath"
	"strings"

	"github.com/adhithyan15/coding-adventures/code/programs/go/kiln/internal/recfmt"
	"github.com/adhithyan15/coding-adventures/code/programs/go/kiln/internal/source"
)

// scanLimit stops the walk once this many problems are known: a tree that is
// wrong in a million places is wrong, and collecting all of them first is a way
// to run out of memory. A variable so a test can lower it.
var scanLimit = 1 << 16

const untrackedBuild = "an untracked BUILD path: it would change the lines a package runs"

// CheckInventory refuses a pre-BUILD inventory that holds a path whose base name
// starts with `BUILD` (an untracked BUILD variant, or a directory that looks
// like one). ScanTree finds the same files inside collapsed directories.
func CheckInventory(inv recfmt.Inventory) error {
	var bad problems
	for _, p := range inv.Paths {
		if strings.HasPrefix(path.Base(p), "BUILD") {
			bad.add(p, untrackedBuild)
		}
	}
	return bad.err("pre-build inventory")
}

// ScanTree walks root as described above and returns a *TreeError naming at
// most MaxReported problems. root must be the top level of its repository.
func ScanTree(root string) error {
	g := source.NewGit(root)
	if err := requireTopLevel(g); err != nil {
		return err
	}
	tracked, err := lsIndex(g)
	if err != nil {
		return err
	}
	var bad problems
	var walk func(dir string) error
	walk = func(dir string) error {
		entries, err := os.ReadDir(filepath.Join(root, filepath.FromSlash(dir)))
		if err != nil {
			return err
		}
		for _, e := range entries {
			rel := e.Name()
			if dir != "" {
				rel = dir + "/" + rel
			}
			if rel == ".git" {
				continue
			}
			if strings.HasPrefix(e.Name(), "BUILD") {
				if _, ok := tracked[rel]; !ok {
					bad.add(rel, untrackedBuild)
				}
			}
			// DirEntry.Info is an lstat: a symlink reports as a symlink.
			info, err := e.Info()
			if err != nil {
				if errors.Is(err, os.ErrNotExist) {
					continue // removed while we walked; nothing to judge
				}
				return err
			}
			switch mode := info.Mode(); {
			case mode.IsDir():
				if err := walk(rel); err != nil {
					return err
				}
			case mode.IsRegular():
				n, ok := linkCount(info)
				if !ok {
					return fmt.Errorf("cannot read the link count of %q on this platform", printable(rel))
				}
				if n > 1 {
					bad.add(rel, fmt.Sprintf("a hard link (%d names): a write through one name would change all of them", n))
				}
			}
			if len(bad.list) >= scanLimit {
				return fmt.Errorf("more than %d problems; stopping", scanLimit)
			}
		}
		return nil
	}
	if err := walk(""); err != nil {
		return err
	}
	return bad.err("pre-build disk scan")
}
