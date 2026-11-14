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
	"os"
	"path/filepath"
	"strings"

	"github.com/adhithyan15/coding-adventures/code/programs/go/kiln/internal/source"
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
		resolved, err := resolveDeepest(clean)
		if err != nil {
			return SafePathResult{}, fmt.Errorf("the recorder's %s entry %q: %v", name, printable(e), err)
		}
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

// maxLinkHops bounds the symlink hops of resolveDeepest (the kernel's limit is
// 40 too).
const maxLinkHops = 40

// resolveDeepest resolves the symlinks of the absolute path p the way the kernel
// would if every missing name were created later as a plain directory:
//
//   - a path that does not exist yet is resolved through its deepest EXISTING
//     ancestor, the missing remainder kept (`/opt/x/bin` with `/opt/x` a link
//     into the checkout is the checkout's `bin`);
//   - a link whose target does not exist (a DANGLING link) is followed too: a
//     step can create the target;
//   - `..` is applied to the RESOLVED path so far, never lexically across a
//     component that is a link: with `a/b -> <checkout>/d` and
//     `<checkout>/d/l -> ../x/bin`, the entry `a/b/l` is `<checkout>/x/bin`, not
//     `a/x/bin`.
//
// It walks the path one component at a time, replacing a link by its target at
// the front of the work list, at most maxLinkHops times; more hops, or a loop,
// is an error and the entry is refused.
func resolveDeepest(p string) (string, error) {
	work := strings.Split(p, "/")
	cur := "/"
	hops := 0
	for len(work) > 0 {
		c := work[0]
		work = work[1:]
		switch c {
		case "", ".":
			continue
		case "..":
			cur = filepath.Dir(cur) // cur is physical: this is the kernel's ".."
			continue
		}
		next := filepath.Join(cur, c)
		st, err := os.Lstat(next)
		if err != nil || st.Mode()&os.ModeSymlink == 0 {
			cur = next // a plain name, or one that is not there (yet)
			continue
		}
		if hops++; hops > maxLinkHops {
			return "", fmt.Errorf("more than %d symbolic links in %q (a loop?)", maxLinkHops, printable(p))
		}
		t, err := os.Readlink(next)
		if err != nil {
			return "", err
		}
		if filepath.IsAbs(t) {
			cur = "/"
		}
		work = append(strings.Split(t, "/"), work...)
	}
	return cur, nil
}

// LoaderEnv validates the variables that steer the dynamic loader of every
// program the recorder starts (KILN04 4.8, P11, M3), and returns the
// "NAME=value" entries that may be passed on, plus the directories they name (as
// regions a state root must stay out of).
//
//	LD_PRELOAD, LD_AUDIT   a library loaded into the process: refused when set
//	                       and non-empty (nothing legitimate sets them for a
//	                       build)
//	LD_LIBRARY_PATH        a library search path: validated like PATH (an empty
//	                       entry means the working directory)
//
// Everything else of the loader and C library family (any other LD_*,
// GLIBC_TUNABLES, GCONV_PATH, LOCPATH, TZDIR, ...) is simply never passed on: the
// child environments are allowlists (buildToolEnv, source.NewGitAt). GCONV_PATH
// and LOCPATH are colon lists, which a single-path check would have mishandled;
// dropping them closes that. An empty variable is the same as an unset one.
func LoaderEnv(environ []string, regions []Region) (entries []string, dirs []Region, err error) {
	last := map[string]string{}
	for _, e := range environ {
		if name, value, ok := strings.Cut(e, "="); ok && strings.HasPrefix(name, "LD_") {
			last[name] = value
		}
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

// buildToolEnv is the build tool's ENTIRE environment, an allowlist (KILN04 4.8,
// P11, M4). The build tool starts `git` without setting an environment, so
// anything it inherits reaches git (a GIT_CONFIG_GLOBAL file in the checkout with
// core.fsmonitor naming a program, GIT_EXEC_PATH, GIT_DIR...) and the dynamic
// loader. What it gets, and why:
//
//	HOME                    git and the Go runtime look for a home directory
//	PATH                    the validated PATH: the build tool runs git, sh and
//	                        other helpers through it (the exports start no
//	                        toolchain probe)
//	LD_LIBRARY_PATH         only if the recorder's own was validated and kept
//	source.PinnedEnv()      git's configuration pinned to nothing (system and
//	                        global are the null device), no optional locks or
//	                        prompts, no replace refs, no lazy fetch, LC_ALL=C
//	source.ConfigEnv()      core.fsmonitor=false, core.hooksPath=null device and
//	                        protocol.file.allow=never, as Kiln's own git passes
//
// The build tool reads no other variable for -emit-kiln-graph, -emit-kiln-steps
// and -emit-plan: its only os.Getenv calls are GITHUB_OUTPUT (the detect and
// plan-printing paths, which these flags do not take), and its executor's
// os.Environ is used only when it RUNS a BUILD, which `prepare` never does. So no
// GITHUB_* value is passed, and none needs validating. TMPDIR is not passed: the
// exports are written next to their destination, and a TMPDIR in the checkout
// would only be a place to plant files.
func buildToolEnv(home, pathValue string, loader []string) []string {
	out := []string{"HOME=" + home, "PATH=" + pathValue}
	out = append(out, loader...)
	out = append(out, source.PinnedEnv()...)
	return append(out, source.ConfigEnv()...)
}
