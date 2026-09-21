# @coding-adventures/forme-deploy-runner-core

The capability-free core of the Forme deployment boundary. It validates an
untrusted deploy manifest, computes a deterministic complete-set plan, verifies
that every referenced content blob has the promised size and SHA-256 digest,
and produces a stable dry-run report. Target adapters remain separate so pure
planning never acquires filesystem, network, environment, shell, or subprocess
authority.

## Public API

```ts
import {
  createDeployPlan,
  createDryRunReport,
  createVerifiedContentReader,
  parseDeployManifest,
  serializeDeployReport,
} from "@coding-adventures/forme-deploy-runner-core";

const current = parseDeployManifest(currentJson);
const previous = previousJson === undefined
  ? undefined
  : parseDeployManifest(previousJson);
const plan = createDeployPlan(current, previous);

// ContentStore is supplied by the caller. Bind once for a publication session:
// preflight verifies each unique digest without retaining the complete site,
// and read returns the exact trusted snapshot the adapter must publish.
const content = createVerifiedContentReader(current, contentStore);
await content.preflight();
await fsTarget.write("index.html", await content.read("index.html"));

const report = createDryRunReport(current, plan, "fs");
process.stdout.write(serializeDeployReport(report));
```

`parseDeployManifest` rejects unknown fields, count and byte-total mismatches,
non-canonical SHA-256 values, unsafe or non-portable output paths, map-key/path
mismatches, and file/directory prefix collisions. Both the current and previous
manifest must pass this boundary before planning. A previous manifest grants
delete authority only for paths it explicitly owns. v0 also enforces reviewed
manifest, file-count, per-file, and total-content limits before store access.

`createDeployPlan` emits sorted `create`, `update`, `skip`, and `delete`
entries. Metadata changes count as updates even when content bytes match.
`preflightDeployContent` calls only the injected `ContentStore` interface,
deduplicates equal digests, supports `AbortSignal`, and retains only a summary.
It is a standalone convenience for dry-run callers; publication adapters use
the bound reader shown above.
`createVerifiedContentReader` parses once per publication session and returns a
plain byte snapshot after verifying the exact bytes an adapter will write.
Dry-run timestamps and all write metrics are zeroed so the canonical serialized
report is reproducible regardless of object insertion order.

Canonical base64 SHA-256 values are opaque content-store keys. Directory and
bundle implementations must use `contentDigestToStoreKey()` to derive one
unpadded base64url filename segment; raw base64 must never be joined into a
filesystem or archive path because `/` is legal digest data.

## Development

```bash
npm ci
npm run build
npm test
npm run test:coverage
```

The package implements the pure FM-B044 slice of
[FM08](../../../specs/FM08-forme-deploy-runner.md). Atomic filesystem
publication, GitHub Pages integration, and `forme deploy` composition are
separate roadmap items because each crosses a different capability boundary.
