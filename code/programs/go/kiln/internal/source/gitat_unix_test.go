//go:build unix

package source

import (
	"os"
	"path/filepath"
	"sort"
	"strings"
	"testing"
)

// NewGitAt runs exactly the program it was given, in "/", with exactly the
// environment it was given plus the fixed hardening, and never looks "git" up
// (KILN04 4.8, H1 and M3). The proof is a fake git that dumps what it sees while
// the process environment is full of loader variables and a planted git sits on
// PATH.
func TestNewGitAtUsesAnAllowlistEnvironment(t *testing.T) {
	d, err := filepath.EvalSymlinks(t.TempDir())
	if err != nil {
		t.Fatal(err)
	}
	planted := filepath.Join(d, "planted")
	marker := filepath.Join(d, "ran-planted")
	os.MkdirAll(planted, 0o755)
	os.WriteFile(filepath.Join(planted, "git"), []byte("#!/bin/sh\ntouch "+marker+"\n"), 0o755)
	pinned := filepath.Join(d, "pinned-git")
	os.WriteFile(pinned, []byte("#!/bin/sh\npwd -P\n/usr/bin/env\n"), 0o755)
	t.Setenv("PATH", planted+":/usr/bin:/bin")
	for k, v := range map[string]string{
		"LD_PRELOAD": "/x.so", "LD_LIBRARY_PATH": ":", "LD_AUDIT": "/a.so", "GCONV_PATH": "/g", "LOCPATH": "/l",
		"TZDIR": "/t", "GIT_DIR": "/elsewhere", "GIT_EXEC_PATH": "/e", "SECRET_THING": "s", "HOME": "/process-home",
	} {
		t.Setenv(k, v)
	}

	out, err := NewGitAt(d, pinned, []string{"HOME=/h", "PATH=/safe/bin", "XDG_CONFIG_HOME=/x"}).Output("status")
	if err != nil {
		t.Fatal(err)
	}
	lines := strings.Split(strings.TrimSpace(string(out)), "\n")
	if lines[0] != "/" {
		t.Fatalf("git started in %q, not in /", lines[0])
	}
	var names []string
	for _, l := range lines[1:] {
		name, _, _ := strings.Cut(l, "=")
		switch name { // added by the shell that runs the fake git, not by us
		case "PWD", "OLDPWD", "SHLVL", "_":
			continue
		}
		names = append(names, l)
	}
	sort.Strings(names)
	want := []string{
		"GIT_CONFIG_GLOBAL=" + os.DevNull, "GIT_CONFIG_NOSYSTEM=1", "GIT_CONFIG_SYSTEM=" + os.DevNull, "GIT_NO_LAZY_FETCH=1",
		"GIT_NO_REPLACE_OBJECTS=1", "GIT_OPTIONAL_LOCKS=0", "GIT_TERMINAL_PROMPT=0", "HOME=/h", "LC_ALL=C", "PATH=/safe/bin", "XDG_CONFIG_HOME=/x",
	}
	if strings.Join(names, "\n") != strings.Join(want, "\n") {
		t.Fatalf("the git child's environment is not exactly the allowlist:\n got %q\nwant %q", names, want)
	}
	if _, err := os.Stat(marker); err == nil {
		t.Fatal("the planted git ran")
	}
	// The default constructor still resolves through PATH (control).
	NewGit(d).Output("status")
	if _, err := os.Stat(marker); err != nil {
		t.Fatalf("control: the default constructor should have used PATH: %v", err)
	}
}
