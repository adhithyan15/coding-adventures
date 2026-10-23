---
category: Workspace & package metadata
---

# A PR that adds a field to a shared struct literal breaks on rows main adds after its merge-base, with no textual conflict

LANG84 4c added `release` and `case` fields to `lang_matrix.rs`'s `Prog`
and filled them in on all ~780 rows. While its CI waited in the queue, main
merged #17973, which added an ALGOL `Prog { .. }` row with neither field.
The two changes touch different lines, so git merges them with no conflict.
But CI tests the PR merged with main, and that merge fails to compile
(`error[E0063]: missing fields 'case' and 'release' in initializer of
'Prog'`). The macOS `build` job went red on `rust/lang-aot`, and the head
itself was fine.

**Fix:** merge main, then add `release: None, case: None` to every literal
that lacks them. A brace- and string-aware scan of `Prog {` literals, plus
`cargo test --test lang_matrix --no-run`, finds them all.

**Do differently:** while a PR adds a required field to a struct that
other PRs keep adding literals of, the merge with main must compile at the
moment CI runs, not just at your branch tip. Run
`git merge-tree --write-tree HEAD origin/main` and compile that tree, or
merge main, just before every push. Do this again whenever a CI build fails
with E0063 on a file you didn't change since. When designing such a change,
a field that can default (`..Prog::DEFAULT` struct-update syntax, or a
builder) avoids the race altogether.
