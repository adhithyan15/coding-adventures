### Schedule the weekly Kiln census (census PR E)

- `.github/workflows/kiln-census.yml` gains a `schedule` trigger at 04:00 UTC
  every Monday. The scheduled run uses the PR8 gate's own configuration
  (`home: runner`, shards `0,1,2` of 15) on both runner images, so the gate's
  evidence accrues without anyone dispatching the workflow by hand.
- A `schedule` event carries no `inputs`, and a `workflow_dispatch` input's
  `default:` applies only to a dispatch. The first step therefore falls back to
  the scheduled configuration; without that fallback a weekly run failed input
  validation ("shard_count must be an integer of at most three digits") before
  doing any work. The dispatch defaults (`15`, `0`, `fresh`) are unchanged, and
  every input stays required, so a deliberate run is unaffected.
- The weekly run is expected to be red until the recorder is clean: a `runner`
  census that leaves a shard "not clean" exits non-zero, which is the PR8 gate
  reporting "not yet". This is documented in the workflow header and in KILN04
  section 4.8 so it is not mistaken for a broken workflow, and it is not a
  required check on any pull request.
