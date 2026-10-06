# Changelog — @coding-adventures/forme-render-terminal

## 1.0.0 — 2026-10-05

- Join the aligned stable Forme `1.0.0` package line targeting kernel API v2; see the [`forme-types` migration guide](../forme-types/MIGRATION-v2.md) for the breaking `RenderedPage` provenance and stage/plugin version changes.

## 0.1.0 — 2026-10-04

### Added

- Pure `Stream<ContentNode> → Stream<TerminalBuffer>` renderer using resolved
  Style IR and validated per-route Interactivity IR.
- Safe ANSI rendering with explicit style, interactivity, raw-node, and asset
  degradation records plus exact provenance.
- Capability-free terminal artifact packager emitting deterministic `.ansi`
  and canonical `.degradations.json` files.
- Bounded descriptor snapshots, AST/HTML/output/artifact limits, renderer- and
  artifact-content-bound identity, ANSI/bidi hardening, portable file/ancestor
  collision checks, adversarial coverage, and live blog product proof.
