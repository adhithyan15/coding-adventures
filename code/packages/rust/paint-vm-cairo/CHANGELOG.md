# Changelog

## 0.1.0

- Added native Linux/BSD Cairo image-surface rendering through `cairo-rs`.
- Added deterministic non-Cairo fallback rendering for non-Linux/BSD targets.
- Added runtime descriptor capabilities and selection tests.
- Added linear and radial gradient rendering through native Cairo patterns and
  deterministic software sampling for the fallback path.
- Added smoke coverage for rects, clipping, text visibility, and runtime
  degraded-text opt-in.
