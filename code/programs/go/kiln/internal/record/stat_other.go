//go:build !unix

package record

import "os"

// linkCount: Windows reports no link count through os.FileInfo. The recorder
// only ever runs on Linux, so this exists to keep the package building; the
// caller treats "unknown" as an error, never as "one".
func linkCount(os.FileInfo) (uint64, bool) { return 0, false }

// deviceOf: no st_dev here either (see linkCount).
func deviceOf(os.FileInfo) (uint64, bool) { return 0, false }

// searchable: unknown here, so assume it is (the caller then fails closed).
func searchable(string) bool { return true }

// ownerOf: no uid here.
func ownerOf(os.FileInfo) (uint32, bool) { return 0, false }
