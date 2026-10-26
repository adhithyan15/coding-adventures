package resolver

// Mosaic package manifests
// ========================
//
// A Mosaic package is a directory of `.mil` (interface), `.mll` (layout) and
// `.msl` (style) sources described by a `mosaic-package.toml` manifest (UI29
// §4.2). Discovery already classifies such directories as language "mosaic"
// — they live under code/packages/mosaic/ and code/programs/mosaic/ and carry
// an ordinary BUILD file whose `cargo test` compiles the package through a
// small Rust smoke harness.
//
// What the build tool used to miss is that Mosaic packages depend on each
// other, and say so in their manifest:
//
//	[package]
//	name = "task-app"
//	version = "0.1.0"
//
//	[dependencies]
//	mosaic-pkg-toolkit = "0.16.0"   # a caret requirement on the toolkit
//	mosaic-pkg-sheet   = "0.1.0"
//
// The package compiler inlines `pkg::mosaic-pkg-toolkit::Button` from the
// toolkit's own `.mil`/`.mll`, so an edit to the toolkit changes what task-app
// compiles to. Without an edge mosaic/mosaic-pkg-toolkit →
// mosaic/programs/task-app, CI re-ran the toolkit's tests and none of its
// consumers'.
//
// The rule is the same one every other ecosystem here follows:
//
//   - only the keys of the root `[dependencies]` table are candidates — the
//     requirement string beside each key is version data, not identity;
//   - a key resolves through the Mosaic name table (directory basename and
//     the manifest's own `[package].name`), so a dependency outside the
//     repository is silently ignored;
//   - the lookup is scoped to the "mosaic" language, so a Mosaic dependency
//     can never accidentally bind to a Rust crate that happens to share its
//     name (the smoke harnesses are named `<pkg>-smoke` for exactly that
//     reason, but scope keeps it from mattering).
//
// Unrelated tables are ignored even when they reuse the word: engram-app's
// `[host_assets]` table has a `dependencies = [ ... ]` array of *third-party
// host* coordinates (`file_selector` for Flutter), which must never become a
// graph edge.
//
// The parser is a deliberately small line reader rather than a TOML library:
// the build tool has no TOML dependency, the manifests are machine-checked by
// `mosaic-package-manifest` (so they are always valid TOML in practice), and
// every other manifest reader in this package takes the same approach.
//
// Because a missed edge silently skips a dependent's tests, the reader FAILS
// CLOSED: a manifest written in a valid TOML shape it does not model returns a
// *MosaicManifestError, which stops the whole plan instead of dropping edges.
// Those shapes are
//
//	dependencies = { mosaic-pkg-toolkit = "0.16.0" }   # any root-level key:
//	dependencies.mosaic-pkg-toolkit = "0.16.0"         #   inline table or
//	package.name = "mosaic-pkg-toolkit"                #   dotted key
//	description = """…"""                              # multi-line string
//	["dependenci\u0065s"]                              # any backslash escape
//
// None appears in the repository; a manifest that needs one can be rewritten
// in the plain `[dependencies]` form the reader understands. The same goes
// for a manifest that is not a regular file (a symlink) or is implausibly
// large, and for two Mosaic packages claiming one name
// (checkMosaicNameCollisions), which would otherwise point every dependent's
// edge at whichever package discovery happened to see last.

import (
	"bytes"
	"fmt"
	"io"
	"os"
	"path/filepath"
	"sort"
	"strings"

	"github.com/adhithyan15/coding-adventures/code/programs/go/build-tool/internal/discovery"
)

// mosaicManifestName is the one file that makes a directory a Mosaic package.
const mosaicManifestName = "mosaic-package.toml"

// maxMosaicManifestBytes caps a manifest read. Real manifests are a few
// hundred bytes; anything past this is refused rather than read into memory.
const maxMosaicManifestBytes = 1 << 20

// MosaicManifestError reports a manifest the resolver refuses to guess about.
// Returning it (rather than resolving no edges) keeps change detection from
// failing open.
type MosaicManifestError struct {
	Path   string
	Reason string
}

func (err *MosaicManifestError) Error() string {
	return fmt.Sprintf("%s: %s; write dependencies as keys of a plain [dependencies] table", err.Path, err.Reason)
}

// readMosaicManifestBytes returns the manifest's bytes, found=false when the
// package has no manifest, or an error when the manifest is not a regular
// file or exceeds maxMosaicManifestBytes.
func readMosaicManifestBytes(pkgPath string) (data []byte, found bool, err error) {
	path := filepath.Join(pkgPath, mosaicManifestName)
	info, statErr := os.Lstat(path)
	if statErr != nil {
		if os.IsNotExist(statErr) {
			return nil, false, nil
		}
		return nil, false, &MosaicManifestError{Path: path, Reason: statErr.Error()}
	}
	if !info.Mode().IsRegular() {
		return nil, false, &MosaicManifestError{Path: path, Reason: "is not a regular file"}
	}
	file, openErr := os.Open(path)
	if openErr != nil {
		return nil, false, &MosaicManifestError{Path: path, Reason: openErr.Error()}
	}
	defer file.Close()
	data, readErr := io.ReadAll(io.LimitReader(file, maxMosaicManifestBytes+1))
	if readErr != nil {
		return nil, false, &MosaicManifestError{Path: path, Reason: readErr.Error()}
	}
	if len(data) > maxMosaicManifestBytes {
		return nil, false, &MosaicManifestError{Path: path, Reason: fmt.Sprintf("is larger than %d bytes", maxMosaicManifestBytes)}
	}
	return data, true, nil
}

// readMosaicManifestTables reads pkgPath/mosaic-package.toml and returns, for
// each top-level table header, the keys assigned directly inside it. Keys of
// a dotted sub-table header such as `[dependencies.mosaic-pkg-card]` are
// reported as the key "mosaic-pkg-card" of the "dependencies" table, which is
// how TOML itself defines that spelling.
//
// Example — for
//
//	[package]
//	name = "card"           # comment
//	[dependencies]
//	"mosaic-pkg-grid" = "0.1.0"
//	[dependencies.mosaic-pkg-sheet]
//	version = "0.1.0"
//
// it returns {"package": {"name"}, "dependencies": {"mosaic-pkg-grid",
// "mosaic-pkg-sheet"}} and the value of each key in values["package.name"].
//
// Multi-line arrays are tracked by bracket depth outside strings, so an array
// element that begins with `[` is never mistaken for a table header. Array
// of tables (`[[x]]`) headers end the current table and are otherwise
// ignored: no Mosaic manifest field the resolver reads uses them.
//
// It returns found=false when there is no manifest, and a *MosaicManifestError
// for any shape listed at the top of this file.
func readMosaicManifestTables(pkgPath string) (keys map[string][]string, values map[string]string, found bool, err error) {
	data, found, err := readMosaicManifestBytes(pkgPath)
	if err != nil || !found {
		return nil, nil, found, err
	}
	path := filepath.Join(pkgPath, mosaicManifestName)
	refuse := func(reason string) (map[string][]string, map[string]string, bool, error) {
		return nil, nil, true, &MosaicManifestError{Path: path, Reason: reason}
	}
	// A leading UTF-8 byte-order mark is valid TOML (toml_edit strips it), and
	// left in place it would hide the first table header from this reader.
	data = bytes.TrimPrefix(data, []byte("\xEF\xBB\xBF"))
	if bytes.Contains(data, []byte(`"""`)) || bytes.Contains(data, []byte("'''")) {
		return refuse("uses a multi-line string")
	}
	// No manifest in the repository needs a backslash. Escapes can spell any
	// key, header or `[package].name` (`["dependenci\u0065s"]`) in a form this
	// reader would not decode, so any backslash at all is refused.
	if bytes.ContainsRune(data, '\\') {
		return refuse("uses a backslash escape")
	}

	keys = make(map[string][]string)
	values = make(map[string]string)
	table := "" // "" is the root table, before any header
	depth := 0  // open '[' / '{' count inside a multi-line value

	for _, rawLine := range strings.Split(string(data), "\n") {
		line := strings.TrimSpace(stripTomlComment(rawLine))
		if line == "" {
			continue
		}

		if depth > 0 {
			// Still inside a value that opened on an earlier line.
			depth += tomlBracketDelta(line)
			if depth < 0 {
				depth = 0
			}
			continue
		}

		if strings.HasPrefix(line, "[[") {
			table = "[[array-of-tables]]"
			continue
		}
		if strings.HasPrefix(line, "[") && strings.HasSuffix(line, "]") {
			path := splitTomlKeyPath(line[1 : len(line)-1])
			if len(path) == 0 {
				table = "[[malformed]]"
				continue
			}
			table = path[0]
			if len(path) == 2 {
				// [dependencies.mosaic-pkg-x] declares key mosaic-pkg-x.
				keys[table] = append(keys[table], path[1])
			}
			if len(path) > 2 {
				table = "[[nested]]"
			}
			continue
		}

		eq := tomlKeyValueSplit(line)
		if eq < 0 {
			continue
		}
		path := splitTomlKeyPath(line[:eq])
		value := strings.TrimSpace(line[eq+1:])
		depth = tomlBracketDelta(value)
		if depth < 0 {
			depth = 0
		}
		if len(path) == 0 {
			continue
		}
		if table == "" {
			// Every manifest field is a table, so no valid manifest needs a
			// root-level key. Spelled at the root (`package.name = ...`,
			// `dependencies = { ... }`), a field would be invisible here.
			return refuse("declares a root-level key or inline table")
		}
		keys[table] = append(keys[table], path[0])
		if len(path) == 1 {
			values[table+"."+path[0]] = tomlBasicStringValue(value)
		}
	}
	return keys, values, true, nil
}

// readMosaicPackageName returns the manifest's `[package].name`, lowercased,
// or "" when the manifest is absent or names nothing.
//
// A manifest the reader refuses also names nothing here; parseMosaicDeps
// reports the error for the same package, so the plan still stops.
func readMosaicPackageName(pkgPath string) string {
	_, values, found, err := readMosaicManifestTables(pkgPath)
	if err != nil || !found {
		return ""
	}
	return strings.ToLower(strings.TrimSpace(values["package.name"]))
}

// parseMosaicDeps returns the qualified names of the repository Mosaic
// packages that pkg's manifest lists under `[dependencies]`, sorted and
// unique. A dependency that names pkg itself is dropped rather than turned
// into a self-loop (the manifest validator reports that case; the build
// graph must stay acyclic regardless).
func parseMosaicDeps(pkg discovery.Package, knownNames map[string]string) ([]string, error) {
	keys, _, found, err := readMosaicManifestTables(pkg.Path)
	if err != nil {
		return nil, err
	}
	if !found {
		return nil, nil
	}

	seen := make(map[string]bool)
	for _, key := range keys["dependencies"] {
		if dependency, found := knownNames[strings.ToLower(key)]; found && dependency != pkg.Name {
			seen[dependency] = true
		}
	}

	deps := make([]string, 0, len(seen))
	for dependency := range seen {
		deps = append(deps, dependency)
	}
	sort.Strings(deps)
	return deps, nil
}

// checkMosaicNameCollisions refuses two Mosaic packages that claim one name,
// by directory basename or by `[package].name`. The name table keeps one
// entry per name, so without this check a package declaring
// `name = "mosaic-pkg-toolkit"` would capture every edge meant for the real
// toolkit, and toolkit edits would stop selecting its dependents. Unlike
// other ecosystems here, a library and a program may not share a name either:
// the library would silently win every edge meant for the program, and no
// Mosaic package or app in the repository shares a name today.
func checkMosaicNameCollisions(packages []discovery.Package) error {
	type claim struct{ pkg, path string }
	claims := make(map[string]claim)
	for _, pkg := range packages {
		if pkg.Language != "mosaic" {
			continue
		}
		names := []string{strings.ToLower(filepath.Base(pkg.Path))}
		if manifestName := readMosaicPackageName(pkg.Path); manifestName != "" && manifestName != names[0] {
			names = append(names, manifestName)
		}
		for _, name := range names {
			key := name
			if existing, taken := claims[key]; taken && existing.pkg != pkg.Name {
				return &MosaicManifestError{
					Path:   filepath.Join(pkg.Path, mosaicManifestName),
					Reason: fmt.Sprintf("claims the Mosaic package name %q, which %s (%s) already has", name, existing.pkg, existing.path),
				}
			}
			claims[key] = claim{pkg: pkg.Name, path: pkg.Path}
		}
	}
	return nil
}

// stripTomlComment removes a '#' comment that is not inside a quoted string.
func stripTomlComment(line string) string {
	var quote byte
	for index := 0; index < len(line); index++ {
		character := line[index]
		switch {
		case quote != 0:
			if quote == '"' && character == '\\' {
				index++
			} else if character == quote {
				quote = 0
			}
		case character == '"' || character == '\'':
			quote = character
		case character == '#':
			return line[:index]
		}
	}
	return line
}

// tomlBracketDelta counts '[' and '{' minus ']' and '}' outside strings: the
// number of value brackets a line leaves open for the following lines.
func tomlBracketDelta(text string) int {
	delta := 0
	var quote byte
	for index := 0; index < len(text); index++ {
		character := text[index]
		switch {
		case quote != 0:
			if quote == '"' && character == '\\' {
				index++
			} else if character == quote {
				quote = 0
			}
		case character == '"' || character == '\'':
			quote = character
		case character == '[' || character == '{':
			delta++
		case character == ']' || character == '}':
			delta--
		}
	}
	return delta
}

// tomlKeyValueSplit returns the index of the '=' separating a key from its
// value, skipping any '=' inside a quoted key, or -1 when there is none.
func tomlKeyValueSplit(line string) int {
	var quote byte
	for index := 0; index < len(line); index++ {
		character := line[index]
		switch {
		case quote != 0:
			if quote == '"' && character == '\\' {
				index++
			} else if character == quote {
				quote = 0
			}
		case character == '"' || character == '\'':
			quote = character
		case character == '=':
			return index
		}
	}
	return -1
}

// splitTomlKeyPath splits a (possibly dotted, possibly quoted) TOML key into
// its parts: `a.b`, `"a".b` and `a . "b"` all give ["a", "b"]. A malformed
// key — an unterminated quote or an empty part — yields nil, which callers
// treat as "nothing to read here" rather than guessing.
func splitTomlKeyPath(key string) []string {
	var parts []string
	var current strings.Builder
	var quote byte
	quotedPart := false
	flush := func() bool {
		part := strings.TrimSpace(current.String())
		current.Reset()
		if part == "" && !quotedPart {
			return false
		}
		parts = append(parts, part)
		quotedPart = false
		return true
	}
	for index := 0; index < len(key); index++ {
		character := key[index]
		switch {
		case quote != 0:
			if quote == '"' && character == '\\' && index+1 < len(key) {
				index++
				current.WriteByte(key[index])
			} else if character == quote {
				quote = 0
			} else {
				current.WriteByte(character)
			}
		case character == '"' || character == '\'':
			quote = character
			quotedPart = true
		case character == '.':
			if !flush() {
				return nil
			}
		default:
			current.WriteByte(character)
		}
	}
	if quote != 0 || !flush() {
		return nil
	}
	return parts
}

// tomlBasicStringValue returns the contents of a single-line quoted string
// value, or "" for any other value shape (numbers, arrays, inline tables).
func tomlBasicStringValue(value string) string {
	if len(value) >= 2 && (value[0] == '"' || value[0] == '\'') && value[len(value)-1] == value[0] {
		return value[1 : len(value)-1]
	}
	return ""
}
