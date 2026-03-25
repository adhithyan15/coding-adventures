# capability-cage

Compile-time capability manifest enforcement for Go packages.

## The Problem

Traditional security approaches declare capabilities in JSON files read at runtime.
An attacker with filesystem write access can escalate permissions by editing the
JSON before the program reads it. This is a classic TOCTOU (time-of-check to
time-of-use) vulnerability applied to capability declarations.

## The Solution

Capability Cage eliminates the runtime JSON read. A code generator reads the JSON
**at build time** and emits a Go source file (`gen_capabilities.go`) with
capabilities baked in as Go constants. The JSON is a development-time artifact —
it is never shipped with or read by the compiled binary.

## How It Works

1. Declare capabilities in `required_capabilities.json`
2. Run: `capability-cage-generator --manifest=required_capabilities.json`
3. Generator emits `gen_capabilities.go` with `var Manifest = cage.NewManifest(...)`
4. Call `cage.ReadFile(Manifest, path)` instead of `os.ReadFile(path)`
5. At runtime the cage checks the manifest; undeclared operations return an error

## Usage

```go
// gen_capabilities.go (auto-generated, do not edit)
var Manifest = cage.NewManifest([]cage.Capability{
    {
        Category:      cage.CategoryFS,
        Action:        cage.ActionRead,
        Target:        "*",
        Justification: "Reads grammar files at startup to configure the lexer DFA.",
    },
})

// lexer.go
import cage "github.com/adhithyan15/coding-adventures/code/packages/go/capability-cage"

data, err := cage.ReadFile(Manifest, grammarPath)
if err != nil {
    return nil, err  // CapabilityViolationError if not declared
}
```

## Capability Categories

| Category  | Actions                              | Description                   |
|-----------|--------------------------------------|-------------------------------|
| `fs`      | read, write, create, delete, list    | Filesystem operations         |
| `net`     | connect, listen, dns                 | Network operations            |
| `proc`    | exec, fork, signal                   | Process management            |
| `env`     | read, write                          | Environment variables         |
| `ffi`     | call, load                           | Foreign function interface    |
| `time`    | read, sleep                          | Time operations               |
| `stdin`   | read                                 | Standard input                |
| `stdout`  | write                                | Standard output               |

## Target Matching

- `"*"` — matches any target (broad access, use sparingly)
- `"*.tokens"` — matches files ending in `.tokens` in the **same directory only** (star does not cross `/`)
- `"grammars/verilog.tokens"` — exact match only

## Pure Computation Packages

Packages that perform only in-memory computation (parsers, evaluators, data
structures) should use `cage.EmptyManifest`:

```go
var Manifest = cage.EmptyManifest
```

## Stack Position

```
capability-cage-generator  →  generates gen_capabilities.go
capability-cage            →  checks manifest, delegates to Backend
OpenBackend                →  calls stdlib (os.ReadFile, net.Dial, etc.)
CageBackend (future)       →  sends JSON-RPC to host (D18 Chief of Staff)
```

## Design Decisions

- **No runtime JSON read**: capabilities are compiled in, not read from disk
- **Immutable manifests**: `NewManifest` copies the slice; callers cannot mutate
- **Backend injection**: `WithBackend` allows test doubles without hitting real OS
- **Zero dependencies**: no external packages required
