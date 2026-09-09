// mosaic-docs — generate the Mosaic component documentation site.
//
// Why generated
// -------------
// The site must republish as soon as a component ships (#14026). A
// hand-maintained catalog drifts the moment a component lands, and a stale
// catalog is worse than none: it looks authoritative and is wrong. So every
// page here is derived from the packages on disk, and the only way to change
// the site is to change a component.
//
// Where the data comes from
// -------------------------
// Two calls to `mosaic-compile` per component, both to the real compiler
// rather than to a regex over the sources:
//
//	--describe        the declared surface: slots, their types, and the closed
//	                  value set of every `one-of` axis (#14505)
//	--backend html    a static fragment inlined into the page, so a reader sees
//	                  the component rather than a screenshot of it
//
// The alternative -- parsing .mil ourselves -- would re-implement part of
// mosmodel-compiler in a second language and drift the moment the grammar
// changes. It changed twice recently.
package main

import (
	"encoding/json"
	"flag"
	"fmt"
	"os"
	"os/exec"
	"path/filepath"
	"sort"
	"strings"
)

// Component is one documented component, assembled from the compiler's own
// answers rather than from anything this program infers.
type Component struct {
	Name    string
	Package string
	Slots   []Slot
	Emits   []Emit
	// PreviewPath is the emitted html project's index.html, relative to the
	// component's own page. Empty when the component could not be emitted,
	// which is recorded rather than hidden -- see PreviewErr.
	//
	// The page iframes this rather than inlining a fragment. The fragment
	// carries unresolved `{{slot}}` markers -- the html runtime hydrates them
	// at load -- so inlining would show a page of raw placeholders, and
	// substituting them here would re-implement that runtime in a second
	// language.
	PreviewPath string
	PreviewErr  string

	// Previews is one entry per story when the component has a stories file.
	// A component with stories shows a gallery -- every variant side by side,
	// which is the thing that makes an inert axis obvious at a glance and is
	// how six components shipped variant slots that did nothing (#14036).
	Previews []Preview
	HasDark  bool
}

// Preview is one rendered story: a name and the project that renders it.
type Preview struct {
	Name string
	Path string
	Err  string
}

// storiesFile is the sibling `<Component>.stories.json`, the same shape
// MosaicBook reads, so one file serves both the dev server and this site.
type storyDecl struct {
	Name     string                 `json:"name"`
	Fixtures map[string]interface{} `json:"fixtures"`
}

type storiesFile struct {
	Stories []storyDecl `json:"stories"`
}

// Slot mirrors mosmodel's SlotDecl as `--describe` reports it.
type Slot struct {
	Name     string
	Type     string
	OneOf    []string
	Required bool
}

type Emit struct {
	Name string
}

// describeJSON is the shape `mosaic-compile --describe` prints. Slot types are
// either a bare string ("text") or an object ({"oneOf": [...]}), so the type
// field is decoded as a raw message and interpreted below.
type describeJSON struct {
	Component string `json:"component"`
	Slots     []struct {
		Name     string          `json:"name"`
		Type     json.RawMessage `json:"type"`
		Required bool            `json:"required"`
	} `json:"slots"`
	Emits []struct {
		Name string `json:"name"`
	} `json:"emits"`
}

func main() {
	root := flag.String("root", ".", "Repository root to scan for Mosaic packages")
	out := flag.String("out", "public-mosaic", "Directory to write the site into")
	compiler := flag.String("compiler", "mosaic-compile", "Path to the mosaic-compile binary")
	// The coverage matrix costs a few hundred extra compiler invocations, so
	// it is opt-in for a quick local run and on for the published site.
	withCoverage := flag.Bool("coverage", true, "Measure and publish per-backend style coverage")
	flag.Parse()

	comps, err := discover(*root, *compiler, *out)
	if err != nil {
		fmt.Fprintf(os.Stderr, "mosaic-docs: %v\n", err)
		os.Exit(1)
	}
	if len(comps) == 0 {
		// Zero components almost certainly means a wrong --root, not a
		// repository with no components. Failing is more useful than
		// publishing an empty site over a working one.
		fmt.Fprintf(os.Stderr, "mosaic-docs: no components found under %s\n", *root)
		os.Exit(1)
	}
	coverage := CoverageReport{Skipped: "not measured: run without -coverage=false to include it"}
	if *withCoverage {
		coverage = measureCoverage(*compiler, *root)
	}
	if err := render(comps, *out, coverage); err != nil {
		fmt.Fprintf(os.Stderr, "mosaic-docs: %v\n", err)
		os.Exit(1)
	}

	previewed := 0
	for _, c := range comps {
		if c.PreviewPath != "" || len(c.Previews) > 0 {
			previewed++
		}
	}
	fmt.Printf("mosaic-docs: %d component(s), %d with a live preview -> %s\n",
		len(comps), previewed, *out)
}

// discover walks the Mosaic packages and asks the compiler about each
// component that has both an interface and a layout.
func discover(root, compiler, outDir string) ([]Component, error) {
	pkgDir := filepath.Join(root, "code", "packages", "mosaic")
	entries, err := os.ReadDir(pkgDir)
	if err != nil {
		return nil, fmt.Errorf("read %s: %w", pkgDir, err)
	}

	var comps []Component
	for _, pkg := range entries {
		if !pkg.IsDir() {
			continue
		}
		srcDir := filepath.Join(pkgDir, pkg.Name(), "src")
		files, err := os.ReadDir(srcDir)
		if err != nil {
			continue // a package without src/ is not an error, just not documented
		}
		for _, f := range files {
			if !strings.HasSuffix(f.Name(), ".mil") {
				continue
			}
			base := strings.TrimSuffix(f.Name(), ".mil")
			mil := filepath.Join(srcDir, f.Name())
			mll := filepath.Join(srcDir, base+".mll")
			if _, err := os.Stat(mll); err != nil {
				// An interface with no layout is a shared fragment, not a
				// renderable component. Same rule MosaicBook applies.
				continue
			}
			light := filepath.Join(srcDir, base+".light.msl")
			dark := filepath.Join(srcDir, base+".dark.msl")
			// Prefer light, fall back to dark, matching MosaicBook: a
			// dark-only component should still preview rather than be
			// silently absent.
			style := light
			if _, err := os.Stat(style); err != nil {
				style = dark
				if _, err := os.Stat(style); err != nil {
					style = ""
				}
			}
			// A package manifest resolves design tokens. Without it a
			// component referencing `$foundation-color-text-light` fails with
			// "Token not found in token map" -- which reads as a broken
			// component and is really a missing flag.
			manifest := filepath.Join(pkgDir, pkg.Name(), "mosaic-package.toml")
			if _, err := os.Stat(manifest); err != nil {
				manifest = ""
			}

			c := Component{Name: base, Package: pkg.Name()}
			if _, err := os.Stat(dark); err == nil {
				c.HasDark = true
			}
			if err := c.describe(compiler, mil); err != nil {
				return nil, fmt.Errorf("describe %s: %w", base, err)
			}
			c.emitStories(compiler, mil, mll, style, manifest, pkgDir, srcDir, outDir)
			comps = append(comps, c)
		}
	}

	sort.Slice(comps, func(i, j int) bool {
		if comps[i].Package != comps[j].Package {
			return comps[i].Package < comps[j].Package
		}
		return comps[i].Name < comps[j].Name
	})
	return comps, nil
}

func (c *Component) describe(compiler, mil string) error {
	cmd := exec.Command(compiler, "--describe", "--interface", mil)
	stdout, err := cmd.Output()
	if err != nil {
		return fmt.Errorf("mosaic-compile --describe failed: %w", err)
	}
	var d describeJSON
	if err := json.Unmarshal(stdout, &d); err != nil {
		return fmt.Errorf("parse --describe output: %w", err)
	}
	for _, s := range d.Slots {
		slot := Slot{Name: s.Name, Required: s.Required}
		var scalar string
		if err := json.Unmarshal(s.Type, &scalar); err == nil {
			slot.Type = scalar
		} else {
			var oneOf struct {
				OneOf []string `json:"oneOf"`
			}
			if err := json.Unmarshal(s.Type, &oneOf); err == nil && len(oneOf.OneOf) > 0 {
				slot.Type = "one-of"
				slot.OneOf = oneOf.OneOf
			} else {
				// A type shape this program does not know yet (a list, a
				// component reference). Say so rather than printing nothing,
				// so a new slot type is visible instead of silently blank.
				slot.Type = strings.TrimSpace(string(s.Type))
			}
		}
		c.Slots = append(c.Slots, slot)
	}
	for _, e := range d.Emits {
		c.Emits = append(c.Emits, Emit{Name: e.Name})
	}
	return nil
}

// sampleFixtures builds a fixture object from the component's declared slots.
//
// A `one-of` slot uses its FIRST declared value, matching what the emitters'
// own sample logic picks, so the preview agrees with the generated projects. A
// text slot gets its own name, which reads better in a preview than a generic
// placeholder and makes it obvious which slot is which.
//
// Slots whose type this program does not model (lists, node slots, component
// references) are left out rather than guessed at: an omitted slot renders as
// unset, which is a real state, while a wrong guess is just wrong.
func (c *Component) sampleFixtures() string {
	var b strings.Builder
	b.WriteString("{")
	first := true
	for _, s := range c.Slots {
		var value string
		switch {
		case len(s.OneOf) > 0:
			value = s.OneOf[0]
		case s.Type == "text":
			value = s.Name
		default:
			continue
		}
		if !first {
			b.WriteString(",")
		}
		first = false
		// Both key and value go through the JSON encoder so a slot name or
		// value containing a quote cannot break the file.
		k, _ := json.Marshal(s.Name)
		v, _ := json.Marshal(value)
		b.Write(k)
		b.WriteString(":")
		b.Write(v)
	}
	b.WriteString("}")
	if first {
		return "" // nothing to supply
	}
	return b.String()
}

// emitPreview compiles the component to a static html fragment for inlining.
//
// A failure is recorded on the component rather than aborting the build: one
// component that cannot render should not take the whole catalog down, and the
// page saying why is more useful than the component silently missing.
// emitStories renders every story the component declares, or a single sample
// preview when it declares none.
//
// A component with a stories file gets a gallery: each story rendered on its
// own, side by side. That is what makes an inert axis visible -- eight
// identical badges in a row is unmissable, and is exactly what nobody could see
// while stories were impossible (#14031) and fixtures were dropped (#14459).
func (c *Component) emitStories(compiler, mil, mll, style, manifest, searchPath, srcDir, outDir string) {
	stories := readStories(filepath.Join(srcDir, c.Name+".stories.json"))
	if len(stories) == 0 {
		c.emitPreview(compiler, mil, mll, style, manifest, searchPath, outDir, "", c.sampleFixtures())
		return
	}
	for _, st := range stories {
		fixtures, err := json.Marshal(st.Fixtures)
		if err != nil {
			c.Previews = append(c.Previews, Preview{Name: st.Name, Err: err.Error()})
			continue
		}
		c.emitPreview(compiler, mil, mll, style, manifest, searchPath, outDir, st.Name, string(fixtures))
	}
}

// readStories returns the declared stories, or nil when there is no file.
//
// A malformed file returns nil rather than aborting: the component still gets
// its sample preview, and one bad JSON file taking the whole catalog down would
// be worse. Once the CI gate in #14012 exists it can reject the file properly.
func readStories(path string) []storyDecl {
	raw, err := os.ReadFile(path)
	if err != nil {
		return nil
	}
	var f storiesFile
	if err := json.Unmarshal(raw, &f); err != nil {
		return nil
	}
	return f.Stories
}

// storySlug makes a story name safe as a directory component. Story names are
// authored text and can contain spaces or anything else.
func storySlug(name string) string {
	var b strings.Builder
	for _, r := range name {
		switch {
		case r >= 'a' && r <= 'z', r >= 'A' && r <= 'Z', r >= '0' && r <= '9':
			b.WriteRune(r)
		default:
			b.WriteByte('-')
		}
	}
	out := strings.Trim(b.String(), "-")
	if out == "" {
		return "story"
	}
	return out
}

func (c *Component) emitPreview(compiler, mil, mll, style, manifest, searchPath, outDir, story, fixtures string) {
	slug := c.Name
	if story != "" {
		slug = filepath.Join(c.Name, storySlug(story))
	}
	dir := filepath.Join(outDir, c.Package, slug)
	if err := os.MkdirAll(dir, 0o755); err != nil {
		c.PreviewErr = err.Error()
		return
	}

	args := []string{
		"--interface", mil, "--layout", mll,
		"--backend", "html", "--emit-project",
		"--output", filepath.Join(dir, c.Name+".html"),
	}
	if style != "" {
		args = append(args, "--style", style)
	}
	if manifest != "" {
		args = append(args, "--package-manifest", manifest)
	}
	if searchPath != "" {
		// Components that compose other packages (Sheet uses Grid, Input uses
		// std-controls) need somewhere to resolve those from, or they fail
		// with "dependency ... not found" -- again a missing flag reading as
		// a broken component.
		args = append(args, "--package-search-path", searchPath)
	}

	// Supply sample slot values, or the preview hydrates to "Sample Label"
	// placeholders. The values come from the same `--describe` output the
	// slots table is built from, so the preview and the documentation cannot
	// disagree about what a slot is.
	if fixtures != "" {
		fx, err := os.CreateTemp("", "mosaic-docs-fixtures-*.json")
		if err == nil {
			fxPath := fx.Name()
			_, _ = fx.WriteString(fixtures)
			fx.Close()
			defer os.Remove(fxPath)
			args = append(args, "--fixtures", fxPath)
		}
	}

	cmd := exec.Command(compiler, args...)
	if out, err := cmd.CombinedOutput(); err != nil {
		msg := strings.TrimSpace(string(out))
		if msg == "" {
			msg = err.Error()
		}
		_ = os.RemoveAll(dir)
		if story == "" {
			c.PreviewErr = msg
		} else {
			c.Previews = append(c.Previews, Preview{Name: story, Err: msg})
		}
		return
	}
	rel := filepath.ToSlash(filepath.Join(slug, "index.html"))
	if story == "" {
		c.PreviewPath = rel
	} else {
		c.Previews = append(c.Previews, Preview{Name: story, Path: rel})
	}
}
