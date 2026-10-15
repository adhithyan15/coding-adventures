---
category: Supply chain & CI pinning
---

# A failed hosted compiler probe must retain its bounded child diagnostic in the job log

Published8f macOS job116607025275 passed all12 portable tests and then failed
the real cabal-refuse-pgmc assertion. The job showed only a runner-local log
path; captured Cabal/GHC stdout/stderr could not be recovered from its artifacts.
That establishes an unproven negative control, not its underlying compiler cause.

Keep the exact refusal check and publish-only-after-success behavior. Append
only the failing command's last4096 captured characters as a JSON-quoted tail,
while preserving the full local log and receipt. Test both ordinary failure and
unproven-refusal paths, truncation, control escaping and no environment export.
Do not print environment/config files or claim the macOS cause repaired until
the actual configured routes and real negative/positive probes pass.
