package main

// Tests for cow-file selection — the path-traversal fix for issue #12169.
//
// Each test builds a throwaway tree like this:
//
//	<tmp>/
//	  secret.cow          ← must NEVER be readable via -f
//	  cows/
//	    default.cow       ← the fallback
//	    tux.cow           ← a legitimate cow
//	    nested/inner.cow  ← exists, but "nested/inner" is not a bare name
//
// and asserts that every hostile spelling of "secret" draws the default cow.

import (
	"os"
	"path/filepath"
	"runtime"
	"testing"
)

func writeCow(t *testing.T, path string, body string) {
	t.Helper()
	if err := os.MkdirAll(filepath.Dir(path), 0o755); err != nil {
		t.Fatal(err)
	}
	if err := os.WriteFile(path, []byte("$the_cow = <<EOC;\n"+body+"\nEOC\n"), 0o644); err != nil {
		t.Fatal(err)
	}
}

// newCowTree returns (base, cowsDir) for a freshly built fixture tree.
func newCowTree(t *testing.T) (string, string) {
	t.Helper()
	base := t.TempDir()
	cows := filepath.Join(base, "cows")
	writeCow(t, filepath.Join(cows, "default.cow"), "DEFAULT")
	writeCow(t, filepath.Join(cows, "tux.cow"), "TUX")
	writeCow(t, filepath.Join(cows, "nested", "inner.cow"), "NESTED")
	writeCow(t, filepath.Join(base, "secret.cow"), "SECRET")
	return base, cows
}

func TestIsSafeCowName(t *testing.T) {
	for _, ok := range []string{"default", "tux", "bud-frogs", "three_eyes", "v2", "dragon.and.cow"} {
		if !isSafeCowName(ok) {
			t.Errorf("isSafeCowName(%q) = false, want true", ok)
		}
	}
	for _, bad := range []string{
		"", "..", "../secret", `..\secret`, "a/b", `a\b`, "/etc/passwd",
		"C:secret", `C:\Windows\win`, "tux\x00", "..%2Fsecret", "%2e%2e/secret",
	} {
		if isSafeCowName(bad) {
			t.Errorf("isSafeCowName(%q) = true, want false", bad)
		}
	}
}

func TestNormalCowNamesStillLoad(t *testing.T) {
	_, cows := newCowTree(t)
	if got := loadCow("tux", cows); got != "TUX\n" {
		t.Errorf("loadCow(tux) = %q", got)
	}
	if got := loadCow("default", cows); got != "DEFAULT\n" {
		t.Errorf("loadCow(default) = %q", got)
	}
}

func TestUnknownCowFallsBackToDefault(t *testing.T) {
	_, cows := newCowTree(t)
	if got := loadCow("does-not-exist", cows); got != "DEFAULT\n" {
		t.Errorf("loadCow(does-not-exist) = %q", got)
	}
}

func TestRelativeTraversalFallsBackToDefault(t *testing.T) {
	_, cows := newCowTree(t)
	// Sanity: the target really is reachable by naive joining.
	if _, err := os.Stat(filepath.Join(cows, "..", "secret.cow")); err != nil {
		t.Fatal(err)
	}
	for _, hostile := range []string{"../secret", `..\secret`, "./../secret", "tux/../../secret"} {
		if got := loadCow(hostile, cows); got != "DEFAULT\n" {
			t.Errorf("loadCow(%q) = %q, want default", hostile, got)
		}
	}
}

func TestAbsolutePathFallsBackToDefault(t *testing.T) {
	base, cows := newCowTree(t)
	if got := loadCow(filepath.Join(base, "secret"), cows); got != "DEFAULT\n" {
		t.Errorf("absolute path leaked: %q", got)
	}
}

func TestNestedNamesAreRefused(t *testing.T) {
	_, cows := newCowTree(t)
	for _, nested := range []string{"nested/inner", `nested\inner`} {
		if got := loadCow(nested, cows); got != "DEFAULT\n" {
			t.Errorf("loadCow(%q) = %q, want default", nested, got)
		}
	}
}

func TestEncodedAndNulNamesFallBackToDefault(t *testing.T) {
	_, cows := newCowTree(t)
	for _, hostile := range []string{"..%2Fsecret", "%2e%2e%2fsecret", "%2E%2E%5Csecret", "tux\x00../secret"} {
		if got := loadCow(hostile, cows); got != "DEFAULT\n" {
			t.Errorf("loadCow(%q) = %q, want default", hostile, got)
		}
	}
}

// Layer 2 in action: "evil" is syntactically fine, but the file it names is a
// symlink leading out of the cows directory.
func TestSymlinkEscapeFallsBackToDefault(t *testing.T) {
	if runtime.GOOS == "windows" {
		t.Skip("creating symlinks needs elevated privileges on Windows")
	}
	base, cows := newCowTree(t)
	if err := os.Symlink(filepath.Join(base, "secret.cow"), filepath.Join(cows, "evil.cow")); err != nil {
		t.Fatal(err)
	}
	if got := loadCow("evil", cows); got != "DEFAULT\n" {
		t.Errorf("symlink escape leaked: %q", got)
	}
}

// The real repository cows must keep working through the hardened path.
func TestRepositoryCowsLoad(t *testing.T) {
	cows := filepath.Join("..", "..", "..", "specs", "cows")
	if got := loadCow("tux", cows); got == loadCow("default", cows) {
		t.Errorf("tux rendered as the default cow")
	}
}
