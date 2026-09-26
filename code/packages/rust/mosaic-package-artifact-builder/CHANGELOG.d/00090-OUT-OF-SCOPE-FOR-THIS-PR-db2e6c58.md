### Out of scope for this PR

The UI30 spec's `[variants]` manifest section (with `all` /
`overrides` / `fallback` keys) is **not** parsed here. Filesystem
discovery is sufficient for the "ship everything you authored"
default policy; manifest declarations are only needed when a
package wants to *constrain* which variants get built. Follow-up
PR will extend `mosaic-package-manifest` to parse the section and
wire it into the builder.

The variant-aware index file (mounting `Grid.desktop` and
`Grid.touch` as separate exports in the React/HTML/qmldir index)
is also deferred — the index continues to list each component
once, which is correct for the most common runtime-picks-variant
model (host imports either the default or the variant, never both).

