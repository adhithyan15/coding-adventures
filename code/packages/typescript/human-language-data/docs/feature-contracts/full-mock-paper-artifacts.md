# Full-mock papers are assessment artifacts

`AssessmentFullMock.timed` is a claim about a sitting, not proof that a
question paper exists. A rubric and answer key are not substitutes for the
questions a learner would receive. Each new full mock therefore needs an
explicit, safe-relative `paper` reference, checked by the same artifact
presence boundary as its rubric and answer key.

## Migration boundary

On `origin/main` at `850986376b1`, 18 assessment contracts declare 254 full
mocks. Fifty-six corresponding papers are checked in (14 each for Bengali,
Hindi, Malayalam, and Sanskrit); 198 are not. Adding 198 dangling paths to the
existing `assessment-artifact-ceiling` would silently increase a strict debt
ceiling, so this migration must not do that.

The 56 built mocks can declare their exact `paper` paths immediately. The
remaining 198 omissions are a separately pinned, per-track, exact set of
legacy mock identities. An omitted `paper` is tolerated only when its
`<track>/<level-or-capstone>/<mock-id>` identity is in that pin. The pin may
only fall: new omissions fail, and removing an omission requires lowering the
pin. These mocks remain incomplete assessment evidence; a parseable legacy
contract is not a pass or exam-readiness claim.

When a missing paper is authored, its mock gains an explicit `paper` reference
and leaves the omission pin. A declared paper is always audited. New dangling
paper paths must not be added to the existing artifact ceiling. The existing
two-timed-mock and human-validation requirements remain unchanged.

## Checks

- Reject unsafe relative references, path escapes, and symlink escapes before
  probing a declared paper.
- Report a missing paper, rubric, or key by its exact mock and field; a key's
  existence never implies the paper's existence.
- Assert the corpus migration counts and exact omission identities, and test
  that a new omission, stale pin, missing declared paper, and dangling declared
  paper fail while a fully built mock passes.
- Keep all readiness and completion reports honest about omitted papers.
