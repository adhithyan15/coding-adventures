# Preserve equivalent XAML list property identities

For VisiCalc #16865. ApplyProps currently deserializes and assigns fresh lists on
every update, even when their contents match the component. Dependency properties
then see different references and invalidate bindings. Row VM reuse alone does
not prevent this work. Detailed native profiling located one 118 ms selection
tail inside property application, but does not establish list replacement as its
cause; a native comparison remains required.

After successful conversion, reuse the current property value only for exact
IReadOnlyList<T> types whose leaves are string, double or bool (including nested
lists), when contents match in order and length. Compare against the live getter,
not cached JSON, so local edits are corrected by subsequent runtime updates.
Continue invoking the setter, preserve strict conversion and batching behavior,
and do not change scalar/custom-object/array property semantics. Getter or list
enumeration failures fall back to the converted value. Bound comparison by the
incoming list length at each level; never walk an unbounded existing sequence.

Validate unchanged nested identity, changed leaves/length/order, null values,
local mutation, unsupported types, write-only and throwing getters/enumerators,
and setter invocation using executed C# host tests. Record native before/after
evidence separately; do not close performance or release acceptance from tests.
