// stories.go — .mosaic and .stories.json discovery
//
// MosaicBook needs to know every component in the project so the sidebar can
// list them all.  This file contains the logic for walking the file tree and
// building the in-memory component catalogue.
//
// # Pairing rule
//
// For every Foo.mosaic file found, we look for Foo.stories.json alongside it.
// If Foo.stories.json exists we parse it for stories and an optional display
// title.  If it does NOT exist we synthesise a single story called "Default"
// with an empty fixtures object — the component still appears in the UI.
//
// # Component ID
//
// The ID is the relative path of the .mosaic file from the root, without the
// .mosaic extension and without a leading "./".  For example:
//
//	src/Button.mosaic  →  "src/Button"
//	ProfileCard.mosaic →  "ProfileCard"
//
// IDs are used in API paths like /preview/html/src%2FButton/Default so they
// must be URL-safe (path separators must be percent-encoded by clients).

package main

import (
	"encoding/json"
	"os"
	"path/filepath"
	"regexp"
	"strings"
	"unicode"
)

// Story represents one named variant of a component, together with the fixture
// values (slot → value) that parameterise it.
//
// Fixtures is map[string]interface{} because fixture values can be strings,
// numbers, booleans, or lists — all valid JSON types.
type Story struct {
	Name     string                 `json:"name"`
	Fixtures map[string]interface{} `json:"fixtures"`
}

// Component is the server-side representation of one .mosaic file together
// with all of its stories.
type Component struct {
	// ID is the relative path from root without the .mosaic extension.
	ID string `json:"id"`

	// Title is the human-readable display name shown in the sidebar.
	// Derived from the filename with CamelCase → "Camel Case" expansion,
	// unless the .stories.json file overrides it with a "title" field.
	Title string `json:"title"`

	// SourcePath is the relative path to the .mosaic file from root.
	//
	// Empty for three-file components (see below), which have no single
	// source file.
	SourcePath string `json:"source_path"`

	// The three-file (UI29) form: a component authored as separate
	// interface/layout/style files inside a Mosaic package, rather than as
	// one bundled .mosaic file.
	//
	// Every component in this repo is authored this way — there are no
	// .mosaic files left — so these are the paths that actually get used.
	// When InterfacePath is non-empty the compiler invokes
	// `--interface/--layout/--style` instead of legacy single-file mode.
	//
	// These are deliberately NOT serialised. They are absolute paths, so
	// marshalling them into GET /api/stories would leak the OS username and
	// the server's directory layout to anything that can reach the port.
	// The browser shell needs only id/title/stories.
	InterfacePath string `json:"-"`
	LayoutPath    string `json:"-"`
	StylePath     string `json:"-"`

	// ManifestPath points at the owning package's mosaic-package.toml.
	// Passing it as --package-manifest is what lets a component reference
	// its siblings (Field referencing Input, for example); without it those
	// references fail to resolve.
	ManifestPath string `json:"-"`

	// Stories is the list of story variants.  Always non-empty (at minimum the
	// auto-generated "Default" story is present).
	Stories []Story `json:"stories"`
}

// isThreeFile reports whether this component is authored in the UI29
// interface/layout/style form rather than as a single .mosaic file.
func (c Component) isThreeFile() bool {
	return c.InterfacePath != ""
}

// threeFileComponent builds a Component from a UI29 interface file and its
// siblings.
//
// The set is paired by base name inside one directory:
//
//	Button.mil            interface  (required — the anchor)
//	Button.mll            layout     (required)
//	Button.light.msl      style      (preferred)
//	Button.dark.msl       style      (fallback if no light variant)
//
// A .mil with no matching .mll is not a renderable component (it may be a
// shared interface fragment), so it is skipped rather than reported as a
// broken component.
//
// The owning package's mosaic-package.toml is located by walking up from the
// source directory. It is optional: a component outside any package still
// renders, it just cannot reference siblings.
func threeFileComponent(root, milPath, fileName string) (Component, bool) {
	base := strings.TrimSuffix(fileName, ".mil")

	// The base name becomes the component name, which the react and
	// webcomponent preview wrappers interpolate into an *executing* script
	// block (`React.createElement(Name, null)`, `<name></name>`). A file
	// named `alert(document.domain)||X.mil` would therefore run script on the
	// dev server's origin. Restrict to identifiers so no filename can be
	// anything but an inert name. This also rules out a leading `-`, which
	// would otherwise let a filename act as a compiler flag.
	if !validComponentBase.MatchString(base) {
		return Component{}, false
	}

	dir := filepath.Dir(milPath)

	layout := filepath.Join(dir, base+".mll")
	if !isRegularFileWithin(root, layout) {
		// Interface with no layout — not renderable on its own.
		return Component{}, false
	}

	// Prefer the light stylesheet; fall back to dark so a dark-only
	// component still previews rather than rendering unstyled.
	style := filepath.Join(dir, base+".light.msl")
	if !isRegularFileWithin(root, style) {
		style = filepath.Join(dir, base+".dark.msl")
		if !isRegularFileWithin(root, style) {
			style = ""
		}
	}

	rel, err := filepath.Rel(root, milPath)
	if err != nil {
		return Component{}, false
	}
	id := strings.TrimSuffix(filepath.ToSlash(rel), ".mil")

	return Component{
		ID:            id,
		Title:         componentTitleFromBase(base),
		InterfacePath: milPath,
		LayoutPath:    layout,
		StylePath:     style,
		ManifestPath:  findPackageManifest(dir, root),
		Stories:       []Story{{Name: "Default", Fixtures: map[string]interface{}{}}},
	}, true
}

// validComponentBase constrains a component's base name to an identifier.
//
// The name reaches two dangerous places: an executing script block in the
// react/webcomponent preview wrappers, and the argv handed to mosaic-compile.
// Restricting it here closes both at the source rather than escaping at each
// sink.
var validComponentBase = regexp.MustCompile(`^[A-Za-z][A-Za-z0-9_]*$`)

// isRegularFileWithin reports whether path is an existing regular file that,
// after symlink resolution, still lives inside root.
//
// Both checks matter. filepath.Walk uses Lstat and so never descends into a
// symlinked directory, but a symlink to a *file* looks like an ordinary entry,
// and a plain os.Stat on a sibling follows it. Without this, a tree containing
//
//	Evil.mil -> /home/user/.ssh/id_rsa
//	Evil.mll -> /home/user/.aws/credentials
//
// would be discovered as a valid component and those absolute paths handed to
// the compiler — whose stderr is rendered back into the preview error page,
// making it an arbitrary-file-read primitive over one unauthenticated GET.
func isRegularFileWithin(root, path string) bool {
	// Lstat does not follow, so a symlink fails IsRegular here and is
	// rejected outright — as is a directory.
	info, err := os.Lstat(path)
	if err != nil || !info.Mode().IsRegular() {
		return false
	}
	// Belt and braces: catches the case where an ancestor directory is a
	// symlink pointing outside the served tree.
	return withinRoot(root, path)
}

// withinRoot reports whether path, fully symlink-resolved, is contained by
// root. Used to keep discovery from handing the compiler a path outside the
// served tree.
func withinRoot(root, path string) bool {
	// Both sides must be absolute before comparing: filepath.Rel returns an
	// error when one argument is relative and the other absolute, and the
	// two do mix here — the walk root is whatever --root was given (often
	// relative) while findPackageManifest builds absolute candidates.
	absRoot, err := filepath.Abs(root)
	if err != nil {
		return false
	}
	absPath, err := filepath.Abs(path)
	if err != nil {
		return false
	}
	resolvedRoot, err := filepath.EvalSymlinks(absRoot)
	if err != nil {
		return false
	}
	resolvedPath, err := filepath.EvalSymlinks(absPath)
	if err != nil {
		return false
	}
	rel, err := filepath.Rel(resolvedRoot, resolvedPath)
	if err != nil {
		return false
	}
	return rel != ".." && !strings.HasPrefix(rel, ".."+string(filepath.Separator))
}

// findPackageManifest walks up from dir looking for a mosaic-package.toml,
// stopping at root so the search cannot escape the served tree.
// Returns "" when the component does not belong to a package.
func findPackageManifest(dir, root string) string {
	absRoot, err := filepath.Abs(root)
	if err != nil {
		return ""
	}
	cur, err := filepath.Abs(dir)
	if err != nil {
		return ""
	}
	for {
		candidate := filepath.Join(cur, "mosaic-package.toml")
		// Same guard as the sibling files: a symlinked manifest would
		// otherwise ride into argv as --package-manifest.
		if isRegularFileWithin(root, candidate) {
			return candidate
		}
		if cur == absRoot {
			return ""
		}
		parent := filepath.Dir(cur)
		if parent == cur {
			// Reached the filesystem root without finding a manifest.
			return ""
		}
		cur = parent
	}
}

// storiesFile is the JSON schema of a .stories.json file.
// We use this intermediate struct for unmarshalling so we can handle the
// optional "title" field cleanly.
type storiesFile struct {
	Title   string  `json:"title"`
	Stories []Story `json:"stories"`
}

// discoverComponents walks the root directory tree and returns one Component
// per .mosaic file found.  The walk is depth-first; ordering of the returned
// slice is deterministic (path-alphabetical) because filepath.Walk is sorted.
func discoverComponents(root string) ([]Component, error) {
	var components []Component

	err := filepath.Walk(root, func(path string, info os.FileInfo, err error) error {
		if err != nil {
			// Non-fatal: skip inaccessible paths and continue walking.
			return nil
		}
		if info.IsDir() {
			// Skip hidden directories (e.g. .git, .mosaicbook) to avoid
			// scanning noise and very large trees.
			if strings.HasPrefix(info.Name(), ".") && path != root {
				return filepath.SkipDir
			}
			return nil
		}

		// Three-file (UI29) components: a .mil interface is the anchor, and
		// the sibling .mll/.msl files complete the set. This is the form
		// every component in this repo actually uses.
		if strings.HasSuffix(info.Name(), ".mil") && info.Mode().IsRegular() {
			if c, ok := threeFileComponent(root, path, info.Name()); ok {
				components = append(components, c)
			}
			return nil
		}

		// Only process .mosaic files.
		if !strings.HasSuffix(info.Name(), ".mosaic") {
			return nil
		}

		// Compute the component ID: relative path from root, no extension, no
		// leading separator.
		rel, err := filepath.Rel(root, path)
		if err != nil {
			return nil
		}
		// Normalise to forward slashes so IDs are consistent across OS.
		rel = filepath.ToSlash(rel)
		id := strings.TrimSuffix(rel, ".mosaic")

		// Derive a default title from the final path segment (filename without
		// extension) by inserting spaces before uppercase runs.
		baseName := strings.TrimSuffix(info.Name(), ".mosaic")
		title := componentTitleFromBase(baseName)

		// Look for a sibling .stories.json file (same base name, same dir).
		storiesPath := strings.TrimSuffix(path, ".mosaic") + ".stories.json"
		stories, overrideTitle, err := loadStoriesFile(storiesPath)
		if err != nil {
			// No stories file or parse error → use a single Default story.
			stories = []Story{{Name: "Default", Fixtures: map[string]interface{}{}}}
		}
		if overrideTitle != "" {
			title = overrideTitle
		}

		components = append(components, Component{
			ID:         id,
			Title:      title,
			SourcePath: rel,
			Stories:    stories,
		})
		return nil
	})

	return components, err
}

// loadStoriesFile parses a .stories.json file and returns the stories list and
// any override title.  Returns an error if the file does not exist or cannot
// be parsed.
func loadStoriesFile(path string) ([]Story, string, error) {
	data, err := os.ReadFile(path)
	if err != nil {
		return nil, "", err
	}

	var sf storiesFile
	if err := json.Unmarshal(data, &sf); err != nil {
		return nil, "", err
	}

	stories := sf.Stories
	// If the file has no stories array, synthesise a Default story rather than
	// returning an empty slice — the UI always needs at least one story.
	if len(stories) == 0 {
		stories = []Story{{Name: "Default", Fixtures: map[string]interface{}{}}}
	}

	return stories, sf.Title, nil
}

// componentTitleFromBase converts a CamelCase (or PascalCase) identifier into
// a human-readable title by inserting a space before each uppercase letter
// that follows a lowercase letter or before a run of uppercase letters
// followed by a lowercase letter.
//
// Examples:
//
//	"Button"       → "Button"
//	"ProfileCard"  → "Profile Card"
//	"TaskBoard"    → "Task Board"
//	"HTMLButton"   → "HTML Button"
func componentTitleFromBase(name string) string {
	if name == "" {
		return name
	}

	runes := []rune(name)
	var result []rune

	for i, r := range runes {
		if i == 0 {
			result = append(result, r)
			continue
		}
		prev := runes[i-1]
		// Insert a space when:
		//   (a) current rune is uppercase and previous is lowercase — "Card" in "ProfileCard"
		//   (b) current rune is uppercase and next is lowercase while previous is also
		//       uppercase — "TML" in "HTMLButton" (transition from acronym to word)
		if unicode.IsUpper(r) && unicode.IsLower(prev) {
			result = append(result, ' ')
		} else if i+1 < len(runes) && unicode.IsUpper(r) && unicode.IsUpper(prev) && unicode.IsLower(runes[i+1]) {
			result = append(result, ' ')
		}
		result = append(result, r)
	}

	return string(result)
}

// componentIDFromPath derives a component ID from a relative path string.
// The path may use either slash style; the result always uses forward slashes.
//
//	"src/Button.mosaic"  → "src/Button"
//	"./Widget.mosaic"    → "Widget"
func componentIDFromPath(rel string) string {
	rel = filepath.ToSlash(rel)
	rel = strings.TrimPrefix(rel, "./")
	return strings.TrimSuffix(rel, ".mosaic")
}
