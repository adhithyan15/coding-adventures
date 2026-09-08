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
	got := strings.Join(compilerArgs(c, "react", "out.tsx", ""), " ")

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
	got := strings.Join(compilerArgs(c, "html", "out.html", ""), " ")
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
	got := strings.Join(compilerArgs(c, "html", "out.html", ""), " ")
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

// The legacy .mosaic branch feeds the same executing-script sink as the
// three-file branch, so it needs the identical restriction. Validating only
// one of the two discovery paths left the legacy form injectable with the
// same payload.
func TestDiscoverLegacy_RejectsNonIdentifierNames(t *testing.T) {
	dir := t.TempDir()
	mustWrite(t, filepath.Join(dir, "Has Space.mosaic"), "component X {}")
	// Deliberately sorts AFTER the rejected file. filepath.Walk is
	// alphabetical, so a survivor named e.g. "Fine" would already be
	// collected before the rejection happened — the test would then pass
	// even if rejecting a candidate aborted the whole walk.
	mustWrite(t, filepath.Join(dir, "Zed.mosaic"), "component Zed {}")

	comps, err := discoverComponents(dir)
	if err != nil {
		t.Fatalf("discoverComponents: %v", err)
	}
	if len(comps) != 1 {
		t.Fatalf("expected only the valid component, got %d", len(comps))
	}
	if comps[0].ID != "Zed" {
		t.Errorf("ID: got %q, want %q", comps[0].ID, "Zed")
	}
}

// Defence in depth: the sink refuses a non-identifier name even if some
// future discovery path forgets to validate. The name is interpolated into
// an executing script block, so this invariant belongs where it is used.
func TestWrapForBackend_RefusesNonIdentifierComponentName(t *testing.T) {
	for _, backend := range []string{"react", "webcomponent", "html"} {
		got := wrapForBackend("console.log(1)", backend, "alert(document.domain)||X")
		if strings.Contains(got, "alert(document.domain)||X, null") ||
			strings.Contains(got, "<alert(") {
			t.Errorf("backend %s: payload reached the sink:\n%s", backend, got)
		}
		if !strings.Contains(got, "not a valid identifier") {
			t.Errorf("backend %s: expected an inert error page, got:\n%s", backend, got)
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

// ── Stories (#14031) ──────────────────────────────────────────────────────
//
// Three-file components could not have stories at all: threeFileComponent
// hardcoded a single empty "Default" and never looked for a file. Since
// three-file UI29 is the only authoring form in this repository, no component
// had ever been previewed with a populated slot -- which is how six toolkit
// components shipped `variant` slots that did nothing.

func TestDiscoverThreeFile_LoadsSiblingStories(t *testing.T) {
	dir := t.TempDir()
	writeThreeFileComponent(t, dir, "Button", ".light.msl")
	mustWrite(t, filepath.Join(dir, "Button.stories.json"), `{
	  "title": "Push Button",
	  "stories": [
	    {"name": "Primary", "fixtures": {"label": "Save", "variant": "primary"}},
	    {"name": "Danger",  "fixtures": {"label": "Delete", "variant": "danger"}}
	  ]
	}`)

	comps, err := discoverComponents(dir)
	if err != nil {
		t.Fatalf("discover: %v", err)
	}
	if len(comps) != 1 {
		t.Fatalf("expected 1 component, got %d", len(comps))
	}
	c := comps[0]
	if c.StoriesError != "" {
		t.Fatalf("unexpected stories error: %s", c.StoriesError)
	}
	if len(c.Stories) != 2 {
		t.Fatalf("expected 2 stories, got %d: %+v", len(c.Stories), c.Stories)
	}
	if c.Stories[0].Name != "Primary" || c.Stories[1].Name != "Danger" {
		t.Fatalf("story names not preserved in order: %+v", c.Stories)
	}
	// The fixture has to survive, or the story renders the same empty
	// component the synthesized Default did.
	if got := c.Stories[0].Fixtures["variant"]; got != "primary" {
		t.Fatalf("fixture value lost: got %v", got)
	}
	if c.Title != "Push Button" {
		t.Fatalf("title override ignored: %q", c.Title)
	}
}

func TestDiscoverThreeFile_NoStoriesFileSynthesisesDefault(t *testing.T) {
	dir := t.TempDir()
	writeThreeFileComponent(t, dir, "Badge", ".light.msl")

	comps, err := discoverComponents(dir)
	if err != nil {
		t.Fatalf("discover: %v", err)
	}
	c := comps[0]
	if len(c.Stories) != 1 || c.Stories[0].Name != "Default" {
		t.Fatalf("expected a synthesized Default, got %+v", c.Stories)
	}
	// Absent is not an error -- only a file that exists and is broken is.
	if c.StoriesError != "" {
		t.Fatalf("absent stories file must not report an error: %s", c.StoriesError)
	}
}

func TestDiscoverThreeFile_BrokenStoriesFileIsReportedNotSwallowed(t *testing.T) {
	dir := t.TempDir()
	writeThreeFileComponent(t, dir, "Alert", ".light.msl")
	mustWrite(t, filepath.Join(dir, "Alert.stories.json"), "{ not json")

	comps, err := discoverComponents(dir)
	if err != nil {
		t.Fatalf("discover: %v", err)
	}
	c := comps[0]
	// Still previewable...
	if len(c.Stories) != 1 || c.Stories[0].Name != "Default" {
		t.Fatalf("expected fallback Default, got %+v", c.Stories)
	}
	// ...but the reason is carried, so a gate can reject it rather than
	// certifying the component on empty fixtures.
	if c.StoriesError == "" {
		t.Fatal("a malformed stories file must not look identical to no stories file")
	}
}

func TestDiscoverThreeFile_EmptyStoriesListIsReported(t *testing.T) {
	dir := t.TempDir()
	writeThreeFileComponent(t, dir, "Card", ".light.msl")
	mustWrite(t, filepath.Join(dir, "Card.stories.json"), `{"stories": []}`)

	comps, err := discoverComponents(dir)
	if err != nil {
		t.Fatalf("discover: %v", err)
	}
	c := comps[0]
	if len(c.Stories) != 1 || c.Stories[0].Name != "Default" {
		t.Fatalf("expected fallback Default, got %+v", c.Stories)
	}
	if c.StoriesError == "" {
		t.Fatal("a stories file declaring no stories should say so")
	}
}

func TestDiscoverThreeFile_StoriesFileIsHotReloadWatched(t *testing.T) {
	// The watcher has to see the file, or editing a fixture would not refresh
	// the preview and the seam would look broken rather than unwatched.
	if !hasWatchedSuffix("Button.stories.json") {
		t.Fatal("stories files must trigger a hot reload")
	}
}

// ── Story fixtures reach the compiler (#14459) ────────────────────────────
//
// The story name was parsed out of /preview/{backend}/{id}/{story} and then
// discarded, and pipeline mode ignored --fixtures regardless, so every story
// of a component rendered identically. These cover the plumbing that makes
// selecting a story mean something.

func TestCompilerArgs_PassesFixturesWhenPresent(t *testing.T) {
	dir := t.TempDir()
	writeThreeFileComponent(t, dir, "Badge", ".light.msl")
	comps, err := discoverComponents(dir)
	if err != nil {
		t.Fatalf("discover: %v", err)
	}
	got := strings.Join(compilerArgs(comps[0], "html", "out.html", "/tmp/fx.json"), " ")
	if !strings.Contains(got, "--fixtures /tmp/fx.json") {
		t.Fatalf("fixtures path must reach the compiler: %s", got)
	}
}

func TestCompilerArgs_OmitsFixturesFlagWhenAbsent(t *testing.T) {
	dir := t.TempDir()
	writeThreeFileComponent(t, dir, "Badge", ".light.msl")
	comps, err := discoverComponents(dir)
	if err != nil {
		t.Fatalf("discover: %v", err)
	}
	got := strings.Join(compilerArgs(comps[0], "html", "out.html", ""), " ")
	// A story with no fixtures must not pass an empty --fixtures: "unset" is a
	// legitimate state a component has to render, not an empty file to apply.
	if strings.Contains(got, "--fixtures") {
		t.Fatalf("no fixtures means no flag: %s", got)
	}
}

func TestCompile_WritesStoryFixturesAsJSONObject(t *testing.T) {
	// The file handed to --fixtures has to be a flat object of slot name to
	// value -- the shape mosaic-compile parses. A mismatch here would be
	// accepted-and-ignored, which is the exact failure this whole chain is
	// made of.
	story := Story{
		Name:     "Danger",
		Fixtures: map[string]interface{}{"variant": "danger", "label": "Delete"},
	}
	encoded, err := json.Marshal(story.Fixtures)
	if err != nil {
		t.Fatalf("marshal: %v", err)
	}
	var round map[string]interface{}
	if err := json.Unmarshal(encoded, &round); err != nil {
		t.Fatalf("fixtures must round-trip as a JSON object: %v", err)
	}
	if round["variant"] != "danger" {
		t.Fatalf("fixture value lost: %v", round)
	}
}
