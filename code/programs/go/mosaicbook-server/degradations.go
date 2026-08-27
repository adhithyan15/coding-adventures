// degradations.go — GET /api/degradations/{backend}/{component_id}
//
// Part 1 of #12027's "deliver two things rather than four broken panes":
// a per-backend "what got dropped" panel for every native backend. Unlike
// the still-unbuilt render-daemon half of #12027 (compile → dotnet build →
// launch → screenshot for XAML; a Qt/SwiftUI/Compose/Flutter daemon for the
// others), degradation analysis needs no platform runtime at all — it is
// pure static analysis over the composed component IR, already implemented
// in mosaic-package-artifact-builder and already run by every `mosaic-compile
// pkg` invocation. That is what makes it worth shipping before any daemon
// exists: it works for xaml/swiftui/qt/flutter/compose alike, on any
// development machine, today.
//
// # A second mosaic-compile invocation shape
//
// compiler.go's compile() invokes mosaic-compile in single-component mode
// (--interface/--layout/--style/--backend/--output) to compile exactly one
// component directly to a backend's source output. That form never computes
// degradations — only the `pkg` subcommand does, because degradation
// analysis is inherently package-scoped (it walks every exported component
// in the package's manifest, per mosaic-package-artifact-builder's
// analyze_package_degradations). This file adds that second invocation
// path alongside the first; it does not replace it.
//
// See code/specs/UI19-mosaicbook.md §13 for the full design.

package main

import (
	"encoding/json"
	"fmt"
	"io"
	"net/http"
	"os"
	"os/exec"
	"path/filepath"
	"strings"
)

// nativeBackends is the set of backends that go through native (XAML/
// SwiftUI/Qt/Flutter/Compose-style) lowering and can therefore drop style or
// capability properties a Tier-1 (html/webcomponent/react) backend never
// would. Degradation analysis is only meaningful for these.
//
// Maps to the same string rather than just `true`: looking a request-derived
// backend name up in this map and using the *returned* value (not the
// original request string) for the subprocess --backend flag and the
// mosaic-degradations.json path is what lets a static analyzer (this map
// literal is a fixed, fully-enumerated set of constants) see the taint from
// the HTTP request as fully replaced rather than merely gated by an
// `if !ok` check next to it — CodeQL's Go path-injection query flagged the
// latter shape even though nativeBackends[backend] already made every path
// reachable here one of these five literals.
var nativeBackends = map[string]string{
	"xaml":    "xaml",
	"swiftui": "swiftui",
	"qt":      "qt",
	"flutter": "flutter",
	"compose": "compose",
}

// maxDegradationReportBytes bounds how much of mosaic-degradations.json we
// read into memory. Generous relative to any real package's report (a few
// KB), but guards against a misbehaving compiler producing a runaway file.
const maxDegradationReportBytes = 4 << 20 // 4 MiB

// DegradationEntry mirrors one element of the Rust `Degradation` struct's
// camelCase JSON exactly (mosaic-package-artifact-builder::Degradation), so
// the wire format matches the CLI's own mosaic-degradations.json byte for
// byte other than the filtering this file applies.
type DegradationEntry struct {
	Code       string  `json:"code"`
	Backend    string  `json:"backend"`
	Component  string  `json:"component"`
	Variant    *string `json:"variant,omitempty"`
	LayoutPath string  `json:"layoutPath"`
	Primitive  *string `json:"primitive,omitempty"`
	Reason     string  `json:"reason"`
}

// degradationReport mirrors the Rust `DegradationReport` struct's camelCase
// JSON exactly, as written to <output>/<backend>/mosaic-degradations.json by
// `mosaic-compile pkg`. Unexported: it is a parsing target, not the response
// shape served to the browser (see DegradationsResponse below, which adds
// Available/Reason and is filtered to one component).
type degradationReport struct {
	SchemaVersion     int                `json:"schemaVersion"`
	Profile           string             `json:"profile"`
	Package           string             `json:"package"`
	Backend           string             `json:"backend"`
	NativeComplete    bool               `json:"nativeComplete"`
	Degradations      []DegradationEntry `json:"degradations"`
	StyleDegradations []DegradationEntry `json:"styleDegradations"`
}

// DegradationsResponse is the JSON shape served by
// GET /api/degradations/{backend}/{component_id}.
type DegradationsResponse struct {
	// Available is false when degradation analysis could not be attempted —
	// today this only happens when the component has no owning package (a
	// mosaic-package.toml is required to run `mosaic-compile pkg`). When
	// false, Reason explains why and every field below is zero-valued.
	Available bool   `json:"available"`
	Reason    string `json:"reason,omitempty"`

	SchemaVersion int    `json:"schemaVersion,omitempty"`
	Profile       string `json:"profile,omitempty"`
	Package       string `json:"package,omitempty"`
	Backend       string `json:"backend,omitempty"`
	// Not omitempty: false is a meaningful, common value (most components
	// with any degradation at all), and omitting it would make "complete"
	// (present) and "incomplete" (absent) look asymmetric on the wire for
	// no reason — see the doc comment on where this is computed below.
	NativeComplete    bool               `json:"nativeComplete"`
	Degradations      []DegradationEntry `json:"degradations,omitempty"`
	StyleDegradations []DegradationEntry `json:"styleDegradations,omitempty"`
}

// analyzeDegradations runs `mosaic-compile pkg` against the package that
// owns c, reads back the resulting mosaic-degradations.json, and filters it
// down to entries about c specifically (plus package-wide entries, tagged
// "*" by the Rust side — e.g. runtime.library-not-bundled).
//
// --profile permissive (never native-complete) is deliberate: the whole
// point of this endpoint is to surface drops, so the request must succeed
// even when drops exist — it should fail only on a genuine compile error.
// --emit-project is omitted: the panel needs the analysis, not a buildable
// project shell, so this stays fast with no dotnet/Qt/Gradle/Flutter
// dependency even for backends whose toolchain isn't installed locally.
func (s *Server) analyzeDegradations(c Component, backend string) (*DegradationsResponse, error) {
	if c.ManifestPath == "" {
		return &DegradationsResponse{
			Available: false,
			Reason:    "component is not part of a mosaic-package.toml package; native-backend degradation analysis needs a package to build",
		}, nil
	}

	packageRoot := filepath.Dir(c.ManifestPath)

	tmpDir, err := os.MkdirTemp("", "mosaicbook-degradations-*")
	if err != nil {
		return nil, fmt.Errorf("cannot create temp directory: %w", err)
	}
	defer os.RemoveAll(tmpDir) //nolint:errcheck

	// packageRoot is a positional argument to mosaic-compile, derived from a
	// directory name on disk. If that name ever began with "-"/"--" it could
	// be misread as a flag by mosaic-compile's own arg parser instead of the
	// intended path — the same argument-injection concern compilerArgs'
	// legacy-form doc comment describes for a positional source path.
	// Placing it after every flag and behind a `--` end-of-options marker
	// (verified against cli-builder's parser, which treats everything after
	// `--` as positional) closes that off structurally rather than relying
	// on upstream validation of directory names.
	cmd := exec.Command(
		s.compilerPath, "pkg",
		"--backend", backend,
		"--output", tmpDir,
		"--profile", "permissive",
		"--", packageRoot,
	)
	out, err := cmd.CombinedOutput()
	if err != nil {
		if isNotFound(err) {
			return nil, fmt.Errorf(
				"mosaic-compile binary %q not found on PATH; "+
					"build it with: cd code/packages/rust/mosaic-compile && cargo build --release",
				s.compilerPath,
			)
		}
		if len(out) > maxCompilerOutputBytes {
			out = append(out[:maxCompilerOutputBytes], []byte("\n...(output truncated)")...)
		}
		if len(out) > 0 {
			return nil, fmt.Errorf("mosaic-compile pkg failed: %s", string(out))
		}
		return nil, fmt.Errorf("mosaic-compile pkg exited with error: %w", err)
	}

	reportPath := filepath.Join(tmpDir, backend, "mosaic-degradations.json")
	// Read through a size-limited reader rather than os.ReadFile so an
	// unexpectedly huge report is rejected without first buffering the
	// whole thing in memory — os.ReadFile has no such limit.
	f, err := os.Open(reportPath)
	if err != nil {
		return nil, fmt.Errorf("mosaic-compile pkg succeeded but did not write %s: %w", reportPath, err)
	}
	data, err := io.ReadAll(io.LimitReader(f, maxDegradationReportBytes+1))
	f.Close() //nolint:errcheck
	if err != nil {
		return nil, fmt.Errorf("cannot read %s: %w", reportPath, err)
	}
	if len(data) > maxDegradationReportBytes {
		return nil, fmt.Errorf("mosaic-degradations.json exceeds the %d byte limit", maxDegradationReportBytes)
	}

	var report degradationReport
	if err := json.Unmarshal(data, &report); err != nil {
		return nil, fmt.Errorf("cannot parse %s: %w", reportPath, err)
	}

	compName := componentNameFromID(c.ID)
	degradations := filterDegradations(report.Degradations, compName)
	styleDegradations := filterDegradations(report.StyleDegradations, compName)

	return &DegradationsResponse{
		Available:     true,
		SchemaVersion: report.SchemaVersion,
		Profile:       report.Profile,
		Package:       report.Package,
		Backend:       report.Backend,
		// Deliberately NOT report.NativeComplete: that field reflects the
		// whole package (analyze_package_degradations computes it once,
		// package-wide), so a component with zero drops of its own would
		// still show "incomplete" whenever any other component in the same
		// package has one. This panel is about the selected component, so
		// recompute it from the filtered lists below instead.
		NativeComplete:    len(degradations) == 0 && len(styleDegradations) == 0,
		Degradations:      degradations,
		StyleDegradations: styleDegradations,
	}, nil
}

// filterDegradations keeps entries about the given component (by its
// PascalCase name) or package-wide entries (component == "*"), dropping
// everything else — `mosaic-compile pkg` analyzes every exported component
// in the package, not just the one MosaicBook has selected.
func filterDegradations(entries []DegradationEntry, compName string) []DegradationEntry {
	filtered := make([]DegradationEntry, 0, len(entries))
	for _, e := range entries {
		if e.Component == compName || e.Component == "*" {
			filtered = append(filtered, e)
		}
	}
	return filtered
}

// handleAPIDegradations handles GET /api/degradations/{backend}/{component_id}.
//
// Unlike /preview/, there is no story_name segment: degradations describe
// what a backend's style/capability lowering dropped from the component's
// definition, which does not vary per fixture/story.
func (s *Server) handleAPIDegradations(w http.ResponseWriter, r *http.Request) {
	suffix := strings.TrimPrefix(r.URL.Path, "/api/degradations/")
	parts := strings.SplitN(suffix, "/", 2)
	if len(parts) < 2 || parts[1] == "" {
		http.Error(w, "invalid path; expected /api/degradations/{backend}/{component_id}", http.StatusBadRequest)
		return
	}
	requestedBackend := parts[0]
	componentID := parts[1]

	// Look up the canonical value rather than branching on a boolean: using
	// backend (below) rather than requestedBackend for every downstream path/
	// argument use is what severs the taint from the raw request path
	// segment, not just the presence of this check — see nativeBackends' doc
	// comment.
	backend, ok := nativeBackends[requestedBackend]
	if !ok {
		http.Error(w, fmt.Sprintf("unknown native backend %q; supported: xaml, swiftui, qt, flutter, compose", requestedBackend), http.StatusBadRequest)
		return
	}

	found := s.findComponent(componentID)
	if found == nil {
		http.Error(w, fmt.Sprintf("component %q not found", componentID), http.StatusNotFound)
		return
	}

	resp, err := s.analyzeDegradations(*found, backend)
	if err != nil {
		http.Error(w, err.Error(), http.StatusInternalServerError)
		return
	}
	writeJSON(w, resp)
}
