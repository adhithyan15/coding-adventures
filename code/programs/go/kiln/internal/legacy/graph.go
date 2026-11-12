// Package legacy computes Kiln keys for today's shell BUILD files (KILN03).
//
// A legacy BUILD has no declared inputs, so a key must over-approximate
// everything the BUILD could observe.  Over-approximation only lowers the hit
// rate; under-approximation would make a false hit, the one failure Kiln must
// not have.  Every rule in this package therefore errs toward "more is
// included", and where a rule is still a heuristic the plan says so with a
// reason (m1-incomplete:...) rather than pretending to be complete.
//
// The package is language-blind.  Everything it knows about packages, edges and
// extra inputs comes from the legacy graph export (this file); everything it
// knows about files comes from a Source.
package legacy

import (
	"bytes"
	"encoding/json"
	"fmt"
	"io"
	"os"
	"strings"
	"unicode"
	"unicode/utf8"

	"github.com/adhithyan15/coding-adventures/code/programs/go/kiln/internal/key"
	"github.com/adhithyan15/coding-adventures/code/programs/go/kiln/internal/source"
)

// GraphSchema is the only export schema this loader understands.
const GraphSchema = "kiln-legacy-graph/1"

// Limits that keep a hostile or broken export from exhausting memory.
const (
	MaxGraphBytes = 64 << 20 // 64 MiB of JSON
	MaxNameBytes  = 512
	MaxPackages   = 100_000   // packages in one export
	MaxEdges      = 5_000_000 // deps across all OSes plus dev_deps, in total
)

// The limits as enforced; tests lower them instead of building huge graphs.
var (
	packageLimit = MaxPackages
	edgeLimit    = MaxEdges
)

// OSes are the platforms a graph describes, in the export's own spelling.
var OSes = []string{"linux", "darwin", "windows"}

// Package is one entry of the export (KILN03 section 2).
type Package struct {
	Name              string              `json:"name"`
	RelPath           string              `json:"rel_path"`
	Language          string              `json:"language"`
	BuildFiles        map[string]*string  `json:"build_files"`
	Deps              map[string][]string `json:"deps"`
	DevDeps           []string            `json:"dev_deps"`
	RepoInputs        []string            `json:"repo_inputs"`
	RepoInputGlobs    []string            `json:"repo_input_globs"`
	RepoInputsUnknown bool                `json:"repo_inputs_unknown"`
	ExtraToolchains   []string            `json:"extra_toolchains"`
}

// Graph is a validated export.
type Graph struct {
	Schema   string    `json:"schema"`
	Packages []Package `json:"packages"`
}

// ReadGraphFile loads and validates the export at path.  The file must be a
// regular file no larger than MaxGraphBytes.
func ReadGraphFile(path string) (*Graph, error) {
	f, err := os.Open(path)
	if err != nil {
		return nil, err
	}
	defer f.Close()
	fi, err := f.Stat()
	if err != nil {
		return nil, err
	}
	if !fi.Mode().IsRegular() {
		return nil, fmt.Errorf("graph %s: not a regular file", path)
	}
	data, err := io.ReadAll(io.LimitReader(f, MaxGraphBytes+1))
	if err != nil {
		return nil, err
	}
	return ParseGraph(data)
}

// ParseGraph decodes and validates an export.  Decoding is strict: unknown
// fields are errors, nothing may follow the JSON value, and every consistency
// rule of KILN03 section 2 is checked, because a graph that merely "mostly
// parses" would key some packages with edges missing.
func ParseGraph(data []byte) (*Graph, error) {
	if len(data) > MaxGraphBytes {
		return nil, fmt.Errorf("graph exceeds the %d byte limit", MaxGraphBytes)
	}
	var g Graph
	if err := strictDecode(data, &g); err != nil {
		return nil, fmt.Errorf("graph: %w", err)
	}
	if g.Schema != GraphSchema {
		return nil, fmt.Errorf("graph: schema %q, want %q", short(g.Schema), GraphSchema)
	}
	if err := g.validate(); err != nil {
		return nil, fmt.Errorf("graph: %w", err)
	}
	return &g, nil
}

// strictDecode decodes one JSON value into v, refusing unknown fields and any
// trailing data.
func strictDecode(data []byte, v any) error {
	dec := json.NewDecoder(bytes.NewReader(data))
	dec.DisallowUnknownFields()
	if err := dec.Decode(v); err != nil {
		return err
	}
	if _, err := dec.Token(); err != io.EOF {
		return fmt.Errorf("unexpected data after the JSON value")
	}
	return nil
}

// short is a log-safe excerpt: no control characters, at most 40 runes.
func short(s string) string {
	r := []rune(strings.Map(func(r rune) rune {
		if unicode.IsControl(r) {
			return -1
		}
		return r
	}, s))
	if len(r) > 40 {
		return string(r[:40]) + "..."
	}
	return string(r)
}

func (g *Graph) validate() error {
	if len(g.Packages) > packageLimit {
		return fmt.Errorf("%d packages exceed the limit of %d", len(g.Packages), packageLimit)
	}
	edges := 0
	for _, p := range g.Packages {
		for _, d := range p.Deps {
			edges += len(d)
		}
		edges += len(p.DevDeps)
		if edges > edgeLimit {
			return fmt.Errorf("more than %d edges (deps and dev_deps)", edgeLimit)
		}
	}
	names := make(map[string]bool, len(g.Packages))
	paths := make(map[string]string, len(g.Packages))
	for i, p := range g.Packages {
		if err := checkName(p.Name); err != nil {
			return fmt.Errorf("package %d: %w", i, err)
		}
		if i > 0 && g.Packages[i-1].Name >= p.Name {
			return fmt.Errorf("packages not strictly sorted by name at %q", short(p.Name))
		}
		names[p.Name] = true
		if err := key.ValidatePath(p.RelPath); err != nil {
			return fmt.Errorf("package %q: rel_path: %w", short(p.Name), err)
		}
		if other, dup := paths[p.RelPath]; dup {
			return fmt.Errorf("packages %q and %q share rel_path %q", short(other), short(p.Name), short(p.RelPath))
		}
		paths[p.RelPath] = p.Name
		if p.Language == "" || !utf8.ValidString(p.Language) || strings.ContainsFunc(p.Language, unicode.IsControl) {
			return fmt.Errorf("package %q: bad language", short(p.Name))
		}
	}
	for _, p := range g.Packages {
		if err := checkPackage(p, names); err != nil {
			return fmt.Errorf("package %q: %w", short(p.Name), err)
		}
	}
	return nil
}

func checkName(n string) error {
	if n == "" || len(n) > MaxNameBytes || !utf8.ValidString(n) || strings.ContainsFunc(n, unicode.IsControl) {
		return fmt.Errorf("invalid name %q", short(n))
	}
	return nil
}

func checkPackage(p Package, names map[string]bool) error {
	if len(p.BuildFiles) != len(OSes) || len(p.Deps) != len(OSes) {
		return fmt.Errorf("build_files and deps must name exactly %v", OSes)
	}
	for _, plat := range OSes {
		bf, ok := p.BuildFiles[plat]
		if !ok {
			return fmt.Errorf("build_files has no %q entry", plat)
		}
		if bf != nil {
			if err := checkBaseName(*bf); err != nil {
				return fmt.Errorf("build_files[%s]: %w", plat, err)
			}
		}
		deps, ok := p.Deps[plat]
		if !ok {
			return fmt.Errorf("deps has no %q entry", plat)
		}
		if err := checkNameList("deps["+plat+"]", deps, names); err != nil {
			return err
		}
	}
	if err := checkNameList("dev_deps", p.DevDeps, names); err != nil {
		return err
	}
	if err := checkSorted("extra_toolchains", p.ExtraToolchains); err != nil {
		return err
	}
	for _, tc := range p.ExtraToolchains {
		if tc == "" || !utf8.ValidString(tc) || strings.ContainsFunc(tc, unicode.IsControl) {
			return fmt.Errorf("extra_toolchains: invalid entry %q", short(tc))
		}
	}
	if err := checkSorted("repo_inputs", p.RepoInputs); err != nil {
		return err
	}
	for _, in := range p.RepoInputs {
		if err := key.ValidatePath(in); err != nil {
			return fmt.Errorf("repo_inputs: %w", err)
		}
	}
	if err := checkSorted("repo_input_globs", p.RepoInputGlobs); err != nil {
		return err
	}
	for _, gl := range p.RepoInputGlobs {
		if err := source.ValidatePattern(gl); err != nil {
			return fmt.Errorf("repo_input_globs: %w", err)
		}
	}
	return nil
}

// checkBaseName: a BUILD variant is a base name, never a path.
func checkBaseName(n string) error {
	if n == "" || n == "." || n == ".." || strings.ContainsAny(n, "/\\") || !utf8.ValidString(n) {
		return fmt.Errorf("invalid file name %q", short(n))
	}
	return nil
}

// checkSorted demands strictly ascending, so duplicates and disorder are both
// errors (the export promises sorted, de-duplicated lists).
func checkSorted(what string, xs []string) error {
	for i := 1; i < len(xs); i++ {
		if xs[i-1] >= xs[i] {
			return fmt.Errorf("%s not strictly sorted at %q", what, short(xs[i]))
		}
	}
	return nil
}

func checkNameList(what string, xs []string, names map[string]bool) error {
	if err := checkSorted(what, xs); err != nil {
		return err
	}
	for _, x := range xs {
		if !names[x] {
			return fmt.Errorf("%s names unknown package %q", what, short(x))
		}
	}
	return nil
}
