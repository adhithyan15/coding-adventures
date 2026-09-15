---
category: CI & GitHub Actions
---

# Miri timeout grows with code, not with test count

`lang-runtime-safety.yml` had `timeout-minutes: 30`; PR 5 (closures) tripled `twig-vm` Miri wallclock and one of two parallel runs failed at 30:15 from runner variance, not a real bug. Bump generously (90 min) and shard by crate when wallclock crosses 60 min. Locally, `MIRIFLAGS="-Zmiri-ignore-leaks" cargo +nightly miri test -p twig-vm` is the canonical pre-push smoke check; don't trust the timeout to catch slowdown.
