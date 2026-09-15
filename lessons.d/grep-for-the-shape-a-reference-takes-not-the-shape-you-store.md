---
category: Repo policy / workflow reminders
---

# Grep for the shape a reference takes, not the shape you store it in

2026-09-15.

A migration was about to turn seven tracked files into generated, gitignored
ones. The previous migration in the same arc had broken two tests by doing
exactly that without checking who read the file, so this time the readers were
searched for first — deliberately, as the lesson said to.

The search found **nothing**, and reported "no tracked file references any of
the seven". Review then named two specs that do.

The search grepped the **full repo-relative path**:

    code/packages/rust/wasm-conformance/CHANGELOG.md

The specs write the **short form**:

    `wasm-conformance/CHANGELOG.md`

which the full-path grep cannot match. Four references, all invisible, and the
report read as a clean bill of health rather than as a failed search.

A path has many written forms: repo-relative, package-relative, a bare
`](CHANGELOG.md)` link from a sibling file, `../CHANGELOG.md`, or the filename
alone in prose. Storing it one way says nothing about how it is cited.

**How to apply.** Search for the most DISTINCTIVE SUFFIX that any form must
contain — here `<package>/CHANGELOG.md`, which the short form and the full path
both end with — rather than the canonical string you happen to hold. And treat
a zero result on a search you expected to be non-trivial as a reason to test
the search, not as an answer: grep for something you KNOW is there and confirm
the tool finds it.

This is the same error as probing for a name instead of enumerating what
exists, and it has the same signature: an empty result that looks like good
news.

Related: enumerate, don't probe for the name you expect; an empty grep silently
swaps your own file in.
