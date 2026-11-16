package warrantcorpus

import (
	"os"
	"path/filepath"
	"testing"
)

func TestRelativeInsideReturnsTheRelativeText(t *testing.T) {
	for _, c := range []struct{ root, child, flavor, want string }{
		{"/r", "/r/.hidden/x", flavorPosix, ".hidden/x"},
		{"/r", "/r", flavorPosix, ""},
		{`D:\r`, `d:/R/x/y`, flavorWin32, "x/y"},
		{`D:\repo`, `D:\repo\..foo`, flavorWin32, "..foo"},
		{`\\s\h`, `\\S\H\a`, flavorWin32, "a"},
		{`\r`, `\r\x`, flavorWin32, "x"},
		{`d:r`, `d:r\x`, flavorWin32, "x"},
		{`\\\r`, `\r\x`, flavorWin32, "x"},
	} {
		if got, ok := relativeInside(c.root, c.child, c.flavor); !ok || got != c.want {
			t.Errorf("%s in %s: got (%q, %v), want (%q, true)", c.child, c.root, got, ok, c.want)
		}
	}
	for _, c := range []struct{ root, child, flavor string }{
		{"/r", "/rr", flavorPosix},
		{`D:\r`, `C:\r\x`, flavorWin32},
		{`\\s\h`, `\\s`, flavorWin32},
		{`D:\r`, `\r\x`, flavorWin32},
		{"r", "../x", flavorPosix},
	} {
		if got, ok := relativeInside(c.root, c.child, c.flavor); ok {
			t.Errorf("%s in %s: accepted as %q", c.child, c.root, got)
		}
	}
	if got, ok := relativeInside("a", "a/../a/b", flavorPosix); !ok || got != "b" {
		t.Errorf("relative paths: (%q, %v)", got, ok)
	}
}

func TestHostFlavorMatchesTheHost(t *testing.T) {
	want := flavorPosix
	if filepath.Separator == '\\' {
		want = flavorWin32
	}
	if hostFlavor != want {
		t.Fatalf("hostFlavor = %q, want %q", hostFlavor, want)
	}
	root := t.TempDir()
	if got, ok := relativeInside(root, filepath.Join(root, "a", "b"), hostFlavor); !ok || got != "a/b" {
		t.Fatalf("(%q, %v)", got, ok)
	}
	if _, ok := relativeInside(root, filepath.Dir(root), hostFlavor); ok {
		t.Fatal("the parent is not inside")
	}
}

func TestResolveInside(t *testing.T) {
	root := t.TempDir()
	inner := filepath.Join(root, "sub", "..foo")
	if err := os.MkdirAll(inner, 0o755); err != nil {
		t.Fatal(err)
	}
	if rel, ok := resolveInside(root, inner, true); !ok || rel != "sub/..foo" {
		t.Fatalf("(%q, %v)", rel, ok)
	}
	if _, ok := resolveInside(root, root, true); ok {
		t.Fatal("strict refuses the root itself")
	}
	if rel, ok := resolveInside(root, root, false); !ok || rel != "" {
		t.Fatalf("non-strict root: (%q, %v)", rel, ok)
	}
	if _, ok := resolveInside(root, filepath.Join(root, "missing"), true); ok {
		t.Fatal("a missing path cannot be resolved")
	}
	if _, ok := resolveInside(filepath.Join(root, "missing"), inner, true); ok {
		t.Fatal("a missing root cannot be resolved")
	}
	outside := t.TempDir()
	if _, ok := resolveInside(root, outside, true); ok {
		t.Fatal("a sibling directory is outside")
	}
}

func TestResolveInsideFollowsSymlinks(t *testing.T) {
	root, outside := t.TempDir(), t.TempDir()
	link := filepath.Join(root, "escape")
	if err := os.Symlink(outside, link); err != nil {
		t.Skip("symlinks are not available: " + err.Error())
	}
	if _, ok := resolveInside(root, link, true); ok {
		t.Fatal("a symlink that leads outside the root must be refused")
	}
}

func TestRequireInsideRefuses(t *testing.T) {
	root := t.TempDir()
	wantCode(t, "path", func() { requireInside(root, t.TempDir(), "corpus directory") })
}

func TestStatIsRegular(t *testing.T) {
	dir := t.TempDir()
	file := filepath.Join(dir, "f")
	if err := os.WriteFile(file, nil, 0o644); err != nil {
		t.Fatal(err)
	}
	if !statIsRegular(file) || statIsRegular(dir) || statIsRegular(filepath.Join(dir, "none")) {
		t.Fatal("statIsRegular")
	}
}

// WD-100b: reserved device names and ASCII-only win32 folding.

func TestRefusedComponent(t *testing.T) {
	for _, name := range []string{"CON", "con", "CON.txt", "NUL.", "nul ", "COM1", "com9.log", "LPT1", "com1.tar.gz", "x."} {
		if !refusedComponent(name) {
			t.Errorf("%q must be refused", name)
		}
	}
	for _, name := range []string{"console.txt", "COM10", "LPT0", "COM0", "CONx", "xCON", ".hidden", "a.CON.b", "\u017fon", "com\u0661"} {
		if refusedComponent(name) {
			t.Errorf("%q is an ordinary name", name)
		}
	}
}

func TestAsciiLowerFoldsOnlyAToZ(t *testing.T) {
	if got := asciiLower("Repo-\u00c9"); got != "repo-\u00c9" {
		t.Errorf("got %q", got)
	}
	for _, text := range []string{"", "abc", "\u017f", "\u212a", "\u0130", "\u03a3", "\u03c2"} {
		if got := asciiLower(text); got != text {
			t.Errorf("asciiLower(%q) = %q, want it unchanged", text, got)
		}
	}
}

func TestWin32RulesThroughTheHostHelper(t *testing.T) {
	if rel, ok := relativeInside(`D:\repo`, `d:\REPO\x`, flavorWin32); !ok || rel != "x" {
		t.Errorf("ASCII case pair: %q %v", rel, ok)
	}
	for _, child := range []string{`D:\repo\NUL`, `D:\repo\con.txt`, `D:\repo\AUX\x`, `D:\repo\a.`} {
		if _, ok := relativeInside(`D:\repo`, child, flavorWin32); ok {
			t.Errorf("%q must be outside", child)
		}
	}
	if _, ok := relativeInside("D:\\\u212a", `D:\k\x`, flavorWin32); ok {
		t.Error("the Kelvin sign must not match k")
	}
	if _, ok := relativeInside("/\u212a", "/k/x", flavorPosix); ok {
		t.Error("posix is exact")
	}
}
