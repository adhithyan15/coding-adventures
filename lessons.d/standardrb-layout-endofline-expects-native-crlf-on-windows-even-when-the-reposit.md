---
category: Cross-platform & Windows BUILD_windows
---

# StandardRB Layout/EndOfLine expects native CRLF on Windows even when the repository requires LF

The repository's `.gitattributes` intentionally enforces
`* text=auto eol=lf`, but StandardRB's `Layout/EndOfLine` cop uses the
host-native line ending on Windows and reports `Carriage return character
missing` for correctly checked-out LF Ruby files. Do not convert source files
to CRLF or relax the repository policy. A Ruby `BUILD_windows` front that runs
StandardRB must exclude only this cop with `--except Layout/EndOfLine`; keep the
ordinary `BUILD` on full StandardRB and keep every other Windows lint cop
enabled.
