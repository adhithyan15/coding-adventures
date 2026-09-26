### Fixed — load owner palettes during standalone package composition

Manifest-aware component composition now loads the owning package's scoped
token palette with the same backend selection, precedence, and containment
checks used by full package builds.

