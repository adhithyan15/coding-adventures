### Fixed - Typed-template helper ownership

Generated `For` row view models now retain their owning component and expose
Mosaic expressions as ordinary computed row properties. Those properties
delegate to assembly-local component helpers in C#, so WinUI's compiled
binding engine never has to resolve a page method from a typed DataTemplate.
Nested loops are projected by their enclosing row VM and capture outer element
and index bindings, while component-slot bindings route through the retained
owner. This keeps flat-list filters, nested grid cells, and editor state in the
correct typed scope without application-specific XAML glue.
Generated nested grid projections also zip authored `column-widths` into each
cell VM and invalidate outer projections when nested sources or widths change,
preserving both visible geometry and live runtime updates.
The shared visibility converter now applies Mosaic truthiness to booleans,
numbers, text, and collections, so string-backed list fields drive `If` blocks
the same way they do on the other native backends.

