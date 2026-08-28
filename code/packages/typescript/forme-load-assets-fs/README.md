# `@coding-adventures/forme-load-assets-fs`

The byte-loading stage of Forme's filesystem asset pipeline:

```text
Stream<ContentNode with AssetRef[]> -> Stream<Asset>
```

The stage buffers one invocation, validates that each logical identity maps to
one normalized source and role, deduplicates references shared by many pages,
and emits assets in portable source-path order. Each `Asset` carries a
domain-separated binary revision, a defensive byte copy, detected MIME type,
and its resolved `sourcePath` in `meta` for the downstream fingerprinting
emitter.

Both lexical and canonical containment are enforced. The configured root and
every referenced file are resolved with `realpath` before bytes are read. An
in-root symlink is allowed; a symlink that resolves outside the storage root
fails closed. Missing paths, directories, malformed source locators, identity
collisions, and role collisions are actionable errors.

The package declares `storage:read` because it reads asset bytes directly while
the FM02 capability-backed storage adapter is still pending. It has no write,
network, environment, or process capability.
