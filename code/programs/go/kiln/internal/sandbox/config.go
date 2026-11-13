// Package sandbox is `kiln sandbox-exec` (KILN04 sections 4.3 and 4.7): the
// helper that runs one traced BUILD step inside new user, mount and PID
// namespaces with a read-only filesystem, no capabilities and a seccomp
// filter.
//
// This file, plan.go, bpf.go and privs.go are PURE: they compute what the
// sandbox does (the validated configuration, the ordered mount list, the BPF
// program, the privilege-drop sequence) and touch no kernel interface, so they
// compile and are tested on every OS.  Only sandbox_linux_amd64.go acts on the
// plan; every other platform gets a stub that refuses to run.
package sandbox

import (
	"bytes"
	"encoding/json"
	"errors"
	"fmt"
	"io"
	"path"
	"sort"
	"strings"
	"unicode/utf8"
)

// MaxConfigBytes bounds the configuration file: it is read whole, and an
// unbounded read of a file a build could influence is a memory bomb.
const MaxConfigBytes = 16 << 20

// Limits on the configuration's shape.
const (
	maxPath       = 4096
	maxStateRoots = 64
	maxMasks      = 256
	maxActions    = 100000
	maxEnvEntries = 4096
)

// InsideID is the non-zero uid and gid the caller is mapped to inside the
// sandbox.  Root inside would make files the caller owns look root-owned and
// would let some tools take a privileged code path; a fixed non-zero id keeps
// the run the same on every runner.
const InsideID = 1000

// Config is the configuration the recorder (KILN04 stage 4) writes.  Every
// path is absolute and clean.
type Config struct {
	// RepoRoot is the checkout.  It is bound writable at this same absolute
	// path, because the trace parser compares absolute paths.
	RepoRoot string `json:"repo_root"`
	// Home and Tmpdir are the paths of the action's $HOME and TMPDIR, each
	// mounted as a fresh, empty tmpfs.
	Home   string `json:"home"`
	Tmpdir string `json:"tmpdir"`
	// StateRoots are the declared `state` host roots (KILN04 section 5.3),
	// bound writable.
	StateRoots []string `json:"state_roots"`
	// Masks are paths made absent: an empty read-only tmpfs over a directory,
	// an empty file over a file.
	Masks []string `json:"masks"`
	// EnvByAction maps a package rel_path (the KILN_ACTION the build tool
	// passes) to the exact scrubbed environment of its steps.
	EnvByAction map[string][]string `json:"env_by_action"`
	// KilnExe is the absolute path the running binary must have.
	KilnExe string `json:"kiln_exe"`
}

// configFields are the only top-level keys, spelled exactly.
var configFields = map[string]bool{
	"repo_root": true, "home": true, "tmpdir": true, "state_roots": true,
	"masks": true, "env_by_action": true, "kiln_exe": true,
}

// ParseConfig decodes and validates a configuration.  It is strict: unknown
// keys, keys spelled in another case, duplicate keys, trailing data and
// missing fields are all refused.  Go's decoder matches field names
// case-insensitively and lets the last duplicate win, and what a reviewer reads
// must be what is run.
func ParseConfig(data []byte) (*Config, error) {
	if len(data) > MaxConfigBytes {
		return nil, errors.New("config too large")
	}
	if err := checkNoDuplicateKeys(data); err != nil {
		return nil, err
	}
	var top map[string]json.RawMessage
	if err := strictUnmarshal(data, &top); err != nil {
		return nil, err
	}
	for k := range top {
		if !configFields[k] {
			return nil, fmt.Errorf("config: unknown key %q", clip(k))
		}
	}
	for k := range configFields {
		if _, ok := top[k]; !ok {
			return nil, fmt.Errorf("config: missing key %q", k)
		}
	}
	var c Config
	dec := json.NewDecoder(bytes.NewReader(data))
	dec.DisallowUnknownFields()
	if err := dec.Decode(&c); err != nil {
		return nil, fmt.Errorf("config: %v", err)
	}
	if err := c.Validate(); err != nil {
		return nil, err
	}
	return &c, nil
}

// strictUnmarshal decodes exactly one JSON value, refusing trailing data.
func strictUnmarshal(data []byte, v any) error {
	dec := json.NewDecoder(bytes.NewReader(data))
	if err := dec.Decode(v); err != nil {
		return fmt.Errorf("config: %v", err)
	}
	if _, err := dec.Token(); !errors.Is(err, io.EOF) {
		return errors.New("config: trailing data after the JSON value")
	}
	return nil
}

// checkNoDuplicateKeys walks the token stream and refuses an object that
// repeats a key.
func checkNoDuplicateKeys(data []byte) error {
	dec := json.NewDecoder(bytes.NewReader(data))
	type frame struct {
		isObj bool
		keys  map[string]bool
		want  bool // an object frame expects a key next
	}
	var stack []*frame
	for {
		tok, err := dec.Token()
		if errors.Is(err, io.EOF) {
			return nil
		}
		if err != nil {
			return fmt.Errorf("config: %v", err)
		}
		if n := len(stack); n > 0 && stack[n-1].isObj && stack[n-1].want {
			if key, ok := tok.(string); ok {
				f := stack[n-1]
				if f.keys[key] {
					return fmt.Errorf("config: duplicate key %q", clip(key))
				}
				f.keys[key] = true
				f.want = false
				continue
			}
		}
		switch v := tok.(type) {
		case json.Delim:
			switch v {
			case '{':
				stack = append(stack, &frame{isObj: true, keys: map[string]bool{}, want: true})
			case '[':
				stack = append(stack, &frame{})
			default: // '}' or ']'
				stack = stack[:len(stack)-1]
				if n := len(stack); n > 0 && stack[n-1].isObj {
					stack[n-1].want = true
				}
			}
		default: // a scalar value
			if n := len(stack); n > 0 && stack[n-1].isObj {
				stack[n-1].want = true
			}
		}
	}
}

// clip shortens attacker-influenced text for an error message.
func clip(s string) string {
	if len(s) > 64 {
		return s[:64] + "..."
	}
	return s
}

// checkPath refuses a path that is not absolute and clean, is the root, or
// holds a character a path list or a mount option could misread.  ':' is the
// list separator of PATH and friends; ',' separates mount options; control
// characters and non-UTF-8 bytes have no business in a checkout path.
func checkPath(what, p string) error {
	switch {
	case p == "":
		return fmt.Errorf("config: %s is empty", what)
	case len(p) > maxPath:
		return fmt.Errorf("config: %s is too long", what)
	case !utf8.ValidString(p):
		return fmt.Errorf("config: %s is not valid UTF-8", what)
	case !path.IsAbs(p):
		return fmt.Errorf("config: %s %q is not absolute", what, clip(p))
	case path.Clean(p) != p:
		return fmt.Errorf("config: %s %q is not clean", what, clip(p))
	case p == "/":
		return fmt.Errorf("config: %s must not be the filesystem root", what)
	}
	for _, r := range p {
		if r < 0x20 || r == 0x7f || r == ':' || r == ',' {
			return fmt.Errorf("config: %s %q holds a list separator or control character", what, clip(p))
		}
	}
	return nil
}

// within reports whether p is root or lies beneath it.  Both are clean
// absolute paths.
func within(p, root string) bool {
	return p == root || strings.HasPrefix(p, root+"/")
}

// Validate checks everything that needs no filesystem.
func (c *Config) Validate() error {
	for _, f := range []struct{ what, p string }{
		{"repo_root", c.RepoRoot}, {"home", c.Home}, {"tmpdir", c.Tmpdir}, {"kiln_exe", c.KilnExe},
	} {
		if err := checkPath(f.what, f.p); err != nil {
			return err
		}
	}
	if len(c.StateRoots) > maxStateRoots || len(c.Masks) > maxMasks {
		return errors.New("config: too many state_roots or masks")
	}
	for _, p := range c.StateRoots {
		if err := checkPath("state root", p); err != nil {
			return err
		}
	}
	for _, p := range c.Masks {
		if err := checkPath("mask", p); err != nil {
			return err
		}
	}
	for _, w := range append([]string{c.RepoRoot, c.Tmpdir}, c.StateRoots...) {
		if within(c.KilnExe, w) {
			return fmt.Errorf("config: kiln_exe lies inside %q, where the build could rewrite the sandbox", clip(w))
		}
	}
	// /dev, /run and /var/run are replaced by the sandbox, so nothing
	// configured can live there.
	for _, f := range []struct{ what, p string }{{"repo_root", c.RepoRoot}, {"home", c.Home}, {"tmpdir", c.Tmpdir}} {
		for _, r := range reserved {
			if within(f.p, r) {
				return fmt.Errorf("config: %s %q lies inside %s, which the sandbox replaces", f.what, clip(f.p), r)
			}
		}
	}
	for _, s := range c.StateRoots {
		for _, r := range reserved {
			if within(s, r) {
				return fmt.Errorf("config: state root %q lies inside %s, which the sandbox replaces", clip(s), r)
			}
		}
	}
	// The checkout may not be $HOME or TMPDIR, and neither may lie inside it.
	if within(c.Home, c.RepoRoot) || within(c.Tmpdir, c.RepoRoot) {
		return errors.New("config: home and tmpdir must not be inside repo_root")
	}
	if c.Home == c.Tmpdir {
		return errors.New("config: home and tmpdir must differ")
	}
	// TMPDIR is the staging area the new root is assembled in, so nothing that
	// must be bound from the host may live beneath it.
	if within(c.RepoRoot, c.Tmpdir) {
		return errors.New("config: repo_root must not be inside tmpdir")
	}
	seen := map[string]bool{}
	for _, s := range c.StateRoots {
		switch {
		case seen[s]:
			return fmt.Errorf("config: duplicate state root %q", clip(s))
		case within(s, c.RepoRoot) || within(c.RepoRoot, s):
			return fmt.Errorf("config: state root %q overlaps repo_root", clip(s))
		case within(s, c.Tmpdir) || within(c.Tmpdir, s):
			return fmt.Errorf("config: state root %q overlaps tmpdir", clip(s))
		case within(c.Home, s):
			return fmt.Errorf("config: state root %q covers home", clip(s))
		}
		seen[s] = true
	}
	seen = map[string]bool{}
	for _, m := range c.Masks {
		if seen[m] {
			return fmt.Errorf("config: duplicate mask %q", clip(m))
		}
		seen[m] = true
		// An in-repo mask is a DIRECT child of the checkout: a step may rename
		// an ancestor of a mount point around the mask, and may not rename a
		// mount point itself.  No mask lies inside a state root, which is
		// writable for the same reason.
		if within(m, c.RepoRoot) && m != c.RepoRoot && path.Dir(m) != c.RepoRoot {
			return fmt.Errorf("config: mask %q is inside the checkout but not a direct child of it", clip(m))
		}
		for _, s := range c.StateRoots {
			if within(m, s) {
				return fmt.Errorf("config: mask %q lies inside the state root %q", clip(m), clip(s))
			}
		}
		// A mask hides what it covers; covering the checkout, the scratch
		// directories or a state root would make the step unrunnable.
		for _, keep := range append([]string{c.RepoRoot, c.Home, c.Tmpdir}, c.StateRoots...) {
			if within(keep, m) {
				return fmt.Errorf("config: mask %q covers %q", clip(m), clip(keep))
			}
		}
	}
	if len(c.EnvByAction) == 0 {
		return errors.New("config: env_by_action is empty")
	}
	if len(c.EnvByAction) > maxActions {
		return errors.New("config: too many actions")
	}
	for action, env := range c.EnvByAction {
		if err := checkAction(action); err != nil {
			return err
		}
		if err := checkEnv(action, env); err != nil {
			return err
		}
	}
	return nil
}

// checkAction requires an action to be a clean relative path inside the
// checkout ("." is the checkout itself).
func checkAction(a string) error {
	switch {
	case a == "" || len(a) > maxPath || !utf8.ValidString(a):
		return errors.New("config: an action name is empty, too long or not UTF-8")
	case path.IsAbs(a) || path.Clean(a) != a || a == ".." || strings.HasPrefix(a, "../"):
		return fmt.Errorf("config: action %q is not a clean relative path", clip(a))
	}
	for _, r := range a {
		if r < 0x20 || r == 0x7f {
			return fmt.Errorf("config: action %q holds a control character", clip(a))
		}
	}
	return nil
}

// checkEnv requires "NAME=value" entries with distinct, plausible names and no
// variable of Kiln's own.
func checkEnv(action string, env []string) error {
	if len(env) > maxEnvEntries {
		return fmt.Errorf("config: action %q has too many environment entries", clip(action))
	}
	names := map[string]bool{}
	for _, e := range env {
		name, _, ok := strings.Cut(e, "=")
		switch {
		case !ok || name == "":
			return fmt.Errorf("config: action %q has an environment entry without a NAME=", clip(action))
		case !utf8.ValidString(e) || strings.IndexByte(e, 0) >= 0:
			return fmt.Errorf("config: action %q has an environment entry that is not UTF-8 text", clip(action))
		case strings.HasPrefix(name, "KILN_"):
			return fmt.Errorf("config: action %q passes the Kiln variable %q to the step", clip(action), clip(name))
		case names[name]:
			return fmt.Errorf("config: action %q repeats the variable %q", clip(action), clip(name))
		}
		names[name] = true
	}
	return nil
}

// EnvFor returns the exact environment for an action; an action the
// configuration does not name is an error, never a default.
func (c *Config) EnvFor(action string) ([]string, error) {
	env, ok := c.EnvByAction[action]
	if !ok {
		return nil, fmt.Errorf("sandbox-exec: KILN_ACTION %q is not in the configuration", clip(action))
	}
	return append([]string{}, env...), nil
}

// PackageDir is the working directory of an action's steps.
func (c *Config) PackageDir(action string) string {
	return path.Join(c.RepoRoot, action)
}

// Actions lists the configured actions, sorted.
func (c *Config) Actions() []string {
	out := make([]string, 0, len(c.EnvByAction))
	for a := range c.EnvByAction {
		out = append(out, a)
	}
	sort.Strings(out)
	return out
}
