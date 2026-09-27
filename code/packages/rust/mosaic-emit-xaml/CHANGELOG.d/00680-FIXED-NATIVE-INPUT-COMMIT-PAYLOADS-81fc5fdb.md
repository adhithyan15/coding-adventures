### Fixed - Native input commit payloads

`HostInput.onCommit` and `onCancel` handlers now inspect the authored MIL emit
schema. Void events remain parameterless, while single text, number, or boolean
events receive the native `TextBox` value with the required conversion. Complete
TaskApp generation therefore constructs `SheetEditCommit(value: text)` correctly
instead of emitting code-behind that cannot compile.

