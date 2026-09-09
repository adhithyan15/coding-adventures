// coverage.go — measures which mosstyle properties each backend can actually
// lower, and renders it as a page.
//
// # Why this is measured rather than written down
//
// mosstyle properties are freeform: the compiler declares an `UnknownProperty`
// error kind but never raises it, so a stylesheet may name any property and
// each emitter translates the subset it happens to know. Nothing in the
// pipeline knows what a backend can express, so nothing can warn that an
// authored declaration reached nobody. The gap is therefore invisible unless
// something goes and looks — and a number written into a document by hand
// stops being a measurement the day after it is written.
//
// # Method
//
// Differential, not sentinel-matching. For each property, emit the same probe
// component twice — once without the property and once with it — and compare
// the generated output. If nothing changes, the backend does not lower it.
//
// Grepping the generated file for the authored value looks simpler and is
// wrong. SwiftUI writes `#abcdef` as `Color(red: 0.671, green: 0.804, blue:
// 0.937)`, so every colour reads as unsupported; and a short value like `3`
// for `flex-grow` matches unrelated output, so unsupported properties read as
// supported. Comparing whole outputs has no encoding assumptions at all.
//
// # Two probe shapes
//
// Probe shape changes the answer. `gap` on Qt and `align` on XAML lower only
// on a Row, and font properties only reach a text part. So each property is
// tried on both a Box probe and a Row probe, and counts as lowered if either
// shows a difference.
//
// # Values come from the corpus
//
// Every probe value is a real value taken from the repository's own `.msl`
// files for that property, so the probe cannot test a value nobody writes.
// Properties are ordered by how often they are actually authored.

package main

import (
	"fmt"
	"os"
	"os/exec"
	"path/filepath"
	"regexp"
	"sort"
	"strings"
)

// probeValuesPerProp is how many distinct authored values to try before
// concluding a backend cannot lower a property. More than one is necessary:
// a value equal to the backend's own default changes nothing even where the
// property is fully supported. Three covers the observed cases without
// tripling the common path, since the first value usually settles it.
const probeValuesPerProp = 3

// coverageBackends is every backend that generates source from a pipeline
// triple. `paint` is excluded: it renders a scene rather than emitting code,
// so "does the output change" is not the same question there.
var coverageBackends = []string{
	"react", "html", "webcomponent", "swiftui", "qt", "flutter", "compose", "xaml",
}

// backendExt is the output filename extension each backend expects. The
// compiler derives sibling project files from it, so it has to be right.
var backendExt = map[string]string{
	"react": "tsx", "html": "html", "webcomponent": "js", "swiftui": "swift",
	"qt": "qml", "flutter": "dart", "compose": "kt", "xaml": "xaml",
}

// PropCoverage is one row of the matrix.
type PropCoverage struct {
	Name string
	// Uses is how many times the property is authored across all .msl files.
	Uses int
	// Values are the authored values probed, most-used first. Several are
	// tried because one unlucky value can hide real support: the most common
	// `background` in this repository is "transparent", which changes nothing
	// on a backend whose default is already transparent. A property counts as
	// lowered if ANY authored value changes the output.
	Values []string
	// Lowered is indexed by coverageBackends.
	Lowered []bool
}

// BackendCoverage is one column's summary.
type BackendCoverage struct {
	Name    string
	Lowered int
	Total   int
	Percent int
}

// CoverageReport is everything the page needs.
type CoverageReport struct {
	Props    []PropCoverage
	Backends []BackendCoverage
	// Skipped records why the report is absent, when it is.
	Skipped string
}

// declRe matches an authored `property : value ;` line in a .msl file.
var declRe = regexp.MustCompile(`(?m)^\s+([a-z-]+)\s*:\s*(.+?)\s*;`)

// censusProperties walks every .msl file under root and counts authored
// properties, keeping the most common value for each as the probe value.
//
// Structural keywords that appear in the same position but are not style
// properties are excluded by name; everything else is taken as authored.
func censusProperties(root string) []PropCoverage {
	notProps := map[string]bool{"state": true, "part": true, "style": true, "transition": true}
	counts := map[string]int{}
	values := map[string]map[string]int{}

	_ = filepath.Walk(root, func(path string, info os.FileInfo, err error) error {
		if err != nil || info.IsDir() || !strings.HasSuffix(path, ".msl") {
			return nil
		}
		if strings.Contains(path, "/target/") {
			return nil
		}
		body, err := os.ReadFile(path)
		if err != nil {
			return nil
		}
		for _, m := range declRe.FindAllStringSubmatch(string(body), -1) {
			name, value := m[1], strings.Trim(m[2], `"`)
			if notProps[name] || strings.Contains(value, "$") {
				// Token references resolve per theme; probing one would
				// measure the token table rather than the property.
				continue
			}
			counts[name]++
			if values[name] == nil {
				values[name] = map[string]int{}
			}
			values[name][value]++
		}
		return nil
	})

	out := make([]PropCoverage, 0, len(counts))
	for name, n := range counts {
		type vc struct {
			v string
			c int
		}
		ranked := make([]vc, 0, len(values[name]))
		for v, c := range values[name] {
			ranked = append(ranked, vc{v, c})
		}
		// Ties break on the lexically smaller value so the report is
		// reproducible across runs; Go map order is not.
		sort.Slice(ranked, func(i, j int) bool {
			if ranked[i].c != ranked[j].c {
				return ranked[i].c > ranked[j].c
			}
			return ranked[i].v < ranked[j].v
		})
		probe := make([]string, 0, probeValuesPerProp)
		for _, r := range ranked {
			if len(probe) == probeValuesPerProp {
				break
			}
			probe = append(probe, r.v)
		}
		out = append(out, PropCoverage{Name: name, Uses: n, Values: probe})
	}
	sort.Slice(out, func(i, j int) bool {
		if out[i].Uses != out[j].Uses {
			return out[i].Uses > out[j].Uses
		}
		return out[i].Name < out[j].Name
	})
	return out
}

const probeMil = "component Probe {\n  slot label : text ;\n}\n"

const probeBoxMll = `layout Probe {
  Box [ panel ] {
    Text [ panel-text ] (
      content : slot: label
    )
  }
}
`

const probeRowMll = `layout Probe {
  Row [ panel ] {
    Text [ panel-text ] (
      content : slot: label
    )
    Text [ panel-text2 ] (
      content : slot: label
    )
  }
}
`

// probeStyle writes a stylesheet setting `height` on both parts, plus the
// property under test when one is given.
//
// `height` is the constant so the parts exist and are styled in the baseline
// too. It must not be the property under test, or the probe would set the
// same property twice and measure nothing.
func probeStyle(prop, value string) string {
	var b strings.Builder
	for _, part := range []string{"panel", "panel-text"} {
		fmt.Fprintf(&b, "  part %s {\n    height : \"11\" ;\n", part)
		if prop != "" {
			fmt.Fprintf(&b, "    %s : \"%s\" ;\n", prop, value)
		}
		b.WriteString("  }\n")
	}
	return "style Probe {\n" + b.String() + "}\n"
}

// emitProbe returns the concatenated generated output, or "" if the emit
// failed. A failure is treated as "not lowered" rather than aborting: one
// backend rejecting one probe value must not lose the whole report.
func emitProbe(compiler, dir, backend, mll, style string) string {
	work := filepath.Join(dir, "out")
	_ = os.RemoveAll(work)
	if err := os.MkdirAll(work, 0o755); err != nil {
		return ""
	}
	mslPath := filepath.Join(dir, "Probe.msl")
	if err := os.WriteFile(mslPath, []byte(style), 0o644); err != nil {
		return ""
	}
	out := filepath.Join(work, "Probe."+backendExt[backend])
	cmd := exec.Command(compiler,
		"--backend", backend,
		"--interface", filepath.Join(dir, "Probe.mil"),
		"--layout", mll,
		"--style", mslPath,
		"--output", out)
	if err := cmd.Run(); err != nil {
		return ""
	}
	entries, err := os.ReadDir(work)
	if err != nil {
		return ""
	}
	names := make([]string, 0, len(entries))
	for _, e := range entries {
		names = append(names, e.Name())
	}
	sort.Strings(names)
	var b strings.Builder
	for _, n := range names {
		body, err := os.ReadFile(filepath.Join(work, n))
		if err != nil {
			continue
		}
		b.Write(body)
	}
	return b.String()
}

// measureCoverage runs the whole matrix. `root` is the repository directory to
// census; `compiler` is the mosaic-compile binary.
func measureCoverage(compiler, root string) CoverageReport {
	props := censusProperties(root)
	if len(props) == 0 {
		return CoverageReport{Skipped: "no .msl files found to census"}
	}

	dir, err := os.MkdirTemp("", "mosaic-docs-coverage-*")
	if err != nil {
		return CoverageReport{Skipped: "could not create a probe directory: " + err.Error()}
	}
	defer os.RemoveAll(dir)

	boxMll := filepath.Join(dir, "Box.mll")
	rowMll := filepath.Join(dir, "Row.mll")
	if os.WriteFile(filepath.Join(dir, "Probe.mil"), []byte(probeMil), 0o644) != nil ||
		os.WriteFile(boxMll, []byte(probeBoxMll), 0o644) != nil ||
		os.WriteFile(rowMll, []byte(probeRowMll), 0o644) != nil {
		return CoverageReport{Skipped: "could not write the probe component"}
	}
	shapes := []string{boxMll, rowMll}

	// One baseline per backend per shape, reused by every property.
	baseline := map[string]string{}
	for _, b := range coverageBackends {
		for _, mll := range shapes {
			baseline[b+"|"+mll] = emitProbe(compiler, dir, b, mll, probeStyle("", ""))
		}
	}
	if allEmpty(baseline) {
		return CoverageReport{Skipped: "the probe component did not compile on any backend; is the compiler path right?"}
	}

	for i := range props {
		props[i].Lowered = make([]bool, len(coverageBackends))
		for bi, b := range coverageBackends {
		probe:
			for _, mll := range shapes {
				base := baseline[b+"|"+mll]
				if base == "" {
					continue
				}
				for _, value := range props[i].Values {
					got := emitProbe(compiler, dir, b, mll, probeStyle(props[i].Name, value))
					if got != "" && got != base {
						props[i].Lowered[bi] = true
						break probe
					}
				}
			}
		}
	}

	backs := make([]BackendCoverage, len(coverageBackends))
	for bi, b := range coverageBackends {
		n := 0
		for _, p := range props {
			if p.Lowered[bi] {
				n++
			}
		}
		pct := 0
		if len(props) > 0 {
			pct = n * 100 / len(props)
		}
		backs[bi] = BackendCoverage{Name: b, Lowered: n, Total: len(props), Percent: pct}
	}
	return CoverageReport{Props: props, Backends: backs}
}

func allEmpty(m map[string]string) bool {
	for _, v := range m {
		if v != "" {
			return false
		}
	}
	return true
}
