### Changed — an etymology is a hook, so the gate stops demanding it be drilled

Project owner's directive: *"Etymology should only be mentioned once. I do not want
that re-emphasized again and again. It is mostly a memory hook for me."*

**The gate was manufacturing the repetition.** HL09 §3.1 requires every atom to be
revisited at least twice, and for an etymon the only way to satisfy that was to
re-state it in the Guided Practice and again in the Wrap-up Recall. The prose was
shaped by the measurement, so the measurement had to change first.

Etymology atoms — `*-ETYMON-*`, a naming convention every track already follows — are
now **waived from the reinforcement criterion**. Spanish's pre-A1 reinforcement blocker
goes from 87 atoms to 53, and says so: *"53 atom(s) at or below pre-A1 are revisited
fewer than twice (35 etymology hook(s) waived)."*

This also settles a question open for weeks: the once-cited `ES-ETYMON-*` atoms should
be **waived, not re-cited**.

**The waiver lives in `level-gate.ts`, not in `continuity.ts`, on purpose.**
`measureContinuity` goes on reporting every atom truthfully, so `atomsTaught`,
`atomsNeverRevisited` and the R-window counts keep meaning what they say, the gap
report stays honest, and **no pinned corpus figure moves**. Only the level *claim*
ignores them — the one place the decision actually applies — and the waiver is printed
in the blocker rather than silently absent.

The `-ETYMON-` convention is not enforced by schema, and a census found a few atoms
that arguably qualify and are not matched (`ES-HISTORY-AL-ANDALUS-LOANS`,
`SA-SOUND-PIE-KW-OUTCOMES`). Naming those consistently is the fix; widening the regex
is not.

