---
category: Repo policy / workflow reminders
---

# Verify local blob bytes before treating a partial-clone fetch error as lost work

Preparing the dependent VER26 PR with Git 2.53 on Windows printed repeated
`upload-pack: not our ref` failures for one blob during cherry-pick. The replay
nevertheless completed. Before assuming data loss, the blob was read locally
with `git cat-file blob`; its bytes and Git object header hashed to the exact
requested SHA. The resulting branch was clean and had the intended focused diff.

Do not reset, prune or delete objects in response to a lazy-fetch error alone.
Preserve the source commits, verify the local object and final branch, and rerun
validation after replay. This observation does not establish the fetch error's
root cause or prove the whole shared object database healthy.
