# Write captured native JVM JSON as explicit UTF-8

PowerShell redirection of a native command can choose a text encoding that
does not match the JSON consumer's UTF-8 input. The generated report looked
like valid JSON in a text editor, but the JVM replay parser rejected the
captured file because its bytes were not UTF-8. When a native process's output
must be saved and replayed by another process, capture the text and write it
with an explicit UTF-8 encoding (without a BOM), or preserve the original
output bytes directly. Exercise the installed executable, not only the
serializer in a unit test.
---
category: Cross-platform & Windows BUILD_windows
---
