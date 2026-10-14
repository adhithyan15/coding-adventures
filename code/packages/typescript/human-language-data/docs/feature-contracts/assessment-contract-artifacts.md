# Assessment contracts may not promise files that do not exist (HL20 §6b)

```bash
npm run check:assessment-artifacts     # fail on any unpinned dangling reference
npm run generate:assessment-artifacts  # lower the per-track ceilings
```

Every `<track>/assessment.json` names paths: a task-shape inventory per skill, and a
rubric, answer key, and (as contracts migrate) human-validation record per timed
mock. Until this gate, nothing checked the paths led anywhere — 13 of 23 tracks
carried a contract, all 13 dangled, and between them they named 351 artifacts that
were not on disk.

The policy has always required human validation. A legacy mock may temporarily omit
`humanValidation` so its enclosing contract remains readable, but omission is now
visible `human-validation/<language>/<level>` work. A declared reference is both
safe-relative-path checked and included in the artifact audit; the work item closes
only when every mock at the rung declares evidence and every evidence file exists.

The known set is pinned per track under `core/assessment-artifact-ceiling/`. It is a
**ceiling**, not a licence: a dangling reference outside the pin fails, and a pinned
path that gets *built* also fails, so the pin has to be lowered rather than left to
rot into a floor. The pin is a set of paths rather than a count, because a count is
satisfied by paying one debt and taking on another.

Presence is probed with `artifactExists`, not `existsSync`: only `ENOENT`/`ENOTDIR`
mean absent, and every other errno throws naming itself. "The file is not there" and
"I was not allowed to look" need different responses from a human, so they must not
arrive as the same boolean.
