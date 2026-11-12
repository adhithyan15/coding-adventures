//go:build !unix

package ledger

import "os"

// openRead is a plain open where O_NONBLOCK and O_NOFOLLOW do not exist.  The
// directory listing has already refused symlinks and the regular-file check on
// the descriptor still applies.
func openRead(path string) (*os.File, error) { return os.Open(path) }
