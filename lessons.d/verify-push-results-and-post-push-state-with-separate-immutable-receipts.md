---
category: Testing & coverage
---

# Verify push results and post-push state with separate immutable receipts

The reviewed CV04a publisher correctly sent an ordinary e183-to3a84 push with
actual exit0, then its combined final REST/ref assertion failed with wrapper
exit1 at line68. The original failing REST response was not saved, so caching
or lag is not a demonstrated cause. Retain the helper, tool traceback identity,
push log and original partial receipt with null actual_published_head.

A separate read-only reconciliation re-queried open/unmerged/autoOFF/base/repo/
branch/fullSHA and the Git remote ref, audited the sole active PR, and completed0
without repushing. Keep its success separate from the original wrapper failure;
never rewrite either result or invent an original response. Future publishers
should save the full post-push response before assertions and leave unverified
partial state fields unknown until checked. Hosted CI/merge acceptance is separate.
