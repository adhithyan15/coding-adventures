# Kotlin CLI Builder

Native Kotlin implementation of the CLI Builder package.

It loads and validates CLI Builder JSON specs, parses argv, and returns one of:

- `ParseResult`
- `HelpResult`
- `VersionResult`

Validation helpers are also exposed through `CliBuilder.validateSpec(...)`.
