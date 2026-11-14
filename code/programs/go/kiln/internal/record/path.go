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
	return safeList("PATH", pathVar, regions)
}

// safeList is SafePath for any colon-separated list of directories (PATH,
// LD_LIBRARY_PATH).
func safeList(name, value string, regions []Region) (SafePathResult, error) {
	if value == "" {
		return SafePathResult{}, fmt.Errorf("the recorder's %s is empty or unset", name)
	}
	var res SafePathResult
	for _, e := range strings.Split(value, ":") {
		switch {
		case e == "":
			return SafePathResult{}, fmt.Errorf("the recorder's %s has an empty entry, which means the current directory", name)
		case !filepath.IsAbs(e):
			return SafePathResult{}, fmt.Errorf("the recorder's %s entry %q is relative", name, printable(e))
		}
		clean := filepath.Clean(e)
		resolved := resolveDeepest(clean)
		for _, p := range []string{clean, resolved} {
			for _, rg := range regions {
				if rg.Path != "" && overlap(p, rg.Path) {
					return SafePathResult{}, fmt.Errorf("the recorder's %s entry %q (%q) overlaps %s %q: a step could plant a program or library there",
						name, printable(e), printable(p), rg.Name, printable(rg.Path))
				}
			}
		}
		res.Entries = append(res.Entries, clean)
		res.Resolved = append(res.Resolved, resolved)
	}
	return res, nil
}

// resolveDeepest resolves the symlinks of p. A path that does not exist yet is
// resolved through its deepest EXISTING ancestor and the missing remainder is
// appended: `/opt/x/bin` with `/opt/x` a symlink into the checkout and no `bin`
// yet is, for a step that can create it, the checkout's `bin`. If nothing can be
// resolved the path is returned as written.
func resolveDeepest(p string) string {
	rest := ""
	for cur := p; ; {
		if r, err := filepath.EvalSymlinks(cur); err == nil {
			return filepath.Join(r, rest)
		}
		parent := filepath.Dir(cur)
		if parent == cur {
			return p
		}
		rest = filepath.Join(filepath.Base(cur), rest)
		cur = parent
	}
}

// LoaderEnv validates the variables that steer the dynamic loader and the C
// library of every program the recorder starts (KILN04 4.8, P11, M3), and
// returns the "NAME=value" entries that may be passed on, plus the directories
// they name (as regions a state root must stay out of).
//
//	LD_PRELOAD, LD_AUDIT   a library loaded into the process: refused when set
//	                       and non-empty (nothing legitimate sets them for a
//	                       build)
//	LD_LIBRARY_PATH        a library search path: validated like PATH (an empty
//	                       entry means the working directory)
//	GCONV_PATH, LOCPATH    a directory of converters or locales loaded as code
//	                       or data: kept only if absolute and clear of every
//	                       region, otherwise dropped
//	any other LD_*         dropped
//
// An empty variable is the same as an unset one and is dropped.
func LoaderEnv(environ []string, regions []Region) (entries []string, dirs []Region, err error) {
	last := map[string]string{}
	for _, e := range environ {
		name, value, ok := strings.Cut(e, "=")
		if !ok || (!strings.HasPrefix(name, "LD_") && name != "GCONV_PATH" && name != "LOCPATH") {
			continue
		}
		last[name] = value
	}
	for _, name := range []string{"LD_AUDIT", "LD_PRELOAD"} {
		if last[name] != "" {
			return nil, nil, fmt.Errorf("%s is set (%q): a library loaded into the recorder's own programs", name, printable(last[name]))
		}
	}
	if v := last["LD_LIBRARY_PATH"]; v != "" {
		r, err := safeList("LD_LIBRARY_PATH", v, regions)
		if err != nil {
			return nil, nil, err
		}
		entries = append(entries, "LD_LIBRARY_PATH="+r.String())
		dirs = append(dirs, r.Regions()...)
	}
	for _, name := range []string{"GCONV_PATH", "LOCPATH"} {
		v := last[name]
		if v == "" || !filepath.IsAbs(v) {
			continue
		}
		clean := filepath.Clean(v)
		resolved := resolveDeepest(clean)
		bad := false
		for _, p := range []string{clean, resolved} {
			for _, rg := range regions {
				bad = bad || rg.Path != "" && overlap(p, rg.Path)
			}
		}
		if bad {
			continue
		}
		entries = append(entries, name+"="+clean)
		dirs = append(dirs, Region{"the directory of " + name, clean}, Region{"the directory of " + name, resolved})
	}
	return entries, dirs, nil
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

// childEnv builds the build tool's environment: the scrubbed process
// environment without PATH and without any loader variable, then the validated
// PATH and the validated loader entries.
func childEnv(environ []string, pathValue string, loader []string) []string {
	out := make([]string, 0, len(environ)+1+len(loader))
	for _, e := range environ {
		name, _, _ := strings.Cut(e, "=")
		if name == "PATH" || strings.HasPrefix(name, "LD_") || name == "GCONV_PATH" || name == "LOCPATH" {
			continue
		}
		out = append(out, e)
	}
	out = append(out, "PATH="+pathValue)
	return append(out, loader...)
}
