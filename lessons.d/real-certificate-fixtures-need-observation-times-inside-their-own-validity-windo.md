---
category: Testing & coverage
---

# Real certificate fixtures need observation times inside their own validity windows

The shared integration helper validated certificates at 2026-02-01, which is
inside the older Let's Encrypt fixture's validity window but before a checked-
in Sectigo leaf becomes valid on 2026-11-11. Once hostname policy was fixed,
the longer-chain test therefore failed with `NotYetValid` before reaching the
new composition. Parameterize server-policy helpers by an explicit observation
time and choose a deterministic time inside each real fixture's own window;
never relax or bypass validity policy to make a fixture pass.
