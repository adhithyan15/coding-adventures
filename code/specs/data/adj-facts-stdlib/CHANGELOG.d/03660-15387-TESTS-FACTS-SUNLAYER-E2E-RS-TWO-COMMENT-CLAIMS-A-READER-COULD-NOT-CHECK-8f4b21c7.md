- **#15387: `facts_sunlayer_e2e.rs` -- two comment claims a reader could not check.**
  Both defects landed in #15385 and were found by that PR's third security-review round, after it had
  merged. Comments only: no assertion, test name or arm changed, and the suite still runs 7 tests.

  ### A provenance claim that was true when measured and went stale one commit later

  The shipped comment said `git log -S` finds the string 4,109 entering these two trees *"only in the
  commit that REMOVES it"*. Measured at `245638bf74`, occurrences of that string in this file:

  | revision | occurrences |
  |---|---|
  | `05aa43d702` (pre-change) | 0 |
  | `1614e47e5b^` (parent of the squash) | 0 |
  | `1614e47e5b` (the squash) | 2 |
  | `origin/main` | 2 |

  `git log -S` over `code/packages/rust/adj-lang-cli/tests` and `code/specs/data/adj-facts-stdlib`
  returns exactly **one** commit on main, `1614e47e5b`. Parent holds the string zero times and the
  squash holds it twice, so that commit **adds** it -- the opposite of what the sentence said.
  Controls: `#[test]` reads 7 at every revision checked, an absent token reads 0, and `log -S 'source'`
  over the stdlib tree returns 465 commits, so the zeros discriminate rather than being an empty search.

  The substantive half survives unchanged: `05aa43d702` contains it **zero** times, so the figure was
  never on main before #15385 -- it lived in a working-tree draft.

  The issue proposed *"…enters these two trees only in this branch's own commits"*. That wording is not
  used, because a reader cannot check it: `24537c70fb` and `7a0378c2b9` are not ancestors of main, each
  is contained by exactly **one** ref -- a *local* branch -- and `git ls-remote --heads origin` matches
  **0** rows for that branch against **1** for `main` and **0** for a deliberately bogus name. A fresh
  clone sees the squash and nothing else, so the comment now names the squash directly.

  ### A number whose predicate is unnamed, which naming would not have fixed

  The comment cited a nearest candidate scope of **4,281**. The issue proposed naming that scope beside
  it. That would not make it reproducible, and the reason is measurable: the instrument counts with
  `os.walk` over the **working tree**, not `git ls-tree` over a revision.

  | measurement | count |
  |---|---|
  | `os.walk` over the working tree | 4282 |
  | `git ls-files` (tracked) | 4278 |
  | difference | 4 |

  All four sit inside ignored build caches -- `.ruff_cache/` (3 files) and `tools/__pycache__/` (1).
  Untracked-but-not-ignored: **0**. Tracked under `code/specs/data` at `1614e47e5b`, the tip when
  #15387 was filed, is **4,277**, so `4,277 + 4 = 4,281` exactly, and `4,278 + 4 = 4,282` is today.

  So **4,281 was right when written and is still not reproducible**: it moves whenever a `.pyc` or a
  ruff cache entry appears, and no revision returns it. The comment now cites **4,277 tracked files at
  `1614e47e5b`**, which `git ls-tree -r --name-only` repeats exactly -- and which still does not
  reproduce 4,109, so the sentence's point is unchanged.

  ### What is not claimed

  The 4,281 figure **was** a correct measurement over an unnamed scope; this change proves it. Nothing
  here shows 4,109 was ever a correct measurement of anything: the reproduction script reports it not
  reproduced by any scope tested, and this change does not claim to know how it was produced. An
  earlier draft of that paragraph said "both were correct measurements", which asserts more than was
  measured; security review caught it.

  The defect being fixed is that a reader could not tell a correct number from an invented one -- the
  same failure the surrounding comment is about, reproduced one paragraph later.
