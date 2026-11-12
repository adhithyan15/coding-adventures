package source

import (
	"bufio"
	"crypto/sha256"
	"encoding/binary"
	"encoding/hex"
	"fmt"
	"io"
	"io/fs"
	"os"
	"path/filepath"
	"sort"
	"strings"
)

// IgnoreFile is the name of the ignore file read from the folder's root.
//
// .kilnignore syntax (intentionally minimal):
//
//	# a comment             (a line whose first non-blank character is #)
//	                        (blank lines are skipped)
//	build                   the path "build" and everything beneath it
//	build/**                same effect (a trailing "/" is an error)
//	**/*.log                every .log file at any depth
//	docs/*.tmp              .tmp files directly in docs
//
// Each line is one glob in the language of Match, matched against the whole
// repo-relative path (there are no negations and no basename shortcuts: "*.log"
// matches only top-level .log files).  A path is ignored when the path itself,
// or any directory above it, matches a line.  Surrounding whitespace is
// trimmed, and a ".git" directory is always skipped.
const IgnoreFile = ".kilnignore"

// Folder is the plain-folder Source: it walks a directory, with no history.
// A build from a folder must produce the same keys and plan as a build from
// the same files under Git, which is why this exists (KILN00 section 13).
type Folder struct {
	dir string
}

// NewFolder returns a Source for the directory dir.
func NewFolder(dir string) *Folder { return &Folder{dir: dir} }

func (f *Folder) loadIgnore() ([]string, error) {
	file, err := os.Open(filepath.Join(f.dir, IgnoreFile))
	if os.IsNotExist(err) {
		return nil, nil
	}
	if err != nil {
		return nil, err
	}
	defer file.Close()
	var pats []string
	sc := bufio.NewScanner(file)
	for n := 1; sc.Scan(); n++ {
		line := strings.TrimSpace(sc.Text())
		if line == "" || strings.HasPrefix(line, "#") {
			continue
		}
		if err := ValidatePattern(line); err != nil {
			return nil, fmt.Errorf("%s line %d: %w", IgnoreFile, n, err)
		}
		pats = append(pats, line)
	}
	return pats, sc.Err()
}

// ignored reports whether path, or any directory above it, matches a pattern.
func ignored(pats []string, path string) bool {
	segs := strings.Split(path, "/")
	for i := 1; i <= len(segs); i++ {
		prefix := strings.Join(segs[:i], "/")
		for _, p := range pats {
			if Match(p, prefix) {
				return true
			}
		}
	}
	return false
}

// list walks the folder and returns every non-ignored regular file and
// symlink (symlinks are entries, never followed), sorted.
func (f *Folder) list() ([]string, error) {
	pats, err := f.loadIgnore()
	if err != nil {
		return nil, err
	}
	var out []string
	err = filepath.WalkDir(f.dir, func(p string, d fs.DirEntry, err error) error {
		if err != nil {
			return err
		}
		if p == f.dir {
			return nil
		}
		rel, err := filepath.Rel(f.dir, p)
		if err != nil {
			return err
		}
		rel = filepath.ToSlash(rel)
		if d.IsDir() {
			if d.Name() == ".git" || ignored(pats, rel) {
				return filepath.SkipDir
			}
			return nil
		}
		if ignored(pats, rel) {
			return nil
		}
		if t := d.Type(); t.IsRegular() || t&fs.ModeSymlink != 0 {
			out = append(out, rel)
		}
		return nil // sockets, FIFOs, devices are not source
	})
	if err != nil {
		return nil, err
	}
	sort.Strings(out)
	return out, nil
}

// Files lists the folder's files; see Source.Files.
func (f *Folder) Files(root string, patterns []string) ([]string, error) {
	all, err := f.list()
	if err != nil {
		return nil, err
	}
	return filter(all, root, patterns)
}

// Open opens a regular file; see Source.Open.
func (f *Folder) Open(path string) (io.ReadCloser, error) { return openRegular(f.dir, path) }

// Revision is a content-derived id: SHA-256 over the sorted list of
// (path, kind, content digest) records, each length-framed.  Two folders with
// the same files and contents have the same Revision on any machine.
func (f *Folder) Revision() (string, error) {
	files, err := f.list()
	if err != nil {
		return "", err
	}
	h := sha256.New()
	var n [8]byte
	for _, p := range files {
		full := filepath.Join(f.dir, filepath.FromSlash(p))
		kind, digest, err := digestEntry(full)
		if err != nil {
			return "", err
		}
		binary.BigEndian.PutUint64(n[:], uint64(len(p)))
		h.Write(n[:])
		h.Write([]byte(p))
		h.Write([]byte{kind})
		h.Write(digest)
	}
	return hex.EncodeToString(h.Sum(nil)), nil
}

// digestEntry hashes a regular file's bytes (kind 1) or a symlink's target
// text (kind 2), mirroring KILN02's file kinds.
func digestEntry(full string) (byte, []byte, error) {
	fi, err := os.Lstat(full)
	if err != nil {
		return 0, nil, err
	}
	h := sha256.New()
	if fi.Mode()&fs.ModeSymlink != 0 {
		target, err := os.Readlink(full)
		if err != nil {
			return 0, nil, err
		}
		h.Write([]byte(filepath.ToSlash(target)))
		return 2, h.Sum(nil), nil
	}
	file, err := os.Open(full)
	if err != nil {
		return 0, nil, err
	}
	defer file.Close()
	if _, err := io.Copy(h, file); err != nil {
		return 0, nil, err
	}
	return 1, h.Sum(nil), nil
}

// Changed has no history to consult: it always answers "unknown".
func (f *Folder) Changed(since string) ([]string, bool, error) { return nil, false, nil }
