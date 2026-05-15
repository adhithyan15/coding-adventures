# Changelog — @coding-adventures/forme-types

## 0.1.0 — 2026-05-14

Initial release. Implements the FM01 §2 kernel types.

### Added

- `KERNEL_API_VERSION = 1` — the kernel-API stability marker.
- `KINDS` tuple and `KindName` union covering all 13 built-in kind names.
- `KindDescriptor` interface — the runtime type tag for a kind.
- `Kinds` canonical descriptor object (one per built-in non-Stream kind).
- `streamOf(inner)` and `isStreamDescriptor(d)` — Stream meta-kind helpers.
- `JsonValue`, `ReadonlyRecord` utility type aliases.
- `LogicalId`, `RevisionId` branded type aliases.
- TypeScript interfaces for all 12 built-in kind shapes:
  `ContentSource`, `ContentNode`, `Collection`, `Asset`, `Document`,
  `RenderedPage`, `PrintForme`, `RequestHandler`, `SearchIndex`,
  `Feed`, `DeployArtifact`, `Stream<T>`.
- Stub `StyleDocument` and `Interactivity` interfaces (FM04 / FM05 will
  replace these without breaking field names).
- `EMPTY_STYLE` and `EMPTY_INTERACTIVITY` sentinel constants.
- `KindPayload<K>` mapped type and augmentable `KindPayloadMap` interface.

### Spec divergences from FM01

- **Stream descriptors** use a closed-shape `{ name: "Stream", inner: K, version: "1.0" }`
  via the `streamOf()` helper instead of FM01's sketched
  `{ ...Kinds.X, kind: "Stream" }` (which adds a `kind` field outside
  the `KindDescriptor` interface). Cleaner, equivalent expressively.
- **`RevisionId`** uses `blake2b:<hex>` (the monorepo's existing
  from-scratch hash) instead of FM01's `blake3:<hex>`. The `<algo>:`
  prefix keeps the format forward-compatible.
