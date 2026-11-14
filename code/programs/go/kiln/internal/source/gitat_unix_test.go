//go:build unix

package source

import (
	"os"
	"path/filepath"
	"strings"
	"testing"
)

// NewGitAt runs exactly the program it was given, with exactly the PATH it was
// given, and never looks "git" up: a `git` planted on the process PATH is not
// run (KILN04 4.8, H1).
func TestNewGitAtNeverUsesTheProcessPath(t *testing.T) {
	d, err := filepath.EvalSymlinks(t.TempDir())
	if err != nil {
		t.Fatal(err)
	}
	planted := filepath.Join(d, "planted")
	marker := filepath.Join(d, "ran-planted")
	os.MkdirAll(planted, 0o755)
	os.WriteFile(filepath.Join(planted, "git"), []byte("#!/bin/sh\ntouch "+marker+"\n"), 0o755)
	pinned := filepath.Join(d, "pinned-git")
	os.WriteFile(pinned, []byte("#!/bin/sh\necho \"PATH=$PATH\"\n"), 0o755)
	t.Setenv("PATH", planted+":/usr/bin:/bin")

	out, err := NewGitAt(d, pinned, "/safe/bin").Output("status")
	if err != nil || !strings.Contains(string(out), "PATH=/safe/bin\n") || strings.Count(string(out), "PATH=") != 1 {
		t.Fatalf("%q %v", out, err)
	}
	if _, err := os.Stat(marker); err == nil {
		t.Fatal("the planted git ran")
	}
	// The default still resolves through PATH (and so would run the planted one).
	NewGit(d).Output("status")
	if _, err := os.Stat(marker); err != nil {
		t.Fatalf("control: the default constructor should have used PATH: %v", err)
	}
}
