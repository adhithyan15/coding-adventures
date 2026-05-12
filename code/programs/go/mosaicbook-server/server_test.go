// server_test.go — unit tests for mosaicbook-server
//
// Tests cover:
//   - Story discovery (finding .mosaic + .stories.json files)
//   - Auto-generation of the "Default" story when no .stories.json exists
//   - Empty directory → no components
//   - Component ID derivation from a relative path
//   - Component title derivation (CamelCase → "Camel Case")
//   - Parsing valid and empty .stories.json files
//   - HTTP handlers: /api/stories, /api/backends
//   - HTTP handlers: /preview with invalid backend, unknown component
//
// The compiler subprocess is not invoked in tests.  When compilation would
// be needed (preview handler tests), we verify the error path because
// mosaic-compile is not available in the test environment.

package main

import (
	"encoding/json"
	"net/http"
	"net/http/httptest"
	"os"
	"path/filepath"
	"strings"
	"testing"
)

// ── Helper: create a temp dir with .mosaic and optional .stories.json files ──

// tempDir creates a temporary directory for tests and returns its path.
// The caller is responsible for removing it (typically via t.Cleanup).
func tempDir(t *testing.T) string {
	t.Helper()
	dir, err := os.MkdirTemp("", "mosaicbook-test-*")
	if err != nil {
		t.Fatalf("MkdirTemp: %v", err)
	}
	t.Cleanup(func() { os.RemoveAll(dir) })
	return dir
}

// writeFile creates a file with the given content at dir/name.
func writeFile(t *testing.T, dir, name, content string) {
	t.Helper()
	path := filepath.Join(dir, name)
	if err := os.WriteFile(path, []byte(content), 0644); err != nil {
		t.Fatalf("WriteFile %s: %v", name, err)
	}
}

// ── Test 1: discovery finds .mosaic + .stories.json files ─────────────────

func TestDiscoverStories_FindsMosaicFiles(t *testing.T) {
	dir := tempDir(t)

	writeFile(t, dir, "Button.mosaic", "component Button {}")
	writeFile(t, dir, "Button.stories.json", `{
		"title": "My Button",
		"stories": [
			{"name": "Default", "fixtures": {}},
			{"name": "Primary", "fixtures": {"label": "Click me"}}
		]
	}`)

	comps, err := discoverComponents(dir)
	if err != nil {
		t.Fatalf("discoverComponents: %v", err)
	}
	if len(comps) != 1 {
		t.Fatalf("expected 1 component, got %d", len(comps))
	}
	c := comps[0]

	if c.ID != "Button" {
		t.Errorf("ID: got %q, want %q", c.ID, "Button")
	}
	if c.Title != "My Button" {
		t.Errorf("Title: got %q, want %q (should use stories.json override)", c.Title, "My Button")
	}
	if c.SourcePath != "Button.mosaic" {
		t.Errorf("SourcePath: got %q, want %q", c.SourcePath, "Button.mosaic")
	}
	if len(c.Stories) != 2 {
		t.Errorf("Stories: got %d, want 2", len(c.Stories))
	}
}

// ── Test 2: auto-generates Default story when no .stories.json exists ──────

func TestDiscoverStories_AutoGeneratesDefault(t *testing.T) {
	dir := tempDir(t)
	writeFile(t, dir, "Widget.mosaic", "component Widget {}")

	comps, err := discoverComponents(dir)
	if err != nil {
		t.Fatalf("discoverComponents: %v", err)
	}
	if len(comps) != 1 {
		t.Fatalf("expected 1 component, got %d", len(comps))
	}
	c := comps[0]

	if len(c.Stories) != 1 {
		t.Fatalf("expected 1 auto-generated story, got %d", len(c.Stories))
	}
	if c.Stories[0].Name != "Default" {
		t.Errorf("auto story name: got %q, want %q", c.Stories[0].Name, "Default")
	}
	if c.Stories[0].Fixtures == nil {
		t.Error("auto story fixtures should be an empty map, not nil")
	}
}

// ── Test 3: empty directory returns no components ─────────────────────────

func TestDiscoverStories_NoFiles(t *testing.T) {
	dir := tempDir(t)

	comps, err := discoverComponents(dir)
	if err != nil {
		t.Fatalf("discoverComponents: %v", err)
	}
	if len(comps) != 0 {
		t.Errorf("expected 0 components in empty dir, got %d", len(comps))
	}
}

// ── Test 4: component ID derivation from path ─────────────────────────────

func TestComponentIDFromPath(t *testing.T) {
	cases := []struct {
		in  string
		out string
	}{
		{"Button.mosaic", "Button"},
		{"src/Button.mosaic", "src/Button"},
		{"./Widget.mosaic", "Widget"},
		{"a/b/c/Deep.mosaic", "a/b/c/Deep"},
	}

	for _, tc := range cases {
		got := componentIDFromPath(tc.in)
		if got != tc.out {
			t.Errorf("componentIDFromPath(%q) = %q, want %q", tc.in, got, tc.out)
		}
	}
}

// ── Test 5: component title derivation from base name ─────────────────────

func TestComponentTitleFromID(t *testing.T) {
	cases := []struct {
		in  string
		out string
	}{
		{"Button", "Button"},
		{"ProfileCard", "Profile Card"},
		{"TaskBoard", "Task Board"},
		{"MyComponent", "My Component"},
		{"A", "A"},
	}

	for _, tc := range cases {
		got := componentTitleFromBase(tc.in)
		if got != tc.out {
			t.Errorf("componentTitleFromBase(%q) = %q, want %q", tc.in, got, tc.out)
		}
	}
}

// ── Test 6: parse valid .stories.json ─────────────────────────────────────

func TestParseStoriesJSON_Valid(t *testing.T) {
	dir := tempDir(t)
	writeFile(t, dir, "Card.stories.json", `{
		"title": "Card Component",
		"stories": [
			{"name": "Empty", "fixtures": {}},
			{"name": "With title", "fixtures": {"title": "Hello"}}
		]
	}`)

	stories, title, err := loadStoriesFile(filepath.Join(dir, "Card.stories.json"))
	if err != nil {
		t.Fatalf("loadStoriesFile: %v", err)
	}
	if title != "Card Component" {
		t.Errorf("title: got %q, want %q", title, "Card Component")
	}
	if len(stories) != 2 {
		t.Errorf("story count: got %d, want 2", len(stories))
	}
	if stories[0].Name != "Empty" {
		t.Errorf("story[0].Name: got %q, want %q", stories[0].Name, "Empty")
	}
}

// ── Test 7: parse empty stories array ─────────────────────────────────────

func TestParseStoriesJSON_Empty(t *testing.T) {
	dir := tempDir(t)
	// stories array is present but empty — should auto-generate Default.
	writeFile(t, dir, "Card.stories.json", `{"stories": []}`)

	stories, _, err := loadStoriesFile(filepath.Join(dir, "Card.stories.json"))
	if err != nil {
		t.Fatalf("loadStoriesFile: %v", err)
	}
	if len(stories) != 1 || stories[0].Name != "Default" {
		t.Errorf("empty stories array should yield Default story; got %v", stories)
	}
}

// ── Test 8: GET /api/stories returns JSON with components array ────────────

func TestApiStoriesHandler(t *testing.T) {
	dir := tempDir(t)
	writeFile(t, dir, "Button.mosaic", "component Button {}")

	srv := newServer(dir, "mosaic-compile")

	req := httptest.NewRequest(http.MethodGet, "/api/stories", nil)
	rec := httptest.NewRecorder()
	srv.mux.ServeHTTP(rec, req)

	if rec.Code != http.StatusOK {
		t.Fatalf("status: got %d, want %d", rec.Code, http.StatusOK)
	}

	ct := rec.Header().Get("Content-Type")
	if !strings.Contains(ct, "application/json") {
		t.Errorf("Content-Type: got %q, want application/json", ct)
	}

	var body struct {
		Components []Component `json:"components"`
	}
	if err := json.NewDecoder(rec.Body).Decode(&body); err != nil {
		t.Fatalf("decode response: %v", err)
	}
	if len(body.Components) != 1 {
		t.Errorf("components: got %d, want 1", len(body.Components))
	}
	if body.Components[0].ID != "Button" {
		t.Errorf("component ID: got %q, want %q", body.Components[0].ID, "Button")
	}
}

// ── Test 9: GET /api/backends returns expected backends ───────────────────

func TestApiBackendsHandler(t *testing.T) {
	dir := tempDir(t)
	srv := newServer(dir, "mosaic-compile")

	req := httptest.NewRequest(http.MethodGet, "/api/backends", nil)
	rec := httptest.NewRecorder()
	srv.mux.ServeHTTP(rec, req)

	if rec.Code != http.StatusOK {
		t.Fatalf("status: got %d, want %d", rec.Code, http.StatusOK)
	}

	var body struct {
		Backends []struct {
			ID        string `json:"id"`
			Tier      int    `json:"tier"`
			Available bool   `json:"available"`
		} `json:"backends"`
	}
	if err := json.NewDecoder(rec.Body).Decode(&body); err != nil {
		t.Fatalf("decode response: %v", err)
	}

	// Build a map for easy lookup.
	byID := make(map[string]struct{ Tier int; Available bool })
	for _, b := range body.Backends {
		byID[b.ID] = struct{ Tier int; Available bool }{b.Tier, b.Available}
	}

	for _, id := range []string{"html", "webcomponent", "react"} {
		b, ok := byID[id]
		if !ok {
			t.Errorf("backend %q missing from response", id)
			continue
		}
		if !b.Available {
			t.Errorf("backend %q should be available", id)
		}
		if b.Tier != 1 {
			t.Errorf("backend %q tier: got %d, want 1", id, b.Tier)
		}
	}

	// Qt should be listed but unavailable.
	qt, ok := byID["qt"]
	if !ok {
		t.Error("qt backend missing from response")
	} else if qt.Available {
		t.Error("qt backend should not be available in Phase 1")
	}
}

// ── Test 10: /preview with an invalid backend returns 400 ─────────────────

func TestPreviewHandler_InvalidBackend(t *testing.T) {
	dir := tempDir(t)
	srv := newServer(dir, "mosaic-compile")

	req := httptest.NewRequest(http.MethodGet, "/preview/qt/Button/Default", nil)
	rec := httptest.NewRecorder()
	srv.mux.ServeHTTP(rec, req)

	if rec.Code != http.StatusBadRequest {
		t.Errorf("status: got %d, want %d (bad backend)", rec.Code, http.StatusBadRequest)
	}
}

// ── Test 11: /preview for an unknown component returns 404 ────────────────

func TestPreviewHandler_NotFound(t *testing.T) {
	dir := tempDir(t)
	srv := newServer(dir, "mosaic-compile")

	req := httptest.NewRequest(http.MethodGet, "/preview/html/NoSuchComponent/Default", nil)
	rec := httptest.NewRecorder()
	srv.mux.ServeHTTP(rec, req)

	if rec.Code != http.StatusNotFound {
		t.Errorf("status: got %d, want %d (not found component)", rec.Code, http.StatusNotFound)
	}
}

// ── Test 12: discovery handles nested directories ─────────────────────────

func TestDiscoverStories_NestedDirectories(t *testing.T) {
	dir := tempDir(t)

	// Create a nested structure: src/ with multiple components.
	os.MkdirAll(filepath.Join(dir, "src", "ui"), 0755)
	writeFile(t, dir, "Button.mosaic", "component Button {}")
	writeFile(t, filepath.Join(dir, "src"), "Card.mosaic", "component Card {}")
	writeFile(t, filepath.Join(dir, "src", "ui"), "Modal.mosaic", "component Modal {}")

	comps, err := discoverComponents(dir)
	if err != nil {
		t.Fatalf("discoverComponents: %v", err)
	}
	if len(comps) != 3 {
		t.Errorf("expected 3 components, got %d: %v", len(comps), comps)
	}

	// IDs should use forward slashes regardless of OS.
	ids := make(map[string]bool)
	for _, c := range comps {
		ids[c.ID] = true
	}
	for _, wantID := range []string{"Button", "src/Card", "src/ui/Modal"} {
		if !ids[wantID] {
			t.Errorf("expected component ID %q, but not found in %v", wantID, ids)
		}
	}
}

// ── Test 13: toKebabCase converts PascalCase correctly ───────────────────

func TestToKebabCase(t *testing.T) {
	cases := []struct {
		in  string
		out string
	}{
		{"Button", "button"},
		{"ProfileCard", "profile-card"},
		{"MyComponent", "my-component"},
	}
	for _, tc := range cases {
		got := toKebabCase(tc.in)
		if got != tc.out {
			t.Errorf("toKebabCase(%q) = %q, want %q", tc.in, got, tc.out)
		}
	}
}

// ── Test 14: /preview with valid component returns HTML (error page) ───────
// When mosaic-compile is not on PATH, the preview endpoint should return an
// HTML error page (not a 5xx JSON error) with Content-Type text/html.

func TestPreviewHandler_CompilerNotFound_ReturnsHTMLErrorPage(t *testing.T) {
	dir := tempDir(t)
	writeFile(t, dir, "Button.mosaic", "component Button {}")

	// Use a definitely-nonexistent compiler path.
	srv := newServer(dir, "/nonexistent/mosaic-compile-does-not-exist")

	req := httptest.NewRequest(http.MethodGet, "/preview/html/Button/Default", nil)
	rec := httptest.NewRecorder()
	srv.mux.ServeHTTP(rec, req)

	// Should return 200 with an HTML error page embedded in the iframe.
	if rec.Code != http.StatusOK {
		t.Errorf("status: got %d, want 200 (error should be shown in HTML page)", rec.Code)
	}
	ct := rec.Header().Get("Content-Type")
	if !strings.Contains(ct, "text/html") {
		t.Errorf("Content-Type: got %q, want text/html", ct)
	}
	body := rec.Body.String()
	if !strings.Contains(body, "Compilation Error") && !strings.Contains(body, "error") {
		t.Errorf("response body should contain error information; got: %s", body[:min(len(body), 200)])
	}
}

// ── Test 15: componentNameFromID extracts the last path segment ───────────

func TestComponentNameFromID(t *testing.T) {
	cases := []struct {
		in  string
		out string
	}{
		{"Button", "Button"},
		{"src/Button", "Button"},
		{"a/b/c/ProfileCard", "ProfileCard"},
	}
	for _, tc := range cases {
		got := componentNameFromID(tc.in)
		if got != tc.out {
			t.Errorf("componentNameFromID(%q) = %q, want %q", tc.in, got, tc.out)
		}
	}
}

// min returns the smaller of a and b (used for body truncation in error messages).
func min(a, b int) int {
	if a < b {
		return a
	}
	return b
}
