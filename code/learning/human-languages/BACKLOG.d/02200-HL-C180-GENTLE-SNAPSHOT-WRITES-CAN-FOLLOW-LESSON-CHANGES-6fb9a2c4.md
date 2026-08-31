## HL-C180 — gentle snapshot writes can follow lesson changes

Issue #13669 exposed a curriculum-wide serialization defect: after a legitimate
lesson addition or removal, `generate:gentle-snapshots` compared the valid prior
owner tree with the new source and narration identities before it could install
the newly generated tree. Curriculum PRs therefore needed a manual recoverable
tree swap even when both the old and staged trees were internally canonical.

Write mode now validates the prior direct-owner tree from its own canonical
metrics while retaining exact current source and narration identity checks for
check mode, staged output, installed output, and publication reads. Recovery
also accepts a valid prior tree after an interrupted pre-install move, then
continues through the same staged atomic replacement. Mixed trees, aggregates,
symlinks, unexpected owners, noncanonical bytes, and ambiguous recovery states
remain rejected.

Regression coverage starts with a valid old owner tree, changes the expected
lesson identities, and proves both ordinary replacement and interrupted-install
recovery produce the exact new canonical tree. This removes a cross-language
tooling blocker before the next Punjabi and Indian-language lesson tranches.

Windows validation also observed one transient `EPERM` during the final staged
directory rename. Rollback restored the prior tree exactly and an unchanged
retry succeeded; follow-up #13711 records the bounded-retry reliability work
without expanding this identity-migration repair.
