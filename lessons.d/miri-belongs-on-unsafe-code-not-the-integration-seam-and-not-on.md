---
category: CI & GitHub Actions
---

# Miri belongs on unsafe code, not the integration seam — and not on every PR

PR 7 moved `twig-vm` Miri off the per-PR critical path entirely.  PRs only run Miri on `lang-runtime-core` + `dynval-runtime` (where the unsafe is); both run in ~5 min total, blocking.  `twig-vm` Miri runs only on **post-merge to main** + a nightly cron, both via `lang-runtime-safety-deep.yml`, and **never gates anything** (`continue-on-error: true`) because twig-vm has zero unsafe — a Miri failure there is an integration-seam regression worth investigating, not a "main is broken" signal.  Engineers run `code/scripts/miri-twig-vm.sh` locally before pushing twig-vm changes — that's the canonical verification.  Lessons: (a) when Miri wallclock exceeds the per-PR budget, split by **where the unsafe lives**, not by tightening the cap further; (b) intensive checks belong on **main, not PRs** — fast PR iteration matters more than 100% per-PR coverage; (c) for crates without unsafe, even main-side Miri stays non-blocking — the workflow run history is the regression marker, not a status-badge red X.
