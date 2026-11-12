package source

import (
	"crypto/sha256"
	"encoding/binary"
	"encoding/hex"
	"errors"
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
// trimmed, and a ".git" directory is always skipped.  Limits: 64 KiB, 1000
// patterns, 64 segments per pattern; exceeding any is an error.  The file is
// opened without following symlinks.
const IgnoreFile = ".kilnignore"

// Folder is the plain-folder Source: it walks a directory, with no history.
// A build from a folder must produce the same keys and plan as a build from
// the same files under Git, which is why this exists (KILN00 section 13).
type Folder struct {
	dir string
}

// NewFolder returns a Source for the directory dir.
func NewFolder(dir string) *Folder { return &Folder{dir: dir} }

// Limits on .kilnignore, so a hostile or accidental file cannot make planning
// slow or memory-hungry.
const (
	MaxIgnoreBytes    = 64 << 10 // 64 KiB
	MaxIgnorePatterns = 1000
)

// loadIgnore reads .kilnignore through os.Root with the regular-file-only
// open: a symlinked or special .kilnignore is an error, never followed.
func (f *Folder) loadIgnore() ([]string, error) {
	root, err := os.OpenRoot(f.dir)
	if err != nil {
		return nil, err
	}
	defer root.Close()
	return loadIgnoreIn(root)
}

func loadIgnoreIn(root *os.Root) ([]string, error) {
	if _, err := root.Lstat(IgnoreFile); errors.Is(err, fs.ErrNotExist) {
		return nil, nil
	}
	file, err := openRegularIn(root, IgnoreFile)
	if err != nil {
		return nil, err
	}
	defer file.Close()
	data, err := io.ReadAll(io.LimitReader(file, MaxIgnoreBytes+1))
	if err != nil {
		return nil, err
	}
	if len(data) > MaxIgnoreBytes {
		return nil, fmt.Errorf("%s exceeds the %d byte limit", IgnoreFile, MaxIgnoreBytes)
	}
	var pats []string
	for n, raw := range strings.Split(string(data), "\n") {
		line := strings.TrimSpace(raw)
		if line == "" || strings.HasPrefix(line, "#") {
			continue
		}
		if err := ValidatePattern(line); err != nil {
			return nil, fmt.Errorf("%s line %d: %w", IgnoreFile, n+1, err)
		}
		if len(pats) == MaxIgnorePatterns {
			return nil, fmt.Errorf("%s has more than %d patterns", IgnoreFile, MaxIgnorePatterns)
		}
		pats = append(pats, line)
	}
	return pats, nil
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
	root, err := os.OpenRoot(f.dir)
	if err != nil {
		return "", err
	}
	defer root.Close()
	h := sha256.New()
	var n [8]byte
	for _, p := range files {
		if err := checkSourcePath(p); err != nil {
			return "", err
		}
		kind, digest, err := digestEntry(root, filepath.FromSlash(p))
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
// text (kind 2), mirroring KILN02's file kinds.  Everything goes through
// os.Root, so nothing outside the folder can be read.
func digestEntry(root *os.Root, name string) (byte, []byte, error) {
	fi, err := root.Lstat(name)
	if err != nil {
		return 0, nil, err
	}
	h := sha256.New()
	if fi.Mode()&fs.ModeSymlink != 0 {
		target, err := root.Readlink(name)
		if err != nil {
			return 0, nil, err
		}
		h.Write([]byte(filepath.ToSlash(target)))
		return 2, h.Sum(nil), nil
	}
	file, err := openRegularIn(root, filepath.ToSlash(name))
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
