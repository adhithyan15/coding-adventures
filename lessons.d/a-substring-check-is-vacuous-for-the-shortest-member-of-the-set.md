---
category: Testing & coverage
---

# A substring check is vacuous for the shortest member of the set it guards

2026-09-15.

Three tests pinned a hand-kept CI list against a registry, so that adding an
entry to one without the other would fail rather than ship an unguarded gate.
They matched like this:

```ts
const missing = REGISTRY.map((plan) => plan.path)
  .filter((path) => !workflowText.includes(path));
expect(missing).toEqual([]);
```

That worked for fifteen entries. The sixteenth was the repository-root
`CHANGELOG.md`, and its path is a **suffix of the other fifteen** —
`code/packages/rust/lang-aot/CHANGELOG.md` contains the string `CHANGELOG.md`.

So `includes` was satisfied by a *different* entry's line, and **all three drift
guards were inert for exactly the entry they had just been asked to guard.**
Deleting the root entry from the CI list left every test green.

The shape generalises past paths. Any `includes` / `in` / `grep -F` check over a
set will be vacuous for whichever member is a substring of another: a short flag
among longer ones (`--v` inside `--verbose`), an id that prefixes another
(`ERR_1` inside `ERR_10`), a short package name inside a scoped one.

And the failure is silent in the worst direction — the guard reports PASS,
which is what you wanted to see.

**How to apply.** When asserting that each member of a set is PRESENT in some
text, match a WHOLE entry: anchor on the delimiters that surround it in that
format — quotes, whitespace, a line boundary, a continuation, a closing paren.
Then falsify it: delete the member you most expect to be safe, and confirm the
test fails. The shortest member is the one to delete.

This was found in review, not by the test suite, because a vacuous assertion
cannot fail.

Related: an unasserted measurement is not a gate; an equality between two empty
sets proves nothing.
