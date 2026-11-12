package source

import (
	"fmt"
	"io/fs"
	"os"
	"path/filepath"
	"runtime"
	"strings"
	"sync"
)

// Info says how one listed path is stored, which is what a KILN02 inputs
// entry needs beyond the path itself (section 3.5):
//
//	Link    the entry is a symbolic link (kind 2); Target is its target TEXT,
//	        "/"-separated, never resolved
//	Exec    a regular file is executable AS THE SOURCE REPORTS IT (Git's mode
//	        100755), never from the disk, so a Windows checkout of the same
//	        commit produces the same entry as a Linux one
type Info struct {
	Link   bool
	Target string
	Exec   bool
}

// Describer is an optional interface of a Source: it describes a path that
// Files listed.  Legacy keys (KILN03) need it because Source.Open refuses
// symlinks and carries no mode.
type Describer interface {
	Describe(path string) (Info, error)
}

// Cleaner is an optional interface of a Source that can tell whether the
// tree is clean in the sense KILN03 section 9 needs: Untracked lists every
// file that exists but is neither tracked nor ignored.  Such a file (a build
// product, say) would otherwise sneak into a package's own digest in some
// adapters or be silently missing in others.
type Cleaner interface {
	Untracked() ([]string, error)
}

// Git mode strings as printed by `git ls-files --stage`.
const (
	modeRegular    = "100644"
	modeExecutable = "100755"
	modeSymlink    = "120000"
)

// treeModes is the lazily loaded path -> "mode stage" table.
type treeModes struct {
	once sync.Once
	m    map[string][2]string
	err  error
}

// modes loads `git ls-files --stage -z` once per Git value.  Each record is
// "<mode> <object> <stage>\t<path>" terminated by NUL.
func (g *Git) modes() (map[string][2]string, error) {
	tm := &g.tm
	tm.once.Do(func() {
		out, err := g.run("ls-files", "--stage", "-z", "--")
		if err != nil {
			tm.err = err
			return
		}
		tm.m = map[string][2]string{}
		for _, rec := range splitNUL(out) {
			tab := strings.IndexByte(rec, '\t')
			fields := strings.Fields(rec[:max(tab, 0)])
			if tab < 0 || len(fields) != 3 {
				tm.err = fmt.Errorf("git ls-files --stage: malformed record %q", short(rec))
				return
			}
			tm.m[rec[tab+1:]] = [2]string{fields[0], fields[2]}
		}
	})
	return tm.m, tm.err
}

// Describe reports a tracked path's storage from the Git index.  Submodules
// (160000), unmerged entries and unknown modes are errors, never guesses.
func (g *Git) Describe(path string) (Info, error) {
	if err := checkSourcePath(path); err != nil {
		return Info{}, err
	}
	m, err := g.modes()
	if err != nil {
		return Info{}, err
	}
	e, ok := m[path]
	if !ok {
		return Info{}, fmt.Errorf("%s: not tracked", short(path))
	}
	if e[1] != "0" {
		return Info{}, fmt.Errorf("%s: unmerged index entry", short(path))
	}
	switch e[0] {
	case modeRegular:
		return Info{}, nil
	case modeExecutable:
		return Info{Exec: true}, nil
	case modeSymlink:
		return readLinkInfo(g.dir, path)
	}
	return Info{}, fmt.Errorf("%s: unsupported git mode %s", short(path), e[0])
}

// readLinkInfo reads a symlink's target text through os.Root.
func readLinkInfo(dir, path string) (Info, error) {
	root, err := os.OpenRoot(dir)
	if err != nil {
		return Info{}, err
	}
	defer root.Close()
	t, err := root.Readlink(filepath.FromSlash(path))
	if err != nil {
		return Info{}, err
	}
	return Info{Link: true, Target: filepath.ToSlash(t)}, nil
}

// Untracked lists untracked, non-ignored files (`git status --porcelain=v1
// -z --untracked-files=all`).  In that format a record is "XY path"; a rename
// or copy is followed by one extra NUL-terminated field (the old path), which
// is skipped.  Only "??" records are untracked files.
func (g *Git) Untracked() ([]string, error) {
	out, err := g.run("status", "--porcelain=v1", "-z", "--untracked-files=all")
	if err != nil {
		return nil, err
	}
	recs := splitNUL(out)
	found := []string{}
	for i := 0; i < len(recs); i++ {
		r := recs[i]
		if len(r) < 4 {
			return nil, fmt.Errorf("git status: malformed record %q", short(r))
		}
		switch {
		case r[:2] == "??":
			found = append(found, r[3:])
		case r[0] == 'R' || r[0] == 'C' || r[1] == 'R' || r[1] == 'C':
			i++
		}
	}
	sortStrings(found)
	return found, nil
}

// Describe reports a folder entry from the disk, without following links.
// A regular file is executable when any execute bit is set; Windows has no
// such bit, so nothing is executable there (a Git source of the same tree
// would say otherwise: that is the documented difference).
func (f *Folder) Describe(path string) (Info, error) {
	if err := checkSourcePath(path); err != nil {
		return Info{}, err
	}
	root, err := os.OpenRoot(f.dir)
	if err != nil {
		return Info{}, err
	}
	defer root.Close()
	name := filepath.FromSlash(path)
	fi, err := root.Lstat(name)
	if err != nil {
		return Info{}, err
	}
	if fi.Mode()&fs.ModeSymlink != 0 {
		t, err := root.Readlink(name)
		if err != nil {
			return Info{}, err
		}
		return Info{Link: true, Target: filepath.ToSlash(t)}, nil
	}
	if !fi.Mode().IsRegular() {
		return Info{}, fmt.Errorf("%s: not a regular file", short(path))
	}
	return Info{Exec: runtime.GOOS != "windows" && fi.Mode()&0o111 != 0}, nil
}
