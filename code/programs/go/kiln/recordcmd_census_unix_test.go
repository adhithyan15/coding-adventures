//go:build unix

package main

import (
	"os"
	"path/filepath"
	"strings"
	"testing"
)

// `kiln record census` is a diagnostic, not a recording step: its usage errors
// are 2, and a refusal (a directory that is not a record directory, an output
// that exists) is 1 and writes nothing.  The behaviour on a real record
// directory is tested in internal/record (census_report_unix_test.go) and, on a
// real recording, in TestSandboxRecordCollect.  Unix only: POSIX paths.
func TestRecordCensusCommand(t *testing.T) {
	recordHooks(t)
	d := t.TempDir()
	out := filepath.Join(d, "o.json")
	for name, args := range map[string][]string{
		"nothing":        {"census"},
		"no out":         {"census", "--dir", d},
		"no dir":         {"census", "--out", out},
		"extra argument": {"census", "--dir", d, "--out", out, "stray"},
		"unknown flag":   {"census", "--dir", d, "--out", out, "--nope"},
	} {
		if code, o, e := runRec(args...); code != 2 || o != "" || e == "" {
			t.Errorf("%s: %d %q %q", name, code, o, e)
		}
	}
	// Refusals: exit 1, nothing written, and the message is ours.
	for name, args := range map[string][]string{
		"relative dir":     {"census", "--dir", "rec", "--out", out},
		"relative out":     {"census", "--dir", d, "--out", "o.json"},
		"not a record dir": {"census", "--dir", d, "--out", out},
	} {
		code, o, e := runRec(args...)
		if code != 1 || o != "" || !strings.Contains(e, "record census") {
			t.Errorf("%s: %d %q %q", name, code, o, e)
		}
	}
	if _, err := os.Stat(out); err == nil {
		t.Error("a refusal wrote the output")
	}
	// The dispatcher lists it.
	if _, _, e := runRec(); !strings.Contains(e, "census") {
		t.Errorf("usage does not name census: %q", e)
	}
}
