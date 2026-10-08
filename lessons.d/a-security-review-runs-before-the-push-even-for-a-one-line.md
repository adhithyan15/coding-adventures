---
category: Repo policy / workflow reminders
---

# A security review runs before the push, even for a one-line fix

A small regex fix in the `language-ladder` app (PR #17158) was pushed and the
PR opened **before** the subagent security review that CLAUDE.md requires
("Before pushing code, always run `/security-review` … Do not push until the
review passes"). The review ran afterwards and found nothing. That outcome does
not make the order acceptable: it was luck, not process.

**Why the order is the rule, not a formality.** A finding that arrives after
the push costs a whole extra cycle: a fix-up commit, a second CI run on the PR,
possibly a reviewer re-reading a diff they already approved, and a window in
which the flawed version is the one on the branch. A finding before the push
costs an edit. Regexes are exactly where a "one-line fix" goes wrong in a way a
reviewer catches and a test does not (catastrophic backtracking on hostile
input, an anchor that silently stops matching), so "it is too small to review"
is backwards for this change in particular. The related
`per-spec-security-review-before-pushing-catches-things-linters-miss.md`
measures how often the review does find something.

**What to do.** Treat the review as part of committing, not part of pushing:
finish the change, run `/security-review`, fix what it reports, and only then
push and open the PR. Size is not an exemption. If a push has already gone out
unreviewed, run the review at once and say so in the PR, rather than letting
the next push quietly cover it.
