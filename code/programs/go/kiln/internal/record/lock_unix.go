//go:build unix

package record

import (
	"fmt"
	"io"
	"os"
	"path/filepath"
	"strconv"
	"syscall"
)

// LockDir takes an flock(2) on the record directory without waiting: shared
// for `kiln record step` (any number of steps may run together), exclusive for
// `collect` (KILN04 4.8, P12; the collect side is stage 4 PR 2b-2b). The two
// exclude each other, so `collect` cannot start while any step is still
// running, an orphaned supervisor included: the lock belongs to the open file
// description, which only that supervisor holds (Go opens it close-on-exec, so
// strace and the step never inherit it), and it goes with the process.
//
// The returned function releases the lock.
func LockDir(dir string, exclusive bool) (func(), error) {
	f, err := os.Open(dir)
	if err != nil {
		return nil, fmt.Errorf("the record directory: %v", err)
	}
	how, what := syscall.LOCK_SH, "shared"
	if exclusive {
		how, what = syscall.LOCK_EX, "exclusive"
	}
	if err := syscall.Flock(int(f.Fd()), how|syscall.LOCK_NB); err != nil {
		f.Close()
		return nil, fmt.Errorf("the record directory cannot be locked %s (a step or `collect` holds it): %v", what, err)
	}
	return func() { f.Close() }, nil // closing the descriptor releases the lock
}

// SerialLock is the exclusive `traces/.serial` lock of decision P13, with the
// `traces/.seq` counter it protects. Recording is serial: the build tool runs
// with `-jobs 1`, and every `kiln record step` holds this lock from before its
// checks until it exits. It is taken WITHOUT waiting, so a second step does not
// queue behind the first (which would make the order depend on scheduling and
// hide the build tool's misconfiguration); it fails, leaves an `.overlap`
// marker and exits 125 (RunStep).
type SerialLock struct {
	lock *os.File
	seq  *os.File
}

// serialFile opens one of the lock directory's two small files, creating it
// 0600 when absent. It never follows a link, and requires a regular file this
// user owns that group and others cannot touch.
func serialFile(dir, name string) (*os.File, error) {
	p := filepath.Join(dir, "traces", name)
	f, err := os.OpenFile(p, os.O_RDWR|os.O_CREATE|syscall.O_NOFOLLOW|syscall.O_NONBLOCK|syscall.O_NOCTTY, 0o600)
	if err != nil {
		return nil, fmt.Errorf("traces/%s: %v", name, err)
	}
	st, err := f.Stat()
	if err != nil || !st.Mode().IsRegular() || st.Mode().Perm()&0o077 != 0 {
		f.Close()
		return nil, fmt.Errorf("traces/%s is not a private regular file", name)
	}
	if uid, ok := ownerOf(st); ok && uid != uint32(os.Geteuid()) {
		f.Close()
		return nil, fmt.Errorf("traces/%s is owned by user %d, not by this user", name, uid)
	}
	return f, nil
}

// LockSerial takes the exclusive, non-blocking lock. The traces directory is
// made when `prepare` did not. The lock belongs to the open file description, which is close-on-exec
// (strace and the step never hold it) and goes with the process.
func LockSerial(dir string) (*SerialLock, error) {
	// traces/ is made when absent and must be a real directory. Its owner and
	// mode are checked later, by createTrace, once pre/ has been verified: the
	// lock comes before every check, so it cannot also be the one to report them.
	traces := filepath.Join(dir, "traces")
	if err := os.Mkdir(traces, 0o700); err != nil && !os.IsExist(err) {
		return nil, fmt.Errorf("traces: %v", err)
	}
	if st, err := os.Lstat(traces); err != nil || st.Mode()&os.ModeSymlink != 0 || !st.IsDir() {
		return nil, fmt.Errorf("traces: %q is not a real directory", printable(traces))
	}
	lock, err := serialFile(dir, ".serial")
	if err != nil {
		return nil, err
	}
	if err := syscall.Flock(int(lock.Fd()), syscall.LOCK_EX|syscall.LOCK_NB); err != nil {
		lock.Close()
		return nil, fmt.Errorf("traces/.serial is held by another step (recording is serial: run the build with -jobs 1): %v", err)
	}
	return &SerialLock{lock: lock}, nil
}

// NextSeq returns the next sequence number and advances the counter: an atomic
// read, increment and write, safe because the exclusive lock is held. The file
// holds the NEXT number, as decimal digits and a newline (empty means 0).
// Anything else in it is an error, never silently reset: a counter that went
// back would let two steps share a number.
func (l *SerialLock) NextSeq() (int, error) {
	if l.seq == nil {
		return 0, fmt.Errorf("traces/.seq: the lock was not taken for this directory")
	}
	if _, err := l.seq.Seek(0, io.SeekStart); err != nil {
		return 0, err
	}
	data, err := io.ReadAll(io.LimitReader(l.seq, 64))
	if err != nil {
		return 0, fmt.Errorf("traces/.seq: %v", err)
	}
	n := 0
	if len(data) > 0 {
		s := string(data)
		if len(s) < 2 || s[len(s)-1] != '\n' {
			return 0, fmt.Errorf("traces/.seq is not a counter")
		}
		if n, err = canonicalInt("traces/.seq", s[:len(s)-1]); err != nil {
			return 0, err
		}
	}
	if err := l.seq.Truncate(0); err != nil {
		return 0, fmt.Errorf("traces/.seq: %v", err)
	}
	if _, err := l.seq.WriteAt([]byte(strconv.Itoa(n+1)+"\n"), 0); err != nil {
		return 0, fmt.Errorf("traces/.seq: %v", err)
	}
	if err := l.seq.Sync(); err != nil {
		return 0, fmt.Errorf("traces/.seq: %v", err)
	}
	return n, nil
}

// OpenSeq opens the counter file; called once the lock is held, by RunStep.
func (l *SerialLock) OpenSeq(dir string) error {
	f, err := serialFile(dir, ".seq")
	if err != nil {
		return err
	}
	l.seq = f
	return nil
}

// Close releases the lock (closing the descriptor does it).
func (l *SerialLock) Close() {
	if l.seq != nil {
		l.seq.Close()
	}
	l.lock.Close()
}
