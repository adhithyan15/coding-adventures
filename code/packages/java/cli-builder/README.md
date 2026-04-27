# Java CLI Builder

Native Java implementation of the CLI Builder package.

It validates CLI specs, parses argv according to the repo's JSON schema, and
returns one of three outcomes:

- `ParseResult`
- `HelpResult`
- `VersionResult`

The package also exposes `CliBuilder.validateSpec(...)` helpers for validating
spec files outside the parse path.
