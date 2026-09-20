### Forme deploy-runner pure core

- Added a capability-free deploy core that strictly validates untrusted current
  and previous manifests, portable output paths, content metadata, and
  file/directory prefix collisions.
- Added deterministic complete-set create/update/skip/delete planning with
  previous-manifest deletion authority limited to its explicitly owned paths.
- Added content-store byte/hash preflight and canonical dry-run reports, then
  split atomic filesystem publication, GitHub Pages integration, and CLI/live
  product composition into separately reviewable roadmap items.
- Enforced manifest and content resource budgets, cancellable deduplicated
  preflight, strict deploy metadata, immutable bound plans, and a session-bound
  reader that verifies the exact trusted byte snapshot an adapter may publish.
