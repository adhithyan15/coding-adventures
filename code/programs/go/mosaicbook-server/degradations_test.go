// degradations_test.go — tests for GET /api/degradations/{backend}/{component_id}
//
// Like preview_test coverage in server_test.go, the real mosaic-compile
// binary is not assumed to be on PATH in the test environment, so the
// success path (a real `pkg` build producing a real mosaic-degradations.json)
// is verified manually against the real toolchain rather than as a checked-in
// test — see this package's CHANGELOG.md for that verification. These tests
// cover the parts that do not need the real compiler: request validation,
// component lookup, the no-manifest short-circuit, and the pure
// filterDegradations helper.

package main

import (
	"encoding/json"
	"net/http"
	"net/http/httptest"
	"path/filepath"
	"strings"
	"testing"
)

// ── Request validation ──────────────────────────────────────────────────────

func TestDegradationsHandler_InvalidBackend(t *testing.T) {
	dir := tempDir(t)
	srv := newServer(dir, "mosaic-compile")

	// html/webcomponent/react have no style-lowering step to drop from, so
	// they are rejected the same as a nonsense backend name.
	for _, backend := range []string{"html", "webcomponent", "react", "bogus"} {
		req := httptest.NewRequest(http.MethodGet, "/api/degradations/"+backend+"/Button", nil)
		rec := httptest.NewRecorder()
		srv.mux.ServeHTTP(rec, req)

		if rec.Code != http.StatusBadRequest {
			t.Errorf("backend %q: status got %d, want %d", backend, rec.Code, http.StatusBadRequest)
		}
	}
}

func TestDegradationsHandler_MissingComponentSegment(t *testing.T) {
	dir := tempDir(t)
	srv := newServer(dir, "mosaic-compile")

	req := httptest.NewRequest(http.MethodGet, "/api/degradations/xaml/", nil)
	rec := httptest.NewRecorder()
	srv.mux.ServeHTTP(rec, req)

	if rec.Code != http.StatusBadRequest {
		t.Errorf("status: got %d, want %d (missing component_id)", rec.Code, http.StatusBadRequest)
	}
}

// ── Component lookup ─────────────────────────────────────────────────────────

func TestDegradationsHandler_ComponentNotFound(t *testing.T) {
	dir := tempDir(t)
	srv := newServer(dir, "mosaic-compile")

	req := httptest.NewRequest(http.MethodGet, "/api/degradations/xaml/NoSuchComponent", nil)
	rec := httptest.NewRecorder()
	srv.mux.ServeHTTP(rec, req)

	if rec.Code != http.StatusNotFound {
		t.Errorf("status: got %d, want %d (component not found)", rec.Code, http.StatusNotFound)
	}
}

// ── No owning package ────────────────────────────────────────────────────────

// A component with no mosaic-package.toml (legacy single-file, or a
// three-file component authored outside any package) has no package_root to
// run `mosaic-compile pkg` against. The endpoint must report this cleanly —
// 200 with available:false — rather than attempting a build that cannot
// succeed, and it must do so without ever invoking the compiler subprocess
// (this test's compiler path does not exist).
func TestDegradationsHandler_NoManifest_ReturnsUnavailableWithoutInvokingCompiler(t *testing.T) {
	dir := tempDir(t)
	writeFile(t, dir, "Button.mosaic", "component Button {}")

	srv := newServer(dir, "/nonexistent/mosaic-compile-does-not-exist")

	req := httptest.NewRequest(http.MethodGet, "/api/degradations/xaml/Button", nil)
	rec := httptest.NewRecorder()
	srv.mux.ServeHTTP(rec, req)

	if rec.Code != http.StatusOK {
		t.Fatalf("status: got %d, want %d", rec.Code, http.StatusOK)
	}

	var resp DegradationsResponse
	if err := json.NewDecoder(rec.Body).Decode(&resp); err != nil {
		t.Fatalf("decode response: %v", err)
	}
	if resp.Available {
		t.Error("Available: got true, want false (component has no owning package)")
	}
	if resp.Reason == "" {
		t.Error("Reason: expected a non-empty explanation")
	}
}

// ── Compiler subprocess errors surface as 500s with a helpful message ──────

func TestDegradationsHandler_CompilerNotFound_Returns500WithHint(t *testing.T) {
	dir := tempDir(t)
	mustWrite(t, filepath.Join(dir, "mosaic-package.toml"), "[package]\nname = \"demo\"\n")
	writeThreeFileComponent(t, dir, "Button", ".light.msl")

	srv := newServer(dir, "/nonexistent/mosaic-compile-does-not-exist")

	req := httptest.NewRequest(http.MethodGet, "/api/degradations/xaml/Button", nil)
	rec := httptest.NewRecorder()
	srv.mux.ServeHTTP(rec, req)

	if rec.Code != http.StatusInternalServerError {
		t.Errorf("status: got %d, want %d", rec.Code, http.StatusInternalServerError)
	}
	body := rec.Body.String()
	if !strings.Contains(body, "not found on PATH") {
		t.Errorf("body should hint at building mosaic-compile; got: %s", body)
	}
}

// ── filterDegradations ───────────────────────────────────────────────────────

func TestFilterDegradations(t *testing.T) {
	entries := []DegradationEntry{
		{Code: "property.checkbox-indeterminate-ignored", Component: "Checkbox"},
		{Code: "property.radio-group-ignored", Component: "Radio"},
		{Code: "runtime.library-not-bundled", Component: "*"},
	}

	got := filterDegradations(entries, "Radio")
	if len(got) != 2 {
		t.Fatalf("expected 2 entries (Radio-specific + package-wide), got %d: %+v", len(got), got)
	}
	codes := map[string]bool{got[0].Code: true, got[1].Code: true}
	if !codes["property.radio-group-ignored"] || !codes["runtime.library-not-bundled"] {
		t.Errorf("unexpected filtered set: %+v", got)
	}

	// A component with no matching entries still gets the package-wide ones.
	got = filterDegradations(entries, "Button")
	if len(got) != 1 || got[0].Code != "runtime.library-not-bundled" {
		t.Errorf("expected only the package-wide entry for Button, got: %+v", got)
	}

	// Never nil — the frontend renders an empty list, not "missing" JSON.
	got = filterDegradations(nil, "Button")
	if got == nil {
		t.Error("filterDegradations(nil, ...) should return an empty slice, not nil")
	}
}
