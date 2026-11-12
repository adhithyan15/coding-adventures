//go:build !unix

package ledger

// syncDir is a no-op where a directory handle cannot be fsynced (Windows
// refuses it).  The rename has happened; a crash can at worst lose a record,
// which is only a miss.
func syncDir(string) error { return nil }
