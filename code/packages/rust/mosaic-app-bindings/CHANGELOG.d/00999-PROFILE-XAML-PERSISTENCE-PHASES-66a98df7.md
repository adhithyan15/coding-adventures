### Profile XAML persistence phases

Split opt-in snapshot timing into adapter serialization, UTF-8 encoding,
temporary-file writes and replacement. Preserve synchronous persistence,
warning recovery and aggregate timing; never record workbook content or paths.
