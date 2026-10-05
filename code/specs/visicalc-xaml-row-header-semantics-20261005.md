# XAML native semantics for numbered-row tables

Related: #14278, #14388, #14276, epic #14267.

RowHeaderGrid currently misses the canonical native table recognizer because
each row has a leading semantic header before its repeated data cells. Recognize
only paired, explicitly marked corner/row-header prefixes, retaining all indexed
loop and source checks. Unmarked/asymmetric extra children remain unsupported.

Reuse native table and cell automation peers without shifting logical data
columns. Emit row-header peers and connect them through ITableProvider.GetRowHeaders
and ITableItemProvider.GetRowHeaderItems; authored row label text supplies names.
Keep normal grids unchanged. Preserve authored widths, click events and state
hosts, and validate generated Windows markup plus actual interaction/UIA output.
This slice does not claim complete keyboard/editing or native-complete acceptance.
