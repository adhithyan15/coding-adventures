---
category: Testing & coverage
---

# An etymology pass can add a payoff to a root a corpus test pins

The Persian etymology pass gave FA-C130-salem (sâlem, "healthy", Arabic
*sālim*) `roots: [s-l-m]`, the same root as salâm. The root ledger then
counted the lesson as a second payoff of the peace root, and
`tests/corpus/persian.test.ts` ("pins Persian's peace-root payoff inside the
chapter-one practice") failed: it pinned `payoffs: ["FA-C01-practice"]` and
`payoffCount: 1`.

The root was right, so the pin was updated to the new, correct state
(`["FA-C01-practice", "FA-C130-salem"]`, count 2) with a comment saying
why. The test was not skipped or loosened: it still pins the exact list.

**What to do:** before an etymology pass adds `roots:` slugs to a track, grep
the track's corpus tests for the roots they pin (`git grep -n "root ===" --
code/packages/typescript/human-language-data/tests/corpus/<track>.test.ts`).
A new lesson that shares a pinned root will change its payoff list; check
that the sharing is a real etymological link before updating the pin, and
say so in the comment and the commit.
