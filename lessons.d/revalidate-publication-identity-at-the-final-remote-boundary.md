---
category: Security boundaries
---

# Revalidate publication identity at the final remote boundary

The minima publisher draft checked only push URLs once and lacked a complete
final PR identity check. Independent preflight withheld it before execution.
Its reviewed successor pins the authorized PR number and URL, repositories,
base ref and observed base SHA, dynamic head ref/SHA, open/unmerged state
and auto-merge OFF before, at the boundary and after publication.

Recheck unique authorized fetch-ALL and push-ALL URL lists at those same
boundaries. Verify the observed base is an ancestor of freshly fetched main.
Persist every actual command exit and raw observation before parsing it.
A push0 followed by failed verification may already be published: reconcile
read-only before any retry; never relabel or automatically repeat it.
