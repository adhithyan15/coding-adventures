## [Unreleased]

### Security

- Fixed all 45 CodeQL `go/unsafe-quoting` alerts (`#261`–`#305`) in `main.go`:
  - Replaced `fmt.Sprintf` JSON template interpolation for TypeScript
    `package.json` and Lua/Swift/Java/Kotlin `required_capabilities.json` with
    `json.MarshalIndent` on typed structs (`tsPackageJSON` and
    `requiredCapabilitiesManifest` via `requiredCapabilitiesJSONWithJustification`).
  - Added `sanitizeIdent` (`identQuoteStripper`) and applied it at `run()`,
    `scaffold()`, `toSnakeCase()`, `toCamelCase()`, `toJoinedLower()`,
    `updateRustWorkspace()`, and language generator entrypoints to sever
    double- and single-quote taint paths on validated kebab-case identifiers.
  - Added syntax-appropriate escaping (`tomlStringEscaper`,
    `rubyElixirStringEscaper`, `perlSingleQuoteEscaper`) for free-text
    `description` values in TOML/Python, Ruby/Elixir, and Perl templates.
  - Added `TestSanitizeIdentStripsQuotes` and
    `TestGenerateEscapesSpecialCharactersInDescriptions` in `main_test.go`.
