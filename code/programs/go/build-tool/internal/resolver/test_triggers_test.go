package resolver

import (
	"errors"
	"path/filepath"
	"reflect"
	"testing"

	"github.com/adhithyan15/coding-adventures/code/programs/go/build-tool/internal/discovery"
)

// devDepWorld lays out the repository's real shape in miniature:
//
//	rust/emit       ← the backend an emitter change touches
//	rust/compiler   [dependencies] emit
//	rust/helper     plain crate, dev-depends on nothing
//	mosaic/pkg-a    Cargo smoke harness: [dev-dependencies] compiler
//
// plus whatever Cargo.toml the test supplies for rust/subject.
func devDepWorld(t *testing.T, subjectCargo string) []discovery.Package {
	t.Helper()
	return devDepWorldWith(t, subjectCargo, nil)
}

// devDepWorldWith is devDepWorld plus extra repository-relative files (for
// example a .git marker and code/BUILD-EXEMPTIONS, or crates that are not
// discovered packages).
func devDepWorldWith(t *testing.T, subjectCargo string, extra map[string]string) []discovery.Package {
	t.Helper()
	tree := map[string]string{
		"code/packages/rust/emit/Cargo.toml": "[package]\nname = \"emit\"\nversion = \"0.1.0\"\n",
		"code/packages/rust/compiler/Cargo.toml": "[package]\nname = \"compiler\"\nversion = \"0.1.0\"\n\n" +
			"[dependencies]\nemit = { path = \"../emit\" }\n",
		"code/packages/rust/helper/Cargo.toml":           "[package]\nname = \"helper-crate\"\nversion = \"0.1.0\"\n",
		"code/packages/mosaic/pkg-a/mosaic-package.toml": "[package]\nname = \"pkg-a\"\nversion = \"0.1.0\"\n",
		"code/packages/mosaic/pkg-a/Cargo.toml": "[workspace]\n\n[package]\nname = \"pkg-a\"\nversion = \"0.1.0\"\n\n" +
			"[dev-dependencies]\ncompiler = { path = \"../../rust/compiler\" }\ntempfile = \"3\"\n",
		"code/packages/rust/subject/Cargo.toml": subjectCargo,
	}
	for path, content := range extra {
		tree[path] = content
	}
	root := makeFixture(t, tree)
	at := func(rel string) string { return filepath.Join(root, filepath.FromSlash(rel)) }
	return []discovery.Package{
		{Name: "rust/emit", Path: at("code/packages/rust/emit"), Language: "rust"},
		{Name: "rust/compiler", Path: at("code/packages/rust/compiler"), Language: "rust"},
		{Name: "rust/helper", Path: at("code/packages/rust/helper"), Language: "rust"},
		{Name: "mosaic/pkg-a", Path: at("code/packages/mosaic/pkg-a"), Language: "mosaic"},
		{Name: "rust/subject", Path: at("code/packages/rust/subject"), Language: "rust"},
	}
}

const plainSubject = "[package]\nname = \"subject\"\nversion = \"0.1.0\"\n"

// TestDevDependenciesBecomeTestTriggersNotGraphEdges is the core contract:
// a path dev-dependency resolves (by key, or by `package =` rename) to a
// trigger, and never to a build-graph edge.
func TestDevDependenciesBecomeTestTriggersNotGraphEdges(t *testing.T) {
	packages := devDepWorld(t, plainSubject+`
[dependencies]
emit = { path = "../emit" }

[dev-dependencies]
compiler = { path = "../compiler", features = ["x"] } # a comment
renamed = { package = "helper-crate", path = "../helper" }
serde = "1"
external = { version = "1", git = "https://example.invalid/x" }

[target.'cfg(unix)'.dev-dependencies]
helper-crate = { path = "../helper" }

[[test]]
name = "smoke"
`)
	graph, triggers, err := ResolveWithTestTriggers(packages)
	if err != nil {
		t.Fatal(err)
	}

	want := TestTriggers{
		"rust/compiler": {"mosaic/pkg-a", "rust/subject"},
		// compiler's own [dependencies] are linked into both suites too.
		"rust/emit":   {"mosaic/pkg-a", "rust/subject"},
		"rust/helper": {"rust/subject"},
	}
	if !reflect.DeepEqual(triggers, want) {
		t.Fatalf("triggers = %v, want %v", triggers, want)
	}

	// The build graph is unchanged by dev-dependencies: only the two
	// [dependencies] edges exist.
	for _, edge := range [][2]string{{"rust/emit", "rust/compiler"}, {"rust/emit", "rust/subject"}} {
		if !graph.HasEdge(edge[0], edge[1]) {
			t.Fatalf("missing build edge %v", edge)
		}
	}
	for _, edge := range [][2]string{{"rust/compiler", "rust/subject"}, {"rust/helper", "rust/subject"}, {"rust/compiler", "mosaic/pkg-a"}} {
		if graph.HasEdge(edge[0], edge[1]) {
			t.Fatalf("dev-dependency %v leaked into the build graph", edge)
		}
	}
	if got := TestTriggers(nil).Dependents("rust/compiler"); got != nil {
		t.Fatalf("nil triggers should have no dependents, got %v", got)
	}
}

// TestDevDependencyCycleKeepsTheBuildGraphAcyclic reproduces the shape that
// exists in the repository today (closure-pass-dce ⇄
// closure-pass-fold-control-flow): B depends on A, A dev-depends on B. As
// graph edges that is a cycle and IndependentGroups would fail every package.
func TestDevDependencyCycleKeepsTheBuildGraphAcyclic(t *testing.T) {
	root := makeFixture(t, map[string]string{
		"code/packages/rust/a/Cargo.toml": "[package]\nname = \"a\"\nversion = \"0.1.0\"\n\n" +
			"[dev-dependencies]\nb = { path = \"../b\" }\na = { path = \".\" }\n",
		"code/packages/rust/b/Cargo.toml": "[package]\nname = \"b\"\nversion = \"0.1.0\"\n\n" +
			"[dependencies]\na = { path = \"../a\" }\n",
	})
	packages := []discovery.Package{
		{Name: "rust/a", Path: filepath.Join(root, "code/packages/rust/a"), Language: "rust"},
		{Name: "rust/b", Path: filepath.Join(root, "code/packages/rust/b"), Language: "rust"},
	}
	graph, triggers, err := ResolveWithTestTriggers(packages)
	if err != nil {
		t.Fatal(err)
	}
	if _, err := graph.IndependentGroups(); err != nil {
		t.Fatalf("a dev-dependency cycle must not make the build graph cyclic: %v", err)
	}
	// The self-reference is dropped; the cross edge survives as a trigger.
	if want := (TestTriggers{"rust/b": {"rust/a"}}); !reflect.DeepEqual(triggers, want) {
		t.Fatalf("triggers = %v, want %v", triggers, want)
	}
}

// TestMalformedDevDependenciesFailClosed lists every shape the reader does
// not model. Each must stop resolution with a *CargoManifestError, never
// resolve to "no triggers".
func TestMalformedDevDependenciesFailClosed(t *testing.T) {
	cases := map[string]string{
		"sub-table header":       "[dev-dependencies.compiler]\npath = \"../compiler\"\n",
		"spaced sub-table":       "[ dev-dependencies . compiler ]\npath = \"../compiler\"\n",
		"dotted key":             "[dev-dependencies]\ncompiler.path = \"../compiler\"\n",
		"quoted key":             "[dev-dependencies]\n\"compiler\" = { path = \"../compiler\" }\n",
		"multi-line inline":      "[dev-dependencies]\ncompiler = { path = \"../compiler\",\n  features = [\"x\"] }\n",
		"multi-line array value": "[dev-dependencies]\ncompiler = [\n  \"x\",\n]\n",
		"workspace inheritance":  "[dev-dependencies]\ncompiler = { workspace = true }\n",
		"boolean value":          "[dev-dependencies]\ncompiler = true\n",
		"bare line":              "[dev-dependencies]\ncompiler\n",
		"root inline table":      "dev-dependencies = { compiler = { path = \"../compiler\" } }\n" + plainSubject,
		"dotted from package":    plainSubject + "dev-dependencies.compiler = { path = \"../compiler\" }\n",
	}
	for name, cargo := range cases {
		t.Run(name, func(t *testing.T) {
			packages := devDepWorld(t, cargo)
			_, _, err := ResolveWithTestTriggers(packages)
			var manifestErr *CargoManifestError
			if !errors.As(err, &manifestErr) {
				t.Fatalf("want *CargoManifestError, got %v", err)
			}
			if filepath.Base(manifestErr.Path) != "Cargo.toml" || manifestErr.Reason == "" {
				t.Fatalf("error should name the manifest and a reason: %+v", manifestErr)
			}
			// ResolveDependencies fails the same way: no caller can get a
			// graph while a trigger is silently dropped.
			if _, err := ResolveDependencies(packages); !errors.As(err, &manifestErr) {
				t.Fatalf("ResolveDependencies should fail closed too, got %v", err)
			}
		})
	}
}

// TestDevDependencyReaderToleratesOtherTables proves the strictness is scoped
// to dev-dependency tables: multi-line values elsewhere (including array
// elements that start with '['), target tables and a missing Cargo.toml are
// all fine.
func TestDevDependencyReaderToleratesOtherTables(t *testing.T) {
	packages := devDepWorld(t, plainSubject+`keywords = [
  "a",
]

[package.metadata.matrix]
cells = [
  [1, 2],
  [3, 4],
]

[features]
default = ["x"]

[target.'cfg(windows)'.dev-dependencies]
windows-sys = { version = "0.52", features = [
    "Win32_Foundation",
] }

[ dev-dependencies ]
compiler = { path = "../compiler" }
`)
	// A Rust package with no Cargo.toml at all contributes nothing.
	packages = append(packages, discovery.Package{
		Name: "rust/no-manifest", Path: filepath.Join(filepath.Dir(packages[0].Path), "no-manifest"), Language: "rust",
	})
	_, triggers, err := ResolveWithTestTriggers(packages)
	if err != nil {
		t.Fatal(err)
	}
	if got, want := triggers.Dependents("rust/compiler"), []string{"mosaic/pkg-a", "rust/subject"}; !reflect.DeepEqual(got, want) {
		t.Fatalf("compiler dev-dependents = %v, want %v", got, want)
	}
}

// TestCargoPathDependencyName pins the shared entry rule both Cargo tables use.
func TestCargoPathDependencyName(t *testing.T) {
	cases := []struct {
		key, value, want string
		ok               bool
	}{
		{"logic-gates", `{ path = "../logic-gates" }`, "logic-gates", true},
		{"Gates", `{ package = "Logic-Gates", path = "../logic-gates" }`, "logic-gates", true},
		{"serde", `"1"`, "", false},
		{"x", `{ version = "1", features = ["path"] }`, "", false},
	}
	for _, tc := range cases {
		got, ok := cargoPathDependencyName(tc.key, tc.value)
		if got != tc.want || ok != tc.ok {
			t.Fatalf("cargoPathDependencyName(%q, %q) = %q, %v; want %q, %v", tc.key, tc.value, got, ok, tc.want, tc.ok)
		}
	}
}
