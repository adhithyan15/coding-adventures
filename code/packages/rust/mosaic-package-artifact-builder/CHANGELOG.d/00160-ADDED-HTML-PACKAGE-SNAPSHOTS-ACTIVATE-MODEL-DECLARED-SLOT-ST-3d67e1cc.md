### Added — HTML package snapshots activate model-declared slot states

HTML package components now bake each `one-of` slot's deterministic first
member into its owning mosstyle state instead of silently dropping all state
blocks. This matches the generated standalone shell's fallback props and gives
UI49 a package-level regression for the static backend (#14368).

