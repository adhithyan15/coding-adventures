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
//	root fd ──openat("demo", O_NOFOLLOW)──▶ demo fd ──openat("Example.class",
//	                                                   O_CREAT|O_NOFOLLOW)──▶ file

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

	fileFD, openErr := unix.Openat(
		currentFD,
		filepath.Base(relativePath),
		unix.O_WRONLY|unix.O_CREAT|unix.O_TRUNC|unix.O_NOFOLLOW|unix.O_CLOEXEC,
		0o644,
	)
	if openErr != nil {
		return "", backendError("class_filename points at a symlinked or invalid output file")
	}
	if err := writeAll(fileFD, artifact.ClassBytes); err != nil {
		_ = unix.Close(fileFD)
		return "", backendError("failed to write class file: %v", err)
	}
	if err := unix.Close(fileFD); err != nil {
		return "", backendError("failed to close class file: %v", err)
	}
	target := filepath.Join(absoluteRoot, relativePath)
	return target, nil
}

// isReparsePoint reports Windows reparse points.  Unix has none; a symlink
// shows up as os.ModeSymlink, which the path-check walk tests separately.
func isReparsePoint(os.FileInfo) bool {
	return false
}

func secureOpenRoot(root string) (int, string, error) {
	if info, err := os.Lstat(root); err == nil {
		if info.Mode()&os.ModeSymlink != 0 || !info.IsDir() {
			return -1, "", backendError("symlinked or invalid path component")
		}
		canonicalRoot, err := filepath.EvalSymlinks(root)
		if err != nil {
			return -1, "", backendError("failed to resolve output root: %v", err)
		}
		rootFD, err := unix.Open(canonicalRoot, unix.O_RDONLY|unix.O_DIRECTORY|unix.O_NOFOLLOW|unix.O_CLOEXEC, 0)
		if err != nil {
			return -1, "", backendError("failed to open output root: %v", err)
		}
		return rootFD, canonicalRoot, nil
	} else if !os.IsNotExist(err) {
		return -1, "", backendError("failed to inspect output root: %v", err)
	}

	absoluteRoot, err := filepath.Abs(root)
	if err != nil {
		return -1, "", backendError("failed to resolve output root: %v", err)
	}
	absoluteRoot = filepath.Clean(absoluteRoot)
	currentFD, err := unix.Open(string(filepath.Separator), unix.O_RDONLY|unix.O_DIRECTORY|unix.O_NOFOLLOW|unix.O_CLOEXEC, 0)
	if err != nil {
		return -1, "", backendError("failed to open filesystem root: %v", err)
	}
	parts := strings.Split(strings.TrimPrefix(filepath.ToSlash(absoluteRoot), "/"), "/")
	if len(parts) == 1 && parts[0] == "" {
		return currentFD, absoluteRoot, nil
	}
	for _, component := range parts {
		if component == "" {
			continue
		}
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
