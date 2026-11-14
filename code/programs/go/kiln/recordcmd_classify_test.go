package main

import (
	"bytes"
	"strings"
	"testing"
)

// `kiln record classify-action` is the child `kiln record step` starts at an
// action's end (internal/record/classify_child.go). It is internal, so its
// command line is strict and it does nothing without a sealed record directory.
func TestRecordClassifyActionCommandLine(t *testing.T) {
	old := recordPlatformOK
	defer func() { recordPlatformOK = old }()
	run := func(args ...string) (int, string) {
		var out, errb bytes.Buffer
		code := runRecord(append([]string{"classify-action"}, args...), &out, &errb)
		return code, errb.String()
	}
	recordPlatformOK = func() bool { return true }
	for name, args := range map[string][]string{
		"no arguments":     nil,
		"a relative dir":   {"--dir", "rec", "--action", "a", "--step", "0"},
		"no action":        {"--dir", "/x", "--step", "0"},
		"no step":          {"--dir", "/x", "--action", "a"},
		"a negative limit": {"--dir", "/x", "--action", "a", "--step", "0", "--data-limit", "-1"},
		"an extra word":    {"--dir", "/x", "--action", "a", "--step", "0", "extra"},
		"an unknown flag":  {"--nope"},
	} {
		if code, e := run(args...); code != 2 {
			t.Errorf("%s: exit %d %q", name, code, e)
		}
	}
	// Well formed, but there is no sealed directory there: a failure (1), with
	// the reason, never a record.
	dir := t.TempDir()
	if code, e := run("--dir", dir, "--action", "code/p", "--step", "0"); code != 1 || !strings.Contains(e, "classify-action") {
		t.Fatalf("%d %q", code, e)
	}
	// Off linux/amd64 it refuses before touching anything.
	recordPlatformOK = func() bool { return false }
	if code, e := run("--dir", dir, "--action", "code/p", "--step", "0"); code != 3 || !strings.Contains(e, "linux/amd64") {
		t.Fatalf("%d %q", code, e)
	}
}
