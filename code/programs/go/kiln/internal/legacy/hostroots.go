package legacy

import (
	"crypto/sha256"
	"fmt"
	"strings"
)

// Root kinds of host-roots.json (KILN04 5.3).
const (
	RootImage     = "image"     // covered by Host.ImageID()
	RootTool      = "tool"      // covered by the tool's pin and probe
	RootState     = "state"     // covered by an ecosystem's lockfile
	RootEphemeral = "ephemeral" // covered by nothing, by design
)

// HostRoot is one entry of `roots`.
type HostRoot struct {
	Path      string   `json:"path"`
	Kind      string   `json:"kind"`
	Tool      string   `json:"tool,omitempty"`
	Ecosystem string   `json:"ecosystem,omitempty"`
	Fetchers  []string `json:"fetchers,omitempty"`
}

// HostRoots is a validated host-roots.json: what an action may read outside
// the repository.  In 7c-3 it feeds one digest, `policy:host-roots.json` in
// the KILN04 weak key; nothing reads the lists yet.
type HostRoots struct {
	Roots     []HostRoot `json:"roots"`
	Ephemeral []string   `json:"ephemeral"`
	AbsentOK  []string   `json:"absent-ok"`
	// Digest is SHA-256 of the file's bytes.
	Digest [32]byte `json:"-"`
}

// ParseHostRoots reads host-roots.json strictly.  tc is the loaded
// toolchains.json: a `tool` root must name a tool it pins.  Every list is
// strictly sorted (by path for `roots`), paths are absolute or begin with
// $HOME or $TMPDIR, none contains `..`, a `state` root names an ecosystem, and
// `tool`, `ecosystem` and `fetchers` appear only where they mean something.
// Also, every toolchains.json root under /opt/hostedtoolcache must lie under a
// `tool` root of the same tool, so the two files cannot drift apart.
func ParseHostRoots(data []byte, tc *Toolchains) (*HostRoots, error) {
	var h HostRoots
	if err := policyDecode(data, &h); err != nil {
		return nil, fmt.Errorf("host-roots: %w", err)
	}
	h.Digest = sha256.Sum256(data)
	for i, r := range h.Roots {
		if i > 0 && h.Roots[i-1].Path >= r.Path {
			return nil, fmt.Errorf("host-roots: roots not strictly sorted by path at %q", short(r.Path))
		}
		if err := checkPolicyPath(r.Path, true, false); err != nil {
			return nil, fmt.Errorf("host-roots: %w", err)
		}
		if r.Path == "$HOME" || r.Path == "$TMPDIR" || r.Path == "/" {
			return nil, fmt.Errorf("host-roots: %q is too broad to be a root", r.Path)
		}
		switch r.Kind {
		case RootImage, RootEphemeral:
			if r.Tool != "" || r.Ecosystem != "" || len(r.Fetchers) > 0 {
				return nil, fmt.Errorf("host-roots: %s root %q may not name a tool, ecosystem or fetchers", r.Kind, short(r.Path))
			}
		case RootTool:
			if r.Ecosystem != "" || len(r.Fetchers) > 0 {
				return nil, fmt.Errorf("host-roots: tool root %q may not name an ecosystem or fetchers", short(r.Path))
			}
			if !tc.HasTool(r.Tool) {
				return nil, fmt.Errorf("host-roots: tool root %q names %q, which toolchains.json does not pin", short(r.Path), short(r.Tool))
			}
		case RootState:
			if r.Tool != "" {
				return nil, fmt.Errorf("host-roots: state root %q may not name a tool", short(r.Path))
			}
			if !validName(r.Ecosystem) {
				return nil, fmt.Errorf("host-roots: state root %q needs an ecosystem", short(r.Path))
			}
			if err := checkSorted("fetchers of "+r.Path, r.Fetchers); err != nil {
				return nil, fmt.Errorf("host-roots: %w", err)
			}
			for _, f := range r.Fetchers {
				if err := checkPolicyPath(f, true, false); err != nil {
					return nil, fmt.Errorf("host-roots: fetcher: %w", err)
				}
			}
		default:
			return nil, fmt.Errorf("host-roots: root %q has unknown kind %q", short(r.Path), short(r.Kind))
		}
	}
	for _, l := range []struct {
		what string
		xs   []string
		glob bool
	}{{"ephemeral", h.Ephemeral, true}, {"absent-ok", h.AbsentOK, false}} {
		if err := checkSorted(l.what, l.xs); err != nil {
			return nil, fmt.Errorf("host-roots: %w", err)
		}
		for _, p := range l.xs {
			if err := checkPolicyPath(p, true, l.glob); err != nil {
				return nil, fmt.Errorf("host-roots: %s: %w", l.what, err)
			}
			if l.glob && literalSegments(p) < 2 {
				return nil, fmt.Errorf("host-roots: ephemeral %q needs at least two literal segments before its first wildcard", short(p))
			}
		}
	}
	if err := h.coversToolRoots(tc); err != nil {
		return nil, err
	}
	return &h, nil
}

// literalSegments counts the path segments before the first one holding a
// wildcard; a leading $HOME or $TMPDIR is not a segment. `/dev/fd/*` has two,
// `/*` and `/proc/*` fewer, so a glob can never allow a whole tree like /proc.
func literalSegments(p string) int {
	if i := strings.IndexByte(p, '/'); strings.HasPrefix(p, "$") && i >= 0 {
		p = p[i:]
	} else if strings.HasPrefix(p, "$") {
		return 0
	}
	n := 0
	for _, seg := range strings.Split(strings.TrimPrefix(p, "/"), "/") {
		if strings.Contains(seg, "*") {
			break
		}
		n++
	}
	return n
}

// coversToolRoots enforces the cross-file rule: a hostedtoolcache directory on
// a tool's PATH must be readable as a `tool` root of that tool.
func (h *HostRoots) coversToolRoots(tc *Toolchains) error {
	const cache = "/opt/hostedtoolcache/"
	for tool, dirs := range tc.roots {
		for _, d := range dirs {
			if !strings.HasPrefix(d, cache) {
				continue
			}
			ok := false
			for _, r := range h.Roots {
				ok = ok || r.Kind == RootTool && r.Tool == tool && (d == r.Path || strings.HasPrefix(d, r.Path+"/"))
			}
			if !ok {
				return fmt.Errorf("host-roots: toolchains.json puts %q on the PATH of %q but no tool root of that tool covers it", short(d), short(tool))
			}
		}
	}
	return nil
}
