package main

import (
	"os"
	"path/filepath"
	"runtime"
	"strings"
	"testing"
)

// TestLogTraceFileRefusesWhatIsNotARegularFile: the diagnostic opens once, never
// follows a link, never waits on a FIFO, and prints only the QUOTED base name
// (a hostile name must not forge a log line).
func TestLogTraceFileRefusesWhatIsNotARegularFile(t *testing.T) {
	dir := t.TempDir()
	hostile := "::error::x\x01\x1b[31m.trace"
	cases := map[string]func(p string) bool{
		"a directory": func(p string) bool { return os.Mkdir(p, 0o700) == nil },
	}
	if runtime.GOOS != "windows" {
		cases["a symlink"] = func(p string) bool {
			target := filepath.Join(dir, "real")
			return os.WriteFile(target, []byte("1  exit_group(0) = ?\n"), 0o600) == nil && os.Symlink(target, p) == nil
		}
	}
	for name, mk := range cases {
		p := filepath.Join(dir, strings.ReplaceAll(name, " ", "_")+hostile)
		if !mk(p) {
			t.Skipf("cannot create %s here", name)
		}
		rec := &recordingTB{TB: t}
		logTraceFile(rec, p)
		got := strings.Join(rec.logs, "\n")
		if !strings.Contains(got, "cannot open") && !strings.Contains(got, "not a regular file") {
			t.Errorf("%s: not refused: %q", name, got)
		}
		if strings.ContainsAny(got, "\x01\x1b") || strings.Contains(got, "Diagnose") || strings.Contains(got, "old scanner") {
			t.Errorf("%s: raw name or a diagnosis printed: %q", name, got)
		}
		if !strings.Contains(got, `\x01`) {
			t.Errorf("%s: the name is not quoted: %q", name, got)
		}
	}
}

func TestLogTraceFileSaysWhenTheReadIsTruncated(t *testing.T) {
	p := filepath.Join(t.TempDir(), "big.trace")
	line := "1  exit_group(0) = ?\n"
	big := strings.Repeat(line, maxDiagnoseRead/len(line)+10)
	if err := os.WriteFile(p, []byte(big), 0o600); err != nil {
		t.Fatal(err)
	}
	rec := &recordingTB{TB: t}
	logTraceFile(rec, p)
	if got := strings.Join(rec.logs, "\n"); !strings.Contains(got, "read truncated at 8 MiB") {
		t.Errorf("no truncation note: %.300q", got)
	}
}
