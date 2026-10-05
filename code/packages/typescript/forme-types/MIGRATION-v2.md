# Forme kernel API v2 migration

Forme's first stable package line is `1.0.0`. Those package versions implement
kernel `apiVersion: 2`; package semver and kernel API version are separate
compatibility axes. Kind descriptors have their own versions as well.

## Breaking changes

### Stages and plugins target kernel API v2

Change every direct `Stage.apiVersion`, plugin `apiVersion`, and
`plugin.toml` `apiVersion` from `1` to `2`. Hosts and runners reject v1 rather
than attempting an implicit conversion.

```diff
- apiVersion: 1
+ apiVersion: 2
```

### `RenderedPage` requires provenance

`RenderedPage.source` is removed. `RenderedPage.provenance` is required, and
the runtime descriptor advances from `RenderedPage@1.1` to
`RenderedPage@2.0`. Use `createOutputProvenance` from
`@coding-adventures/forme-identity`; do not copy a logical ID into a synthetic
revision.

For one input:

```diff
- source: node.identity,
+ provenance: createOutputProvenance([{
+   identity: node.identity,
+   revision: node.revision,
+ }]),
```

For aggregate output, include every exact contributor. The helper normalizes
their ordering, rejects conflicting revisions for one identity, and computes
the aggregate revision:

```typescript
const provenance = createOutputProvenance(nodes.map(node => ({
  identity: node.identity,
  revision: node.revision,
})));
```

There is no implicit v1 adapter. If a migration must temporarily compose an
external v1 producer, place an explicit reviewed adapter before the v2 edge so
the missing revisions are obtained from an authoritative source rather than
invented.

## Version checks

- npm packages in the stable Forme release report `1.0.0`.
- `KERNEL_API_VERSION` is `2`.
- `Kinds.RenderedPage.version` is `"2.0"`.
- Other built-in kind versions are unchanged.

A mismatch on any axis fails closed with upgrade guidance. Upgrade all Forme
packages in one dependency graph together; mixing pre-1.0 packages with the
stable line is unsupported.
