---
category: Testing & coverage
---

# Adding a letter lesson with a filmstrip raises that track's exact filmstrip-target count pin

Russian B1 added RU-W266-tvyordyy-znak, a letter lesson for ъ with a
stroke-order filmstrip. The targeted gate tests (letter-anchoring,
script-closure) passed, but the full human-language-data suite failed
`tests/figure-targets.test.ts`: the real corpus now draws 28 Russian
filmstrip targets, while `tests/filmstrip-target-counts/russian.json`
still pinned 27.

That pin is an exact count, not a may-only-fall ratchet. It changes
whenever a track gains or loses a drawn letter, so raising it here was
correct.

What to do differently: whenever a change adds a `W`-series letter lesson
whose figure is a filmstrip, update
`tests/filmstrip-target-counts/<track>.json` in the same commit, and
include `tests/figure-targets.test.ts` in the targeted gate run.

A related trap: on a loaded machine the full suite also showed several
120-second timeouts (plan-cli, hash, metalanguage, script-closure). Rerun
those files on their own before you treat them as real. They passed
cleanly here.
