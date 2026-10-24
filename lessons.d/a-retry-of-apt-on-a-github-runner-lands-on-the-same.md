---
category: CI & GitHub Actions
---

# A retry of apt on a GitHub runner lands on the same slow mirror unless the mirror list is rotated

The books job's TeX install (180 MB) failed three runs in a row, on main and on a PR,
with `apt-get install failed on all 3 attempt(s) (no result after 270s; treated as a
stalled mirror)`. The log showed it was not a stall: the Azure mirror was serving
50-170 KB/s, and every attempt crawled on that same mirror, about 15 MB in, before
the per-attempt cap cut it off. The .deb cache could not help, because main's own run
had failed and so had never saved one.

The runner's `ubuntu.sources` names `mirror+file:/etc/apt/apt-mirrors.txt` (Azure,
then archive.ubuntu.com, then security.ubuntu.com, each with a `priority:N`). apt
leaves a mirror only when a fetch FAILS, so a slow mirror is never left, and
"the next attempt usually lands on a healthy mirror" was false for this case.

Fix: `code/scripts/ci/apt-install.sh` rotates the list before each retry. The head
moves to the tail, priorities are renumbered, and the result is written beside the
list and renamed over it. Package integrity does not depend on the mirror: apt checks
every archive against the signed index.

Do differently: when a retry loop exists for a flaky network dependency, read the
log for what each attempt actually did (which host, how many bytes) before calling
it a stall. A retry that cannot change the host only re-rolls the same dice. Never
answer a slow mirror by raising the per-attempt cap.
