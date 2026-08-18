// threefile_test.go — discovery and compilation of UI29 three-file components
//
// Every component in this repo is authored as separate .mil/.mll/.msl files
// inside a Mosaic package; there are no .mosaic files left anywhere in the
// tree.  MosaicBook originally discovered only .mosaic, which meant it could
// not display a single real component on any backend.  These tests cover the
// pairing rules and the compiler invocation that make them visible.

package main

import (
	"encoding/json"
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

// Regression: --root is frequently relative (it defaults to "."), while
// findPackageManifest builds absolute candidates. filepath.Rel errors when
// one side is relative and the other absolute, which silently broke manifest
// discovery for every component. Every other test here uses t.TempDir(),
// which is absolute, so this case had no coverage.
func TestDiscoverThreeFile_WorksFromRelativeRoot(t *testing.T) {
	dir := t.TempDir()
	mustWrite(t, filepath.Join(dir, "mosaic-package.toml"), "[package]\nname = \"demo\"\n")
	writeThreeFileComponent(t, dir, "Button", ".light.msl")

	wd, err := os.Getwd()
	if err != nil {
		t.Fatalf("getwd: %v", err)
	}
	if err := os.Chdir(dir); err != nil {
		t.Fatalf("chdir: %v", err)
	}
	t.Cleanup(func() { _ = os.Chdir(wd) })

	comps, err := discoverComponents(".")
	if err != nil {
		t.Fatalf("discoverComponents: %v", err)
	}
	if len(comps) != 1 {
		t.Fatalf("expected 1 component, got %d", len(comps))
	}
	if comps[0].ManifestPath == "" {
		t.Error("ManifestPath: empty when discovering from a relative root")
	}
	if comps[0].StylePath == "" {
		t.Error("StylePath: empty when discovering from a relative root")
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
// repo today, but removing it is a separate decision. The `--` separator
// stops a filename such as `--output=pwned.html.mosaic` from being parsed as
// a flag by the compiler.
func TestCompilerArgs_LegacySingleFileForm(t *testing.T) {
	c := Component{SourcePath: "Button.mosaic"}
	got := strings.Join(compilerArgs(c, "html", "out.html"), " ")
	want := "--backend html --output out.html -- Button.mosaic"
	if got != want {
		t.Errorf("got %q, want %q", got, want)
	}
}

// ── Hardening ─────────────────────────────────────────────────────────────

// A component name is interpolated into an executing script block by the
// react/webcomponent preview wrappers, so filenames that are not plain
// identifiers must never become components.
// The dangerous names are checked against the predicate directly rather than
// on disk: Windows refuses to create a file containing `|`, `<` or `"`, so a
// filesystem-only test would silently skip the most important cases.
func TestValidComponentBase_RejectsInjectionPayloads(t *testing.T) {
	for _, base := range []string{
		`alert(document.domain)||X`, // script injection into the react wrapper
		`--output=pwned.html`,       // argument injection into the compiler
		`X"></script><script>y`,     // markup break-out in the webcomponent wrapper
		`X, null)); alert(1); //`,   // argument break-out in React.createElement
		`../../etc/passwd`,
		`Has Space`,
		`has-dash`,
		`9Leading`,
		``,
	} {
		if validComponentBase.MatchString(base) {
			t.Errorf("base %q was accepted; expected rejection", base)
		}
	}
}

func TestDiscoverThreeFile_RejectsNonIdentifierNames(t *testing.T) {
	// Only names the filesystem will actually accept on every OS.
	for _, base := range []string{
		"Has Space",
		"has-dash",
		"9Leading",
	} {
		dir := t.TempDir()
		writeThreeFileComponent(t, dir, base, ".light.msl")

		comps, err := discoverComponents(dir)
		if err != nil {
			t.Fatalf("discoverComponents(%q): %v", base, err)
		}
		if len(comps) != 0 {
			t.Errorf("base %q: expected rejection, got %d component(s)", base, len(comps))
		}
	}
}

func TestDiscoverThreeFile_AcceptsIdentifierNames(t *testing.T) {
	for _, base := range []string{"Button", "ButtonGroup", "Input2", "a_b"} {
		dir := t.TempDir()
		writeThreeFileComponent(t, dir, base, ".light.msl")

		comps, err := discoverComponents(dir)
		if err != nil {
			t.Fatalf("discoverComponents(%q): %v", base, err)
		}
		if len(comps) != 1 {
			t.Errorf("base %q: expected 1 component, got %d", base, len(comps))
		}
	}
}

// The absolute filesystem paths used to drive the compiler must not be
// serialised into GET /api/stories — they leak the OS username and the
// server's directory layout to anything that can reach the port.
func TestComponentJSON_OmitsAbsolutePaths(t *testing.T) {
	c := Component{
		ID:            "Button",
		Title:         "Button",
		InterfacePath: "/home/someone/secret/Button.mil",
		LayoutPath:    "/home/someone/secret/Button.mll",
		StylePath:     "/home/someone/secret/Button.light.msl",
		ManifestPath:  "/home/someone/secret/mosaic-package.toml",
	}
	blob, err := json.Marshal(c)
	if err != nil {
		t.Fatalf("marshal: %v", err)
	}
	if strings.Contains(string(blob), "/home/someone") {
		t.Errorf("absolute paths leaked into JSON: %s", blob)
	}
}
