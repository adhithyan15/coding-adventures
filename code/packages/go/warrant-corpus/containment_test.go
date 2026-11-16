package warrantcorpus

import (
	"errors"
	"os"
	"path/filepath"
	"strings"
	"testing"
)

// winOps behaves like Go's filepath on Windows for the handful of paths these
// tests use: separators are backslashes, a volume prefix ("D:") is its own root,
// and Rel REFUSES to cross volumes (an error), as filepath.Rel does on Windows.
func winOps() pathOps {
	volume := func(p string) string {
		if len(p) >= 2 && p[1] == ':' {
			return strings.ToUpper(p[:2])
		}
		return ""
	}
	return pathOps{
		sep:   '\\',
		isAbs: func(p string) bool { return volume(p) != "" && len(p) > 2 && p[2] == '\\' },
		rel: func(base, target string) (string, error) {
			if volume(base) != volume(target) {
				return "", errors.New("Rel: can't make " + target + " relative to " + base)
			}
			b := strings.Split(strings.Trim(base[len(volume(base)):], `\`), `\`)
			tt := strings.Split(strings.Trim(target[len(volume(target)):], `\`), `\`)
			if len(b) == 1 && b[0] == "" {
				b = nil
			}
			if len(tt) == 1 && tt[0] == "" {
				tt = nil
			}
			shared := 0
			for shared < len(b) && shared < len(tt) && strings.EqualFold(b[shared], tt[shared]) {
				shared++
			}
			var parts []string
			for i := shared; i < len(b); i++ {
				parts = append(parts, "..")
			}
			parts = append(parts, tt[shared:]...)
			if len(parts) == 0 {
				return ".", nil
			}
			return strings.Join(parts, `\`), nil
		},
	}
}

func TestRelativeInsideWindowsShapes(t *testing.T) {
	win := winOps()
	inside := map[string]string{
		`D:\repo\code\specs`: `code\specs`,
		`D:\repo\..foo`:      `..foo`, // a directory NAMED ..foo is inside
		`D:\repo\...`:        `...`,
		`d:\REPO\x`:          `x`,
	}
	for child, want := range inside {
		if got, ok := relativeInside(`D:\repo`, child, win); !ok || got != want {
			t.Errorf("%s: got (%q, %v), want (%q, true)", child, got, ok, want)
		}
	}
	for _, child := range []string{
		`C:\tmp\x`,      // another drive: Rel returns an error
		`D:\other`,      // sibling
		`D:\`,           // the parent
		`D:\repo\..\x`,  // climbs out
		`D:\repository`, // a prefix of the name is not containment
	} {
		if got, ok := relativeInside(`D:\repo`, child, win); ok {
			t.Errorf("%s: accepted as %q", child, got)
		}
	}
	// The root itself is inside, as the empty relative path.
	if got, ok := relativeInside(`D:\repo`, `D:\repo`, win); !ok || got != "" {
		t.Errorf("root: (%q, %v)", got, ok)
	}
}

func TestRelativeInsideRefusesAnAbsoluteResult(t *testing.T) {
	// A path module that reports an absolute target instead of failing (Node's
	// behaviour across drives) must still be refused: `C:\tmp\x` has no leading
	// "..", so a prefix test alone would wave it through.
	win := winOps()
	win.rel = func(base, target string) (string, error) { return `C:\tmp\x`, nil }
	if got, ok := relativeInside(`D:\repo`, `C:\tmp\x`, win); ok {
		t.Fatalf("accepted an absolute result: %q", got)
	}
}

func TestRelativeInsideDotDotPrefixRules(t *testing.T) {
	// The bare prefix test is wrong in both directions; check the three results.
	for rel, wantInside := range map[string]bool{
		"..":      false,
		"../x":    false,
		"..foo":   true,
		"..foo/x": true,
		"...":     true,
		"a/..b":   true,
		".":       true,
		"":        true,
	} {
		ops := pathOps{sep: '/', isAbs: func(string) bool { return false },
			rel: func(string, string) (string, error) { return rel, nil }}
		if _, ok := relativeInside("/root", "/child", ops); ok != wantInside {
			t.Errorf("rel %q: inside = %v, want %v", rel, ok, wantInside)
		}
	}
}

func TestOSPathAgreesOnThisHost(t *testing.T) {
	root := t.TempDir()
	if got, ok := relativeInside(root, filepath.Join(root, "a", "b"), osPath); !ok || got != filepath.Join("a", "b") {
		t.Fatalf("(%q, %v)", got, ok)
	}
	if _, ok := relativeInside(root, filepath.Dir(root), osPath); ok {
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
