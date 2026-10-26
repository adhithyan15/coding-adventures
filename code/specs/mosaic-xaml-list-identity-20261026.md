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

## Validation receipt

Implementation a63fb132f8f passed 44 binding unit tests and the executed emitted
C# host suite, including the new props-lists scenario. Security review passed.
The XAML host was regenerated through the public binding generator with namespace
Mosaic.Generated and application id visicalc. Other previously generated component
and project files were retained from the #18278 candidate; no generated source
was manually patched. The unchanged Oct16 Rust runtime was reused. Release x64
build: zero errors and two existing CS0067 warnings. Managed VisiCalc.dll SHA256:
51431AF4D73686617B6B8B410E6D913B5D05BE849E13A82924101ECEF1261240.

Native Computer Use: click B2=14, alternate Right/Left four times (C2=7/B2=14),
select blank G6, focus the formula field, type =20+22, press Return and observe 42.
Close normally, relaunch with the same isolated snapshot path, observe G6=42 and
formula =20+22 restored, close normally again. This proves snapshot recovery for
that edit, not the separate workbook Save/Open flow or full durability acceptance.

The four keyboard selection next-render intervals were 16.2312, 19.8412, 23.7030
and 20.6830 ms. Each reports 20 projection reads, 0 created, 513 reused, 0 removed
and zero prepared/cleared callbacks, versus 21 reads/539 reused in the previous
populated-selection receipt. Conversion still allocates; this is less projection
work, not proof of a latency or allocation threshold. The formula commit next-render
interval was 68.4726 ms with 27 created/486 reused/1 removed projections and 26/3
prepared/cleared callbacks. Nine total first-process interaction intervals are
smoke coverage, not a percentile workload. #16865 and release acceptance stay open.

Raw local evidence is retained under
C:/Users/adhit/.codex/worktrees/mosaic-xaml-equal-lists-20261026-output/:
native-release-build.log, native-selection-01.jsonl and native-restart-01.jsonl.
Screenshots were inspected in the chat. Repeat the matched warm Release/Debug
workload and investigate editing/scrolling tails before claiming performance closure.
