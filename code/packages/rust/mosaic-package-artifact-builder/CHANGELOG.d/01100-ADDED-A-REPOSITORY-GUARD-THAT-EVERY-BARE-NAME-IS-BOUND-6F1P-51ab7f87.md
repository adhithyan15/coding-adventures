### Added — a repository guard that every bare name is bound (6f1p)

`tests/every_repository_bare_name_is_bound.rs` runs the layout validator's
new `UnboundName` rule (UI29 §3.4, "A bare name must be bound") over the
whole repository, in the two places it applies:

- `every_authored_layout_binds_every_bare_name` compiles every `.mll`
  beside its `.mil` (about 120 layouts) against that interface. This is
  where an unbound bare name in a component's own primitives is found.
- `every_composed_export_binds_every_bare_name` composes every export of
  every `mosaic-package.toml` the way a package build does: sibling exports
  qualified, `pkg::` references inlined, then validated again against the
  consumer's interface. This is where a bare value passed at a call site,
  which the authored check skips, is found.

Only `UnboundName` failures count; count assertions catch a walk that
checks nothing. When it was added, nothing in the repository tripped it.
Injecting `title : zzcallsite` at a JournalApp `EmptyState` call site and
`content: zzauthored` into `Card.mll` made both tests fail, naming the
node path of each.
