//go:build unix

package irtojvmclassfile

import (
	"errors"
	"os"
	"path/filepath"
	"strings"

	"golang.org/x/sys/unix"
)

// ── Unix: descriptor-relative traversal ────────────────────────────────────
//
// Every directory on the way down is opened with O_DIRECTORY|O_NOFOLLOW
// *relative to the descriptor of its parent* (openat(2)).  Once a directory is
// open, renaming it or swapping a symlink in at its old name cannot redirect
// what happens inside it: later lookups go through the descriptor, not the
// path.  That is what makes this walk race-free, and it is why the Windows
// build cannot share it (golang.org/x/sys/unix has no Windows half, and
// Windows has no openat).
//
//	existing root fd ──mkdirat+openat("new")──▶ new fd ──openat("demo")──▶ demo fd
//	                                                                         │
//	        openat(".Example.class.<hex>.tmp", O_CREAT|O_EXCL|O_NOFOLLOW) ◀──┘
//	        write, close
//	        renameat(".Example.class.<hex>.tmp" → "Example.class")
//
// Step 1 (secureOpenRoot): find the deepest part of the output root that
// already exists, exactly as the path-check walk does (locateExistingRoot in
// backend_pathwalk.go).  It must be a plain directory; the path *above* it is
// canonicalised with EvalSymlinks, so ordinary system links such as macOS's
// /var -> /private/var and /tmp -> /private/tmp are accepted.  Only the parts
// below it — the ones this call creates — are walked with mkdirat + openat.
// Walking from "/" with O_NOFOLLOW instead would refuse those system links and
// make every missing root under /tmp or $TMPDIR fail on macOS.
//
// Step 2: the package directories of the class (demo/ for demo/Example.class)
// get the same mkdirat + openat(O_NOFOLLOW) treatment.
//
// Step 3: the bytes never go through the target's name.  Opening
// "Example.class" with O_TRUNC — even with O_NOFOLLOW — would follow a *hard*
// link planted there and truncate the file it shares an inode with (any file
// the attacker can name and that lives on the same filesystem).  O_NOFOLLOW
// only covers symlinks; a hard link is indistinguishable from the file itself.
// So the bytes go into a fresh temp sibling created with O_CREAT|O_EXCL, which
// cannot open anything that already exists, and renameat(2) then swaps the
// directory entry.  Rename replaces the *name*: whatever inode the old name
// pointed at keeps its contents, and a symlink at the target is replaced, not
// followed.
//
//	before:  demo/Example.class ──▶ inode 42 ◀── /home/victim/notes.txt
//	after:   demo/Example.class ──▶ inode 77 (new class bytes)
//	                                inode 42 ◀── /home/victim/notes.txt (unchanged)

// writeClassFilePlatform is the writeStrategyAuto walk on Unix.
func writeClassFilePlatform(artifact *JVMClassArtifact, root string, relativePath string) (string, error) {
	rootFD, absoluteRoot, err := secureOpenRoot(root)
	if err != nil {
		return "", err
	}
	openFDs := []int{rootFD}
	defer func() {
		for i := len(openFDs) - 1; i >= 0; i-- {
			_ = unix.Close(openFDs[i])
		}
	}()

	currentFD := rootFD
	for _, component := range strings.Split(filepath.Dir(relativePath), string(filepath.Separator)) {
		if component == "." || component == "" {
			continue
		}
		if mkdirErr := unix.Mkdirat(currentFD, component, 0o755); mkdirErr != nil && !errors.Is(mkdirErr, unix.EEXIST) {
			return "", backendError("failed to create output directory %q: %v", component, mkdirErr)
		}
		nextFD, openErr := openDirectoryNoFollowAt(currentFD, component)
		if openErr != nil {
			return "", backendError("class_filename contains a symlinked or invalid directory component: %s", component)
		}
		openFDs = append(openFDs, nextFD)
		currentFD = nextFD
	}

	if err := replaceFileAtomicallyAt(currentFD, filepath.Base(relativePath), artifact.ClassBytes); err != nil {
		return "", err
	}
	// absoluteRoot is canonical and absolute, so the reported path is too,
	// even for outputDir "" (which used to come back as "demo/Example.class").
	return filepath.Join(absoluteRoot, relativePath), nil
}

// replaceFileAtomicallyAt performs step 3 inside the directory dirFD: refuse
// an existing target that is not a regular file, write data to an O_EXCL temp
// sibling, then renameat it over name.  On any failure after the temp file is
// created it is unlinked again, so a failed write leaves the classpath as it
// was.  It is the descriptor-relative twin of replaceFileAtomically.
func replaceFileAtomicallyAt(dirFD int, name string, data []byte) error {
	// The pre-check keeps the error for a directory or symlink at the target
	// the same as on the path-check walk.  It is not what makes the write
	// safe — renameat replaces whatever is there by then without following it
	// (and fails on a directory, which the cleanup below handles).
	var stat unix.Stat_t
	if err := unix.Fstatat(dirFD, name, &stat, unix.AT_SYMLINK_NOFOLLOW); err == nil {
		//	st_mode & S_IFMT   what it is        accepted?
		//	────────────────   ───────────────   ─────────
		//	S_IFREG            regular file      yes (replaced by rename)
		//	S_IFLNK            symlink           no
		//	S_IFDIR            directory         no
		//	anything else      fifo, socket, …   no
		if stat.Mode&unix.S_IFMT != unix.S_IFREG {
			return backendError("class_filename points at a symlinked or invalid output file")
		}
	} else if !errors.Is(err, unix.ENOENT) {
		return backendError("failed to inspect output file: %v", err)
	}

	tempName, err := temporaryClassFileName(name)
	if err != nil {
		return err
	}
	// O_EXCL with O_CREAT fails if anything — file, hard link, directory,
	// dangling symlink — already has this name, so the descriptor we get is
	// always a brand-new inode that only this call has seen.
	fileFD, err := unix.Openat(dirFD, tempName, unix.O_WRONLY|unix.O_CREAT|unix.O_EXCL|unix.O_NOFOLLOW|unix.O_CLOEXEC, 0o644)
	if err != nil {
		return backendError("failed to create temporary class file: %v", err)
	}
	writeErr := writeAll(fileFD, data)
	closeErr := unix.Close(fileFD)
	if writeErr != nil || closeErr != nil {
		_ = unix.Unlinkat(dirFD, tempName, 0)
		return backendError("failed to write class file: %v", errors.Join(writeErr, closeErr))
	}
	if err := unix.Renameat(dirFD, tempName, dirFD, name); err != nil {
		_ = unix.Unlinkat(dirFD, tempName, 0)
		return backendError("failed to move class file into place: %v", err)
	}
	return nil
}

// isReparsePoint reports Windows reparse points.  Unix has none; a symlink
// shows up as os.ModeSymlink, which the path-check walk tests separately.
func isReparsePoint(os.FileInfo) bool {
	return false
}

// secureOpenRoot performs step 1.  It returns a descriptor for the output root
// (creating any missing parts of it) and the root's canonical absolute path.
//
//	root = /var/folders/x/T/out/new     /var/folders/x/T exists, out/ and new/ do not
//	locateExistingRoot ─▶ /private/var/folders/x/T, ["out", "new"]
//	open(/private/var/folders/x/T) ─▶ mkdirat+openat "out" ─▶ mkdirat+openat "new"
//	returns (fd of new/, "/private/var/folders/x/T/out/new")
func secureOpenRoot(root string) (int, string, error) {
	canonicalRoot, missingRootParts, err := locateExistingRoot(root)
	if err != nil {
		return -1, "", err
	}
	// canonicalRoot has no symlinks left in it, so O_NOFOLLOW on its last
	// component only rejects a link swapped in since locateExistingRoot looked.
	currentFD, err := unix.Open(canonicalRoot, unix.O_RDONLY|unix.O_DIRECTORY|unix.O_NOFOLLOW|unix.O_CLOEXEC, 0)
	if err != nil {
		return -1, "", backendError("failed to open output root: %v", err)
	}
	absoluteRoot := canonicalRoot
	for _, component := range missingRootParts {
		if mkdirErr := unix.Mkdirat(currentFD, component, 0o755); mkdirErr != nil && !errors.Is(mkdirErr, unix.EEXIST) {
			_ = unix.Close(currentFD)
			return -1, "", backendError("failed to create output root component %q: %v", component, mkdirErr)
		}
		nextFD, openErr := openDirectoryNoFollowAt(currentFD, component)
		if openErr != nil {
			_ = unix.Close(currentFD)
			return -1, "", backendError("symlinked or invalid path component")
		}
		_ = unix.Close(currentFD)
		currentFD = nextFD
		absoluteRoot = filepath.Join(absoluteRoot, component)
	}
	return currentFD, absoluteRoot, nil
}

func openDirectoryNoFollowAt(parentFD int, component string) (int, error) {
	return unix.Openat(parentFD, component, unix.O_RDONLY|unix.O_DIRECTORY|unix.O_NOFOLLOW|unix.O_CLOEXEC, 0)
}

func writeAll(fd int, data []byte) error {
	for len(data) > 0 {
		written, err := unix.Write(fd, data)
		if err != nil {
			return err
		}
		data = data[written:]
	}
	return nil
}
