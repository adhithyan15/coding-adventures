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
// is Cargo's legacy spelling of the same table and is read too. So is every
// `[patch.<source>]` table: Cargo builds the crate's tests against a patched
// path even when the dependency itself is a registry version.
//
// For a Mosaic package's smoke harness, additionally `[dependencies]` and
// `[build-dependencies]` (and their target forms): a Mosaic package has no
// Cargo build-graph edges at all (mosaic.go reads only mosaic-package.toml),
// so ANY path entry in its harness is a test trigger.
//
// # Resolution by path, not by name
//
// A `path = "..."` entry names a directory, so it resolves by directory:
// filepath.Join(manifest directory, path) with every symlink resolved
// (EvalSymlinks), matched against the discovered packages' real
// directories. A lookup by Cargo name would let any crate declaring
// `name = "compiler"` capture every trigger meant for the real compiler, and
// a textual comparison would let `../compiler-link` (a symlink) or
// `../Compiler` (another case on a case-insensitive disk) slip past. The
// target is:
//
//   - a discovered package root → that package is a trigger;
//   - any other crate → if it lies inside a discovered package — another
//     package, or the dependent itself — that package is a trigger, because
//     prefix matching attributes a change there to it. If no package covers
//     it, it must be listed exactly in code/BUILD-EXEMPTIONS (the
//     orphan-crate ledger): engram-core-wasm, for one, is how engram-app's
//     tests reach engram-core. Anything else — a crate under a skipped
//     directory such as vendor/, outside code/, or a case variant — is
//     refused;
//   - nothing (or an absolute or empty path, or a backslash) → refused.
//
// In the first two cases the chain continues: the target's own
// [dependencies] and [build-dependencies] (target forms included) are linked
// into the dependent's tests too, so they are followed transitively. A
// package root is no exception, because the build graph records only a Rust
// or WASM package's top-level [dependencies] — not its target-specific or
// build dependencies, and not a Cargo.toml at the root of another language's
// package (a native extension). Each crate's closure is cached
// (crateClosure), and a cycle among these tables is refused, as Cargo
// refuses it.
//
// `workspace = true` inherits the path from a workspace manifest this reader
// does not follow, so it is refused; no repository harness uses it. So is
// `[replace]`, the deprecated predecessor of `[patch]`, and a `git` source
// that is not an https:// or ssh:// URL. A `.cargo/config` or
// `.cargo/config.toml` from the package directory up to the repository root
// that sets `paths`, `[patch]` or `[source]` is refused too
// (checkCargoConfigs).
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
//   - a manifest that is not a regular file or exceeds 1 MiB, or one entry
//     continuing past 256 lines or 64 KiB (lines are joined once, so even a
//     1 MiB manifest is read in linear time).
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
	names := map[string]bool{"dev-dependencies": true, "dev_dependencies": true, "patch": true}
	if language == "mosaic" {
		for name := range dependencyTableNames() {
			names[name] = true
		}
	}
	return names
}

// dependencyTableNames are the tables read inside a crate that is not a
// discovered package root (see follow): what links into whoever depends on
// it. `[patch]` is absent because Cargo honours it only in the root manifest.
func dependencyTableNames() map[string]bool {
	return map[string]bool{"dependencies": true, "build-dependencies": true, "build_dependencies": true}
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
	if len(path) > 0 && path[0] == "replace" {
		return cargoOtherTable, "uses [replace], which this reader does not model; use [patch.<source>]"
	}
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
		// [patch.<source>] is a table of dependency-shaped overrides; a
		// bare [patch] or [target.<cfg>.patch] is not a shape Cargo uses.
		if len(path) == 2 && path[0] == "patch" {
			return cargoTriggerTable, ""
		}
		if len(path) == 1 && path[0] != "patch" {
			return cargoTriggerTable, ""
		}
		if len(path) == 3 && path[0] == "target" && names[path[2]] && path[2] != "patch" {
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
	if len(full) > 0 && full[0] == "replace" {
		return true
	}
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

// readCargoTriggerEntries reads the Cargo.toml at manifest and returns every
// entry of the tables named by names (their target forms included), or
// found=false when there is no such file.
func readCargoTriggerEntries(manifest string, names map[string]bool) (entries []cargoEntry, found bool, err error) {
	data, found, err := readCargoManifestBytes(manifest)
	if err != nil || !found {
		return nil, found, err
	}
	refuse := func(format string, args ...any) ([]cargoEntry, bool, error) {
		return nil, true, &CargoManifestError{Path: manifest, Reason: fmt.Sprintf(format, args...)}
	}
	data = bytes.TrimPrefix(data, []byte("\xEF\xBB\xBF"))
	if bytes.Contains(data, []byte(`"""`)) || bytes.Contains(data, []byte("'''")) {
		return refuse("uses a multi-line string, which could hide a dependency table")
	}

	var table []string // current header path; nil is the root table
	kind := cargoOtherTable
	var stack []byte     // brackets open across lines
	var pending []string // a read-table entry continuing onto later lines
	var pendingBytes int // its length so far, capped at maxCargoEntryBytes
	var pendingLine int  // where it started, for messages

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
			if pending != nil {
				// Collect the lines and join them once: appending to one
				// string per line is quadratic in the entry's length.
				pending = append(pending, line)
				pendingBytes += len(line) + 1
				if len(pending) > maxCargoEntryLines || pendingBytes > maxCargoEntryBytes {
					return refuse("line %d: one dependency entry spans more than %d lines or %d bytes",
						pendingLine, maxCargoEntryLines, maxCargoEntryBytes)
				}
				if len(stack) > 0 && stack[len(stack)-1] == '{' {
					return refuse("line %d: an inline table in a dependency table spans lines", pendingLine)
				}
				if len(stack) == 0 {
					entry, reason := parseCargoTriggerEntry(strings.Join(pending, " "))
					if reason != "" {
						return refuse("line %d: %s", pendingLine, reason)
					}
					entries = append(entries, entry)
					pending, pendingBytes = nil, 0
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
			pending, pendingBytes, pendingLine = []string{line}, len(line), lineNumber
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
	return entries, true, nil
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
		case "git":
			// A local git URL (file://, a bare path) can name a predictable
			// checkout directory and is linked with no trigger; only remote
			// transports are accepted.
			url, ok := oneLineTomlString(fieldValue)
			if !ok || !(strings.HasPrefix(url, "https://") || strings.HasPrefix(url, "ssh://")) {
				return entry, fmt.Sprintf("%s has a git source that is not an https:// or ssh:// URL", entry.key)
			}
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

// maxCargoPathDepth caps how many non-package crates deep a path chain is
// followed. The repository's deepest chain is two; a longer one is refused
// rather than truncated, because truncating would drop triggers.
const maxCargoPathDepth = 16

// maxCargoEntryLines and maxCargoEntryBytes cap one dependency entry that
// continues onto later lines (a long `features = [ ... ]`). Real entries span
// a handful of lines.
const (
	maxCargoEntryLines = 256
	maxCargoEntryBytes = 64 << 10
)

// buildExemptionsLedger is the repository-relative orphan-crate ledger owned
// by internal/validator (orphans.go). Only directories it lists exactly may
// be crates that no discovered package covers.
const buildExemptionsLedger = "code/BUILD-EXEMPTIONS"

// triggerWalk holds the per-resolution caches for following path entries.
type triggerWalk struct {
	index   map[string]string          // real package directory → package name
	roots   map[string]string          // directory → repository root ("" if none)
	ledgers map[string]map[string]bool // repository root → real exempt directories
	configs map[string]error           // directory → result of checking it and its ancestors
	// closures caches crateClosure per (repository root, real crate
	// directory); expanding marks the crates currently being expanded.
	closures  map[string]map[string]bool
	expanding map[string]bool
}

// realDirectory resolves every symlink in path. A path that does not exist
// is an error: a path dependency on nothing would fail in Cargo anyway, and
// silently resolving it could hide a typo that drops a trigger.
func realDirectory(path string) (string, error) {
	real, err := filepath.EvalSymlinks(path)
	if err != nil {
		return "", err
	}
	return filepath.Clean(real), nil
}

// hasExactPathSpelling reports whether every named component in path has the
// same spelling as its directory entry. filepath.EvalSymlinks deliberately
// follows the host filesystem, so on Windows (and case-insensitive macOS
// volumes) ../Compiler can otherwise resolve to an existing ../compiler.
// Dot and dot-dot components name no directory entry and are only traversed.
// Missing paths are left for realDirectory to diagnose below.
func hasExactPathSpelling(fromDir, path string) bool {
	current := filepath.Clean(fromDir)
	components := strings.Split(filepath.Clean(filepath.FromSlash(path)), string(filepath.Separator))
	for _, component := range components {
		switch component {
		case "", ".":
			continue
		case "..":
			current = filepath.Dir(current)
			continue
		}

		entries, err := os.ReadDir(current)
		if err != nil {
			return true
		}
		exact := false
		caseVariant := false
		for _, entry := range entries {
			if entry.Name() == component {
				exact = true
				break
			}
			if strings.EqualFold(entry.Name(), component) {
				caseVariant = true
			}
		}
		if !exact {
			// Inspect the directory entries instead of asking the host whether
			// the misspelling resolves. That makes the result identical on
			// case-sensitive and case-insensitive filesystems.
			if caseVariant {
				return false
			}
			return true
		}
		current = filepath.Join(current, component)
	}
	return true
}

func newTriggerWalk(known []discovery.Package) *triggerWalk {
	walk := &triggerWalk{
		index:     make(map[string]string, len(known)),
		roots:     make(map[string]string),
		ledgers:   make(map[string]map[string]bool),
		configs:   make(map[string]error),
		closures:  make(map[string]map[string]bool),
		expanding: make(map[string]bool),
	}
	for _, pkg := range known {
		directory := filepath.Clean(pkg.Path)
		if real, err := realDirectory(directory); err == nil {
			directory = real
		}
		walk.index[directory] = pkg.Name
	}
	return walk
}

// repositoryRoot returns the nearest ancestor of directory (itself included)
// holding a `.git` entry — a directory in a checkout, a file in a worktree —
// or "" when there is none.
func (walk *triggerWalk) repositoryRoot(directory string) string {
	if root, ok := walk.roots[directory]; ok {
		return root
	}
	root := ""
	if _, err := os.Lstat(filepath.Join(directory, ".git")); err == nil {
		root = directory
	} else if parent := filepath.Dir(directory); parent != directory {
		root = walk.repositoryRoot(parent)
	}
	walk.roots[directory] = root
	return root
}

// exempt reports whether code/BUILD-EXEMPTIONS lists real exactly. A ledger
// line is `<EXCLUDED|PENDING> <repo-relative path>  # reason`; a line in any
// other shape is skipped here (the validator rejects it), which can only
// make this check stricter.
func (walk *triggerWalk) exempt(root, real string) bool {
	if root == "" {
		return false
	}
	ledger, ok := walk.ledgers[root]
	if !ok {
		ledger = make(map[string]bool)
		if data, err := os.ReadFile(filepath.Join(root, filepath.FromSlash(buildExemptionsLedger))); err == nil {
			for _, line := range strings.Split(string(data), "\n") {
				fields := strings.Fields(line)
				if len(fields) < 2 || (fields[0] != "EXCLUDED" && fields[0] != "PENDING") {
					continue
				}
				if directory, err := realDirectory(filepath.Join(root, filepath.FromSlash(fields[1]))); err == nil {
					ledger[directory] = true
				}
			}
		}
		walk.ledgers[root] = ledger
	}
	return ledger[real]
}

// containing returns the discovered package whose directory strictly
// contains real, or "".
func (walk *triggerWalk) containing(real string) string {
	for directory := filepath.Dir(real); ; directory = filepath.Dir(directory) {
		if name, ok := walk.index[directory]; ok {
			return name
		}
		if filepath.Dir(directory) == directory {
			return ""
		}
	}
}

// follow resolves one path entry, written in the manifest at fromDir, into
// out: the trigger for the target itself, then everything the target links.
//
//   - A discovered package root is a trigger. The chain does NOT end there:
//     the build graph records only a Rust or WASM package's top-level
//     [dependencies], so its target-specific and build dependencies, and any
//     Cargo.toml at the root of a package in another language (a Python, Lua
//     or TypeScript native extension), would otherwise be invisible.
//   - Any other crate — nested inside a package (the dependent included) or
//     a listed orphan — makes its containing package a trigger, if any.
//
// Either way the target's [dependencies] and [build-dependencies] (target
// forms included) are linked into the dependent's tests, so they are
// followed (crateClosure). Its [dev-dependencies] are not: they build only
// that crate's own tests.
func (walk *triggerWalk) follow(fromDir, manifest, path, root string, depth int, out map[string]bool) error {
	refuse := func(format string, args ...any) error {
		return &CargoManifestError{Path: manifest, Reason: fmt.Sprintf(format, args...)}
	}
	if !hasExactPathSpelling(fromDir, path) {
		return refuse("the path dependency %q does not use the directory's exact spelling", path)
	}
	target := filepath.Clean(filepath.Join(fromDir, filepath.FromSlash(path)))
	real, err := realDirectory(target)
	if err != nil {
		return refuse("the path dependency %q does not exist", path)
	}
	if name, ok := walk.index[real]; ok {
		out[name] = true
	} else if container := walk.containing(real); container != "" {
		out[container] = true
	} else if !walk.exempt(root, real) {
		return refuse("the path dependency %q is not a discovered package, not inside one, "+
			"and not listed exactly in %s", path, buildExemptionsLedger)
	}
	closure, err := walk.crateClosure(target, real, manifest, path, root, depth)
	if err != nil {
		return err
	}
	for name := range closure {
		out[name] = true
	}
	return nil
}

// crateClosure returns every package the crate at target (real directory
// real) links through its [dependencies] and [build-dependencies], target
// forms included, transitively. Results are cached per crate, so each
// manifest is read once per resolution however many dependents reach it,
// and the walk stays linear in the number of crates.
//
// Cargo rejects a cycle among these tables, so meeting a crate that is still
// being expanded is refused rather than cut short — a cut would leave a
// cached closure missing triggers. Cycles through [dev-dependencies] (which
// the repository has) never reach here: dev-dependencies are read only at the
// dependent itself.
func (walk *triggerWalk) crateClosure(target, real, manifest, path, root string, depth int) (map[string]bool, error) {
	key := root + "\x00" + real
	if closure, ok := walk.closures[key]; ok {
		return closure, nil
	}
	refuse := func(format string, args ...any) (map[string]bool, error) {
		return nil, &CargoManifestError{Path: manifest, Reason: fmt.Sprintf(format, args...)}
	}
	if walk.expanding[key] {
		return refuse("the path dependency %q is part of a dependency cycle, which Cargo rejects", path)
	}
	if depth >= maxCargoPathDepth {
		return refuse("the path dependency %q is more than %d crates deep", path, maxCargoPathDepth)
	}
	walk.expanding[key] = true
	defer delete(walk.expanding, key)

	crateManifest := filepath.Join(target, "Cargo.toml")
	entries, found, err := readCargoTriggerEntries(crateManifest, dependencyTableNames())
	if err != nil {
		return nil, err
	}
	if !found {
		return refuse("the path dependency %q has no Cargo.toml", path)
	}
	closure := make(map[string]bool)
	for _, entry := range entries {
		if entry.path == "" {
			continue
		}
		if err := walk.follow(target, crateManifest, entry.path, root, depth+1, closure); err != nil {
			return nil, err
		}
	}
	walk.closures[key] = closure
	return closure, nil
}

// cargoConfigNames are the configuration files Cargo reads from a
// `.cargo` directory in the invocation directory and every ancestor.
var cargoConfigNames = []string{"config", "config.toml"}

// checkCargoConfigs refuses a Cargo configuration file, in directory or any
// ancestor up to and including root (or the filesystem root when root is
// ""), that could redirect dependencies: a `paths` override list or a
// `[patch]` table. Either would make a package's tests build against a
// directory no manifest names, which no trigger can see.
//
// Refusing is the fail-closed choice. Treating such a file as a shared input
// that rebuilds everything would also be safe, but it would turn every later
// pull request into a full rebuild without anyone being told why; a refusal
// names the file on the first run. The repository's configs only choose a
// Windows linker, so they pass. Directories above the repository root are
// not read: they belong to the machine, not the change under test.
func (walk *triggerWalk) checkCargoConfigs(directory, root string) error {
	if result, ok := walk.configs[directory]; ok {
		return result
	}
	var result error
	for _, name := range cargoConfigNames {
		path := filepath.Join(directory, ".cargo", name)
		data, found, err := readCargoManifestBytes(path)
		if err != nil {
			result = err
			break
		}
		if found {
			if result = scanCargoConfig(path, data); result != nil {
				break
			}
		}
	}
	if result == nil && directory != root {
		if parent := filepath.Dir(directory); parent != directory {
			result = walk.checkCargoConfigs(parent, root)
		}
	}
	walk.configs[directory] = result
	return result
}

// scanCargoConfig refuses `paths`, `patch` and `source` in a Cargo
// configuration file, in every spelling a header or key can give them, and
// anything it cannot decode. `source` is source replacement:
// `[source.crates-io] replace-with = "v"` plus `[source.v] directory = ...`
// builds a registry dependency from repository files.
func scanCargoConfig(path string, data []byte) error {
	refuse := func(format string, args ...any) error {
		return &CargoManifestError{Path: path, Reason: fmt.Sprintf(format, args...)}
	}
	data = bytes.TrimPrefix(data, []byte("\xEF\xBB\xBF"))
	if bytes.Contains(data, []byte(`"""`)) || bytes.Contains(data, []byte("'''")) {
		return refuse("uses a multi-line string")
	}
	redirects := func(full []string) bool {
		return len(full) > 0 && (full[0] == "paths" || full[0] == "patch" || full[0] == "source")
	}
	var table []string
	var stack []byte
	for number, rawLine := range strings.Split(string(data), "\n") {
		line := strings.TrimSpace(stripTomlComment(rawLine))
		if line == "" {
			continue
		}
		var ok bool
		if len(stack) > 0 {
			if stack, ok = tomlBracketStack(line, stack); !ok {
				return refuse("line %d: unbalanced brackets or an unterminated string", number+1)
			}
			continue
		}
		if strings.HasPrefix(line, "[") {
			inner := strings.TrimSuffix(strings.TrimPrefix(line, "[["), "]]")
			if inner == line {
				inner = strings.TrimSuffix(strings.TrimPrefix(line, "["), "]")
			}
			header, decoded := decodeTomlKey(inner)
			if !decoded || !strings.HasSuffix(line, "]") {
				return refuse("line %d: cannot decode the table header %q", number+1, line)
			}
			if redirects(header) {
				return refuse("line %d: redirects dependencies with %q, which no test trigger can see", number+1, line)
			}
			table = header
			continue
		}
		eq := tomlKeyValueSplit(line)
		if eq < 0 {
			return refuse("line %d: %q is neither a table header nor a key = value line", number+1, line)
		}
		key, decoded := decodeTomlKey(line[:eq])
		if !decoded {
			return refuse("line %d: cannot decode the key %q", number+1, strings.TrimSpace(line[:eq]))
		}
		if redirects(append(append([]string{}, table...), key...)) {
			return refuse("line %d: redirects dependencies with %q, which no test trigger can see", number+1, strings.TrimSpace(line[:eq]))
		}
		if stack, ok = tomlBracketStack(strings.TrimSpace(line[eq+1:]), nil); !ok {
			return refuse("line %d: unbalanced brackets or an unterminated string", number+1)
		}
	}
	if len(stack) > 0 {
		return refuse("ends with a value still open")
	}
	return nil
}

// parseRustDevDeps returns the repository packages pkg's trigger tables
// reach by path, following non-package crates (see follow). It returns nil
// for a package without a Cargo.toml.
func (walk *triggerWalk) parseRustDevDeps(pkg discovery.Package) ([]string, error) {
	manifest := filepath.Join(pkg.Path, "Cargo.toml")
	entries, found, err := readCargoTriggerEntries(manifest, triggerTableNames(pkg.Language))
	if err != nil || !found {
		return nil, err
	}
	directory := filepath.Clean(pkg.Path)
	real, err := realDirectory(directory)
	if err != nil {
		return nil, &CargoManifestError{Path: manifest, Reason: err.Error()}
	}
	root := walk.repositoryRoot(real)
	if err := walk.checkCargoConfigs(real, root); err != nil {
		return nil, err
	}
	out := make(map[string]bool)
	for _, entry := range entries {
		if entry.path == "" {
			continue
		}
		if err := walk.follow(directory, manifest, entry.path, root, 0, out); err != nil {
			return nil, err
		}
	}
	deps := make([]string, 0, len(out))
	for name := range out {
		deps = append(deps, name)
	}
	sort.Strings(deps)
	return deps, nil
}

// resolveTestTriggers reads the trigger tables of every Cargo-backed package
// in packages and inverts them into TestTriggers. Path entries resolve
// against known (every discovered package, before any -language filter), so
// a filtered-out package still resolves to its own name rather than looking
// like an unlisted orphan. Self-references are dropped. A malformed manifest
// anywhere is an error for the whole resolution.
func resolveTestTriggers(packages, known []discovery.Package) (TestTriggers, error) {
	walk := newTriggerWalk(append(append([]discovery.Package{}, known...), packages...))
	sets := make(map[string]map[string]bool)
	for _, pkg := range packages {
		if !usesCargoDevDependencies(pkg.Language) {
			continue
		}
		devDeps, err := walk.parseRustDevDeps(pkg)
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
