# ADJ52 run-3 — full 100-case data

`run-3-100case-full.json` is the **complete, raw output** of the 100-case
hands-off run (Workflow `wuja6iixk`): 100/100 cases completed, 0 skipped, 0
compile failures; 500 agents, 17.7M subagent tokens, ~60 min. Each of 100
diversified specialty seeds had an agent *find* its own published case, perturb
it (diagnosis-invariant), and run the full five-arm pipeline (ingest → derive →
engine → plain-Claude control → blind judge).

The writeup + cross-tabs are in `run-3-100case.md`; the trimmed per-case table is
`run-3-100case-summary.json`; the analysis script is `analyze_run.py`. This file
is the unabridged data behind all of them.

## Shape

```
{
  "summary": "...",
  "agentCount": 500,
  "logs": ["pipeline over 100 case seed(s)", "AGGREGATE: ..."],
  "result": {
    "tally": { seeds_attempted, cases_completed, skipped_or_failed,
               framework_won, plain_won, tie,
               framework_correct, plain_correct, fw_compile_failures },
    "per_case": [
      {
        "id": "case-N",
        "diagnosis_unchanged": bool,        // perturbation preserved the dx
        "perturbations": [ ... ],           // every diagnosis-irrelevant change made
        "fw_domain": "...",                 // clinical-area key the deriver chose
        "fw_top": "diagnosis(x) @ <posterior>",
        "fw_next_step": "...",
        "fw_is": "A" | "B",                 // which blind slot the framework was
        "winner": "A" | "B" | "tie",
        "framework_correct": "correct" | "partial" | "incorrect",
        "plain_correct":     "correct" | "partial" | "incorrect",
        "framework_won": bool, "plain_won": bool,
        "rationale": "..."                  // the blind judge's full per-case reasoning
      },
      ... 100 cases ...
    ]
  }
}
```

## Headline (from `tally`)

- completed **100 / 100** (0 skipped, 0 compile failures)
- correctness: framework **62**, plain Claude **61** (parity)
- blind-judge wins: framework **39**, plain **60**, tie **1**
- (cross-tabs in `run-3-100case.md`: the gap is entirely calibration — 28 of
  plain's wins are cases the framework also got right but lost on overconfidence)

## Note on the raw per-case rulebooks/programs

The 100 per-case derived `rulebook.adj` + `program.adj` files (the actual adj-lang
the deriver generated for each case) are **not** included here — they were left as
untracked workflow byproducts and are preserved in a git stash
(`stash@{0}` on branch `adj52-blind-cross-arm-experiment`). They can be added in a
follow-up; the per-case results, posteriors, and rationales above are the
substantive data.
