package gitdiff

import (
	"testing"

	"github.com/adhithyan15/coding-adventures/code/programs/go/build-tool/internal/discovery"
)

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
