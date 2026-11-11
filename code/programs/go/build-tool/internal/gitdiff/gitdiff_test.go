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
// whether the change can reach dependents. Package-local metadata at the root
// of a shell package is local; anything else, including a metadata name that
// is nested or that a Starlark package declares as a source, propagates.
func TestClassifyChangedFiles_PackageLocalMetadata(t *testing.T) {
	packages := []discovery.Package{
		{Name: "rust/meta", Path: "/repo/code/packages/rust/meta", Language: "rust"},
		{Name: "rust/mixed", Path: "/repo/code/packages/rust/mixed", Language: "rust"},
		{Name: "rust/nested", Path: "/repo/code/packages/rust/nested", Language: "rust"},
		{
			Name: "python/declares-readme", Path: "/repo/code/packages/python/declares-readme",
			Language: "python", IsStarlark: true, DeclaredSrcs: []string{"src/**/*.py", "README.md"},
		},
		{
			Name: "python/strict", Path: "/repo/code/packages/python/strict",
			Language: "python", IsStarlark: true, DeclaredSrcs: []string{"src/**/*.py"},
		},
	}
	changed := []string{
		// Metadata only: local.
		"code/packages/rust/meta/README.md",
		"code/packages/rust/meta/CHANGELOG.md",
		"code/packages/rust/meta/CHANGELOG.d/2026-11-11-0123abcd-x.md",
		"code/packages/rust/meta/required_capabilities.json",
		"code/packages/rust/meta/effect_certificate.json",
		"code/packages/rust/meta/assurance_report.json",
		"code/packages/rust/meta/evidence/test-manifest.json",
		// Metadata plus source: the source wins.
		"code/packages/rust/mixed/README.md",
		"code/packages/rust/mixed/src/lib.rs",
		// A nested README is a test input somewhere: it propagates.
		"code/packages/rust/nested/tests/fixtures/README.md",
		// A declared Starlark source propagates whatever its name.
		"code/packages/python/declares-readme/README.md",
		// An undeclared one selects nothing at all, as before.
		"code/packages/python/strict/CHANGELOG.md",
	}
	propagating, local := ClassifyChangedFiles(changed, packages, "/repo")
	if got, want := keys(propagating), []string{"python/declares-readme", "rust/mixed", "rust/nested"}; !slices.Equal(got, want) {
		t.Errorf("propagating = %v, want %v", got, want)
	}
	if got, want := keys(local), []string{"rust/meta"}; !slices.Equal(got, want) {
		t.Errorf("local = %v, want %v", got, want)
	}
	// MapFilesToPackages is still the union: every package whose own BUILD
	// must re-run.
	if got, want := keys(MapFilesToPackages(changed, packages, "/repo")), []string{"python/declares-readme", "rust/meta", "rust/mixed", "rust/nested"}; !slices.Equal(got, want) {
		t.Errorf("MapFilesToPackages = %v, want %v", got, want)
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
