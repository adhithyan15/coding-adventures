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
	SourcePath string `json:"source_path"`

	// Stories is the list of story variants.  Always non-empty (at minimum the
	// auto-generated "Default" story is present).
	Stories []Story `json:"stories"`
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
