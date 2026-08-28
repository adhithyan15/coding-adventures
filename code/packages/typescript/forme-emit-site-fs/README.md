# @coding-adventures/forme-emit-site-fs

The asset-aware static-site emitter for Forme. It joins rendered pages with
loaded `Asset` IR, replaces renderer-owned `forme-asset:<logical-id>`
placeholders, and writes one complete `DeployArtifact` to disk.

## Stage contract

```ts
emitSiteFs.consumes   // streamOf(Kinds.RenderedPage)
emitSiteFs.inputPorts // { assets: streamOf(Kinds.Asset) }
emitSiteFs.produces   // Kinds.DeployArtifact
emitSiteFs.capabilities // ["filesystem:write"]
```

The default input and named asset input are both materialized by the
orchestrator, so the emitter runs exactly once. This preserves the page/asset
split established by the resolver and loader without frontmatter, event-bus,
or hidden-filesystem side channels.

## Output policy

- `outDir` is required.
- `assetDir` defaults to `assets` and must be a normalized portable relative
  path.
- `publicPathPrefix` defaults to empty. A GitHub Pages project site can set it
  to `/coding-adventures`; each URL segment is encoded before emission.
- An asset at `images/cat.png` is emitted as
  `assets/cat.<full-sha256>.png` and referenced as
  `/assets/cat.<full-sha256>.png` (or beneath `publicPathPrefix`).
- Query strings and fragments follow the placeholder and therefore survive
  substitution unchanged.
- `manifest.assets` records each logical ID, artifact path, MIME type, and
  complete SHA-256 digest.
- `manifest.buildId` covers the hashes of every rewritten page and asset file.

The stage copies asset bytes defensively, sorts output paths before writing,
and rejects duplicate identities, conflicting output paths, route traversal,
missing `meta.sourcePath`, inconsistent byte lengths, missing assets, and
undeclared placeholders. Validation finishes before the first write except for
cancellation, which is checked throughout collection and materialization.

`filesystem:write` follows the same adapter exception as `forme-emit-fs`: the
stage declares the capability and directly materializes through
`node:fs/promises` until FM02 supplies host filesystem adapters.

## Verification

```sh
npm run build
npm test
npm run test:coverage
```

Tests include a real orchestrator pipeline with explicit default and `assets`
wires, real temporary-directory writes, deterministic fingerprint and manifest
checks, suffix preservation, and failure-path coverage.
