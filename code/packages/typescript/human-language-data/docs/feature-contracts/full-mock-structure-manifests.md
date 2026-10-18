# Full-mock structure manifests

A timed flag says how a mock should be administered; it does not show that
its learner paper covers the published reading, listening, writing, and
speaking task shapes. A level mock may declare `manifest` in its
`assessment.json` entry. The reference names a JSON file under the same
track, with no fragment. Declared paths go through the assessment-artifact
gate and cannot be added to the legacy missing-artifact ceiling.

```json
{
  "version": 1,
  "language": "example-track",
  "level": "A1",
  "mockId": "a1-mock-1",
  "sections": [
    { "skill": "reading", "minutes": 20, "partIds": ["reading-part-1"] },
    { "skill": "listening", "minutes": 20, "partIds": ["listening-part-1"] },
    { "skill": "writing", "minutes": 20, "partIds": ["writing-part-1"] },
    { "skill": "speaking", "minutes": 10, "partIds": ["speaking-part-1"] }
  ]
}
```

The IDs and minutes above are illustrative, not a task-shape template. Use
the IDs and exact time (or a time within a published range) from that track's
`task-shapes/<level>.json`. An implicit form lists every part of its skill
once. When the task shape declares variants, add `variantId` and list exactly
that variant's parts. Missing, duplicate, cross-skill, and invented parts fail
validation. The manifest's language, level, and mock ID must match its owner.

The learner paper must also bind each listed part to its actual prompt span:

```markdown
<!-- mock-part:reading-part-1 -->
Read the notice and answer its questions.
<!-- /mock-part:reading-part-1 -->
```

Use one matching start/end pair for each selected part ID in the learner
paper, scoring rubric, and answer key, with visible content between them.
The validator rejects duplicate,
unexpected, unclosed, empty, or missing spans. This prevents a JSON manifest
beside an empty paper from passing as a complete four-skill mock. The required
`check:assessment-artifacts` CI command checks every declared manifest and its
paper, rubric, and key bindings, even when nobody runs the optional
completion-plan command.

`mock-assessment/<language>/<level>` counts the deficit toward the required
two complete timed mocks. A complete mock needs distinct, existing learner
paper, rubric, answer key, and valid manifest. A legacy mock without a
manifest stays readable but incomplete. An absent task-shape inventory is a
named blocker, never assumed to mean that the mock has covered its target.
The machine-checkable manifest and document spans are structural evidence,
not a judgment that the prompts or scoring guidance are pedagogically sound;
independent human validation remains necessary before a readiness claim.
