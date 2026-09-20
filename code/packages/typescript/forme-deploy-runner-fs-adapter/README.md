# @coding-adventures/forme-deploy-runner-fs-adapter

Atomic complete-tree publication for validated Forme deploy manifests. The
adapter writes every core-verified byte snapshot into a private sibling staging
tree, validates the staged tree, then swaps it with the selected root while
retaining the previous root for explicit rollback.

## Public API

```ts
import {
  prepareFilesystemPublication,
  publishFilesystemSite,
} from "@coding-adventures/forme-deploy-runner-fs-adapter";

// One-shot publication commits and finalizes the complete tree.
const result = await publishFilesystemSite({
  root: "/srv/example",
  manifest: deployManifest,
  contentStore,
  signal,
});

// Callers with post-publish checks can retain rollback authority explicitly.
const transaction = await prepareFilesystemPublication({
  root: "/srv/example",
  manifest: deployManifest,
  contentStore,
  signal,
});
try {
  await transaction.commit();
  await verifyPublishedSite();
  await transaction.finalize();
} catch (error) {
  if (transaction.state === "prepared" || transaction.state === "committed") {
    await transaction.rollback();
  }
  throw error;
}
```

`prepareFilesystemPublication` revalidates the untrusted manifest before any
target access. It rejects linked roots, linked path components, non-regular
entries, and multiply linked files. New files are opened exclusively beneath a
private sibling tree and populated only with the exact plain `Uint8Array`
snapshots returned by `forme-deploy-runner-core`.

Existing and staged trees are enumerated with streaming directory handles and
fail closed above 200,000 entries, 256 directory levels, or 64 MiB of portable
path metadata. `scanLimits` may lower, but never raise, those reviewed ceilings
for constrained hosts.

The adapter binds the canonical parent, lock, stage, backup, and rollback
containers to their device/inode identities. Every rename, deletion, and lock
release revalidates that identity and same-parent containment, so replacing a
transaction pathname cannot redirect cleanup into an unowned tree.

An exact existing file-and-directory tree is verified without acquiring the
write lock or touching target metadata. Changed trees acquire an exclusive
sibling lock, stage and reverify the complete manifest, and use a same-parent
backup for reversible publication. This naturally prunes stale files while
leaving every sibling outside the selected root untouched.

The transaction lifecycle is:

1. `prepared`: the complete private stage is ready; the visible root is
   unchanged.
2. `committed`: the stage is visible and the old root remains as a private
   backup. `rollback()` can still restore it.
3. `finalized`: the backup and lock are removed; rollback is intentionally no
   longer possible.

If final cleanup fails, the new root remains published and the adapter reports
`CLEANUP_FAILED`; it never destroys the new root in an attempt to restore a
backup that cleanup may already have partially removed. A failure before backup
deletion begins retains the lock and committed state for a safe retry; once
recursive deletion begins, the transaction is irreversibly finalized even if
residue remains.

Rollback is phase-tracked and retry-safe: once the old root is restored, a
cleanup retry can only revisit transaction residue and cannot move that restored
root again. When rollback or cleanup also fails after a primary content,
cancellation, or commit failure, the primary error and code remain authoritative
and secondary failures are attached as `secondaryErrors` diagnostics.

## Capability boundary

The package performs filesystem access only. Project-contained roots use the
host's project-storage grant. A root outside project storage requires the
caller's explicit `filesystem:user` approval before this adapter is invoked.
The adapter reads no environment variables and opens no network, shell, or
subprocess boundary.

## Development

```bash
npm ci
npm run build
npm test
npm run test:coverage
```

This package implements the FM-B045 filesystem slice of
[FM08](../../../specs/FM08-forme-deploy-runner.md).
