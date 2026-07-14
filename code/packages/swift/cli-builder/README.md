# swift/cli-builder

A pure Swift implementation of the declarative CLI parser described by
[`cli-builder-spec.md`](../../../specs/cli-builder-spec.md). A JSON document
defines commands, aliases, flags, positional arguments, defaults, dependencies,
and mutually exclusive groups; `Parser` turns an `argv` array into a typed
`ParseOutcome`.

The implementation uses the sibling `DirectedGraph` package for transitive
`requires` validation and the sibling `StateMachine` package to drive scanner
modes. It supports GNU, POSIX, subcommand-first, and traditional parsing,
stacked short flags, `--flag=value`, single-dash-long flags, variadic arguments,
help/version built-ins, enum defaults, repeatable flags, fuzzy suggestions, and
aggregated parse errors.

## Usage

```swift
import CliBuilder

let parser = try Parser(specPath: "tool.json", argv: ["tool", "-vv", "input.txt"])
switch try parser.parse() {
case .parsed(let result):
    print(result.flags["verbose"] ?? .null)
case .help(let result):
    print(result.text)
case .version(let result):
    print(result.version)
}
```

## Development

```sh
swift test
```
