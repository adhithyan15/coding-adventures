package legacy

// The gate of one package, for the recorder (KILN04 13 item 5, stage 4 PR 2b-2c-3)
// ===============================================================================
//
// `kiln legacy-plan --ledger` skips lookup for a package whose gate is set: the
// never list, a KILN03 hard reason or a probe that disagreed.  The shard
// recorder has the same question for the same reason: a package that is gated
// is never RECORDED as a measurement (the merge turns the gate into
// `precondition:<gate>`), because a PR could otherwise choose what a recording
// means.
//
// The gate is computed by planner.gateFacts and planner.gateFrom, the very
// functions BuildPlan uses, so the plan and the recorder cannot drift.  What
// the plan does around them and the recorder does not need is skipped:
//
//	BuildPlan                         Traced.Gate
//	  parseBoundaries                   parseBoundaries      (same)
//	  prepareToolsEnv (runs probes)     prepareToolsEnv      (same: the probes)
//	  computeOwn                        -- (digests of the own files: unused)
//	  memberItems                       memberItems          (same)
//	  per package: record               per package: gateFacts + gateFrom
//
// The preparation runs once, on the first call of Gate or Probes.  A Traced made
// for lookup (newTracedOn) comes from a planner BuildPlan already prepared, so
// it is marked done and never prepares twice.

import "fmt"

// prepare runs the one-time work Gate needs, once.
func (t *Traced) prepare() error {
	t.prep.Do(func() {
		pl := t.pl
		data, err := readSmall(pl.opt.Source, BoundaryPath, maxPolicyBytes)
		if err != nil {
			t.prepErr = err
			return
		}
		if pl.bounds, err = parseBoundaries(data); err != nil {
			t.prepErr = err
			return
		}
		if err := pl.prepareToolsEnv(); err != nil { // runs the probes, when asked
			t.prepErr = err
			return
		}
		pl.memberItems()
	})
	return t.prepErr
}

// Gate is the reason the package at rel_path rel must not be recorded as a
// measurement, or "" when nothing gates it.  It is the plan's gate: never, then
// the other hard reasons, then probes, the smallest reason winning within a
// priority.  The first call runs the toolchain probes (Options.Probe).
func (t *Traced) Gate(rel string) (string, error) {
	p, ok := t.pkgAt(rel)
	if !ok {
		return "", fmt.Errorf("traced key: no package at %q", short(rel))
	}
	if err := t.prepare(); err != nil {
		return "", err
	}
	pl := t.pl
	members := pl.closure(p)
	f, err := pl.gateFacts(p, members, pl.unionItems(members))
	if err != nil {
		return "", fmt.Errorf("package %s: %w", short(rel), err)
	}
	return pl.gateFrom(pl.g.Packages[p].Name, f), nil
}

// Probes are the toolchain probe results (empty unless Options.Probe was set),
// running them first if no call has yet.  Unlike the plan's, a failure to
// prepare is an error here: the recorder seals the list.
func (t *Traced) Probes() ([]ProbeResult, error) {
	if err := t.prepare(); err != nil {
		return nil, err
	}
	return append([]ProbeResult(nil), t.pl.probes...), nil
}
