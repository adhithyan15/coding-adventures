package main

import (
	"bytes"
	"os"
	"path/filepath"
	"strings"
	"testing"
)

const profile = `mode: set
m/pbt/a.go:1.1,2.2 8 1
m/pbt/a.go:3.1,4.2 2 0
m/prng/b.go:1.1,2.2 10 3
`

func TestGateReportsPerPackage(t *testing.T) {
	report, ok, err := gate(strings.NewReader(profile), 80)
	if err != nil || !ok {
		t.Fatalf("80%% threshold: ok=%v err=%v\n%s", ok, err, report)
	}
	if !strings.Contains(report, "80.0%  m/pbt") || !strings.Contains(report, "100.0%  m/prng") {
		t.Fatalf("report:\n%s", report)
	}
	report, ok, err = gate(strings.NewReader(profile), 90)
	if err != nil || ok || !strings.Contains(report, "BELOW") {
		t.Fatalf("90%% threshold must fail pbt: ok=%v err=%v\n%s", ok, err, report)
	}
}

func TestGateRefusesMalformedAndEmptyProfiles(t *testing.T) {
	for name, text := range map[string]string{
		"empty":        "",
		"mode only":    "mode: set\n",
		"garbage":      "mode: set\nnot a block\n",
		"bad numbers":  "mode: set\nm/p/a.go:1.1,2.2 x 1\n",
		"negative":     "mode: set\nm/p/a.go:1.1,2.2 -1 1\n",
		"missing part": "mode: set\nm/p/a.go:1.1,2.2 1\n",
	} {
		if _, ok, err := gate(strings.NewReader(text), 0); ok || err == nil {
			t.Errorf("%s: must be an error, ok=%v err=%v", name, ok, err)
		}
	}
}

func TestZeroStatementPackageCountsAsCovered(t *testing.T) {
	if _, ok, err := gate(strings.NewReader("mode: set\nm/p/a.go:1.1,2.2 0 0\n"), 100); err != nil || !ok {
		t.Fatalf("ok=%v err=%v", ok, err)
	}
}

func TestRunExitCodes(t *testing.T) {
	dir := t.TempDir()
	good := filepath.Join(dir, "good.out")
	if err := os.WriteFile(good, []byte(profile), 0o600); err != nil {
		t.Fatal(err)
	}
	var out, errb bytes.Buffer
	if code := run([]string{good, "50"}, &out, &errb); code != 0 {
		t.Fatalf("exit %d: %s", code, errb.String())
	}
	if code := run([]string{good, "99"}, &out, &errb); code != 1 {
		t.Fatalf("below threshold must exit 1, got %d", code)
	}
	for _, args := range [][]string{{}, {good}, {good, "x"}, {good, "101"}, {filepath.Join(dir, "missing.out"), "50"}} {
		if code := run(args, &out, &errb); code != 2 {
			t.Errorf("args %v: exit %d, want 2", args, code)
		}
	}
	bad := filepath.Join(dir, "bad.out")
	if err := os.WriteFile(bad, []byte("junk\n"), 0o600); err != nil {
		t.Fatal(err)
	}
	if code := run([]string{bad, "50"}, &out, &errb); code != 2 {
		t.Fatalf("malformed profile must exit 2, got %d", code)
	}
}
