//go:build unix && !linux && !darwin

package record

import (
	"errors"
	"os"
	"time"
)

// Other Unixes have neither /proc/self/fd nor a portable F_GETPATH here, so the
// check that no directory above a file is a link cannot be made: every live
// read refuses (fails closed). The recorder runs on Linux.
func fdPath(*os.File) (string, error) {
	return "", errors.New("no way to name an open file on this platform")
}

// ctimeOf: unknown, which the classifier treats as a failure, never as "old".
func ctimeOf(os.FileInfo) time.Time { return time.Time{} }
