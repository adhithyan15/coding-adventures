//go:build !unix

package record

import "os"

// openNoFollow is a plain open where O_NOFOLLOW, O_NONBLOCK and O_NOCTTY do
// not exist. VerifyTree has already `lstat`ed the path and refuses anything
// that is not a regular file, and it checks the opened handle again; the
// recorder itself only runs on Linux (KILN04 stage 4), so this path exists to
// keep the package building and testable elsewhere.
func openNoFollow(path string) (*os.File, error) { return os.Open(path) }

// execMismatch: Windows has no executable bit, and Git on Windows records
// whatever `core.filemode` says, so there is nothing meaningful to compare.
func execMismatch(os.FileMode, string) string { return "" }
