package legacy

import (
	"bytes"
	"crypto/sha256"
	"encoding/json"
	"fmt"
	"path/filepath"
	"regexp"
	"sort"
	"strings"
	"unicode"
	"unicode/utf8"

	"github.com/adhithyan15/coding-adventures/code/programs/go/kiln/internal/key"
)

// Paths of the policy files 7c-3 adds (KILN03 sections 4 and 5, KILN04 5.3).
const (
	ToolchainsPath = "code/kiln/legacy/toolchains.json"
	EnvPath        = "code/kiln/legacy/env.json"
	HostRootsPath  = "code/kiln/legacy/host-roots.json"
)

// ImagePin is the pin of a tool the runner image supplies: its identity in a
// key is Host.ImageID() rather than a version string.
const ImagePin = "image"

// Limits on the shape of toolchains.json: a policy file is reviewed by a
// person, so anything larger than this is a mistake or an attack.
const (
	maxNameLen    = 64
	maxProbes     = 8
	maxProbeArgs  = 8
	maxProbeArg   = 256
	maxRootsPer   = 16
	maxToolchains = 256
)

// Toolchain is one entry of toolchains.json: what a language (or an extra
// toolchain, or an always-included tool group) contributes to a key.
type Toolchain struct {
	Name string
	// Always marks entries every package gets, whatever its closure holds:
	// the shell, the C toolchain and linker, strace (KILN03 4, KILN04 3.1).
	Always bool
	// OS limits the entry to some platforms (graph spelling); empty means all.
	OS []string
	// Pins maps a tool (a binary name) to its exact version, or ImagePin.
	Pins map[string]string
	// Probes are argv lists run by --probe; argv[0] is a bare binary name.
	Probes [][]string
	// Roots maps a tool to the directories its binaries live in, in PATH
	// order; a leading $HOME is expanded by ConstructPATH.
	Roots map[string][]string
}

// Toolchains is a validated toolchains.json.
type Toolchains struct {
	entries map[string]*Toolchain
	names   []string // sorted
	pin     map[string]string
	roots   map[string][]string
	// Digest is SHA-256 of the file's bytes: the `policy:toolchains.json`
	// consumed entry of the static key (KILN03 4).
	Digest [32]byte
}

type toolchainJSON struct {
	Always bool            `json:"always"`
	OS     []string        `json:"os"`
	Pins   json.RawMessage `json:"pins"`
	Probes [][]string      `json:"probes"`
	Roots  json.RawMessage `json:"roots"`
}

// objectMembers reads one JSON object keeping its key order, which a Go map
// would lose: the loaders demand sorted keys, so a reordering is a diff a
// reviewer sees and a duplicate key (which a map would silently merge) is an
// error.
func objectMembers(data []byte) ([]string, []json.RawMessage, error) {
	if len(bytes.TrimSpace(data)) == 0 {
		return nil, nil, nil // an omitted member
	}
	dec := json.NewDecoder(bytes.NewReader(data))
	if t, err := dec.Token(); err != nil || t != json.Delim('{') {
		return nil, nil, fmt.Errorf("want a JSON object")
	}
	var keys []string
	var vals []json.RawMessage
	for dec.More() {
		t, err := dec.Token()
		if err != nil {
			return nil, nil, err
		}
		k, _ := t.(string)
		var raw json.RawMessage
		if err := dec.Decode(&raw); err != nil {
			return nil, nil, err
		}
		keys = append(keys, k)
		vals = append(vals, raw)
	}
	return keys, vals, nil
}

// validName is the spelling of a toolchain, tool or ecosystem name.  The ':'
// is excluded on purpose: names containing it are reserved for Kiln (KILN04
// 3.1), and a '/' would make a binary name a path.
func validName(n string) bool {
	if n == "" || len(n) > maxNameLen {
		return false
	}
	for i := 0; i < len(n); i++ {
		c := n[i]
		alnum := c >= 'a' && c <= 'z' || c >= 'A' && c <= 'Z' || c >= '0' && c <= '9'
		if !alnum && (i == 0 || !strings.ContainsRune("._+-", rune(c))) {
			return false
		}
	}
	return true
}

// validPin accepts ImagePin or an exact version: it starts with a digit and
// holds only letters, digits and `._+-`, so no range, no `latest`, no space.
func validPin(p string) bool {
	if p == ImagePin {
		return true
	}
	if p == "" || len(p) > maxNameLen || p[0] < '0' || p[0] > '9' {
		return false
	}
	for i := 0; i < len(p); i++ {
		c := p[i]
		if !(c >= 'a' && c <= 'z' || c >= 'A' && c <= 'Z' || c >= '0' && c <= '9' || strings.ContainsRune("._+-", rune(c))) {
			return false
		}
	}
	return true
}

// checkPolicyPath is the shared rule for a directory named by policy: an
// absolute path, or one that begins with $HOME or $TMPDIR (host-roots only);
// clean, with no `..`, no backslash and no control characters.  glob admits
// `*` for the ephemeral list.
func checkPolicyPath(p string, allowTmp, glob bool) error {
	if p == "" || len(p) > 1024 || !utf8.ValidString(p) || strings.ContainsFunc(p, unicode.IsControl) || strings.Contains(p, "\\") {
		return fmt.Errorf("invalid path %q", short(p))
	}
	// The list separators would let one entry smuggle a second PATH directory.
	if strings.ContainsAny(p, ":;") {
		return fmt.Errorf("path %q contains a path-list separator", short(p))
	}
	rest := p
	switch {
	case strings.HasPrefix(p, "/"):
	case p == "$HOME" || strings.HasPrefix(p, "$HOME/"):
		rest = p[len("$HOME"):]
	case allowTmp && (p == "$TMPDIR" || strings.HasPrefix(p, "$TMPDIR/")):
		rest = p[len("$TMPDIR"):]
	default:
		return fmt.Errorf("path %q must be absolute or begin with $HOME", short(p))
	}
	if strings.Contains(rest, "$") || !glob && strings.Contains(rest, "*") {
		return fmt.Errorf("path %q has a variable or wildcard where none is allowed", short(p))
	}
	if rest == "" {
		return nil
	}
	if rest == "/" || strings.HasSuffix(rest, "/") || strings.Contains(rest, "//") {
		return fmt.Errorf("path %q is not clean", short(p))
	}
	for _, seg := range strings.Split(rest[1:], "/") {
		if seg == ".." || seg == "." {
			return fmt.Errorf("path %q has a %q segment", short(p), seg)
		}
	}
	return nil
}

var probeArgRe = regexp.MustCompile(`^(-{1,2}[A-Za-z][A-Za-z0-9._=-]*|[A-Za-z][A-Za-z0-9._-]*)$`)

// evalFlags are interpreter flags that run supplied code; a probe may never
// carry one, in either dash spelling and with or without `=value`.
var evalFlags = map[string]bool{"c": true, "e": true, "E": true, "r": true, "eval": true, "exec": true, "command": true}

// checkProbeArg accepts a flag-shaped argument (`-v`, `--version`,
// `--pkg-info=x`) or a bare word (`version`), never a path, a space or an
// eval flag.
func checkProbeArg(a string) error {
	if len(a) > maxProbeArg || !probeArgRe.MatchString(a) {
		return fmt.Errorf("argument %q is not a flag or a bare word", short(a))
	}
	name, _, _ := strings.Cut(strings.TrimLeft(a, "-"), "=")
	if strings.HasPrefix(a, "-") && evalFlags[name] {
		return fmt.Errorf("argument %q evaluates code", short(a))
	}
	return nil
}

// ParseToolchains reads toolchains.json strictly: unknown fields, duplicate
// or unsorted keys, a malformed name, a floating or empty pin, a probe that is
// not a bare binary plus arguments, a root that is not a clean absolute path,
// and two entries that disagree about one tool are all errors.
func ParseToolchains(data []byte) (*Toolchains, error) {
	keys, vals, err := objectMembers(data)
	if err != nil {
		return nil, fmt.Errorf("toolchains: %w", err)
	}
	if err := checkSorted("toolchains", keys); err != nil {
		return nil, fmt.Errorf("toolchains: %w", err)
	}
	if len(keys) > maxToolchains {
		return nil, fmt.Errorf("toolchains: more than %d entries", maxToolchains)
	}
	tc := &Toolchains{entries: map[string]*Toolchain{}, pin: map[string]string{}, roots: map[string][]string{}, Digest: sha256.Sum256(data)}
	// objectMembers stops at the end of the object; anything after it (or
	// inside a value the entry decoder would not accept) must still fail.
	if err := strictDecode(data, &map[string]json.RawMessage{}); err != nil {
		return nil, fmt.Errorf("toolchains: %w", err)
	}
	for i, name := range keys {
		if !validName(name) {
			return nil, fmt.Errorf("toolchains: invalid name %q", short(name))
		}
		e, err := parseToolchain(name, vals[i])
		if err != nil {
			return nil, fmt.Errorf("toolchains: %s: %w", name, err)
		}
		tc.entries[name] = e
		tc.names = append(tc.names, name)
		for tool, v := range e.Pins {
			if old, ok := tc.pin[tool]; ok && old != v {
				return nil, fmt.Errorf("toolchains: tool %q is pinned to %q and to %q", tool, old, v)
			}
			tc.pin[tool] = v
		}
		for tool, dirs := range e.Roots {
			if old, ok := tc.roots[tool]; ok && strings.Join(old, "\x00") != strings.Join(dirs, "\x00") {
				return nil, fmt.Errorf("toolchains: tool %q has two different root lists", tool)
			}
			tc.roots[tool] = dirs
		}
	}
	return tc, nil
}

func parseToolchain(name string, raw json.RawMessage) (*Toolchain, error) {
	var j toolchainJSON
	if err := policyDecode(raw, &j); err != nil {
		return nil, err
	}
	e := &Toolchain{Name: name, Always: j.Always, OS: j.OS, Probes: j.Probes, Pins: map[string]string{}, Roots: map[string][]string{}}
	if err := checkSorted("os", j.OS); err != nil {
		return nil, err
	}
	for _, o := range j.OS {
		if o != "linux" && o != "darwin" && o != "windows" {
			return nil, fmt.Errorf("os %q is not linux, darwin or windows", short(o))
		}
	}
	pk, pv, err := objectMembers(j.Pins)
	if err != nil {
		return nil, fmt.Errorf("pins: %w", err)
	}
	if err := checkSorted("pins", pk); err != nil {
		return nil, err
	}
	for i, tool := range pk {
		var v string
		if err := json.Unmarshal(pv[i], &v); err != nil {
			return nil, fmt.Errorf("pins[%s]: %w", tool, err)
		}
		if !validName(tool) {
			return nil, fmt.Errorf("pins: invalid tool name %q", short(tool))
		}
		if !validPin(v) {
			return nil, fmt.Errorf("pins[%s]: %q is neither %q nor an exact version", tool, short(v), ImagePin)
		}
		e.Pins[tool] = v
	}
	if len(j.Probes) > maxProbes {
		return nil, fmt.Errorf("more than %d probes", maxProbes)
	}
	for _, argv := range j.Probes {
		if len(argv) == 0 || len(argv) > maxProbeArgs || !validName(argv[0]) {
			return nil, fmt.Errorf("probe %q must be a binary name plus at most %d arguments", short(strings.Join(argv, " ")), maxProbeArgs-1)
		}
		// A probe runs a binary named by a PR-editable file, so it may only run
		// a tool this entry pins, with arguments that cannot make it evaluate
		// code.
		if _, ok := e.Pins[argv[0]]; !ok {
			return nil, fmt.Errorf("probe of %q: not one of this entry's pins", short(argv[0]))
		}
		for _, a := range argv[1:] {
			if err := checkProbeArg(a); err != nil {
				return nil, fmt.Errorf("probe of %q: %w", short(argv[0]), err)
			}
		}
	}
	rk, rv, err := objectMembers(j.Roots)
	if err != nil {
		return nil, fmt.Errorf("roots: %w", err)
	}
	if err := checkSorted("roots", rk); err != nil {
		return nil, err
	}
	for i, tool := range rk {
		if _, ok := e.Pins[tool]; !ok {
			return nil, fmt.Errorf("roots: %q is not one of this entry's pins", short(tool))
		}
		var dirs []string
		if err := json.Unmarshal(rv[i], &dirs); err != nil {
			return nil, fmt.Errorf("roots[%s]: %w", tool, err)
		}
		if len(dirs) == 0 || len(dirs) > maxRootsPer {
			return nil, fmt.Errorf("roots[%s]: want 1 to %d directories", tool, maxRootsPer)
		}
		seen := map[string]bool{}
		for _, d := range dirs {
			if err := checkPolicyPath(d, false, false); err != nil {
				return nil, fmt.Errorf("roots[%s]: %w", tool, err)
			}
			if seen[d] {
				return nil, fmt.Errorf("roots[%s]: %q is listed twice", tool, short(d))
			}
			seen[d] = true
		}
		e.Roots[tool] = dirs
	}
	return e, nil
}

// Names lists the entries, sorted.
func (t *Toolchains) Names() []string { return append([]string(nil), t.names...) }

// Entry returns the entry called name.
func (t *Toolchains) Entry(name string) (*Toolchain, bool) {
	e, ok := t.entries[name]
	return e, ok
}

// HasTool reports whether some entry pins a tool of this name.
func (t *Toolchains) HasTool(tool string) bool { _, ok := t.pin[tool]; return ok }

// applies reports whether an entry counts on this platform.
func (e *Toolchain) applies(osName string) bool {
	if len(e.OS) == 0 {
		return true
	}
	for _, o := range e.OS {
		if o == osName {
			return true
		}
	}
	return false
}

// selected lists the entries a closure with these language and toolchain names
// uses on osName: every always-included entry, then each named entry.  Names
// that have no entry come back separately and sorted, so the planner can make
// them a hard reason: a missing entry must never read as "no tools" (KILN03 4).
func (t *Toolchains) selected(names []string, osName string) (used []*Toolchain, unknown []string) {
	seen := map[string]bool{}
	for _, n := range t.names {
		if e := t.entries[n]; e.Always && e.applies(osName) {
			used = append(used, e)
			seen[n] = true
		}
	}
	for _, n := range names {
		if seen[n] {
			continue
		}
		seen[n] = true
		e, ok := t.entries[n]
		if !ok {
			unknown = append(unknown, n)
			continue
		}
		if e.applies(osName) {
			used = append(used, e)
		}
	}
	sort.Strings(unknown)
	return used, unknown
}

// ToolsFor is the `tools` section of a key for a closure whose members use
// these languages and extra toolchains (KILN03 4): the pins of every named
// entry plus the always-included set, as KILN02 kind-2 (host-supplied)
// entries sorted by tool name.  A tool's identity is its pin, or imageID for an
// ImagePin tool.  It is the one function both the static key and the KILN04
// weak key call.  unknown names the entries that do not exist.
func (t *Toolchains) ToolsFor(names []string, osName, imageID string) (tools []key.Tool, unknown []string) {
	used, unknown := t.selected(names, osName)
	pins := map[string]string{}
	for _, e := range used {
		for tool, v := range e.Pins {
			pins[tool] = v // consistent across entries: ParseToolchains checked
		}
	}
	tools = make([]key.Tool, 0, len(pins))
	for tool, v := range pins {
		id := v
		if v == ImagePin {
			id = imageID
		}
		tools = append(tools, key.Tool{Name: tool, Version: v, Kind: key.ToolHost, Identity: []byte(id)})
	}
	sort.Slice(tools, func(a, b int) bool { return tools[a].Name < tools[b].Name })
	return tools, unknown
}

// systemDir reports whether a PATH directory is a system one: those come last,
// so a pinned toolchain's directory wins over the image's copy of the tool.
func systemDir(d string) bool {
	for _, p := range []string{"/usr", "/bin", "/sbin", "/opt/homebrew"} {
		if d == p || strings.HasPrefix(d, p+"/") {
			return true
		}
	}
	return false
}

// validHome says whether home may be spliced into a PATH: absolute, already
// clean, and free of list separators. A relative or odd HOME would put a
// relative directory on the PATH, which is the checkout when the probe runs.
func validHome(home string) bool {
	return home != "" && filepath.IsAbs(home) && filepath.Clean(home) == home &&
		!strings.ContainsRune(home, filepath.ListSeparator)
}

// ConstructPATH builds a PATH from the roots of exactly the given tools
// (KILN04 4.2), so recording and PR jobs get the same value and a pin bump for
// an unrelated language changes nothing.  Order is fixed: the directories of
// non-system roots first, then system ones; within each class tools go in
// name order and a tool's directories in file order; the first occurrence of a
// directory wins.  A leading $HOME is replaced by home; a $HOME root with no
// home is left out.  Never `.`, never `node_modules/.bin`, never a shim: only
// what toolchains.json names.
func (t *Toolchains) ConstructPATH(tools []key.Tool, home string) string {
	if !validHome(home) {
		home = ""
	}
	names := make([]string, 0, len(tools))
	for _, tl := range tools {
		names = append(names, tl.Name)
	}
	sort.Strings(names)
	var first, last []string
	seen := map[string]bool{}
	for _, n := range names {
		for _, d := range t.roots[n] {
			if d == "$HOME" || strings.HasPrefix(d, "$HOME/") {
				if home == "" {
					continue
				}
				d = filepath.ToSlash(home) + d[len("$HOME"):]
			}
			if seen[d] {
				continue
			}
			seen[d] = true
			if systemDir(d) {
				last = append(last, d)
			} else {
				first = append(first, d)
			}
		}
	}
	return strings.Join(append(first, last...), string(filepath.ListSeparator))
}
