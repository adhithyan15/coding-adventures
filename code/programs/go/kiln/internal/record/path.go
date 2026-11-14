package record

// The recorder's own PATH (KILN04 section 4.8, stage 4b-1, decision P11; H1)
// ==========================================================================
//
// Kiln, the build tool and every program they start look programs up on the
// PATH they were started with. On a CI runner that PATH is whatever earlier
// workflow steps left it, and one of them (ci.yml) puts
// `$GITHUB_WORKSPACE/.lua/bin` in front. The checkout is bound WRITABLE into
// every sandboxed step, so a BUILD can create `<checkout>/.lua/bin/git`; the
// next host-side `git` call then runs that file, unsandboxed, with the
// recorder's privileges, and it can forge the shard. A tracked file at the
// same path would be run by `prepare` itself.
//
// Two defences, both here:
//
//  1. The programs the recorder needs are resolved ONCE, to absolute canonical
//     paths outside every writable place (ResolveBinary + CheckOutside), and
//     used by path from then on (source.NewGitAt). Nothing the recorder does
//     looks a program up.
//  2. The programs the recorder does not control (the build tool starts `git`,
//     `sh`, compilers) get a PATH made of entries that cannot be written by a
//     step. SafePath refuses a PATH with an empty entry (it means "the current
//     directory") or a relative one, and any entry that overlaps a region a step
//     can write or the recorder keeps private, either spelled out or after the
//     symlinks in it are resolved.
//
// The same entries are also kept out of the state roots (sandboxcfg.go): a state
// root is writable from the sandbox, and one over a PATH directory would let a
// step plant a program there.

import (
	"fmt"
	"path/filepath"
	"strings"
)

// SafePathResult is a validated PATH.
type SafePathResult struct {
	// Entries are the PATH's directories, cleaned, in order.
	Entries []string
	// Resolved are the same directories after EvalSymlinks (an entry that does
	// not exist is kept as written), for the checks that must see through links.
	Resolved []string
}

// String is the PATH value.
func (r SafePathResult) String() string { return strings.Join(r.Entries, ":") }

// Regions returns every entry, written and resolved, as private regions.
func (r SafePathResult) Regions() []Region {
	var out []Region
	seen := map[string]bool{}
	for _, e := range append(append([]string{}, r.Entries...), r.Resolved...) {
		if !seen[e] {
			seen[e] = true
			out = append(out, Region{"a directory of the recorder's PATH", e})
		}
	}
	return out
}

// SafePath validates pathVar against the regions a step can write or the
// recorder keeps private.
func SafePath(pathVar string, regions []Region) (SafePathResult, error) {
	if pathVar == "" {
		return SafePathResult{}, fmt.Errorf("the recorder's PATH is empty or unset")
	}
	var res SafePathResult
	for _, e := range strings.Split(pathVar, ":") {
		switch {
		case e == "":
			return SafePathResult{}, fmt.Errorf("the recorder's PATH has an empty entry, which means the current directory")
		case !filepath.IsAbs(e):
			return SafePathResult{}, fmt.Errorf("the recorder's PATH entry %q is relative", printable(e))
		}
		clean := filepath.Clean(e)
		resolved := clean
		if r, err := filepath.EvalSymlinks(clean); err == nil {
			resolved = r
		}
		for _, p := range []string{clean, resolved} {
			for _, rg := range regions {
				if rg.Path != "" && overlap(p, rg.Path) {
					return SafePathResult{}, fmt.Errorf("the recorder's PATH entry %q (%q) overlaps %s %q: a step could plant a program there",
						printable(e), printable(p), rg.Name, printable(rg.Path))
				}
			}
		}
		res.Entries = append(res.Entries, clean)
		res.Resolved = append(res.Resolved, resolved)
	}
	return res, nil
}

// pathOf returns the value of the LAST PATH entry of an environment ("" if none).
func pathOf(environ []string) string {
	v := ""
	for _, e := range environ {
		if x, ok := strings.CutPrefix(e, "PATH="); ok {
			v = x
		}
	}
	return v
}

// withPathEnv replaces every PATH entry of environ by one holding value.
func withPathEnv(environ []string, value string) []string {
	out := make([]string, 0, len(environ)+1)
	for _, e := range environ {
		if !strings.HasPrefix(e, "PATH=") {
			out = append(out, e)
		}
	}
	return append(out, "PATH="+value)
}
