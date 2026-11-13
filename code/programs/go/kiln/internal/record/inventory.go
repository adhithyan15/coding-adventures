package record

// Inventory (KILN04 section 4.8)
// ==============================
//
// "What was on the disk before the build that the commit does not hold?"
//
// The merge job needs this to tell a leftover from a product. If an action
// observed an untracked path and that path is in the pre-build inventory, then
// no action CREATED it during the run: it was already there, and whatever the
// action saw is a fact about the machine, not about the commit (rules M2 and M9
// of section 7.1). The inventory is taken before any BUILD runs and travels
// with the shard's binding.
//
// Granularity. A directory that holds no tracked file is ONE entry, however
// much is inside it: `code/packages/rust/foo/target` is a single path, not
// forty thousand. That is the granularity of a write root, so "was it there
// before" and "did an action write under it" compare like with like. A
// directory that does hold tracked files is entered, and its untracked members
// are listed one by one:
//
//	tracked:   code/p/BUILD  code/p/src/lib.rs
//	on disk:   code/p/BUILD  code/p/src/lib.rs  code/p/notes.txt
//	           code/p/target/ (many files)      code/p/src/scratch/ (empty)
//	inventory: code/p/notes.txt  code/p/src/scratch  code/p/target
//
// What is deliberately not done:
//
//   - Git is asked only which paths are TRACKED. Ignored files are NOT
//     skipped: a `.gitignore`d `target/` is exactly the sort of leftover this
//     list exists to name.
//   - Symlinks are entries and are never followed, so a link loop, or a link
//     out of the checkout, cannot make the walk wander or hang.
//   - The root `.git` (a directory, or the file a linked worktree has) is not
//     part of the checkout and is skipped. A `.git` further down is just an
//     untracked directory, and is listed like any other.
//
// The result is sorted by byte order, which is what the record formats demand
// (`recfmt.Inventory.Validate`), so the same tree always yields the same bytes.
// A name the formats refuse (not UTF-8, a trailing slash, over-long, too deep)
// fails the call: a tree whose inventory cannot be written cannot be recorded.

import (
	"fmt"
	"os"
	"path"
	"path/filepath"
	"sort"

	"github.com/adhithyan15/coding-adventures/code/programs/go/kiln/internal/recfmt"
	"github.com/adhithyan15/coding-adventures/code/programs/go/kiln/internal/source"
)

// Inventory lists everything under root that the tracked tree does not hold.
func Inventory(root string) (recfmt.Inventory, error) {
	out, err := source.NewGit(root).Output("ls-files", "-z")
	if err != nil {
		return recfmt.Inventory{}, err
	}
	tracked := map[string]bool{}
	haveTracked := map[string]bool{} // directories that contain a tracked path
	for _, p := range splitNUL(out) {
		tracked[p] = true
		for d := path.Dir(p); d != "."; d = path.Dir(d) {
			haveTracked[d] = true
		}
	}

	paths := []string{}
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
			switch {
			case rel == ".git":
				continue
			case tracked[rel]:
				// A tracked file, symlink or gitlink: not an inventory
				// entry, and a gitlink's directory is not entered.
				continue
			case e.IsDir() && haveTracked[rel]:
				if err := walk(rel); err != nil {
					return err
				}
			default:
				// An untracked file, symlink, FIFO..., or a directory with
				// nothing tracked inside it (collapsed to one entry).
				paths = append(paths, rel)
			}
		}
		return nil
	}
	if err := walk(""); err != nil {
		return recfmt.Inventory{}, err
	}

	sort.Strings(paths)
	inv := recfmt.Inventory{Schema: recfmt.SchemaInventory, Paths: paths}
	if err := inv.Validate(); err != nil {
		return recfmt.Inventory{}, fmt.Errorf("inventory cannot be recorded: %w", err)
	}
	return inv, nil
}
