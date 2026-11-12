//go:build !unix

package host

import "os"

// openEvent is a plain open where O_NOFOLLOW and O_NONBLOCK do not exist;
// the fstat regular-file check still applies.
func openEvent(path string) (*os.File, error) { return os.Open(path) }
