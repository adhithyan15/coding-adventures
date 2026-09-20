# forme-deploy-runner-github-pages-adapter

Atomic source-branch publication for Forme sites hosted by GitHub Pages. The
adapter validates and preflights an FM08 deploy manifest, creates immutable Git
blobs and a tree, commits against the observed source-branch head, and advances
that ref with a non-forced update. A reader therefore sees either the previous
complete commit or the new complete commit.

This is intentionally a source-branch adapter. Coding Adventures serves many
independently deployed applications from one legacy `gh-pages` branch. A Pages
artifact deployment replaces the complete site and cannot preserve those
sibling publishers. Source-branch commits let each Forme site own one prefix
while retaining the repository's current topology.

## Ownership and stale paths

Every deployment owner has one reserved target-side manifest:

```text
.forme/deployments/<owner>.json
```

It records the owner's configured destination, portable output paths, content
digests, and Git blob IDs. Before reuse or deletion, every recorded blob ID is
matched against a bounded snapshot of regular files in the target commit. A
later deployment may delete only those verified paths. All other branch paths
are inherited from the base tree. The same bounded tree snapshot rejects a
desired file that would replace an existing directory or sit below an existing
blob, preventing file/directory transitions from deleting unowned siblings.
The adapter rejects malformed ownership state, owner/destination rebinding,
file/directory prefix collisions, and overlap between owners before it creates
a commit or updates the ref. It also projects per-manifest, aggregate, and owner
count bounds before publication, then rereads the immutable candidate commit's
complete ownership set and target tree before advancing the ref. A candidate
that GitHub truncates or that the next run could not safely read is abandoned.
An existing exact target blob may be changed only when this owner's validated
prior manifest already records it; first-time adoption is deliberately a
separate migration concern rather than implicit overwrite authority.

## Usage

```ts
import {
  createGitHubRestBoundary,
  publishGitHubPagesSite,
} from "@coding-adventures/forme-deploy-runner-github-pages-adapter";

const result = await publishGitHubPagesSite({
  manifest,
  contentStore,
  boundary: createGitHubRestBoundary({ token }),
  owner: "example",
  repository: "site",
  ref: "heads/gh-pages",
  deploymentOwner: "blog",
  destination: "blog",
  retryLimit: 3,
  signal,
});
```

The REST boundary uses only GitHub's ref, commit, tree, and blob APIs. It
accepts the credential as an explicit value; the package never reads ambient
environment variables. FM-B047's CLI composition owns selection of the single
credential variable and capability grant.

## Retry and cancellation contract

- `409`/`422` from the non-forced ref update means another publisher advanced
  the branch. The adapter rereads ownership state and replans from the new head
  within a bounded retry budget.
- `429`, `502`, `503`, and `504` retry with bounded delay. A rate-limited `403`
  carrying `Retry-After` is normalized to `429`.
- Authentication, authorization, missing-target, malformed-response, and other
  validation errors are permanent.
- A network or gateway failure during the ref update triggers a fresh target
  read instead of an unsafe blind retry. Matching ownership proves success; an
  unprovable outcome is reported as `INDETERMINATE`, never as a definite
  rollback. A definite `429` rejection remains safe to retry with bounded
  delay.
- Cancellation aborts in-flight requests and starts no new work. Git objects
  created before cancellation remain unreachable. Cancellation after the ref
  update begins is `INDETERMINATE` unless the update has already returned
  success. Once the atomic ref update succeeds, the adapter never force-moves
  the branch for rollback.

## Security boundary

The target owner, repository, branch ref, deployment owner, destination, token,
response sizes, ownership manifest count, and retry budget are all
bounded and validated. The adapter has no filesystem, shell, subprocess, or
ambient-environment authority. Its declared runtime authority is outbound HTTPS
and DNS for the exact `api.github.com` host plus wall-clock reads and bounded
retry sleeps. The v0 concrete REST boundary intentionally does not accept a
caller-selected origin; a future GitHub Enterprise transport must remain an
explicit separately reviewed capability boundary.

This package implements FM-B046 of
[FM08](../../../specs/FM08-forme-deploy-runner.md).
