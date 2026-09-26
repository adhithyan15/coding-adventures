### Known limitations carried forward to later PRs

- The `BoolToVisibilityConverter` class itself is referenced by
  `x:Key` but not emitted. Hosts need to ship one (a 5-line C# class).
  A follow-up may bundle it as a fixed asset or emit it once per
  component.
- `HostInput`, `HostButton`, `HostScroll`, `HostTable` (+ section
  sub-tags), component references — all still `UnsupportedPrimitive`
  pending PR-3 / PR-4 / PR-5.

## [0.1.0] — Unreleased — PR-1 scaffold

