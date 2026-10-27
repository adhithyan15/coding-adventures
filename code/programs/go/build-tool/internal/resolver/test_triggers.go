package resolver

// Cargo dev-dependencies as test triggers
// =======================================
//
// The dependency graph built by ResolveDependencies answers one question:
// "what must be built before what?" For Rust that comes from the
// `[dependencies]` table of each Cargo.toml. Cargo has a second table that
// the graph deliberately leaves out:
//
//	[dev-dependencies]
//	mosmodel-compiler  = { path = "../../rust/mosmodel-compiler" }
//	moslayout-compiler = { path = "../../rust/moslayout-compiler" }
//
// A dev-dependency is linked into a crate's tests, examples and benches,
// never into its library. That makes it invisible to build ordering — and,
// before this file existed, invisible to change detection too, which is the
// expensive half. Every Mosaic package and app (code/packages/mosaic/*,
// code/programs/mosaic/*) is tested by a small Rust smoke harness whose ONLY
// in-repository dependencies are dev-dependencies on the Mosaic compilers and
// the artifact builder, and through those on every `mosaic-emit-*` backend.
// So a pull request that changed mosaic-emit-qt re-ran the emitter's own
// tests and none of the thirty Mosaic package and app suites that compile
// real `.mll` through it. CI was green on exactly the regressions those
// suites exist to catch.
//
// # Why a separate relation, not more graph edges
//
// The obvious fix is to add a dev-dependency edge D → P to the graph. That is
// wrong for three reasons, the last of which is fatal:
//
//  1. Over-selection. A graph edge makes P a *dependent* of D, so every
//     dependent of P is selected too. But nothing that links P's library has
//     changed — only P's tests have something new to run against. The
//     repository-input consumers in main's repository_inputs.go make the same
//     argument for the same reason: select a test the way a test is selected,
//     not the way a library is.
//  2. Contract. The cross-implementation conformance fixture
//     resolution/rust-field-aware pins the build graph's edges and says
//     `[dev-dependencies]` contribute none. The graph stays what that
//     contract describes.
//  3. Cycles. Cargo allows a dev-dependency cycle: B depends on A for real,
//     and A dev-depends on B to test the two together. As graph edges that is
//     A → B → A, IndependentGroups() reports a cycle, and the executor marks
//     EVERY package in the repository failed. One innocent test harness
//     would take down the whole build.
//
// So dev-dependencies form their own relation, TestTriggers:
//
//	TestTriggers[D] = the packages whose Cargo.toml dev-depends on D
//
// and change detection applies it exactly ONE hop, after the normal closure:
//
//	affected = AffectedNodes(changed)         — build closure over the graph
//	         ∪ TestTriggers[a] for every a in that closure
//	         + prerequisites of all of the above
//
// One hop is not a shortcut, it is the whole semantics. P is re-tested when
// something P's tests link has changed — D itself, or anything D depends on,
// which is precisely "D is in the build closure". P's own dependents are not
// added, because P's library is unchanged; and the triggers are never chased
// transitively, so a cycle through a dev-dependency simply cannot loop.
// Prerequisites added afterwards (expandAffectedSetWithPrereqs) are scheduled
// only so a fresh runner can build, not because they changed, so they are
// added AFTER the hop and never trigger anyone.
//
// # Which tables are read
//
// For every Rust or WASM crate: `[dev-dependencies]` and every
// `[target.<cfg>.dev-dependencies]` (the union over all targets — CI runs on
// several hosts, and any of them may compile the entry). `dev_dependencies`
// is Cargo's legacy spelling of the same table and is read too.
//
// For a Mosaic package's smoke harness, additionally `[dependencies]` and
// `[build-dependencies]` (and their target forms): a Mosaic package has no
// Cargo build-graph edges at all (mosaic.go reads only mosaic-package.toml),
// so ANY path entry in its harness is a test trigger.
//
// # Resolution by path, not by name
//
// A `path = "..."` entry names a directory, so it resolves by directory:
// filepath.Join(manifest directory, path), cleaned, matched against the
// discovered packages' directories. A lookup by Cargo name would let any
// crate declaring `name = "compiler"` capture every trigger meant for the
// real compiler (the name table keeps one entry per name). The target is:
//
//   - a discovered package          → that package;
//   - inside a discovered package   → the containing package, because prefix
//     matching attributes a change there to that package;
//   - an existing crate directory that no discovered package covers → no
//     trigger. This is the one tolerated miss and it is not a silent one:
//     such a crate is not a package to this tool at all — a change inside it
//     selects nothing, not even its own tests — and that state is governed
//     by the orphan-crate gate, which requires each one to be listed in
//     code/BUILD-EXEMPTIONS. Four entries in the repository point at such
//     crates today (jit-loader-macos, paint-vm-cairo ×2, engram-core-wasm),
//     and refusing them would fail every plan. The same rule covers a
//     package excluded by -language: it cannot be in that run's changed set;
//   - anything else (no Cargo.toml there, an absolute or empty path, a
//     backslash) → refused.
//
// `workspace = true` inherits the path from a workspace manifest this reader
// does not follow, so it is refused; no repository harness uses it.
//
// # The strict reader
//
// The reader is a line reader over the plain TOML subset the repository
// writes, and it FAILS CLOSED — a *CargoManifestError stops the plan — on
// anything that could move an entry out of its sight:
//
//   - any multi-line string (`"""` or `'''` anywhere): its content could
//     hide brackets or a whole table from a line reader;
//   - a quoted string left open at the end of a line, brackets that do not
//     match, or a value still open at the end of the file;
//   - a header or key it cannot decode: an escape (any backslash), an empty
//     or malformed segment, a bracket outside quotes. Quoted segments are
//     decoded, so `["dev-dependencies"]` IS the dev-dependency table;
//   - a read table in any shape but the two above: a sub-table
//     (`[dev-dependencies.x]`, `[target.x.dev-dependencies.y]`), an array of
//     tables, a dotted key or an inline table that spells it
//     (`dev-dependencies.x = ...`, `target = { ... }`);
//   - inside a read table, an entry that is not `bare-key = "version"` or
//     `bare-key = { field = value, ... }`. Every field key must be bare and
//     exactly lowercase (Cargo ignores `Package`, so it must not be read as
//     a rename), unique, and its value a one-line string, true/false, or an
//     array of strings; nested inline tables and trailing commas are
//     refused. Only an array may continue onto the next line (TOML 1.0).
//   - a manifest that is not a regular file or exceeds 1 MiB.
//
// A leading UTF-8 byte-order mark is stripped, as TOML allows. Every Cargo
// manifest of a Rust, WASM or Mosaic package in the repository passes.

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

// TestTriggers maps a package to the packages that dev-depend on it, sorted
// and unique. A change that puts the key in the build closure must re-run the
// tests of every package listed — and nothing further (see the file comment).
type TestTriggers map[string][]string

// Dependents returns the packages that dev-depend on name, or nil.
func (triggers TestTriggers) Dependents(name string) []string {
	if triggers == nil {
		return nil
	}
	return triggers[name]
}

// CargoManifestError reports a Cargo.toml whose dependency tables the
// resolver refuses to guess about. Returning it (rather than resolving no
// triggers) keeps change detection from failing open.
type CargoManifestError struct {
	Path   string
	Reason string
}

func (err *CargoManifestError) Error() string {
	return fmt.Sprintf(
		"%s: %s; write each dependency as one line of a plain table, "+
			"e.g. [dev-dependencies] name = { path = \"../name\" }",
		err.Path, err.Reason,
	)
}

// maxCargoManifestBytes caps a Cargo.toml read. Real manifests are a few
// kilobytes; anything past this is refused rather than read into memory.
const maxCargoManifestBytes = 1 << 20

// usesCargoDevDependencies reports which discovered languages carry a Cargo
// crate or smoke harness whose triggers are read.
func usesCargoDevDependencies(language string) bool {
	switch language {
	case "rust", "wasm", "mosaic":
		return true
	default:
		return false
	}
}

// triggerTableNames returns the dependency-table names whose path entries
// are test triggers for a package of the given language (see "Which tables
// are read"). Both spellings Cargo accepts are listed.
func triggerTableNames(language string) map[string]bool {
	names := map[string]bool{"dev-dependencies": true, "dev_dependencies": true}
	if language == "mosaic" {
		for _, name := range []string{"dependencies", "build-dependencies", "build_dependencies"} {
			names[name] = true
		}
	}
	return names
}

// readCargoManifestBytes returns the manifest, found=false when there is
// none, or an error when it is not a regular file or is too large.
func readCargoManifestBytes(path string) ([]byte, bool, error) {
	info, err := os.Lstat(path)
	if err != nil {
		if os.IsNotExist(err) {
			return nil, false, nil
		}
		return nil, false, &CargoManifestError{Path: path, Reason: err.Error()}
	}
	if !info.Mode().IsRegular() {
		return nil, false, &CargoManifestError{Path: path, Reason: "is not a regular file"}
	}
	file, err := os.Open(path)
	if err != nil {
		return nil, false, &CargoManifestError{Path: path, Reason: err.Error()}
	}
	defer file.Close()
	data, err := io.ReadAll(io.LimitReader(file, maxCargoManifestBytes+1))
	if err != nil {
		return nil, false, &CargoManifestError{Path: path, Reason: err.Error()}
	}
	if len(data) > maxCargoManifestBytes {
		return nil, false, &CargoManifestError{Path: path, Reason: fmt.Sprintf("is larger than %d bytes", maxCargoManifestBytes)}
	}
	return data, true, nil
}

// decodeTomlKey decodes a header or key into its dotted segments. Each
// segment is a bare key (letters, digits, '-', '_') or a one-line quoted key
// without escapes:
//
//	dev-dependencies                 → [dev-dependencies]
//	target.'cfg(unix)'.dev-dependencies → [target cfg(unix) dev-dependencies]
//	"dev-dependencies"               → [dev-dependencies]
//
// Anything else — a backslash, an empty segment, an unterminated quote, a
// stray character — is reported as ok=false, which callers refuse.
func decodeTomlKey(key string) ([]string, bool) {
	if strings.ContainsRune(key, '\\') {
		return nil, false
	}
	var segments []string
	index := 0
	for {
		for index < len(key) && (key[index] == ' ' || key[index] == '\t') {
			index++
		}
		if index == len(key) {
			return nil, false // empty segment (or empty key)
		}
		var segment string
		if quote := key[index]; quote == '"' || quote == '\'' {
			end := strings.IndexByte(key[index+1:], quote)
			if end < 0 {
				return nil, false
			}
			segment = key[index+1 : index+1+end]
			index += end + 2
		} else {
			start := index
			for index < len(key) && isBareTomlKey(key[index:index+1]) {
				index++
			}
			segment = key[start:index]
			if segment == "" {
				return nil, false
			}
		}
		segments = append(segments, segment)
		for index < len(key) && (key[index] == ' ' || key[index] == '\t') {
			index++
		}
		if index == len(key) {
			return segments, true
		}
		if key[index] != '.' {
			return nil, false
		}
		index++
	}
}

// tomlBracketStack pushes every '[' and '{' and pops every ']' and '}'
// outside one-line strings. It reports ok=false for a mismatched or
// unexpected closing bracket, or a string still open at the end of the line.
func tomlBracketStack(text string, stack []byte) ([]byte, bool) {
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
			stack = append(stack, character)
		case character == ']' || character == '}':
			open := byte('[')
			if character == '}' {
				open = '{'
			}
			if len(stack) == 0 || stack[len(stack)-1] != open {
				return stack, false
			}
			stack = stack[:len(stack)-1]
		}
	}
	return stack, quote == 0
}

// cargoTableKind classifies a decoded table header for one package.
type cargoTableKind int

const (
	cargoOtherTable   cargoTableKind = iota // not read; its keys are only screened
	cargoTriggerTable                       // its entries are triggers
)

// classifyCargoHeader decides what a decoded header path means. A read
// table is exactly `[name]` or `[target.<cfg>.name]` for a trigger table
// name; any other header that mentions one (or an array of tables that
// does) is refused with a reason.
func classifyCargoHeader(path []string, arrayOfTables bool, names map[string]bool) (cargoTableKind, string) {
	mentions := false
	for _, segment := range path {
		if names[segment] {
			mentions = true
		}
	}
	if !mentions {
		return cargoOtherTable, ""
	}
	if !arrayOfTables {
		if len(path) == 1 {
			return cargoTriggerTable, ""
		}
		if len(path) == 3 && path[0] == "target" && names[path[2]] {
			return cargoTriggerTable, ""
		}
	}
	return cargoOtherTable, "declares a dependency table in a shape this reader does not model"
}

// keyReachesTriggerTable reports whether a key assigned outside every read
// table could nonetheless write into one: its full path (table + key) names
// a trigger table, or it assigns an inline table at `target` or
// `target.<cfg>`, which could spell `{ dev-dependencies = { ... } }`.
func keyReachesTriggerTable(full []string, value string, names map[string]bool) bool {
	for _, segment := range full {
		if names[segment] {
			return true
		}
	}
	if len(full) >= 1 && len(full) <= 2 && full[0] == "target" && strings.HasPrefix(value, "{") {
		return true
	}
	return false
}

// cargoEntry is one dependency entry from a read table.
type cargoEntry struct {
	key  string
	path string // "" for a registry dependency
}

// readCargoTriggerEntries reads a package's Cargo.toml and returns every
// entry of its trigger tables, or found=false when it has no Cargo.toml.
func readCargoTriggerEntries(pkg discovery.Package) (entries []cargoEntry, manifest string, found bool, err error) {
	manifest = filepath.Join(pkg.Path, "Cargo.toml")
	data, found, err := readCargoManifestBytes(manifest)
	if err != nil || !found {
		return nil, manifest, found, err
	}
	refuse := func(format string, args ...any) ([]cargoEntry, string, bool, error) {
		return nil, manifest, true, &CargoManifestError{Path: manifest, Reason: fmt.Sprintf(format, args...)}
	}
	data = bytes.TrimPrefix(data, []byte("\xEF\xBB\xBF"))
	if bytes.Contains(data, []byte(`"""`)) || bytes.Contains(data, []byte("'''")) {
		return refuse("uses a multi-line string, which could hide a dependency table")
	}

	names := triggerTableNames(pkg.Language)
	var table []string // current header path; nil is the root table
	kind := cargoOtherTable
	var stack []byte    // brackets open across lines
	var pending string  // a read-table entry continuing onto the next line
	var pendingLine int // where it started, for messages

	for number, rawLine := range strings.Split(string(data), "\n") {
		lineNumber := number + 1
		line := strings.TrimSpace(stripTomlComment(rawLine))
		if line == "" {
			continue
		}
		var ok bool
		if len(stack) > 0 {
			// Inside a value that opened on an earlier line.
			stack, ok = tomlBracketStack(line, stack)
			if !ok {
				return refuse("line %d: unbalanced brackets or an unterminated string", lineNumber)
			}
			if pending != "" {
				pending += " " + line
				if len(stack) > 0 && stack[len(stack)-1] == '{' {
					return refuse("line %d: an inline table in a dependency table spans lines", pendingLine)
				}
				if len(stack) == 0 {
					entry, reason := parseCargoTriggerEntry(pending)
					if reason != "" {
						return refuse("line %d: %s", pendingLine, reason)
					}
					entries = append(entries, entry)
					pending = ""
				}
			}
			continue
		}

		if strings.HasPrefix(line, "[") {
			arrayOfTables := strings.HasPrefix(line, "[[")
			inner, closed := strings.CutSuffix(line, "]")
			inner = inner[1:]
			if arrayOfTables {
				inner, closed = strings.CutSuffix(inner, "]")
				inner = inner[1:]
				closed = closed && strings.HasSuffix(line, "]]")
			}
			path, decoded := decodeTomlKey(inner)
			if !closed || !decoded {
				return refuse("line %d: cannot decode the table header %q", lineNumber, line)
			}
			var reason string
			kind, reason = classifyCargoHeader(path, arrayOfTables, names)
			if reason != "" {
				return refuse("line %d: the header %q %s", lineNumber, line, reason)
			}
			table = path
			continue
		}

		eq := tomlKeyValueSplit(line)
		if eq < 0 {
			return refuse("line %d: %q is neither a table header nor a key = value line", lineNumber, line)
		}
		key, decoded := decodeTomlKey(line[:eq])
		if !decoded {
			return refuse("line %d: cannot decode the key %q", lineNumber, strings.TrimSpace(line[:eq]))
		}
		value := strings.TrimSpace(line[eq+1:])
		stack, ok = tomlBracketStack(value, nil)
		if !ok {
			return refuse("line %d: unbalanced brackets or an unterminated string", lineNumber)
		}

		if kind != cargoTriggerTable {
			full := append(append([]string{}, table...), key...)
			if keyReachesTriggerTable(full, value, names) {
				return refuse("line %d: the key %q writes a dependency table in a shape this reader does not model", lineNumber, strings.TrimSpace(line[:eq]))
			}
			continue
		}

		rawKey := strings.TrimSpace(line[:eq])
		if len(key) != 1 || !isBareTomlKey(rawKey) {
			return refuse("line %d: the dependency key %q must be one bare key", lineNumber, rawKey)
		}
		if len(stack) > 0 {
			if stack[len(stack)-1] == '{' {
				return refuse("line %d: an inline table in a dependency table spans lines", lineNumber)
			}
			pending, pendingLine = line, lineNumber
			continue
		}
		entry, reason := parseCargoTriggerEntry(line)
		if reason != "" {
			return refuse("line %d: %s", lineNumber, reason)
		}
		entries = append(entries, entry)
	}
	if len(stack) > 0 {
		return refuse("ends with a value still open")
	}
	return entries, manifest, true, nil
}

// oneLineTomlString returns the contents of a one-line basic or literal
// string with no escapes, or ok=false for any other value.
func oneLineTomlString(value string) (string, bool) {
	if len(value) < 2 || (value[0] != '"' && value[0] != '\'') || value[len(value)-1] != value[0] {
		return "", false
	}
	inner := value[1 : len(value)-1]
	if strings.ContainsRune(inner, rune(value[0])) || strings.ContainsRune(inner, '\\') {
		return "", false
	}
	return inner, true
}

// splitTopLevel splits text at separator bytes that sit outside strings and
// brackets. Its input has already passed tomlBracketStack.
func splitTopLevel(text string, separator byte) []string {
	var parts []string
	depth, start := 0, 0
	var quote byte
	for index := 0; index < len(text); index++ {
		character := text[index]
		switch {
		case quote != 0:
			if character == quote {
				quote = 0
			}
		case character == '"' || character == '\'':
			quote = character
		case character == '[' || character == '{':
			depth++
		case character == ']' || character == '}':
			depth--
		case character == separator && depth == 0:
			parts = append(parts, text[start:index])
			start = index + 1
		}
	}
	return append(parts, text[start:])
}

// isCargoFieldKey reports whether key is a bare, exactly-lowercase key, the
// only spelling of an inline-table field Cargo acts on.
func isCargoFieldKey(key string) bool {
	return isBareTomlKey(key) && key == strings.ToLower(key)
}

// validCargoFieldValue accepts the value shapes Cargo dependency fields
// use: a one-line string, true or false, or an array of one-line strings
// (a trailing comma allowed).
func validCargoFieldValue(value string) bool {
	if _, ok := oneLineTomlString(value); ok {
		return true
	}
	if value == "true" || value == "false" {
		return true
	}
	if !strings.HasPrefix(value, "[") || !strings.HasSuffix(value, "]") {
		return false
	}
	elements := splitTopLevel(value[1:len(value)-1], ',')
	for index, element := range elements {
		element = strings.TrimSpace(element)
		if element == "" && index == len(elements)-1 {
			continue // [] or a trailing comma
		}
		if _, ok := oneLineTomlString(element); !ok {
			return false
		}
	}
	return true
}

// parseCargoTriggerEntry parses `key = value` from a read table (the key
// is already known to be one bare key). It returns a reason when the entry
// is in a shape the reader refuses.
func parseCargoTriggerEntry(line string) (cargoEntry, string) {
	eq := tomlKeyValueSplit(line)
	entry := cargoEntry{key: strings.TrimSpace(line[:eq])}
	value := strings.TrimSpace(line[eq+1:])
	// No escape is decoded, so none is accepted: `"a\", path = "..."` must
	// not be split as if the escaped quote ended the string.
	if strings.ContainsRune(value, '\\') {
		return entry, fmt.Sprintf("%s uses a backslash escape", entry.key)
	}

	if _, ok := oneLineTomlString(value); ok {
		return entry, "" // a registry version requirement
	}
	if !strings.HasPrefix(value, "{") || !strings.HasSuffix(value, "}") {
		return entry, fmt.Sprintf("%s is neither a version string nor an inline table", entry.key)
	}
	body := strings.TrimSpace(value[1 : len(value)-1])
	if body == "" {
		return entry, ""
	}
	seen := make(map[string]bool)
	for _, field := range splitTopLevel(body, ',') {
		fieldEq := tomlKeyValueSplit(field)
		if fieldEq < 0 {
			return entry, fmt.Sprintf("%s has an inline-table field %q that is not key = value (a trailing comma?)", entry.key, strings.TrimSpace(field))
		}
		fieldKey := strings.TrimSpace(field[:fieldEq])
		fieldValue := strings.TrimSpace(field[fieldEq+1:])
		if !isCargoFieldKey(fieldKey) {
			return entry, fmt.Sprintf("%s has the inline-table key %q; write it bare and lowercase", entry.key, fieldKey)
		}
		if seen[fieldKey] {
			return entry, fmt.Sprintf("%s repeats the inline-table key %q", entry.key, fieldKey)
		}
		seen[fieldKey] = true
		if !validCargoFieldValue(fieldValue) {
			return entry, fmt.Sprintf("%s gives %s a value that is not a string, boolean or array of strings", entry.key, fieldKey)
		}
		switch fieldKey {
		case "workspace":
			return entry, fmt.Sprintf("%s inherits from a workspace, whose path this reader does not follow", entry.key)
		case "path":
			path, ok := oneLineTomlString(fieldValue)
			if !ok || path == "" {
				return entry, fmt.Sprintf("%s has a path that is not a non-empty string", entry.key)
			}
			if filepath.IsAbs(path) || strings.HasPrefix(path, "/") {
				return entry, fmt.Sprintf("%s has the absolute path %q", entry.key, path)
			}
			entry.path = path
		}
	}
	return entry, ""
}

// packageDirectoryIndex maps each discovered package's cleaned directory to
// its name, for resolving path entries.
func packageDirectoryIndex(packages []discovery.Package) map[string]string {
	index := make(map[string]string, len(packages))
	for _, pkg := range packages {
		index[filepath.Clean(pkg.Path)] = pkg.Name
	}
	return index
}

// resolveCargoPath maps a path entry of pkg to a discovered package (see
// "Resolution by path"). It returns "" with no error for an existing crate
// directory no discovered package covers, and an error for anything else
// that matches nothing.
func resolveCargoPath(pkg discovery.Package, manifest, path string, index map[string]string) (string, error) {
	target := filepath.Clean(filepath.Join(pkg.Path, filepath.FromSlash(path)))
	for directory := target; ; directory = filepath.Dir(directory) {
		if name, ok := index[directory]; ok {
			return name, nil
		}
		if filepath.Dir(directory) == directory {
			break
		}
	}
	if info, err := os.Stat(filepath.Join(target, "Cargo.toml")); err == nil && info.Mode().IsRegular() {
		return "", nil
	}
	return "", &CargoManifestError{
		Path:   manifest,
		Reason: fmt.Sprintf("the path dependency %q is neither a discovered package nor a crate directory", path),
	}
}

// parseRustDevDeps returns the repository packages pkg's trigger tables
// name by path, in file order (duplicates allowed; the caller de-duplicates).
func parseRustDevDeps(pkg discovery.Package, index map[string]string) ([]string, error) {
	entries, manifest, found, err := readCargoTriggerEntries(pkg)
	if err != nil || !found {
		return nil, err
	}
	var deps []string
	for _, entry := range entries {
		if entry.path == "" {
			continue
		}
		name, err := resolveCargoPath(pkg, manifest, entry.path, index)
		if err != nil {
			return nil, err
		}
		if name != "" {
			deps = append(deps, name)
		}
	}
	return deps, nil
}

// resolveTestTriggers reads every Cargo-backed package's trigger tables and
// inverts them into TestTriggers. Self-references (including a path into
// the package's own directory) are dropped. A malformed manifest anywhere is
// an error for the whole resolution.
func resolveTestTriggers(packages []discovery.Package) (TestTriggers, error) {
	index := packageDirectoryIndex(packages)
	sets := make(map[string]map[string]bool)
	for _, pkg := range packages {
		if !usesCargoDevDependencies(pkg.Language) {
			continue
		}
		devDeps, err := parseRustDevDeps(pkg, index)
		if err != nil {
			return nil, err
		}
		for _, dep := range devDeps {
			if dep == pkg.Name {
				continue
			}
			if sets[dep] == nil {
				sets[dep] = make(map[string]bool)
			}
			sets[dep][pkg.Name] = true
		}
	}

	triggers := make(TestTriggers, len(sets))
	for dep, dependents := range sets {
		names := make([]string, 0, len(dependents))
		for name := range dependents {
			names = append(names, name)
		}
		sort.Strings(names)
		triggers[dep] = names
	}
	return triggers, nil
}
