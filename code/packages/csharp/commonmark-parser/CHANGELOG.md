# Changelog

All notable changes to this package will be documented in this file.

## [0.1.0] - 2026-04-17

### Added

- Pure C# two-phase CommonMark parser built on the C# document AST package
- xUnit coverage for headings, paragraphs, lists, blockquotes, code fences, links, inline formatting, and entities
- Parser guardrails that reject oversized markdown inputs and excessively deep nesting before recursive parsing can exhaust the stack
- Ordered list parsing now rejects oversized numeric markers without throwing overflow exceptions on hostile input
- Inline parsing now caps unmatched delimiter and bracket searches so malformed hostile input cannot force full-tail rescans
- BUILD scripts now use `dotnet test --artifacts-path .artifacts` so transitive .NET project builds do not collide under parallel CI
- Linux BUILD scripts pin both `HOME` and `DOTNET_CLI_HOME` to the package-local `.dotnet` directory so parallel CI avoids `.NET` first-run migration races
- `BUILD_windows` now uses defensive `set "VAR=value"` quoting so path metacharacters cannot alter the command stream
