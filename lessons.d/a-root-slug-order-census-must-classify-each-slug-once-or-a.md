---
category: Repo policy / workflow reminders
---

# A root-slug order census must classify each slug once, or a sed pipeline relabels prefixes as suffixes

**What went wrong.** Before briefing the Russian B1 drafters, the root slugs
were counted with two chained `sed` substitutions: first `^slavic-.*` to
`PREFIX-slavic`, then `.*-slavic$` to `SUFFIX-slavic`. The second rule also
matched the output of the first, so every prefix slug was counted as a suffix.
The census reported "594 `<root>-slavic`", and the brief told drafters to write
`<root>-slavic`. The track actually has 947 `slavic-<root>` slugs and 171
`<root>-slavic` ones. Two drafters followed the brief and one did not, which
would have split etyma such as `slavic-vezati` / `vezati-slavic` in
`root-slug-splits.test.ts`.

**Fix.** Recount with one classifier per slug (awk with if / else), correct the
briefs, and have the drafters and reviewers switch new slugs to
`slavic-<root>`.

**Next time.** Classify each item exactly once (if / else, or a single regex
with alternation), never with chained substitutions whose outputs can match a
later pattern. Spot-check the census against a handful of real slugs before a
brief depends on it.
