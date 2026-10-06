### Forme pull-request CI now has one authoritative suite

- Stop launching the broad Linux push suite for feature-branch commits; the
  pull-request suite is now their single authoritative required CI event, while
  pushes to `main` still validate the committed squash.
- Make the final gate distinguish a freshly verified superseded PR head from a
  cancelled dependency on the current head, avoiding stale failures without
  treating missing current-head evidence as success.
- Teach the repository PR babysitter to pin the current pull-request head,
  inspect only required checks, distinguish superseded evidence from genuine
  current-head failures, and verify the expected head is the one merged.
- Close FM-B029 after a fresh audit found every recorded Forme completion item
  done and no actionable backlog remaining.
