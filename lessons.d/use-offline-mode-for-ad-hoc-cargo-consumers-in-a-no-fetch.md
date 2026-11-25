# Use offline mode for ad hoc Cargo consumers in a no-fetch task

A temporary consumer manifest without a lockfile can cause Cargo to update the
crates.io index even when every production dependency uses a local path.
Create or reuse a lockfile and pass `--offline` before running isolated
consumer checks when the task forbids fetches. A local path dependency alone
does not prevent Cargo from resolving registry transitive dependencies.
---
category: Supply chain & CI pinning
---
