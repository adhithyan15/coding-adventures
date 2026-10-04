# Changelog — @coding-adventures/forme-render-terminal

## 0.1.0 — 2026-10-04

### Added

- Pure `Stream<ContentNode> → Stream<TerminalBuffer>` renderer using resolved
  Style IR and validated per-route Interactivity IR.
- Safe ANSI rendering with explicit style, interactivity, raw-node, and asset
  degradation records plus exact provenance.
- Capability-free terminal artifact packager emitting deterministic `.ansi`
  and canonical `.degradations.json` files.
- Bounded hostile-data snapshots, AST/HTML/output limits, portable path and
  case-collision checks, adversarial coverage, and live blog product proof.
