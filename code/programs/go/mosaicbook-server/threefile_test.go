// threefile_test.go — discovery and compilation of UI29 three-file components
//
// Every component in this repo is authored as separate .mil/.mll/.msl files
// inside a Mosaic package; there are no .mosaic files left anywhere in the
// tree.  MosaicBook originally discovered only .mosaic, which meant it could
// not display a single real component on any backend.  These tests cover the
// pairing rules and the compiler invocation that make them visible.

package main

import (
	"os"
	"path/filepath"
	"strings"
	"testing"
)

// writeThreeFileComponent creates a complete .mil/.mll/.msl set in dir.
// styleSuffix selects the stylesheet variant (".light.msl", ".dark.msl") or
// is empty to omit the stylesheet entirely.
func writeThreeFileComponent(t *testing.T, dir, name, styleSuffix string) {
	t.Helper()
	mustWrite(t, filepath.Join(dir, name+".mil"), "component "+name+" {}")
	mustWrite(t, filepath.Join(dir, name+".mll"), "layout "+name+" {}")
	if styleSuffix != "" {
		mustWrite(t, filepath.Join(dir, name+styleSuffix), "style "+name+" {}")
	}
}

func mustWrite(t *testing.T, path, content string) {
	t.Helper()
	if err := os.WriteFile(path, []byte(content), 0o644); err != nil {
		t.Fatalf("write %s: %v", path, err)
	}
}

// ── Discovery ─────────────────────────────────────────────────────────────

func TestDiscoverThreeFile_PairsSiblingsAndManifest(t *testing.T) {
	dir := t.TempDir()
	mustWrite(t, filepath.Join(dir, "mosaic-package.toml"), "[package]\nname = \"demo\"\n")
	writeThreeFileComponent(t, dir, "Button", ".light.msl")

	comps, err := discoverComponents(dir)
	if err != nil {
		t.Fatalf("discoverComponents: %v", err)
	}
	if len(comps) != 1 {
		t.Fatalf("expected 1 component, got %d", len(comps))
	}
	c := comps[0]

	if !c.isThreeFile() {
		t.Error("expected the component to be recognised as three-file")
	}
	if c.ID != "Button" {
		t.Errorf("ID: got %q, want %q", c.ID, "Button")
	}
	if !strings.HasSuffix(c.StylePath, "Button.light.msl") {
		t.Errorf("StylePath: got %q, want the light stylesheet", c.StylePath)
	}
	// Load-bearing rather than cosmetic: without the manifest the compiler
	// cannot resolve sibling component references (Field -> Input).
	if c.ManifestPath == "" {
		t.Error("ManifestPath: expected mosaic-package.toml to be located")
	}
	if len(c.Stories) != 1 || c.Stories[0].Name != "Default" {
		t.Errorf("Stories: expected one auto-generated Default, got %+v", c.Stories)
	}
}

// A .mil with no sibling .mll is not renderable on its own — it may be a
// shared interface fragment. Skip it silently rather than surfacing a broken
// component in the sidebar.
func TestDiscoverThreeFile_SkipsInterfaceWithoutLayout(t *testing.T) {
	dir := t.TempDir()
	mustWrite(t, filepath.Join(dir, "Fragment.mil"), "component Fragment {}")

	comps, err := discoverComponents(dir)
	if err != nil {
		t.Fatalf("discoverComponents: %v", err)
	}
	if len(comps) != 0 {
		t.Fatalf("expected 0 components, got %d", len(comps))
	}
}

// A dark-only component still previews rather than rendering unstyled.
func TestDiscoverThreeFile_FallsBackToDarkStylesheet(t *testing.T) {
	dir := t.TempDir()
	writeThreeFileComponent(t, dir, "Badge", ".dark.msl")

	comps, err := discoverComponents(dir)
	if err != nil {
		t.Fatalf("discoverComponents: %v", err)
	}
	if len(comps) != 1 {
		t.Fatalf("expected 1 component, got %d", len(comps))
	}
	if !strings.HasSuffix(comps[0].StylePath, "Badge.dark.msl") {
		t.Errorf("StylePath: got %q, want the dark stylesheet", comps[0].StylePath)
	}
}

// A component with neither stylesheet variant is still discoverable.
func TestDiscoverThreeFile_ToleratesMissingStylesheet(t *testing.T) {
	dir := t.TempDir()
	writeThreeFileComponent(t, dir, "Plain", "")

	comps, err := discoverComponents(dir)
	if err != nil {
		t.Fatalf("discoverComponents: %v", err)
	}
	if len(comps) != 1 {
		t.Fatalf("expected 1 component, got %d", len(comps))
	}
	if comps[0].StylePath != "" {
		t.Errorf("StylePath: got %q, want empty", comps[0].StylePath)
	}
}

// The manifest search walks up from the source directory, so components in a
// package's src/ subdirectory still find their package root.
func TestDiscoverThreeFile_FindsManifestFromSubdirectory(t *testing.T) {
	dir := t.TempDir()
	mustWrite(t, filepath.Join(dir, "mosaic-package.toml"), "[package]\nname = \"demo\"\n")
	src := filepath.Join(dir, "src")
	if err := os.Mkdir(src, 0o755); err != nil {
		t.Fatalf("mkdir: %v", err)
	}
	writeThreeFileComponent(t, src, "Nested", ".light.msl")

	comps, err := discoverComponents(dir)
	if err != nil {
		t.Fatalf("discoverComponents: %v", err)
	}
	if len(comps) != 1 {
		t.Fatalf("expected 1 component, got %d", len(comps))
	}
	if comps[0].ManifestPath == "" {
		t.Error("expected the manifest to be found by walking up from src/")
	}
	if comps[0].ID != "src/Nested" {
		t.Errorf("ID: got %q, want %q", comps[0].ID, "src/Nested")
	}
}

// A component outside any package is still renderable; it just cannot
// reference siblings.
func TestDiscoverThreeFile_NoManifestIsNotAnError(t *testing.T) {
	dir := t.TempDir()
	writeThreeFileComponent(t, dir, "Loose", ".light.msl")

	comps, err := discoverComponents(dir)
	if err != nil {
		t.Fatalf("discoverComponents: %v", err)
	}
	if len(comps) != 1 {
		t.Fatalf("expected 1 component, got %d", len(comps))
	}
	if comps[0].ManifestPath != "" {
		t.Errorf("ManifestPath: got %q, want empty", comps[0].ManifestPath)
	}
}

// ── Compiler invocation form ──────────────────────────────────────────────

func TestCompilerArgs_ThreeFileFormPassesManifest(t *testing.T) {
	c := Component{
		InterfacePath: "src/Field.mil",
		LayoutPath:    "src/Field.mll",
		StylePath:     "src/Field.light.msl",
		ManifestPath:  "mosaic-package.toml",
	}
	got := strings.Join(compilerArgs(c, "react", "out.tsx"), " ")

	for _, want := range []string{
		"--interface src/Field.mil",
		"--layout src/Field.mll",
		"--style src/Field.light.msl",
		"--package-manifest mosaic-package.toml",
		"--backend react",
		"--output out.tsx",
	} {
		if !strings.Contains(got, want) {
			t.Errorf("args missing %q; got: %s", want, got)
		}
	}
}

// The stylesheet is optional — omitting --style lets the backend apply its
// defaults rather than failing on a path that does not exist.
func TestCompilerArgs_OmitsStyleWhenAbsent(t *testing.T) {
	c := Component{InterfacePath: "A.mil", LayoutPath: "A.mll"}
	got := strings.Join(compilerArgs(c, "html", "out.html"), " ")
	if strings.Contains(got, "--style") {
		t.Errorf("expected no --style flag; got: %s", got)
	}
}

// The legacy single-file form must keep working — nothing uses it in this
// repo today, but removing it is a separate decision.
func TestCompilerArgs_LegacySingleFileForm(t *testing.T) {
	c := Component{SourcePath: "Button.mosaic"}
	got := strings.Join(compilerArgs(c, "html", "out.html"), " ")
	want := "--backend html --output out.html Button.mosaic"
	if got != want {
		t.Errorf("got %q, want %q", got, want)
	}
}
