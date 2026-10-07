package main

import (
	"os"
	"path/filepath"
	"strings"
	"testing"
)

func TestParsePackageSearchPaths(t *testing.T) {
	one := t.TempDir()
	two := t.TempDir()
	list := strings.Join([]string{one, "", two, one}, string(os.PathListSeparator))
	got, err := parsePackageSearchPaths(list)
	if err != nil {
		t.Fatalf("parse: %v", err)
	}
	if len(got) != 2 || got[0] != one || got[1] != two {
		t.Fatalf("got %q, want [%q %q]: in order, empty skipped, no repeats", got, one, two)
	}
	if got, err := parsePackageSearchPaths(""); err != nil || len(got) != 0 {
		t.Fatalf("empty list: got %q, %v", got, err)
	}
}

func TestParsePackageSearchPathsRefusesWhatTheCompilerCannotTake(t *testing.T) {
	file := filepath.Join(t.TempDir(), "not-a-dir")
	mustWrite(t, file, "x")
	for _, entry := range []string{file, filepath.Join(t.TempDir(), "missing")} {
		if _, err := parsePackageSearchPaths(entry); err == nil {
			t.Errorf("%q: accepted, want refused as not a directory", entry)
		}
	}
	// mosaic-compile splits on ':' on every OS, so a directory whose
	// absolute path holds one is refused rather than split in two.
	colon := filepath.Join(t.TempDir(), "a:b")
	if err := os.Mkdir(colon, 0o755); err != nil {
		t.Skipf("this file system refuses ':' in a name: %v", err)
	}
	if _, err := packageSearchDir(colon); err == nil ||
		!strings.Contains(err.Error(), "list separator") {
		t.Fatalf("%q: got %v, want refused for its ':'", colon, err)
	}
}

func TestWithPackageSearchPaths(t *testing.T) {
	for _, tc := range []struct {
		base   string
		extras []string
		want   string
	}{
		{"", nil, ""},
		{"/a", nil, "/a"},
		{"/a", []string{"/b", "/a"}, "/a:/b"},
		{"", []string{"/b"}, "/b"},
	} {
		if got := withPackageSearchPaths(tc.base, tc.extras); got != tc.want {
			t.Errorf("(%q, %q): got %q, want %q", tc.base, tc.extras, got, tc.want)
		}
	}
}

// An app's package and its dependencies live in different trees, as
// code/programs/mosaic and code/packages/mosaic do. The extra path reaches the
// compiler after the app's own siblings; a component with no package is left
// on the compiler's default search.
func TestPackagedComponentsSearchTheExtraPathsAfterTheirSiblings(t *testing.T) {
	programs := t.TempDir()
	packages := t.TempDir()
	app := filepath.Join(programs, "spreadsheet")
	if err := os.MkdirAll(filepath.Join(app, "src"), 0o755); err != nil {
		t.Fatalf("mkdir: %v", err)
	}
	mustWrite(t, filepath.Join(app, "mosaic-package.toml"), `[package]
name = "spreadsheet"

[components]
exports = ["Sheet"]

[dependencies]
mosaic-pkg-grid = "0.2.0"
`)
	writeThreeFileComponent(t, filepath.Join(app, "src"), "Sheet", ".light.msl")
	loose := filepath.Join(programs, "loose")
	if err := os.MkdirAll(loose, 0o755); err != nil {
		t.Fatalf("mkdir: %v", err)
	}
	writeThreeFileComponent(t, loose, "Loose", ".light.msl")

	server := &Server{root: programs, compilerPath: "unused", packageSearchPaths: []string{packages}}
	comps, err := server.discoverValidatedComponents()
	if err != nil {
		t.Fatalf("discover: %v", err)
	}
	byID := map[string]Component{}
	for _, comp := range comps {
		byID[comp.ID] = comp
	}
	sheet, ok := byID["spreadsheet/src/Sheet"]
	if !ok {
		t.Fatalf("Sheet not discovered: %v", comps)
	}
	if want := programs + ":" + packages; sheet.PackageSearchPath != want {
		t.Fatalf("Sheet search path: got %q, want %q (siblings first)", sheet.PackageSearchPath, want)
	}
	args := strings.Join(compilerArgs(sheet, "html", "out.html", "", false), " ")
	if !strings.Contains(args, "--package-search-path "+programs+":"+packages) {
		t.Fatalf("compiler args: %s", args)
	}
	if got := byID["loose/Loose"].PackageSearchPath; got != "" {
		t.Fatalf("a component with no package: search path %q, want the compiler default", got)
	}
}
