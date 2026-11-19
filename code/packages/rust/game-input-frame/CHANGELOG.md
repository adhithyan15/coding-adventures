# Changelog

## 0.1.0

- Add a pure per-player action-frame reducer with caller-assigned `u64` ticks,
  64 button slots, 16 symmetric signed axes and a 256-change bound.
- Pin ordered transition semantics, first-error precedence and atomic refusal
  with the independent `game-input-frame-v1` corpus.
- Add deterministic property/fuzz, SAT-miter, and source-helper evidence with
  explicit finite claim limits.
