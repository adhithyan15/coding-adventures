//go:build unix

package record

// The live, no-follow disk view (KILN04 section 4.8, decision P13)
// ================================================================
//
// `traced.Classify` resolves every path a BUILD touched against "the disk" with
// three questions: what is at this path (lstat), where does this link point
// (readlink), and what are the first bytes of this file (for `#!` lines and
// ELF headers). Until P13 the disk was read after the whole build; now it is
// read at the end of one action, by the step that ran its last command, so this
// is a thin, careful window onto the real file system:
//
//	Lstat     os.Lstat, never following the last component; the kind is
//	          file, link, directory, or "other" (FIFO, socket, device); the
//	          change time comes from the inode (an image-root check needs it)
//	Readlink  os.Readlink
//	ReadHead  the only call that OPENS anything, and it opens a path only after
//	          lstat said it is a regular file, then
//	            1  O_NOFOLLOW|O_NONBLOCK|O_NOCTTY: a link in the last component
//	               is refused, a FIFO cannot hang the open, a terminal cannot
//	               become ours
//	            2  fstat on the handle: still a regular file, and the very inode
//	               lstat saw (a swap between the two is caught)
//	            3  the kernel's own name for the open handle (/proc/self/fd/N on
//	               Linux) equals the path that was asked for. O_NOFOLLOW guards
//	               only the LAST component; if any directory above it was a
//	               link, the kernel's name is the resolved path, which differs,
//	               and the read is refused
//	            4  at most 64 KiB are read
//
// "Never follows a symlink" therefore holds for every component, not just the
// last, and a non-regular file is never opened.

import (
	"errors"
	"fmt"
	"io"
	"os"
	"path/filepath"

	"github.com/adhithyan15/coding-adventures/code/programs/go/kiln/internal/traced"
)

// maxLiveHead is the most ReadHead reads: traced's headLimit.
const maxLiveHead = 64 << 10

// LiveFS implements traced.FS over the real file system.
type LiveFS struct{}

// kindOfMode maps a file mode to the classifier's node kind.
func kindOfMode(m os.FileMode) traced.NodeKind {
	switch {
	case m&os.ModeSymlink != 0:
		return traced.KindLink
	case m.IsDir():
		return traced.KindDir
	case m.IsRegular():
		return traced.KindFile
	}
	return traced.KindOther
}

// Lstat implements traced.FS.
func (LiveFS) Lstat(p string) (traced.FileInfo, error) {
	st, err := os.Lstat(p)
	if err != nil {
		return traced.FileInfo{}, err
	}
	return traced.FileInfo{Kind: kindOfMode(st.Mode()), Ctime: ctimeOf(st)}, nil
}

// Readlink implements traced.FS.
func (LiveFS) Readlink(p string) (string, error) { return os.Readlink(p) }

// ReadHead implements traced.FS: up to min(n, 64 KiB) leading bytes of the
// regular file at p, opened as the file comment says.
func (LiveFS) ReadHead(p string, n int) ([]byte, error) {
	if n < 0 || n > maxLiveHead {
		n = maxLiveHead
	}
	f, _, err := openRegularChecked(p)
	if err != nil {
		return nil, err
	}
	defer f.Close()
	buf := make([]byte, n)
	got, err := io.ReadFull(f, buf)
	if err != nil && !errors.Is(err, io.ErrUnexpectedEOF) && !errors.Is(err, io.EOF) {
		return nil, err
	}
	return buf[:got], nil
}

// openRegularChecked opens p for reading under steps 1 to 3 of the file
// comment, and returns the handle and its fstat.
func openRegularChecked(p string) (*os.File, os.FileInfo, error) {
	if !filepath.IsAbs(p) || filepath.Clean(p) != p {
		return nil, nil, fmt.Errorf("live read: %q is not an absolute, clean path", printable(p))
	}
	lst, err := os.Lstat(p)
	if err != nil {
		return nil, nil, err
	}
	if !lst.Mode().IsRegular() {
		return nil, nil, fmt.Errorf("live read: %q is not a regular file", printable(p))
	}
	f, err := openNoFollow(p)
	if err != nil {
		return nil, nil, err
	}
	st, err := f.Stat()
	if err != nil {
		f.Close()
		return nil, nil, err
	}
	if !st.Mode().IsRegular() || !os.SameFile(lst, st) {
		f.Close()
		return nil, nil, fmt.Errorf("live read: %q changed between lstat and open", printable(p))
	}
	got, err := fdPath(f)
	if err != nil {
		f.Close()
		return nil, nil, fmt.Errorf("live read: the open file's name: %v", err)
	}
	if got != p {
		f.Close()
		return nil, nil, fmt.Errorf("live read: %q opened as %q (a link in the path?)", printable(p), printable(got))
	}
	return f, st, nil
}

// readVerifiedHead reads the tracked file at p through openRegularChecked and
// returns its first keep bytes, but only when the WHOLE file, hashed as a Git
// blob, equals id. A file over maxSize, one that shrinks or grows while it is
// read, and one that hashes differently are errors.
func readVerifiedHead(p, id string, maxSize int64, keep int) ([]byte, error) {
	h, err := newHasher(id)
	if err != nil {
		return nil, err
	}
	f, st, err := openRegularChecked(p)
	if err != nil {
		return nil, err
	}
	defer f.Close()
	if st.Size() > maxSize {
		return nil, fmt.Errorf("live read: %q is over %d bytes", printable(p), maxSize)
	}
	cw := &captureWriter{limit: keep}
	if why := hashBody(h, io.TeeReader(f, cw), st.Size()); why != "" {
		return nil, fmt.Errorf("live read: %q %s", printable(p), why)
	}
	if got := fmt.Sprintf("%x", h.Sum(nil)); got != id {
		return nil, fmt.Errorf("live read: %q is not the committed content (blob %s)", printable(p), short(id))
	}
	return cw.buf, nil
}

var _ traced.FS = LiveFS{}
