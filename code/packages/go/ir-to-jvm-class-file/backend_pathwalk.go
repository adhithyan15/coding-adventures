package irtojvmclassfile

import (
	"crypto/rand"
	"encoding/hex"
	"errors"
	"io/fs"
	"os"
	"path/filepath"
	"strings"
)

// ── The path-check walk ────────────────────────────────────────────────────
//
// This is the writer Windows uses, where there is no openat(2).  It contains
// nothing Windows-specific except isReparsePoint (defined per platform), so it
// builds on every OS and the tests run it on POSIX too, through
// writeStrategyPathChecks.
//
// Step 1: find the deepest component of the output root that already exists.
// It must be a real directory: not a file, not a symlink, not a junction.
// Everything above it is canonicalised with EvalSymlinks, so an ordinary
// system link high in the tree (macOS's /var -> /private/var) is fine;
// everything below it is created by the walk, one component at a time.
//
//	root = C:\out\new\deeper     C:\out exists, new\ and deeper\ do not
//	canonicalRoot = C:\out       missingParts = ["new", "deeper"]
//
// Step 2: for each missing root part, then each package directory of the
// class (demo/ for demo/Example.class): Mkdir it, then Lstat it, and refuse
// anything that is not a plain directory.  Mkdir comes *first* so that an
// entry planted before we arrive is seen by the Lstat: Mkdir on an existing
// link or junction fails with "exists", and the Lstat then rejects it.
//
// Step 3: if the class file already exists it must be a plain file.  The new
// bytes are written to a fresh sibling opened with O_CREATE|O_EXCL, which
// fails rather than follow anything already at that name, and the sibling is
// then os.Rename'd over the target.  Rename (MoveFileEx on Windows, rename(2)
// on POSIX) replaces the directory entry itself and never follows a link at
// the destination.  Opening the target with O_TRUNC instead would follow a
// link swapped in after our Lstat and truncate whatever it points at.
//
// Residual race, accepted on Windows (as in the Rust and TypeScript ports):
// the walk catches every link that is in place when it looks, but not a
// directory swapped for a junction *between* its Lstat and the write.  Every
// check here is by path, and a path is resolved afresh on each call.  Closing
// that window needs handle-relative NtCreateFile traversal with
// FILE_OPEN_REPARSE_POINT; even holding an open handle on each ancestor does
// not pin it (see lessons.d: "A Windows directory handle does not make
// subsequent path-based writes safe from a directory swap").  On Unix the
// descriptor walk in backend_unix.go has no such window.

// errNotPlainDirectory marks a component that exists but is not a directory
// we are willing to write into.
var errNotPlainDirectory = errors.New("not a plain directory")

func writeClassFileViaPathChecks(artifact *JVMClassArtifact, root string, relativePath string) (string, error) {
	canonicalRoot, missingRootParts, err := locateExistingRoot(root)
	if err != nil {
		return "", err
	}

	current := canonicalRoot
	for _, component := range missingRootParts {
		current = filepath.Join(current, component)
		if err := ensurePlainDirectory(current); err != nil {
			if errors.Is(err, errNotPlainDirectory) {
				return "", backendError("symlinked or invalid path component")
			}
			return "", backendError("failed to create output root component %q: %v", component, err)
		}
	}

	for _, component := range strings.Split(filepath.Dir(relativePath), string(filepath.Separator)) {
		if component == "." || component == "" {
			continue
		}
		current = filepath.Join(current, component)
		if err := ensurePlainDirectory(current); err != nil {
			if errors.Is(err, errNotPlainDirectory) {
				return "", backendError("class_filename contains a symlinked or invalid directory component: %s", component)
			}
			return "", backendError("failed to create output directory %q: %v", component, err)
		}
	}

	name := filepath.Base(relativePath)
	target := filepath.Join(current, name)
	if info, err := os.Lstat(target); err == nil {
		if !isPlainFile(info) {
			return "", backendError("class_filename points at a symlinked or invalid output file")
		}
	} else if !errors.Is(err, fs.ErrNotExist) {
		return "", backendError("failed to inspect output file: %v", err)
	}

	if err := replaceFileAtomically(current, name, artifact.ClassBytes); err != nil {
		return "", err
	}
	return target, nil
}

// locateExistingRoot performs step 1: it returns the canonical form of the
// deepest existing ancestor of root (root itself, if it exists) and the
// names below it that still have to be created, outermost first.
func locateExistingRoot(root string) (string, []string, error) {
	absoluteRoot, err := filepath.Abs(root)
	if err != nil {
		return "", nil, backendError("failed to resolve output root: %v", err)
	}

	current := absoluteRoot
	var missingReversed []string
	for {
		info, err := os.Lstat(current)
		if err == nil {
			if !isPlainDirectory(info) {
				return "", nil, backendError("symlinked or invalid path component")
			}
			canonicalRoot, err := filepath.EvalSymlinks(current)
			if err != nil {
				return "", nil, backendError("failed to resolve output root: %v", err)
			}
			missing := make([]string, 0, len(missingReversed))
			for i := len(missingReversed) - 1; i >= 0; i-- {
				missing = append(missing, missingReversed[i])
			}
			return canonicalRoot, missing, nil
		}
		if !errors.Is(err, fs.ErrNotExist) {
			return "", nil, backendError("failed to inspect output root: %v", err)
		}
		parent := filepath.Dir(current)
		if parent == current {
			// Even the volume root is missing (e.g. an unmapped drive letter).
			return "", nil, backendError("failed to inspect output root: %v", err)
		}
		missingReversed = append(missingReversed, filepath.Base(current))
		current = parent
	}
}

// ensurePlainDirectory performs step 2 for one component: create it if it is
// missing, then confirm that what is there now is a plain directory.  It
// returns errNotPlainDirectory for a link, junction or file, and the Mkdir
// error itself for anything else (permission denied, read-only volume, ...).
func ensurePlainDirectory(path string) error {
	if err := os.Mkdir(path, 0o755); err != nil && !errors.Is(err, fs.ErrExist) {
		return err
	}
	info, err := os.Lstat(path)
	if err != nil {
		return err
	}
	if !isPlainDirectory(info) {
		return errNotPlainDirectory
	}
	return nil
}

// replaceFileAtomically performs step 3: write data to an O_EXCL temporary
// sibling of dir/name, then rename it over dir/name.  On any failure the
// temporary file is removed, so a failed write leaves the classpath as it was.
func replaceFileAtomically(dir string, name string, data []byte) error {
	tempName, err := temporaryClassFileName(name)
	if err != nil {
		return err
	}
	tempPath := filepath.Join(dir, tempName)

	// O_EXCL with O_CREATE fails if anything — file, directory, dangling
	// link — already has this name; it never follows a link.  Go opens files
	// close-on-exec (non-inheritable on Windows) without being asked.
	file, err := os.OpenFile(tempPath, os.O_WRONLY|os.O_CREATE|os.O_EXCL, 0o644)
	if err != nil {
		return backendError("failed to create temporary class file: %v", err)
	}
	_, writeErr := file.Write(data)
	closeErr := file.Close()
	if writeErr != nil || closeErr != nil {
		_ = os.Remove(tempPath)
		return backendError("failed to write class file: %v", errors.Join(writeErr, closeErr))
	}
	if err := os.Rename(tempPath, filepath.Join(dir, name)); err != nil {
		_ = os.Remove(tempPath)
		return backendError("failed to move class file into place: %v", err)
	}
	return nil
}

// temporaryClassFileName names the temp sibling both walks write before the
// final rename: ".Example.class.3f9c0a1b2d4e5f60.tmp".  The leading dot keeps it
// out of casual listings, and 64 random bits make a collision with anything
// already in the directory (planted or left over) vanishingly unlikely — and
// harmless if it happens, since the temp file is opened with O_EXCL.
func temporaryClassFileName(name string) (string, error) {
	var nonce [8]byte
	if _, err := rand.Read(nonce[:]); err != nil {
		return "", backendError("failed to name temporary class file: %v", err)
	}
	return "." + name + "." + hex.EncodeToString(nonce[:]) + ".tmp", nil
}

// isPlainDirectory: a directory that is neither a symlink nor a reparse point.
//
//	entry                     Mode()            reparse bit  plain dir?
//	────────────────────────  ────────────────  ───────────  ──────────
//	ordinary directory        ModeDir           no           yes
//	symlink to a directory    ModeSymlink       Windows: yes no
//	Windows junction          ModeIrregular*    yes          no
//	regular file              0                 no           no
//
//	* Go 1.23+; ModeSymlink with GODEBUG=winsymlink=0.
func isPlainDirectory(info os.FileInfo) bool {
	return info.Mode()&os.ModeSymlink == 0 && info.IsDir() && !isReparsePoint(info)
}

// isPlainFile: a regular file that is neither a symlink nor a reparse point.
func isPlainFile(info os.FileInfo) bool {
	return info.Mode().IsRegular() && !isReparsePoint(info)
}
