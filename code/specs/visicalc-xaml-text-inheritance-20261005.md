# XAML nested text-style inheritance

Related: #14274 and epic #14267.

Generated VisiCalc cells use an alignment-only implicit TextBlock style. WinUI resolves the nearest implicit style, so this replaces the ancestor style carrying the light foreground on a dark background.

Emit a uniquely keyed TextBlock style for every container text-style resource. Derive it from the enclosing generated style with BasedOn, and expose an implicit TextBlock style based on that key. Track the active resource lexically during emission, restoring it after each node and structural table row. Keys are compiler-generated, never authored strings. Preserve local setter precedence, template inheritance and sibling isolation; do not change generated output by hand or add application-specific branches.

Validate nested color/alignment and local overrides, siblings, repeated/conditional content and structural table rows. Run the emitter suite, generate and build VisiCalc with its standard Rust adapter, then inspect the actual Windows app and commit a formula in an isolated state fixture. This fixes text-style composition only; overall native design, accessibility, persistence and release acceptance remain open.
