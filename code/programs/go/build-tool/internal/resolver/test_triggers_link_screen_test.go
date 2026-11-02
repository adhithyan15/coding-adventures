package resolver

// Regression tests for the security review of the Cargo link-table edges
// (build-dependencies and target tables as graph edges). The build graph's
// edge reader, parseRustDeps, follows a LENIENT cross-implementation line
// contract, so a TOML spelling outside that contract contributes no edge —
// a silently dropped edge, and a dependent CI never rebuilds. The strict
// test-trigger reader screens the same link tables of every Rust and WASM
// package and must refuse each such spelling, while accepting every shape
// the contract does model. Every "refused" case below resolved without an
// error (and without the edge) before the screen existed, except the root
// `target = { ... }` inline table and the multi-line strings, which the
// reader already refused; they are pinned here so the backstop the edge
// reader relies on cannot quietly go away.

import (
	"errors"
	"path/filepath"
	"testing"
)

// TestLinkScreenRefusesShapesTheEdgeReaderDrops proves each Cargo-valid
// spelling the lenient reader cannot see stops the plan for a Rust package.
func TestLinkScreenRefusesShapesTheEdgeReaderDrops(t *testing.T) {
	for name, body := range map[string]string{
		// Dotted keys inside a read table.
		"dotted key in [build-dependencies]": "[build-dependencies]\nemit.path = \"../emit\"\n",
		"dotted key in [dependencies]":       "[dependencies]\nemit.path = \"../emit\"\n",
		"dotted key in a target table":       "[target.'cfg(unix)'.dependencies]\nemit.path = \"../emit\"\n",
		"dotted key in a sub-table":          "[dependencies.emit]\npath.x = \"../emit\"\n",
		// Root-level and other-table dotted keys that reach a link table.
		"root dotted key":                 "build-dependencies.emit.path = \"../emit\"\n" + plainSubject,
		"root dotted inline table":        "dependencies.emit = { path = \"../emit\" }\n" + plainSubject,
		"root link table as inline table": "dependencies = { emit = { path = \"../emit\" } }\n" + plainSubject,
		"dotted key under [target.C]":     "[target.'cfg(unix)']\ndependencies.emit = { path = \"../emit\" }\n",
		"dotted key under [target]":       "[target]\n'cfg(unix)'.build-dependencies.emit.path = \"../emit\"\n",
		"root dotted target key":          "target.'cfg(unix)'.dependencies.emit.path = \"../emit\"\n" + plainSubject,
		// Inline tables that spell a link table.
		"inline build-dependencies under [target.C]": "[target.'cfg(unix)']\nbuild-dependencies = { emit = { path = \"../emit\" } }\n",
		"root target inline table":                   "target = { 'cfg(unix)' = { dependencies = { emit = { path = \"../emit\" } } } }\n" + plainSubject,
		// Headers and arrays of tables deeper than the contract reads.
		"features sub-table":    "[dependencies.emit.features]\nx = []\n",
		"array of link tables":  "[[dependencies]]\nemit = { path = \"../emit\" }\n",
		"array of sub-tables":   "[[target.'cfg(unix)'.build-dependencies.emit]]\npath = \"../emit\"\n",
		"deep target sub-table": "[target.'cfg(unix)'.dependencies.emit.x]\npath = \"../emit\"\n",
		// Values the lenient reader does not see as Cargo does.
		"array entry value":                 "[dependencies]\njunk = [\n[x]\n]\nemit = { path = \"../emit\" }\n",
		"number entry value":                "[dependencies]\nemit = 1\n",
		"sub-table value spanning lines":    "[dependencies.emit]\nfeatures = [\n[x]\n]\npath = \"../emit\"\n",
		"escape in an entry":                "[dependencies]\nemit = { path = \"..\\u002femit\" }\n",
		"escaped field key":                 "[dependencies]\nx = { \"pack\\u0061ge\" = \"emit\", path = \"../emit\" }\n",
		"escape in a sub-table value":       "[dependencies.x]\npackage = \"emi\\u0074\"\npath = \"../emit\"\n",
		"dotted field key":                  "[dependencies]\nemit = { path.x = \"../emit\" }\n",
		"trailing comma in an inline table": "[dependencies]\nemit = { path = \"../emit\", }\n",
		// Workspace inheritance: Cargo takes the path from the workspace
		// root's [workspace.dependencies], which the edge reader never opens.
		"inherited inline entry":         "[dependencies]\nemit = { workspace = true }\n",
		"inherited quoted inline key":    "[build-dependencies]\nemit = { \"workspace\" = true, features = [\"x\"] }\n",
		"inherited target entry":         "[target.'cfg(unix)'.dependencies]\nemit = { workspace = true }\n",
		"inherited sub-table":            "[dependencies.emit]\nworkspace = true\n",
		"inherited quoted sub-table key": "[target.'cfg(unix)'.build-dependencies.emit]\n'workspace' = true\n",
	} {
		t.Run(name, func(t *testing.T) {
			manifest := body
			if body[0] == '[' {
				manifest = plainSubject + "\n" + body
			}
			wantRefused(t, manifest)
		})
	}
}

// TestLinkScreenRefusesMultiLineStrings pins item 4 of the review: a
// multi-line string can hide a header from the lenient reader (a line inside
// it that looks like `[package]` would end [dependencies] early), and the
// strict reader refuses one anywhere in a Rust or WASM manifest.
func TestLinkScreenRefusesMultiLineStrings(t *testing.T) {
	for _, delimiter := range []string{`"""`, `'''`} {
		t.Run(delimiter, func(t *testing.T) {
			wantRefused(t, plainSubject+"description = "+delimiter+"\n[package]\n"+delimiter+"\n\n"+
				"[dependencies]\nemit = { path = \"../emit\" }\n")
		})
	}
}

// TestLinkScreenAcceptsModelledShapes proves the screen refuses nothing the
// contract reads: inline entries (quoted keys and multi-line arrays and
// tables included), registry versions, every link-table spelling, and
// sub-tables with any one-segment keys. Each manifest resolves, and its
// edges are exactly the ones the lenient reader draws.
func TestLinkScreenAcceptsModelledShapes(t *testing.T) {
	cases := map[string]struct {
		body string
		want []string
	}{
		"inline entries": {"[dependencies]\nemit = { path = \"../emit\", features = [\n  \"x\", # [not-a-header]\n] }\n" +
			"\"compiler\" = { 'path' = \"../compiler\", default-features = false }\nserde = \"1\"\ntokio = { version = \"1\" }\n" +
			"empty = {}\n", []string{"rust/emit", "rust/compiler"}},
		"inline table spanning lines": {"[build-dependencies]\nemit = {\n  path = \"../emit\" }\n", []string{"rust/emit"}},
		"every link-table spelling": {"[build_dependencies]\nemit = { path = \"../emit\" }\n" +
			"[target.x86_64-unknown-linux-gnu.build-dependencies]\ncompiler = { path = \"../compiler\" }\n" +
			"[\"dependencies\"]\nserde = \"1\"\n", []string{"rust/emit", "rust/compiler"}},
		"sub-tables": {"[dependencies.renamed]\n\"package\" = \"helper-crate\"\npath = \"../helper\"\nfeatures = [\"x\"]\n" +
			"optional = true\n[target.'cfg(unix)'.build-dependencies.emit]\n'path' = \"../emit\"\nversion = \"1\"\n",
			[]string{"rust/helper", "rust/emit"}},
		"target and other tables": {"[target.'cfg(unix)']\nrustflags = [\"-C\", \"x\"]\n[features]\ndefault = [\"dependencies\"]\n" +
			"[package.metadata.dependencies]\nx = 1\n[dev-dependencies]\ncompiler = { path = \"../compiler\" }\n", nil},
	}
	for name, tc := range cases {
		t.Run(name, func(t *testing.T) {
			packages := devDepWorld(t, plainSubject+"\n"+tc.body)
			graph, _, err := ResolveWithTestTriggers(packages)
			if err != nil {
				t.Fatalf("a modelled shape was refused: %v", err)
			}
			got, err := graph.Predecessors("rust/subject")
			if err != nil {
				t.Fatal(err)
			}
			if !sameSet(got, tc.want) {
				t.Fatalf("rust/subject edges from %v, want %v", got, tc.want)
			}
		})
	}
}

// TestLinkScreenOnlyScreensRustAndWasmRoots checks the scope: a WASM
// package is screened like a Rust one, and the screen is the only thing
// that refuses the shape — the same manifest read without it (as for a
// crate reached through a path entry, whose link tables are read strictly as
// trigger tables instead) passes.
func TestLinkScreenOnlyScreensRustAndWasmRoots(t *testing.T) {
	packages := devDepWorld(t, plainSubject+"\n[build-dependencies]\nemit.path = \"../emit\"\n")
	subject := packages[len(packages)-1]
	subject.Language = "wasm"
	packages[len(packages)-1] = subject
	_, _, err := ResolveWithTestTriggers(packages)
	var manifestErr *CargoManifestError
	if !errors.As(err, &manifestErr) {
		t.Fatalf("a WASM package's dotted link key = %v, want a *CargoManifestError", err)
	}

	manifest := filepath.Join(subject.Path, "Cargo.toml")
	if _, _, err := readCargoTriggerEntries(manifest, triggerTableNames("rust"), false); err != nil {
		t.Fatalf("unscreened read = %v, want no error", err)
	}
	if _, _, err := readCargoTriggerEntries(manifest, triggerTableNames("rust"), true); !errors.As(err, &manifestErr) {
		t.Fatalf("screened read = %v, want a *CargoManifestError", err)
	}
}
