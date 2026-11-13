package legacy

import (
	"sort"
	"strings"

	"github.com/adhithyan15/coding-adventures/code/programs/go/kiln/internal/key"
)

// toolsResult is what one set of closure language names yields: the key's
// tools section, its digest, the names the plan shows, and the entries that
// do not exist.
type toolsResult struct {
	tools   []key.Tool
	digest  [32]byte
	labels  []string
	unknown []string
}

// closureNames is the distinct languages and extra toolchains of a closure,
// sorted (KILN03 4: every member counts, not only P).
func (pl *planner) closureNames(members []int) []string {
	set := map[string]bool{}
	for _, q := range members {
		m := pl.g.Packages[q]
		set[m.Language] = true
		for _, x := range m.ExtraToolchains {
			set[x] = true
		}
	}
	out := make([]string, 0, len(set))
	for n := range set {
		out = append(out, n)
	}
	sort.Strings(out)
	return out
}

// toolsOf computes (once per distinct name set) the tools section for names.
func (pl *planner) toolsOf(names []string) (toolsResult, error) {
	return pl.tools.get(strings.Join(names, "\x00"), func() (toolsResult, error) {
		tools, unknown := pl.pol.tc.ToolsFor(names, pl.opt.OS, pl.opt.Host.ImageID())
		d, err := key.ToolsDigest(tools)
		if err != nil {
			return toolsResult{}, err
		}
		labels := make([]string, len(tools))
		for i, t := range tools {
			labels[i] = t.Name + "@" + t.Version
		}
		return toolsResult{tools: tools, digest: d, labels: labels, unknown: unknown}, nil
	})
}

// prepareToolsEnv computes what every package shares: the env section (from
// opt.Environ and env.json), its digest and names, and, with --probe, the probe
// results over every entry any package uses.
func (pl *planner) prepareToolsEnv() error {
	vars, err := pl.pol.env.EnvFor(pl.opt.Environ)
	if err != nil {
		return err
	}
	pl.envVars = vars
	pl.envNames = make([]string, len(vars))
	for i, v := range vars {
		pl.envNames[i] = v.Name
	}
	if pl.envDigest, err = key.EnvDigest(vars); err != nil {
		return err
	}
	pl.mismatch = map[string]bool{}
	pl.timedOut = map[string]bool{}
	pl.refused = map[string]bool{}
	if pl.opt.Probe == nil {
		return nil
	}
	used := map[string]bool{}
	for _, p := range pl.g.Packages {
		used[p.Language] = true
		for _, x := range p.ExtraToolchains {
			used[x] = true
		}
	}
	names := make([]string, 0, len(used))
	for n := range used {
		names = append(names, n)
	}
	sort.Strings(names)
	if pl.probes, err = pl.pol.tc.Probe(names, pl.opt.OS, *pl.opt.Probe); err != nil {
		return err
	}
	for _, r := range pl.probes {
		if r.Status == ProbeMismatch {
			pl.mismatch[r.Tool] = true
		}
		if r.Status == ProbeTimedOut {
			pl.timedOut[r.Tool] = true
		}
		if r.Status == ProbeRefused {
			pl.refused[r.Tool] = true
		}
	}
	return nil
}
