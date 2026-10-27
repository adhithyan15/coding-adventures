package resolver

// Regression tests for the round-1 security review of the Cargo
// dev-dependency reader. Each crafted manifest is one the reviewer showed
// slipping past the reader: a trigger silently dropped (so a dependent's
// tests never re-run) or silently redirected to the wrong package. Every
// test here failed before the fix it names.

import (
	"errors"
	"os"
	"path/filepath"
	"reflect"
	"strings"
	"testing"

	"github.com/adhithyan15/coding-adventures/code/programs/go/build-tool/internal/discovery"
)

// wantSubjectTriggers resolves devDepWorld with the given subject manifest
// and requires rust/subject to be a dev-dependent of exactly the packages in
// want (and of nothing else).
func wantSubjectTriggers(t *testing.T, subjectCargo string, want ...string) {
	t.Helper()
	packages := devDepWorld(t, subjectCargo)
	_, triggers, err := ResolveWithTestTriggers(packages)
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	var got []string
	for dep, dependents := range triggers {
		for _, dependent := range dependents {
			if dependent == "rust/subject" {
				got = append(got, dep)
			}
		}
	}
	if !sameSet(got, want) {
		t.Fatalf("rust/subject dev-depends on %v, want %v", got, want)
	}
}

// wantRefused requires resolution of devDepWorld with the given subject
// manifest to stop with a *CargoManifestError naming the subject's manifest.
func wantRefused(t *testing.T, subjectCargo string) {
	t.Helper()
	packages := devDepWorld(t, subjectCargo)
	_, _, err := ResolveWithTestTriggers(packages)
	var manifestErr *CargoManifestError
	if !errors.As(err, &manifestErr) {
		t.Fatalf("want *CargoManifestError, got %v", err)
	}
	if !strings.Contains(filepath.ToSlash(manifestErr.Path), "rust/subject/Cargo.toml") {
		t.Fatalf("error names %s, want the subject manifest", manifestErr.Path)
	}
}

func sameSet(a, b []string) bool {
	seen := map[string]int{}
	for _, x := range a {
		seen[x]++
	}
	for _, x := range b {
		seen[x]--
	}
	for _, count := range seen {
		if count != 0 {
			return false
		}
	}
	return true
}

// Finding 1: a quoted header that decodes to [dev-dependencies] IS the table.
func TestSecurityQuotedDevDependencyHeadersAreRead(t *testing.T) {
	for _, header := range []string{`["dev-dependencies"]`, `['dev-dependencies']`, `[ "dev-dependencies" ]`, `[dev_dependencies]`} {
		t.Run(header, func(t *testing.T) {
			wantSubjectTriggers(t, plainSubject+"\n"+header+"\ncompiler = { path = \"../compiler\" }\n", "rust/compiler", "rust/emit")
		})
	}
}

// Finding 1: quoted sub-tables, escapes and undecodable headers are refused.
func TestSecurityQuotedOrUndecodableHeadersAreRefused(t *testing.T) {
	for name, header := range map[string]string{
		"quoted sub-table":    `["dev-dependencies".compiler]`,
		"literal sub-table":   `['dev-dependencies'.compiler]`,
		"escaped header":      `["dev-dependenci\u0065s"]`,
		"unterminated quote":  `["dev-dependencies]`,
		"unclosed header":     `[dev-dependencies`,
		"empty segment":       `[dev-dependencies..compiler]`,
		"array of dev tables": `[[dev-dependencies]]`,
		"stray bracket":       `[dev-dependencies]]`,
	} {
		t.Run(name, func(t *testing.T) {
			wantRefused(t, plainSubject+"\n"+header+"\ncompiler = { path = \"../compiler\" }\n")
		})
	}
}

// Finding 2: a multi-line string can hide brackets (and so a whole table)
// from a line reader. Any triple quote is refused, as is a file that ends with
// a value still open.
func TestSecurityMultiLineStringsAndOpenBracketsAreRefused(t *testing.T) {
	for name, cargo := range map[string]string{
		"bracket inside multi-line string": "[package]\nname = \"subject\"\ndescription = \"\"\"\nx = [\n\"\"\"\n\n[dev-dependencies]\ncompiler = { path = \"../compiler\" }\n",
		"quote inside one-line triple":     "[package]\nname = \"subject\"\ndescription = \"\"\"a\"[\" \"\"\"\n\n[dev-dependencies]\ncompiler = { path = \"../compiler\" }\n",
		"literal triple quote":             "[package]\nname = \"subject\"\ndescription = '''\n[x\n'''\n\n[dev-dependencies]\ncompiler = { path = \"../compiler\" }\n",
		"bracket left open at end of file": plainSubject + "keywords = [\n\"a\",\n\n[dev-dependencies]\ncompiler = { path = \"../compiler\" }\n",
		"unterminated one-line string":     plainSubject + "description = \"open\n\n[dev-dependencies]\ncompiler = { path = \"../compiler\" }\n",
	} {
		t.Run(name, func(t *testing.T) { wantRefused(t, cargo) })
	}
}

// Finding 3: every inline-table field key must be a bare, exactly-lowercase
// key Cargo would read, and no field may be a nested inline table.
func TestSecurityInlineTableFieldKeysAreValidated(t *testing.T) {
	for name, entry := range map[string]string{
		"quoted path key":     `compiler = { "path" = "../compiler" }`,
		"literal path key":    `compiler = { 'path' = "../compiler" }`,
		"quoted package key":  `compiler = { "package" = "helper-crate", path = "../compiler" }`,
		"capitalised package": `compiler = { Package = "helper-crate", path = "../compiler" }`,
		"capitalised path":    `compiler = { Path = "../compiler" }`,
		"dotted field":        `compiler = { path.x = "../compiler" }`,
		"nested inline table": `compiler = { path = "../compiler", features = { a = 1 } }`,
		"duplicate field":     `compiler = { path = "../helper", path = "../compiler" }`,
		"trailing comma":      `compiler = { path = "../compiler", }`,
		"non-string path":     `compiler = { path = 1 }`,
		"escape in a string":  `compiler = { path = "..\u002fcompiler" }`,
		"unknown value shape": `compiler = { path = "../compiler", optional = yes }`,
	} {
		t.Run(name, func(t *testing.T) {
			wantRefused(t, plainSubject+"\n[dev-dependencies]\n"+entry+"\n")
		})
	}
	// The well-formed spellings still resolve, including a features array
	// that spans lines (valid TOML: the newlines are inside an array value).
	wantSubjectTriggers(t, plainSubject+`
[dev-dependencies]
compiler = { path = "../compiler", default-features = false, features = [
    "a", # why a
    "b",
] }
renamed = { package = "helper-crate", path = '../helper', optional = true }
`, "rust/compiler", "rust/emit", "rust/helper")
}

// Finding 4: target-specific dev-dependency tables are read with the same
// strict rules, union over every target.
func TestSecurityTargetDevDependencyTablesAreRead(t *testing.T) {
	wantSubjectTriggers(t, plainSubject+`
[target.'cfg(unix)'.dev-dependencies]
compiler = { path = "../compiler" }

["target"."cfg(windows)"."dev-dependencies"]
helper-crate = { path = "../helper" }

[target.x86_64-pc-windows-msvc.dev-dependencies]
windows-sys = { version = "0.52", features = [
    "Win32_Foundation",
] }
`, "rust/compiler", "rust/emit", "rust/helper")

	for name, cargo := range map[string]string{
		"target sub-table":          "[target.'cfg(unix)'.dev-dependencies.compiler]\npath = \"../compiler\"\n",
		"quoted target sub-table":   "[\"target\".'cfg(unix)'.\"dev-dependencies\".compiler]\npath = \"../compiler\"\n",
		"dotted key under target":   "[target.'cfg(unix)']\ndev-dependencies.compiler = { path = \"../compiler\" }\n",
		"inline table under target": "[target.'cfg(unix)']\ndev-dependencies = { compiler = { path = \"../compiler\" } }\n",
		"target as inline table":    "target = { 'cfg(unix)' = { dev-dependencies = { compiler = { path = \"../compiler\" } } } }\n",
		"target cfg inline table":   "[target]\n'cfg(unix)' = { dev-dependencies = { compiler = { path = \"../compiler\" } } }\n",
		"multi-line inline entry":   "[target.'cfg(unix)'.dev-dependencies]\ncompiler = { path = \"../compiler\",\n  features = [] }\n",
	} {
		// Root-level spellings must come before the first header, so the
		// crafted text goes first and [package] after it.
		t.Run(name, func(t *testing.T) { wantRefused(t, cargo+"\n"+plainSubject) })
	}
}

// Finding 5: a path entry resolves by its path, so another crate claiming the
// same Cargo name cannot steal the trigger.
func TestSecurityPathEntriesResolveByPathNotName(t *testing.T) {
	root := makeFixture(t, map[string]string{
		"code/packages/rust/compiler/Cargo.toml": "[package]\nname = \"compiler\"\nversion = \"0.1.0\"\n",
		// Discovered later, claims the same Cargo name.
		"code/packages/rust/zz-impostor/Cargo.toml": "[package]\nname = \"compiler\"\nversion = \"0.1.0\"\n",
		"code/packages/rust/subject/Cargo.toml": plainSubject +
			"\n[dev-dependencies]\ncompiler = { path = \"../compiler\" }\n",
	})
	at := func(rel string) string { return filepath.Join(root, filepath.FromSlash(rel)) }
	packages := []discovery.Package{
		{Name: "rust/compiler", Path: at("code/packages/rust/compiler"), Language: "rust"},
		{Name: "rust/subject", Path: at("code/packages/rust/subject"), Language: "rust"},
		{Name: "rust/zz-impostor", Path: at("code/packages/rust/zz-impostor"), Language: "rust"},
	}
	_, triggers, err := ResolveWithTestTriggers(packages)
	if err != nil {
		t.Fatal(err)
	}
	if want := (TestTriggers{"rust/compiler": {"rust/subject"}}); !reflect.DeepEqual(triggers, want) {
		t.Fatalf("triggers = %v, want %v", triggers, want)
	}
}

// Finding 5: how a path that is not exactly a package resolves.
func TestSecurityPathEntryResolution(t *testing.T) {
	// Into a package's sub-directory: the containing package, because a change
	// there is attributed to that package by prefix matching. (Round 2: the
	// nested crate must exist, and its own dependencies are followed.)
	world := plainSubject + "\n[dev-dependencies]\nnested = { path = \"../compiler/crates/nested\" }\n"
	wantSubjectTriggersWith(t, world, map[string]string{
		"code/packages/rust/compiler/crates/nested/Cargo.toml": "[package]\nname = \"nested\"\n",
	}, "rust/compiler")

	// A crate directory that is not a discovered package resolves to no
	// trigger of its own only when code/BUILD-EXEMPTIONS lists it (round 2
	// finding A; see TestSecurityRound2* for what is then read inside it).
	packages := devDepWorldWith(t, plainSubject+"\n[dev-dependencies]\norphan = { path = \"../orphan\" }\n", map[string]string{
		".git/HEAD":                            "ref: refs/heads/main\n",
		"code/BUILD-EXEMPTIONS":                "PENDING code/packages/rust/orphan  # test orphan\n",
		"code/packages/rust/orphan/Cargo.toml": "[package]\nname = \"orphan\"\n",
	})
	if _, triggers, err := ResolveWithTestTriggers(packages); err != nil || len(triggers.Dependents("rust/subject")) != 0 {
		t.Fatalf("a listed orphan crate with no dependencies should resolve to nothing, got %v / %v", triggers, err)
	}

	for name, entry := range map[string]string{
		"missing target":   `ghost = { path = "../ghost" }`,
		"absolute path":    `compiler = { path = "/tmp/compiler" }`,
		"empty path":       `compiler = { path = "" }`,
		"workspace member": `compiler = { workspace = true }`,
	} {
		t.Run(name, func(t *testing.T) { wantRefused(t, plainSubject+"\n[dev-dependencies]\n"+entry+"\n") })
	}
}

// Finding 6: a Mosaic harness's [dependencies] and [build-dependencies] path
// entries are triggers too (the harness has no build-graph edges of its own).
func TestSecurityMosaicHarnessNormalAndBuildDependenciesAreTriggers(t *testing.T) {
	root := makeFixture(t, map[string]string{
		"code/packages/rust/compiler/Cargo.toml":         "[package]\nname = \"compiler\"\n",
		"code/packages/rust/helper/Cargo.toml":           "[package]\nname = \"helper\"\n",
		"code/packages/rust/emit/Cargo.toml":             "[package]\nname = \"emit\"\n",
		"code/packages/rust/gen/Cargo.toml":              "[package]\nname = \"gen\"\n",
		"code/packages/mosaic/pkg-b/mosaic-package.toml": "[package]\nname = \"pkg-b\"\nversion = \"0.1.0\"\n",
		"code/packages/mosaic/pkg-b/Cargo.toml": "[workspace]\n\n[package]\nname = \"pkg-b\"\n\n" +
			"[dependencies]\ncompiler = { path = \"../../rust/compiler\" }\n\n" +
			"[build-dependencies]\nhelper = { path = \"../../rust/helper\" }\n\n" +
			"[target.'cfg(unix)'.dependencies]\nemit = { path = \"../../rust/emit\" }\n\n" +
			"[build_dependencies]\ngen = { path = \"../../rust/gen\" }\n",
	})
	at := func(rel string) string { return filepath.Join(root, filepath.FromSlash(rel)) }
	packages := []discovery.Package{
		{Name: "rust/compiler", Path: at("code/packages/rust/compiler"), Language: "rust"},
		{Name: "rust/helper", Path: at("code/packages/rust/helper"), Language: "rust"},
		{Name: "rust/emit", Path: at("code/packages/rust/emit"), Language: "rust"},
		{Name: "rust/gen", Path: at("code/packages/rust/gen"), Language: "rust"},
		{Name: "mosaic/pkg-b", Path: at("code/packages/mosaic/pkg-b"), Language: "mosaic"},
	}
	_, triggers, err := ResolveWithTestTriggers(packages)
	if err != nil {
		t.Fatal(err)
	}
	for _, dep := range []string{"rust/compiler", "rust/helper", "rust/emit", "rust/gen"} {
		if got := triggers.Dependents(dep); !reflect.DeepEqual(got, []string{"mosaic/pkg-b"}) {
			t.Fatalf("%s dependents = %v, want [mosaic/pkg-b]", dep, got)
		}
	}
}

// Finding 7: a leading UTF-8 byte-order mark is stripped, as TOML allows.
func TestSecurityLeadingByteOrderMarkIsStripped(t *testing.T) {
	wantSubjectTriggers(t, "\xEF\xBB\xBF[dev-dependencies]\ncompiler = { path = \"../compiler\" }\n\n"+plainSubject, "rust/compiler", "rust/emit")
}

// Finding 8: the read is bounded, and a manifest that is not a regular file
// is refused rather than followed.
func TestSecurityOversizedOrIrregularManifestsAreRefused(t *testing.T) {
	huge := plainSubject + "# " + strings.Repeat("x", maxCargoManifestBytes) + "\n[dev-dependencies]\ncompiler = { path = \"../compiler\" }\n"
	wantRefused(t, huge)

	packages := devDepWorld(t, plainSubject)
	subjectManifest := filepath.Join(packages[4].Path, "Cargo.toml")
	if err := os.Remove(subjectManifest); err != nil {
		t.Fatal(err)
	}
	if err := os.Symlink(filepath.Join(packages[1].Path, "Cargo.toml"), subjectManifest); err != nil {
		t.Skipf("symlinks unavailable: %v", err)
	}
	var manifestErr *CargoManifestError
	if _, _, err := ResolveWithTestTriggers(packages); !errors.As(err, &manifestErr) {
		t.Fatalf("a symlinked Cargo.toml should be refused, got %v", err)
	}
}
