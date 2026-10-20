### Profile XAML persistence phases

Split opt-in snapshot timing into adapter serialization, UTF-8 encoding,
temporary-file writes and replacement. Preserve synchronous persistence,
warning recovery and aggregate timing; never record workbook content or paths.

Apply-props attribution now includes `apply-props-convert` per converted property,
`apply-props-set` per assigned property, and `apply-props-notify` around batch
completion. These fixed names omit property names and values. Sum repeated
conversion/setter samples within an apply-props interval; each consumes one slot
in the shared 4096-sample cap. Notification time includes synchronous bindings,
not subsequent layout or compositor presentation.
