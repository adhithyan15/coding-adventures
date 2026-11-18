//go:build !unix

package main

import "os"

// openTraceNoFollow is the portable fallback: no O_NOFOLLOW off unix.  The
// sandbox tests that use the diagnostic are linux-only; this only lets the
// portable unit tests build.  The caller's Stat still refuses a non-regular file.
func openTraceNoFollow(path string) (*os.File, error) { return os.Open(path) }
