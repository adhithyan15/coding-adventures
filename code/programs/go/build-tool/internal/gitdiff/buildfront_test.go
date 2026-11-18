package gitdiff

import (
	"os"
	"os/exec"
	"path/filepath"
	"slices"
	"testing"

	"github.com/adhithyan15/coding-adventures/code/programs/go/build-tool/internal/discovery"
)

func TestClassifyChangedFiles_BuildFronts(t *testing.T) {
	const rustPkg = "code/packages/rust/p/"
	packages := []discovery.Package{
		{Name: "rust/p", Path: "/repo/code/packages/rust/p", Language: "rust"},
		{Name: "typescript/p", Path: "/repo/code/packages/typescript/p", Language: "typescript"},
		{Name: "lua/p", Path: "/repo/code/packages/lua/p", Language: "lua"},
		{Name: "rust/star", Path: "/repo/code/packages/rust/star", Language: "rust", IsStarlark: true},
		{
			Name: "rust/starsrcs", Path: "/repo/code/packages/rust/starsrcs",
			Language: "rust", IsStarlark: true, DeclaredSrcs: []string{"src/**/*.rs"},
		},
	}
	for _, tc := range []struct {
		name     string
		changed  []string
		statuses ChangeStatuses
		local    []string
		prop     []string
	}{
		{"BUILD M", []string{rustPkg + "BUILD"}, ChangeStatuses{rustPkg + "BUILD": 'M'}, []string{"rust/p"}, nil},
		{"BUILD_linux M", []string{rustPkg + "BUILD_linux"}, ChangeStatuses{rustPkg + "BUILD_linux": 'M'}, []string{"rust/p"}, nil},
		{"BUILD_mac M", []string{rustPkg + "BUILD_mac"}, ChangeStatuses{rustPkg + "BUILD_mac": 'M'}, []string{"rust/p"}, nil},
		{"BUILD_mac_and_linux M", []string{rustPkg + "BUILD_mac_and_linux"}, ChangeStatuses{rustPkg + "BUILD_mac_and_linux": 'M'}, []string{"rust/p"}, nil},
		{"BUILD_windows M", []string{rustPkg + "BUILD_windows"}, ChangeStatuses{rustPkg + "BUILD_windows": 'M'}, []string{"rust/p"}, nil},
		{"BUILD A", []string{rustPkg + "BUILD"}, ChangeStatuses{rustPkg + "BUILD": 'A'}, nil, []string{"rust/p"}},
		{"BUILD D", []string{rustPkg + "BUILD"}, ChangeStatuses{rustPkg + "BUILD": 'D'}, nil, []string{"rust/p"}},
		{"BUILD T", []string{rustPkg + "BUILD"}, ChangeStatuses{rustPkg + "BUILD": 'T'}, nil, []string{"rust/p"}},
		{"BUILD R", []string{rustPkg + "BUILD"}, ChangeStatuses{rustPkg + "BUILD": 'R'}, nil, []string{"rust/p"}},
		{"BUILD ordinary pseudo-status", []string{rustPkg + "BUILD"}, ChangeStatuses{rustPkg + "BUILD": StatusOrdinary}, nil, []string{"rust/p"}},
		{"BUILD nil statuses", []string{rustPkg + "BUILD"}, nil, nil, []string{"rust/p"}},
		{"BUILD M + source M", []string{rustPkg + "BUILD", rustPkg + "src/lib.rs"}, ChangeStatuses{rustPkg + "BUILD": 'M', rustPkg + "src/lib.rs": 'M'}, nil, []string{"rust/p"}},
		{"BUILD M + changelog M", []string{rustPkg + "BUILD", rustPkg + "CHANGELOG.md"}, ChangeStatuses{rustPkg + "BUILD": 'M', rustPkg + "CHANGELOG.md": 'M'}, []string{"rust/p"}, nil},
		{"nested BUILD M", []string{rustPkg + "nested/BUILD"}, ChangeStatuses{rustPkg + "nested/BUILD": 'M'}, nil, []string{"rust/p"}},
		{"BUILD.bak M", []string{rustPkg + "BUILD.bak"}, ChangeStatuses{rustPkg + "BUILD.bak": 'M'}, nil, []string{"rust/p"}},
		{"BUILD_debug M", []string{rustPkg + "BUILD_debug"}, ChangeStatuses{rustPkg + "BUILD_debug": 'M'}, nil, []string{"rust/p"}},
		{"typescript BUILD M", []string{"code/packages/typescript/p/BUILD"}, ChangeStatuses{"code/packages/typescript/p/BUILD": 'M'}, nil, []string{"typescript/p"}},
		{"lua BUILD M", []string{"code/packages/lua/p/BUILD"}, ChangeStatuses{"code/packages/lua/p/BUILD": 'M'}, nil, []string{"lua/p"}},
		{"rust Starlark without srcs BUILD M", []string{"code/packages/rust/star/BUILD"}, ChangeStatuses{"code/packages/rust/star/BUILD": 'M'}, nil, []string{"rust/star"}},
		{"rust Starlark with srcs BUILD M", []string{"code/packages/rust/starsrcs/BUILD"}, ChangeStatuses{"code/packages/rust/starsrcs/BUILD": 'M'}, nil, []string{"rust/starsrcs"}},
	} {
		t.Run(tc.name, func(t *testing.T) {
			prop, local := ClassifyChangedFiles(tc.changed, packages, "/repo", tc.statuses)
			if got := keys(prop); !slices.Equal(got, tc.prop) && !(len(got) == 0 && len(tc.prop) == 0) {
				t.Errorf("propagating = %v, want %v", got, tc.prop)
			}
			if got := keys(local); !slices.Equal(got, tc.local) && !(len(got) == 0 && len(tc.local) == 0) {
				t.Errorf("local = %v, want %v", got, tc.local)
			}
		})
	}
}

func TestChangeStatusesModifiedAndWithOrdinary(t *testing.T) {
	var none ChangeStatuses
	if none.Modified("a") {
		t.Error("nil statuses: nothing is modified")
	}
	if none.WithOrdinary(map[string]bool{"a": true}) != nil {
		t.Error("nil stays nil")
	}
	statuses := ChangeStatuses{"a": 'M', "b": 'M', "c": 'A'}
	if !statuses.Modified("a") || statuses.Modified("c") || statuses.Modified("missing") {
		t.Error("Modified is exactly status M")
	}
	if got := statuses.WithOrdinary(nil); len(got) != len(statuses) || got["a"] != 'M' {
		t.Error("empty set returns the statuses unchanged")
	}
	copied := statuses.WithOrdinary(map[string]bool{"a": true})
	if copied["a"] != StatusOrdinary || copied["b"] != 'M' || copied["c"] != 'A' {
		t.Errorf("copy = %v", copied)
	}
	if statuses["a"] != 'M' {
		t.Error("WithOrdinary must not mutate its receiver")
	}
	if copied.Modified("a") || copied.Additive("a") {
		t.Error("an ordinary path is neither modified nor additive")
	}
}

// markerRepo builds a repository whose base commit has the given files, then
// applies a second commit; it returns the statuses for base...HEAD.
func markerRepo(t *testing.T, base, change map[string]string) ChangeStatuses {
	t.Helper()
	repo := t.TempDir()
	git := func(args ...string) {
		t.Helper()
		cmd := exec.Command("git", append([]string{"-c", "user.name=T", "-c", "user.email=t@example.invalid"}, args...)...)
		cmd.Dir = repo
		if out, err := cmd.CombinedOutput(); err != nil {
			t.Fatalf("git %v: %v: %s", args, err, out)
		}
	}
	write := func(files map[string]string) {
		for rel, content := range files {
			path := filepath.Join(repo, filepath.FromSlash(rel))
			if err := os.MkdirAll(filepath.Dir(path), 0o755); err != nil {
				t.Fatal(err)
			}
			if err := os.WriteFile(path, []byte(content), 0o644); err != nil {
				t.Fatal(err)
			}
		}
	}
	git("init", "-q")
	write(base)
	git("add", ".")
	git("commit", "-qm", "base")
	git("branch", "base")
	write(change)
	git("add", "-A")
	git("commit", "-qm", "change")
	_, statuses := GetChangedFilesWithStatus(repo, "base")
	return statuses
}

func TestGetChangedFilesWithStatusFlagsUnsupportedMarkerTransitions(t *testing.T) {
	marked := "echo BUILD_TOOL_UNSUPPORTED:NO_X -- skipped\n"
	t.Run("marker removed", func(t *testing.T) {
		statuses := markerRepo(t,
			map[string]string{"p/BUILD": marked, "q/BUILD": "cargo test\n"},
			map[string]string{"p/BUILD": "cargo build\n", "q/BUILD": "cargo test --all\n"})
		if statuses["p/BUILD"] != StatusOrdinary || statuses["q/BUILD"] != 'M' {
			t.Fatalf("statuses = %v, want p/BUILD '!' and q/BUILD 'M'", statuses)
		}
	})
	t.Run("marker added", func(t *testing.T) {
		statuses := markerRepo(t,
			map[string]string{"p/BUILD_windows": "cargo test\n", "q/BUILD": "cargo test\n"},
			map[string]string{"p/BUILD_windows": marked, "q/BUILD": "cargo test --all\n"})
		if statuses["p/BUILD_windows"] != StatusOrdinary || statuses["q/BUILD"] != 'M' {
			t.Fatalf("statuses = %v", statuses)
		}
	})
	t.Run("marker text in a non-BUILD file stays modified", func(t *testing.T) {
		statuses := markerRepo(t,
			map[string]string{"p/BUILD": "cargo test\n", "p/NOTES.md": "x\n"},
			map[string]string{"p/BUILD": "cargo test --all\n", "p/NOTES.md": "BUILD_TOOL_UNSUPPORTED:X\n"})
		if statuses["p/BUILD"] != 'M' || statuses["p/NOTES.md"] != 'M' {
			t.Fatalf("statuses = %v", statuses)
		}
	})
	t.Run("a marker line untouched by the diff does not flag the front", func(t *testing.T) {
		statuses := markerRepo(t,
			map[string]string{"p/BUILD": marked + "a\n"},
			map[string]string{"p/BUILD": marked + "b\n"})
		if statuses["p/BUILD"] != 'M' {
			t.Fatalf("statuses = %v (the head veto, not the diff, covers this)", statuses)
		}
	})
}

func TestWithMarkerTouchedFrontsFailsClosed(t *testing.T) {
	statuses := ChangeStatuses{"p/BUILD": 'M', "p/src/lib.rs": 'M'}
	// A repository path with no Git: the -G query fails, so every changed
	// BUILD front is ordinary and a non-front keeps its status.
	got := withMarkerTouchedFronts(filepath.Join(t.TempDir(), "missing"), "base", true,
		[]string{"p/BUILD", "p/src/lib.rs"}, statuses)
	if got["p/BUILD"] != StatusOrdinary || got["p/src/lib.rs"] != 'M' {
		t.Fatalf("statuses = %v", got)
	}
	// No BUILD front changed: no query, statuses returned as they are.
	if got := withMarkerTouchedFronts("/nonexistent", "base", false, []string{"a.go"}, ChangeStatuses{"a.go": 'M'}); got["a.go"] != 'M' {
		t.Fatalf("statuses = %v", got)
	}
}
