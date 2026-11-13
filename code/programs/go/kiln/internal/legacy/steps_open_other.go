//go:build !unix

package legacy

import "os"

// openNoFollow is a plain open where O_NOFOLLOW and O_NONBLOCK do not exist;
// ReadStepsFile has already refused a non-regular path with Lstat and checks
// the opened handle again.
func openNoFollow(path string) (*os.File, error) { return os.Open(path) }
