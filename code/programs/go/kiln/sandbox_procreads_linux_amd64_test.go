package main

import (
	"bytes"
	"os"
	"path/filepath"
	"strconv"
	"strings"
	"testing"
)

// logProcReadsOnFailure registers a cleanup that, when the test failed, prints
// the trace lines that mention a /proc path.  The classifier's reason is
// deliberately sanitised ("trace-incomplete:proc-path" names no path), so a CI
// failure on a new runner image would otherwise not say which read voided the
// run.  Test-only output: at most 25 lines, each cut to 200 bytes and
// ASCII-quoted (no raw control characters, and no line that starts with a
// workflow-command marker).
func logProcReadsOnFailure(t *testing.T, recDir string) {
	t.Cleanup(func() {
		if !t.Failed() {
			return
		}
		shown := 0
		filepath.Walk(recDir, func(p string, info os.FileInfo, err error) error {
			if err != nil || info.IsDir() || !strings.HasSuffix(p, ".trace") {
				return nil
			}
			raw, err := os.ReadFile(p)
			if err != nil {
				return nil
			}
			for _, l := range bytes.Split(raw, []byte{'\n'}) {
				if shown >= 25 {
					return nil
				}
				if !bytes.Contains(l, []byte("/proc/")) {
					continue
				}
				if len(l) > 200 {
					l = l[:200]
				}
				t.Logf("PROC-READ %s/%s: %s", filepath.Base(filepath.Dir(p)), filepath.Base(p), strconv.QuoteToASCII(string(l)))
				shown++
			}
			return nil
		})
	})
}
