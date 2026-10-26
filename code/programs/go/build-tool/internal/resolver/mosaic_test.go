package resolver

import (
	"errors"
	"os"
	"path/filepath"
	"reflect"
	"strings"
	"testing"

	"github.com/adhithyan15/coding-adventures/code/programs/go/build-tool/internal/discovery"
)

// mosaicWorld lays out a small code/ tree in the repository's real shape:
// three library packages under packages/mosaic, one app under
// programs/mosaic, and a Rust crate whose Cargo name collides with a Mosaic
// dependency key so the test can prove the lookup stays in the Mosaic scope.
func mosaicWorld(t *testing.T, appManifest string) (string, []discovery.Package) {
	t.Helper()
	root := makeFixture(t, map[string]string{
		"code/packages/mosaic/mosaic-pkg-toolkit/mosaic-package.toml": `[package]
name = "mosaic-pkg-toolkit"
version = "0.16.0"

[dependencies]
`,
		"code/packages/mosaic/mosaic-pkg-sheet/mosaic-package.toml": `[package]
name = "mosaic-pkg-sheet"
version = "0.1.0"

[dependencies]
mosaic-pkg-toolkit = "0.16.0"
`,
		// The directory and manifest names differ: dependents spell the
		// manifest name, and that must resolve too.
		"code/packages/mosaic/calendar/mosaic-package.toml": `[package]
name = "mosaic-pkg-calendar"
version = "0.1.0"
`,
		"code/programs/mosaic/task-app/mosaic-package.toml": appManifest,
		"code/packages/rust/mosaic-pkg-grid/Cargo.toml": `[package]
name = "mosaic-pkg-grid"
version = "0.1.0"
`,
	})
	packages := []discovery.Package{
		{Name: "mosaic/mosaic-pkg-toolkit", Path: filepath.Join(root, "code/packages/mosaic/mosaic-pkg-toolkit"), Language: "mosaic"},
		{Name: "mosaic/mosaic-pkg-sheet", Path: filepath.Join(root, "code/packages/mosaic/mosaic-pkg-sheet"), Language: "mosaic"},
		{Name: "mosaic/calendar", Path: filepath.Join(root, "code/packages/mosaic/calendar"), Language: "mosaic"},
		{Name: "mosaic/programs/task-app", Path: filepath.Join(root, "code/programs/mosaic/task-app"), Language: "mosaic"},
		{Name: "rust/mosaic-pkg-grid", Path: filepath.Join(root, "code/packages/rust/mosaic-pkg-grid"), Language: "rust"},
	}
	return root, packages
}

// TestParseMosaicDepsReadsOnlyTheDependenciesTable is the field-aware table:
// each row is a manifest and the exact edges it must produce. Every row that
// expects nothing is a place where the word "dependencies" or a package name
// appears outside the one field that may create an edge.
func TestParseMosaicDepsReadsOnlyTheDependenciesTable(t *testing.T) {
	cases := []struct {
		name     string
		manifest string
		want     []string
	}{
		{
			name: "bare keys in the dependencies table",
			manifest: `[package]
name = "task-app"
[dependencies]
mosaic-pkg-toolkit = "0.16.0"
mosaic-pkg-sheet = "0.1.0"
`,
			want: []string{"mosaic/mosaic-pkg-sheet", "mosaic/mosaic-pkg-toolkit"},
		},
		{
			name: "quoted key, trailing comment, and a manifest-name alias",
			manifest: `[dependencies]
"mosaic-pkg-toolkit" = "0.16.0"   # caret requirement
mosaic-pkg-calendar = "0.1.0"
`,
			want: []string{"mosaic/calendar", "mosaic/mosaic-pkg-toolkit"},
		},
		{
			name: "dotted sub-table header names a dependency",
			manifest: `[dependencies.mosaic-pkg-sheet]
version = "0.1.0"
`,
			want: []string{"mosaic/mosaic-pkg-sheet"},
		},
		{
			name: "external, commented-out, and self dependencies are dropped",
			manifest: `[package]
name = "task-app"
[dependencies]
# mosaic-pkg-toolkit = "0.16.0"
mosaic-pkg-from-elsewhere = "1.0.0"
task-app = "0.1.0"
`,
			want: []string{},
		},
		{
			name: "a Rust crate with the same name is out of scope",
			manifest: `[dependencies]
mosaic-pkg-grid = "0.1.0"
`,
			want: []string{},
		},
		{
			name: "package identity, exports and other tables never create edges",
			manifest: `[package]
name = "mosaic-pkg-toolkit"
description = "depends on mosaic-pkg-sheet in spirit only"
[components]
exports = [
  "mosaic-pkg-sheet",
  ["mosaic-pkg-calendar"],
]
[host_assets]
dependencies = [
  { backend = "flutter", coordinate = "mosaic-pkg-sheet" },
]
[kernel]
version = "1"
`,
			want: []string{},
		},
		{
			name: "a multi-line value in dependencies does not swallow the next key",
			manifest: `[dependencies]
mosaic-pkg-sheet = { version = "0.1.0", note = [
  "[not-a-header]",
] }
mosaic-pkg-toolkit = "0.16.0"
[kernel]
mosaic-pkg-calendar = "never a dependency"
`,
			want: []string{"mosaic/mosaic-pkg-sheet", "mosaic/mosaic-pkg-toolkit"},
		},
		{
			name: "an array-of-tables header ends the dependencies table",
			manifest: `[dependencies]
mosaic-pkg-toolkit = "0.16.0"
[[screens]]
mosaic-pkg-sheet = "0.1.0"
`,
			want: []string{"mosaic/mosaic-pkg-toolkit"},
		},
	}

	for _, tc := range cases {
		t.Run(tc.name, func(t *testing.T) {
			_, packages := mosaicWorld(t, tc.manifest)
			known := buildKnownNamesForLanguage(packages, "mosaic")
			got, err := parseMosaicDeps(packages[3], known)
			if err != nil {
				t.Fatalf("parseMosaicDeps: %v", err)
			}
			if !reflect.DeepEqual(got, tc.want) {
				t.Fatalf("parseMosaicDeps = %v, want %v", got, tc.want)
			}
		})
	}
}

func TestParseMosaicDepsWithoutAManifest(t *testing.T) {
	pkg := discovery.Package{Name: "mosaic/empty", Path: t.TempDir(), Language: "mosaic"}
	if got, err := parseMosaicDeps(pkg, map[string]string{"x": "mosaic/x"}); err != nil || len(got) != 0 {
		t.Fatalf("expected no dependencies and no error without a manifest, got %v, %v", got, err)
	}
	if got := readMosaicPackageName(pkg.Path); got != "" {
		t.Fatalf("expected no manifest name, got %q", got)
	}
}

// TestParseMosaicDepsFailsClosedOnShapesItDoesNotModel: each row is valid
// TOML that declares (or could hide) a dependency in a shape the line reader
// does not model. Resolving no edge there would silently skip the dependent's
// tests on every later PR, so each must be an error that stops the plan.
func TestParseMosaicDepsFailsClosedOnShapesItDoesNotModel(t *testing.T) {
	cases := []struct{ name, manifest, reason string }{
		{"plain form (control)", "[package]\nname = \"task-app\"\n", ""},
		{"root inline table", "dependencies = { mosaic-pkg-sheet = \"0.1.0\" }\n[package]\nname = \"task-app\"\n", "root-level"},
		{"root dotted key", "dependencies.mosaic-pkg-sheet = \"0.1.0\"\n[package]\nname = \"task-app\"\n", "root-level"},
		{"basic multi-line string", "[package]\nname = \"task-app\"\ndescription = \"\"\"\nx = [\n\"\"\"\n[dependencies]\nmosaic-pkg-sheet = \"0.1.0\"\n", "multi-line"},
		{"literal multi-line string", "[package]\nname = \"task-app\"\nnotes = \x27\x27\x27\n[dependencies]\n\x27\x27\x27\n", "multi-line"},
		{"escaped key", "[package]\nname = \"task-app\"\n[dependencies]\n\"mosaic-pkg-\\u0073heet\" = \"0.1.0\"\n", "escape"},
	}
	for index, tc := range cases {
		t.Run(tc.name, func(t *testing.T) {
			_, packages := mosaicWorld(t, tc.manifest)
			known := buildKnownNamesForLanguage(packages, "mosaic")
			_, err := parseMosaicDeps(packages[3], known)
			if index == 0 {
				// Control: the plain form still parses.
				if err != nil {
					t.Fatalf("plain manifest refused: %v", err)
				}
				return
			}
			var manifestErr *MosaicManifestError
			if !errors.As(err, &manifestErr) || !strings.Contains(err.Error(), tc.reason) {
				t.Fatalf("expected a MosaicManifestError mentioning %q, got %v", tc.reason, err)
			}
			if _, resolveErr := ResolveDependencies(packages); resolveErr == nil {
				t.Fatal("ResolveDependencies must stop on a refused manifest")
			}
		})
	}
}

// TestParseMosaicDepsRefusesASymlinkedOrHugeManifest: the manifest is read
// only as a bounded regular file.
func TestParseMosaicDepsRefusesASymlinkedOrHugeManifest(t *testing.T) {
	dir := t.TempDir()
	target := filepath.Join(t.TempDir(), "elsewhere.toml")
	if err := os.WriteFile(target, []byte("[package]\nname = \"x\"\n"), 0o644); err != nil {
		t.Fatal(err)
	}
	if err := os.Symlink(target, filepath.Join(dir, mosaicManifestName)); err != nil {
		t.Skipf("symlinks unavailable: %v", err)
	}
	pkg := discovery.Package{Name: "mosaic/linked", Path: dir, Language: "mosaic"}
	if _, err := parseMosaicDeps(pkg, nil); err == nil || !strings.Contains(err.Error(), "not a regular file") {
		t.Fatalf("expected a symlinked manifest to be refused, got %v", err)
	}

	big := t.TempDir()
	huge := "[package]\nname = \"big\"\n# " + strings.Repeat("x", maxMosaicManifestBytes) + "\n"
	if err := os.WriteFile(filepath.Join(big, mosaicManifestName), []byte(huge), 0o644); err != nil {
		t.Fatal(err)
	}
	pkg = discovery.Package{Name: "mosaic/big", Path: big, Language: "mosaic"}
	if _, err := parseMosaicDeps(pkg, nil); err == nil || !strings.Contains(err.Error(), "larger than") {
		t.Fatalf("expected an oversized manifest to be refused, got %v", err)
	}
}

// TestResolveDependenciesRefusesACapturedMosaicName: a second library that
// claims the toolkit's name must stop the plan, not silently take over the
// edges of everything that requires the toolkit.
func TestResolveDependenciesRefusesACapturedMosaicName(t *testing.T) {
	root := makeFixture(t, map[string]string{
		"code/packages/mosaic/mosaic-pkg-toolkit/mosaic-package.toml": "[package]\nname = \"mosaic-pkg-toolkit\"\n",
		"code/packages/mosaic/zzz/mosaic-package.toml":                "[package]\nname = \"mosaic-pkg-toolkit\"\n",
	})
	packages := []discovery.Package{
		{Name: "mosaic/mosaic-pkg-toolkit", Path: filepath.Join(root, "code/packages/mosaic/mosaic-pkg-toolkit"), Language: "mosaic"},
		{Name: "mosaic/zzz", Path: filepath.Join(root, "code/packages/mosaic/zzz"), Language: "mosaic"},
	}
	_, err := ResolveDependencies(packages)
	if err == nil || !strings.Contains(err.Error(), `claims the Mosaic package name "mosaic-pkg-toolkit"`) {
		t.Fatalf("expected a name-collision error, got %v", err)
	}
}

// TestResolveDependenciesAddsMosaicManifestEdges proves the end-to-end shape
// the CI gap needed: editing the toolkit affects the package that requires it
// and, transitively, the app that requires that package.
func TestResolveDependenciesAddsMosaicManifestEdges(t *testing.T) {
	_, packages := mosaicWorld(t, `[package]
name = "task-app"
[dependencies]
mosaic-pkg-sheet = "0.1.0"
mosaic-pkg-calendar = "0.1.0"
`)
	graph := mustResolveDependencies(t, packages)

	for _, edge := range [][2]string{
		{"mosaic/mosaic-pkg-toolkit", "mosaic/mosaic-pkg-sheet"},
		{"mosaic/mosaic-pkg-sheet", "mosaic/programs/task-app"},
		{"mosaic/calendar", "mosaic/programs/task-app"},
	} {
		if !graph.HasEdge(edge[0], edge[1]) {
			t.Fatalf("missing edge %s -> %s; edges %v", edge[0], edge[1], graph.Edges())
		}
	}

	affected := graph.AffectedNodesList(map[string]bool{"mosaic/mosaic-pkg-toolkit": true})
	want := []string{"mosaic/mosaic-pkg-sheet", "mosaic/mosaic-pkg-toolkit", "mosaic/programs/task-app"}
	if !reflect.DeepEqual(affected, want) {
		t.Fatalf("toolkit edit affects %v, want %v", affected, want)
	}
	if graph.HasEdge("rust/mosaic-pkg-grid", "mosaic/programs/task-app") {
		t.Fatal("Mosaic dependency resolution leaked into the Rust scope")
	}
}

// TestBuildKnownNamesMosaicPrefersPackagesOverPrograms mirrors the rule every
// other ecosystem follows: when a library and an app share a name, a
// dependency means the library.
func TestBuildKnownNamesMosaicPrefersPackagesOverPrograms(t *testing.T) {
	root := makeFixture(t, map[string]string{
		"code/programs/mosaic/notes/mosaic-package.toml": "[package]\nname = \"notes\"\n",
		"code/packages/mosaic/notes/mosaic-package.toml": "[package]\nname = \"notes\"\n",
	})
	packages := []discovery.Package{
		{Name: "mosaic/programs/notes", Path: filepath.Join(root, "code/programs/mosaic/notes"), Language: "mosaic"},
		{Name: "mosaic/notes", Path: filepath.Join(root, "code/packages/mosaic/notes"), Language: "mosaic"},
	}
	known := buildKnownNamesForLanguage(packages, "mosaic")
	if known["notes"] != "mosaic/notes" {
		t.Fatalf("expected the library to own the name, got %q", known["notes"])
	}
}

func TestSplitTomlKeyPath(t *testing.T) {
	cases := map[string][]string{
		"dependencies":                  {"dependencies"},
		" dependencies . mosaic-pkg-x ": {"dependencies", "mosaic-pkg-x"},
		`"dependencies"."mosaic.pkg"`:   {"dependencies", "mosaic.pkg"},
		`'literal'`:                     {"literal"},
		`"esc\"aped"`:                   {`esc"aped`},
		`"unterminated`:                 nil,
		"a..b":                          nil,
		"":                              nil,
	}
	for input, want := range cases {
		if got := splitTomlKeyPath(input); !reflect.DeepEqual(got, want) {
			t.Errorf("splitTomlKeyPath(%q) = %#v, want %#v", input, got, want)
		}
	}
}
