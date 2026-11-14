//go:build !unix

package record

import "os"

// linkCount: Windows reports no link count through os.FileInfo. The recorder
// only ever runs on Linux, so this exists to keep the package building; the
// caller treats "unknown" as an error, never as "one".
func linkCount(os.FileInfo) (uint64, bool) { return 0, false }

// deviceOf: no st_dev here either (see linkCount).
func deviceOf(os.FileInfo) (uint64, bool) { return 0, false }
