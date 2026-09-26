# Recheck origin main after long package validation windows

A clean exact-main inventory became stale again while the Kotlin package and
coverage gates ran. Re-fetch and compare `origin/main` after long validation
windows; when intervening commits only modify existing package roots, preserve
the measured inventory counts, update the exact revision, and rebase before
the lane commit is considered final.
