//go:build !unix

package key

import "os"

// openNoBlock is a plain open where there is no O_NONBLOCK (Windows has no
// FIFOs to hang on); the regular-file check still applies.
func openNoBlock(path string) (*os.File, error) { return os.Open(path) }
