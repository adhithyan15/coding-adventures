//go:build !unix

package record

import "errors"

// SerialLock is the serial lock of decision P13; unsupported here.
type SerialLock struct{}

// LockSerial refuses: there is no flock here.
func LockSerial(string) (*SerialLock, error) {
	return nil, errors.New("the serial lock is not supported on this platform")
}

// NextSeq refuses.
func (*SerialLock) NextSeq() (int, error) { return 0, errors.New("not supported on this platform") }

// OpenSeq refuses.
func (*SerialLock) OpenSeq(string) error { return errors.New("not supported on this platform") }

// Close does nothing.
func (*SerialLock) Close() {}

// LockDir: there is no flock here, and the recorder runs only on Linux; the
// refusal keeps the package building and fails closed.
func LockDir(string, bool) (func(), error) {
	return nil, errors.New("locking the record directory is not supported on this platform")
}
