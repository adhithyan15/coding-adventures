//go:build !unix

package record

import "errors"

// LockDir: there is no flock here, and the recorder runs only on Linux; the
// refusal keeps the package building and fails closed.
func LockDir(string, bool) (func(), error) {
	return nil, errors.New("locking the record directory is not supported on this platform")
}
