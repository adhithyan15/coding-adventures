//go:build unix

package ledger

import "os"

// syncDir flushes directory metadata so a just-renamed segment survives a
// crash.  A failure is returned: the caller then knows the record may not be
// durable and reports it rather than pretending success.
func syncDir(dir string) error {
	d, err := os.Open(dir)
	if err != nil {
		return err
	}
	if err := d.Sync(); err != nil {
		_ = d.Close()
		return err
	}
	return d.Close()
}
