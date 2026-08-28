# `@coding-adventures/forme-resolve-asset-refs-fs`

The first stage of Forme's filesystem asset pipeline. It consumes a stream of
parsed `ContentNode` values, discovers local Document AST images, resolves each
path beneath a configured storage root, and records `AssetRef` entries.

```text
Stream<ContentNode> -> Stream<ContentNode with AssetRef[]>
```

Relative image paths resolve beside the content source. Root-relative paths
resolve beneath `root`. HTTP(S), protocol-relative, data, and fragment-only
destinations remain ordinary authored URLs. A path that escapes `root` fails
closed.

The stage reads an optional `.<asset-name>.id.json` sidecar containing
`{"logicalId":"<uuid-v7>"}`. Without a sidecar it creates one in-memory
identity per normalized path for that run; it never writes into the source
tree.

The static renderer clones only AST branches containing resolved images and
replaces their destinations with `forme-asset:<logical-id>` placeholders. The
asset-aware emitter introduced later replaces only these collision-free
placeholders with fingerprinted public paths. Authored query strings and SVG
fragments remain on the placeholder so emission can preserve their semantics.

The package declares `storage:read` because it reads identity sidecars directly
while the FM02 capability-backed storage adapter is still pending.
