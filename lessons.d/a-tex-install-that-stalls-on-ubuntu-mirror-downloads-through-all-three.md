---
category: CI & GitHub Actions
---

# A TeX install that stalls on Ubuntu mirror downloads through all three watchdog retries is infrastructure failure

PR #17354's human-language book build failed before checkout content reached
the compiler. `apt-install.sh` attempted the focused TeX installation three
times; each attempt downloaded from `azure.archive.ubuntu.com`, made partial
progress, then produced no result for the 270-second watchdog. The final exit
was 124, with every book-build and publication step skipped. This is an Ubuntu
mirror transport failure, not evidence about the branch. Confirm the failure
step and that no repository validation ran after it, then rerun the failed job
unchanged; do not edit assessment content or weaken the installer watchdog.
