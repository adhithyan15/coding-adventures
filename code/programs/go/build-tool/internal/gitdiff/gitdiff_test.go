package gitdiff

import (
	"os"
	"os/exec"
	"path/filepath"
	"slices"
	"testing"

	"github.com/adhithyan15/coding-adventures/code/programs/go/build-tool/internal/discovery"
)

func TestGetChangedFilesPreservesRawGitPaths(t *testing.T) {
	repo := t.TempDir()
	runGit := func(args ...string) {
		t.Helper()
		cmd := exec.Command("git", args...)
		cmd.Dir = repo
		if out, err := cmd.CombinedOutput(); err != nil {
			t.Fatalf("git %v: %v: %s", args, err, out)
		}
	}
	runGit("init", "-q")
	if err := os.WriteFile(filepath.Join(repo, "README.md"), []byte("base"), 0o644); err != nil {
		t.Fatal(err)
	}
	runGit("add", ".")
	runGit("-c", "user.name=Test", "-c", "user.email=test@example.invalid", "commit", "-qm", "base")
	caseDir := filepath.Join(repo, "code", "specs", "fixtures", "build-tool-v1", "cases")
	if err := os.MkdirAll(caseDir, 0o755); err != nil {
		t.Fatal(err)
	}
	want := []string{
		"code/specs/fixtures/build-tool-v1/cases/plan-affected-empty.json",
		"code/specs/fixtures/build-tool-v1/cases/plan-caf\u00e9.json",
	}
	for _, name := range want {
		if err := os.WriteFile(filepath.Join(repo, filepath.FromSlash(name)), []byte("{}"), 0o644); err != nil {
			t.Fatal(err)
		}
	}
	runGit("add", ".")
	runGit("-c", "user.name=Test", "-c", "user.email=test@example.invalid", "commit", "-qm", "fixtures")
	got := GetChangedFiles(repo, "HEAD~1")
	slices.Sort(got)
	if !slices.Equal(got, want) {
		t.Fatalf("changed files = %q, want %q", got, want)
	}
}

func TestMapFilesToPackages_ShellBuild(t *testing.T) {
	// Shell BUILD packages: any file under the package triggers rebuild.
	packages := []discovery.Package{
		{Name: "python/foo", Path: "/repo/code/packages/python/foo", Language: "python"},
	}

	tests := []struct {
		file string
		want bool
	}{
		{"code/packages/python/foo/src/main.py", true},
		{"code/packages/python/foo/README.md", true},
		{"code/packages/python/foo/CHANGELOG.md", true},
		{"code/packages/python/bar/src/main.py", false},
	}

	for _, tt := range tests {
		changed := MapFilesToPackages([]string{tt.file}, packages, "/repo")
		got := changed["python/foo"]
		if got != tt.want {
			t.Errorf("ShellBuild: file %q → changed=%v, want %v", tt.file, got, tt.want)
		}
	}
}

func TestMapFilesToPackages_StarlarkStrict(t *testing.T) {
	// Starlark BUILD with declared srcs: only matching files trigger.
	packages := []discovery.Package{
		{
			Name:         "python/foo",
			Path:         "/repo/code/packages/python/foo",
			Language:     "python",
			IsStarlark:   true,
			DeclaredSrcs: []string{"src/**/*.py", "tests/**/*.py", "pyproject.toml"},
		},
	}

	tests := []struct {
		file string
		want bool
		desc string
	}{
		{"code/packages/python/foo/src/main.py", true, "source file matches"},
		{"code/packages/python/foo/src/a/b.py", true, "nested source matches"},
		{"code/packages/python/foo/tests/test_main.py", true, "test file matches"},
		{"code/packages/python/foo/pyproject.toml", true, "literal pattern matches"},
		{"code/packages/python/foo/README.md", false, "README not in declared srcs"},
		{"code/packages/python/foo/CHANGELOG.md", false, "CHANGELOG not in declared srcs"},
		{"code/packages/python/foo/docs/guide.md", false, "docs not in declared srcs"},
		{"code/packages/python/foo/BUILD", true, "BUILD always triggers"},
		{"code/packages/python/foo/BUILD_linux", true, "BUILD_linux always triggers"},
	}

	for _, tt := range tests {
		changed := MapFilesToPackages([]string{tt.file}, packages, "/repo")
		got := changed["python/foo"]
		if got != tt.want {
			t.Errorf("Starlark(%s): file %q → changed=%v, want %v",
				tt.desc, tt.file, got, tt.want)
		}
	}
}

func TestMapFilesToPackages_StarlarkBuildFrontsAreExact(t *testing.T) {
	packages := []discovery.Package{
		{
			Name:         "python/foo",
			Path:         "/repo/code/packages/python/foo",
			Language:     "python",
			IsStarlark:   true,
			DeclaredSrcs: []string{"src/**/*.py"},
		},
	}

	tests := []struct {
		file string
		want bool
	}{
		{"code/packages/python/foo/BUILD", true},
		{"code/packages/python/foo/nested/BUILD_windows", true},
		{"code/packages/python/foo/nested/BUILD_mac", true},
		{"code/packages/python/foo/nested/BUILD_linux", true},
		{"code/packages/python/foo/nested/BUILD_mac_and_linux", true},
		{"code/packages/python/foo/BUILD_debug", false},
		{"code/packages/python/foo/BUILD.bak", false},
		{"code/packages/python/foo/nested/BUILD_windows.old", false},
	}

	for _, tt := range tests {
		t.Run(tt.file, func(t *testing.T) {
			changed := MapFilesToPackages([]string{tt.file}, packages, "/repo")
			if got := changed["python/foo"]; got != tt.want {
				t.Fatalf("changed=%v, want %v", got, tt.want)
			}
		})
	}
}

func TestMapFilesToPackages_StarlarkNoDeclaredSrcs(t *testing.T) {
	// Starlark BUILD but with empty DeclaredSrcs: falls back to any-file behavior.
	packages := []discovery.Package{
		{
			Name:       "go/bar",
			Path:       "/repo/code/packages/go/bar",
			Language:   "go",
			IsStarlark: true,
			// DeclaredSrcs is empty — no strict filtering.
		},
	}

	changed := MapFilesToPackages(
		[]string{"code/packages/go/bar/README.md"},
		packages, "/repo",
	)

	if !changed["go/bar"] {
		t.Error("Starlark with empty DeclaredSrcs should trigger on any file")
	}
}

func TestMapFilesToPackages_MixedPackages(t *testing.T) {
	// Mix of shell and Starlark packages.
	packages := []discovery.Package{
		{
			Name:         "python/strict",
			Path:         "/repo/code/packages/python/strict",
			Language:     "python",
			IsStarlark:   true,
			DeclaredSrcs: []string{"src/**/*.py"},
		},
		{
			Name:     "python/loose",
			Path:     "/repo/code/packages/python/loose",
			Language: "python",
		},
	}

	// README change in strict package: no trigger.
	changed := MapFilesToPackages(
		[]string{"code/packages/python/strict/README.md"},
		packages, "/repo",
	)
	if changed["python/strict"] {
		t.Error("strict package should not trigger on README.md")
	}

	// README change in loose package: triggers.
	changed = MapFilesToPackages(
		[]string{"code/packages/python/loose/README.md"},
		packages, "/repo",
	)
	if !changed["python/loose"] {
		t.Error("loose package should trigger on README.md")
	}
}

func TestMapFilesToPackages_MultipleFiles(t *testing.T) {
	packages := []discovery.Package{
		{
			Name:         "python/a",
			Path:         "/repo/code/packages/python/a",
			Language:     "python",
			IsStarlark:   true,
			DeclaredSrcs: []string{"src/**/*.py"},
		},
		{
			Name:     "ruby/b",
			Path:     "/repo/code/packages/ruby/b",
			Language: "ruby",
		},
	}

	changed := MapFilesToPackages(
		[]string{
			"code/packages/python/a/src/foo.py",
			"code/packages/ruby/b/lib/bar.rb",
			"code/packages/python/a/README.md", // should NOT trigger
		},
		packages, "/repo",
	)

	if !changed["python/a"] {
		t.Error("python/a should be changed (src/foo.py matches)")
	}
	if !changed["ruby/b"] {
		t.Error("ruby/b should be changed (shell BUILD, any file)")
	}
}

// ClassifyChangedFiles splits the changed packages of MapFilesToPackages by
// whether the change can reach dependents. An added or modified piece of
// package-local metadata at the root of a shell package is local; anything
// else propagates: README.md, a nested or misspelled name, a non-data file in
// a metadata directory, a metadata name a Starlark package declares as a
// source, and every deletion, rename or type change.
func TestClassifyChangedFiles_PackageLocalMetadata(t *testing.T) {
	packages := []discovery.Package{
		{Name: "rust/meta", Path: "/repo/code/packages/rust/meta", Language: "rust"},
		{Name: "rust/mixed", Path: "/repo/code/packages/rust/mixed", Language: "rust"},
		{Name: "rust/nested", Path: "/repo/code/packages/rust/nested", Language: "rust"},
		{Name: "rust/readme", Path: "/repo/code/packages/rust/readme", Language: "rust"},
		{Name: "rust/deleted", Path: "/repo/code/packages/rust/deleted", Language: "rust"},
		{Name: "rust/retyped", Path: "/repo/code/packages/rust/retyped", Language: "rust"},
		{Name: "rust/script", Path: "/repo/code/packages/rust/script", Language: "rust"},
		{Name: "rust/unknown", Path: "/repo/code/packages/rust/unknown", Language: "rust"},
		{
			Name: "python/declares", Path: "/repo/code/packages/python/declares",
			Language: "python", IsStarlark: true, DeclaredSrcs: []string{"src/**/*.py", "CHANGELOG.md"},
		},
		{
			Name: "python/strict", Path: "/repo/code/packages/python/strict",
			Language: "python", IsStarlark: true, DeclaredSrcs: []string{"src/**/*.py"},
		},
	}
	statuses := ChangeStatuses{
		"code/packages/rust/meta/CHANGELOG.md":                         'M',
		"code/packages/rust/meta/CHANGELOG.d/2026-11-11-0123abcd-x.md": 'A',
		"code/packages/rust/meta/required_capabilities.json":           'A',
		"code/packages/rust/meta/effect_certificate.json":              'M',
		"code/packages/rust/meta/assurance_report.json":                'M',
		"code/packages/rust/meta/evidence/test-manifest.json":          'M',
		"code/packages/rust/mixed/CHANGELOG.md":                        'M',
		"code/packages/rust/mixed/src/lib.rs":                          'M',
		"code/packages/rust/nested/sub/required_capabilities.json":     'A',
		"code/packages/rust/readme/README.md":                          'M',
		"code/packages/rust/deleted/CHANGELOG.md":                      'D',
		"code/packages/rust/retyped/required_capabilities.json":        'T',
		"code/packages/rust/script/evidence/run.sh":                    'A',
		"code/packages/python/declares/CHANGELOG.md":                   'M',
		"code/packages/python/strict/CHANGELOG.md":                     'M',
		// rust/unknown has no recorded status.
	}
	changed := []string{
		"code/packages/rust/meta/CHANGELOG.md",
		"code/packages/rust/meta/CHANGELOG.d/2026-11-11-0123abcd-x.md",
		"code/packages/rust/meta/required_capabilities.json",
		"code/packages/rust/meta/effect_certificate.json",
		"code/packages/rust/meta/assurance_report.json",
		"code/packages/rust/meta/evidence/test-manifest.json",
		"code/packages/rust/mixed/CHANGELOG.md",
		"code/packages/rust/mixed/src/lib.rs",
		"code/packages/rust/nested/sub/required_capabilities.json",
		"code/packages/rust/readme/README.md",
		"code/packages/rust/deleted/CHANGELOG.md",
		"code/packages/rust/retyped/required_capabilities.json",
		"code/packages/rust/script/evidence/run.sh",
		"code/packages/rust/unknown/CHANGELOG.md",
		"code/packages/python/declares/CHANGELOG.md",
		"code/packages/python/strict/CHANGELOG.md",
	}
	propagating, local := ClassifyChangedFiles(changed, packages, "/repo", statuses)
	wantPropagating := []string{
		"python/declares", "rust/deleted", "rust/mixed", "rust/nested",
		"rust/readme", "rust/retyped", "rust/script", "rust/unknown",
	}
	if got := keys(propagating); !slices.Equal(got, wantPropagating) {
		t.Errorf("propagating = %v, want %v", got, wantPropagating)
	}
	if got, want := keys(local), []string{"rust/meta"}; !slices.Equal(got, want) {
		t.Errorf("local = %v, want %v", got, want)
	}
	// Without statuses nothing is local.
	propagatingUnknown, localUnknown := ClassifyChangedFiles(changed, packages, "/repo", nil)
	if len(localUnknown) != 0 || !propagatingUnknown["rust/meta"] {
		t.Errorf("nil statuses: propagating = %v, local = %v", keys(propagatingUnknown), keys(localUnknown))
	}
	// MapFilesToPackages is still the union: every package whose own BUILD
	// must re-run.
	wantUnion := append(append([]string{}, wantPropagating...), "rust/meta")
	slices.Sort(wantUnion)
	if got := keys(MapFilesToPackages(changed, packages, "/repo")); !slices.Equal(got, wantUnion) {
		t.Errorf("MapFilesToPackages = %v, want %v", got, wantUnion)
	}
}

func TestParseNameStatus(t *testing.T) {
	files, statuses := parseNameStatus([]byte("M\x00a/x.go\x00D\x00b/CHANGELOG.md\x00A\x00c/y.md\x00T\x00d/z\x00R100\x00e/old\x00e/new\x00"))
	if want := []string{"a/x.go", "b/CHANGELOG.md", "c/y.md", "d/z", "e/old", "e/new"}; !slices.Equal(files, want) {
		t.Fatalf("files = %v, want %v", files, want)
	}
	for path, want := range map[string]bool{"a/x.go": true, "b/CHANGELOG.md": false, "c/y.md": true, "d/z": false, "e/old": false, "e/new": false, "missing": false} {
		if got := statuses.Additive(path); got != want {
			t.Errorf("Additive(%q) = %t, want %t", path, got, want)
		}
	}
	// Empty and undecodable output both mean "no usable diff": nil, nil, never
	// a non-nil empty slice (callers test for nil) and never a partial list.
	for name, out := range map[string][]byte{
		"nil":          nil,
		"empty":        {},
		"only NULs":    []byte("\x00\x00"),
		"truncated":    []byte("M\x00a/x.go\x00D"),
		"empty path":   []byte("M\x00a/x.go\x00M\x00\x00"),
		"short rename": []byte("M\x00a/x.go\x00R100\x00e/old"),
	} {
		if files, statuses := parseNameStatus(out); files != nil || statuses != nil {
			t.Errorf("%s: files = %#v, statuses = %v, want nil, nil", name, files, statuses)
		}
	}
}

// A real Git history: modify, add, delete and rename metadata, then check the
// statuses GetChangedFilesWithStatus reports. --no-renames turns the rename
// into a deletion plus an addition, so the rename still carries a D.
func TestGetChangedFilesWithStatusReportsDeletionsAndRenames(t *testing.T) {
	repo := t.TempDir()
	git := func(args ...string) {
		t.Helper()
		cmd := exec.Command("git", append([]string{"-c", "user.name=T", "-c", "user.email=t@example.invalid"}, args...)...)
		cmd.Dir = repo
		if out, err := cmd.CombinedOutput(); err != nil {
			t.Fatalf("git %v: %v: %s", args, err, out)
		}
	}
	write := func(rel, content string) {
		t.Helper()
		path := filepath.Join(repo, filepath.FromSlash(rel))
		if err := os.MkdirAll(filepath.Dir(path), 0o755); err != nil {
			t.Fatal(err)
		}
		if err := os.WriteFile(path, []byte(content), 0o644); err != nil {
			t.Fatal(err)
		}
	}
	git("init", "-q")
	write("p/CHANGELOG.md", "one\n")
	write("p/required_capabilities.json", "{}\n")
	write("p/effect_certificate.json", "{}\n")
	git("add", ".")
	git("commit", "-qm", "base")
	git("branch", "base")
	write("p/CHANGELOG.md", "two\n")
	write("p/CHANGELOG.d/x.md", "shard\n")
	if err := os.Remove(filepath.Join(repo, "p", "required_capabilities.json")); err != nil {
		t.Fatal(err)
	}
	git("mv", "p/effect_certificate.json", "p/assurance_report.json")
	git("add", "-A")
	git("commit", "-qm", "change")

	files, statuses := GetChangedFilesWithStatus(repo, "base")
	slices.Sort(files)
	if want := []string{"p/CHANGELOG.d/x.md", "p/CHANGELOG.md", "p/assurance_report.json", "p/effect_certificate.json", "p/required_capabilities.json"}; !slices.Equal(files, want) {
		t.Fatalf("files = %v, want %v", files, want)
	}
	for path, want := range map[string]byte{
		"p/CHANGELOG.md": 'M', "p/CHANGELOG.d/x.md": 'A', "p/required_capabilities.json": 'D',
		"p/effect_certificate.json": 'D', "p/assurance_report.json": 'A',
	} {
		if statuses[path] != want {
			t.Errorf("status(%s) = %q, want %q", path, statuses[path], want)
		}
	}
	if plain := GetChangedFiles(repo, "base"); len(plain) != len(files) {
		t.Errorf("GetChangedFiles = %v, want the same %d paths", plain, len(files))
	}
	if files, statuses := GetChangedFilesWithStatus(repo, "no-such-ref"); files != nil || statuses != nil {
		t.Errorf("unknown base = %v %v, want nil, nil", files, statuses)
	} // HEAD equal to the base is an empty diff, which must look exactly like
	// "no answer" so -emit-plan keeps its full-plan fallback on every platform.
	if files, statuses := GetChangedFilesWithStatus(repo, "HEAD"); files != nil || statuses != nil {
		t.Errorf("empty diff = %#v %v, want nil, nil", files, statuses)
	}
}

func keys(set map[string]bool) []string {
	result := make([]string, 0, len(set))
	for name, selected := range set {
		if selected {
			result = append(result, name)
		}
	}
	slices.Sort(result)
	return result
}
