# Capability Analyzer (Go)

A static analyzer that walks Go ASTs to detect OS capability usage (filesystem, network, process, environment, FFI) and banned constructs. Part of the [coding-adventures](https://github.com/adhithyan15/coding-adventures) capability-based security system.

## What it does

The analyzer parses Go source files using `go/ast`, `go/parser`, and `go/token` to detect:

1. **Import-level capabilities** — importing `"os"` implies filesystem access, `"net"` implies network access, etc.
2. **Call-level capabilities** — `os.Open("file.txt")` implies `fs:read:file.txt`, `exec.Command("ls")` implies `proc:exec:ls`
3. **Banned constructs** — `reflect.Value.Call()`, `plugin.Open()`, `//go:linkname`, `unsafe.Pointer`, cgo

## Usage

### As a library

```go
import analyzer "github.com/adhithyan15/coding-adventures/code/packages/go/ca-capability-analyzer"

// Detect capabilities in a source string
caps, err := analyzer.AnalyzeSource("main.go", sourceCode)

// Detect capabilities in a file
caps, err := analyzer.AnalyzeFile("path/to/main.go")

// Detect capabilities in a directory
caps, err := analyzer.AnalyzeDirectory("path/to/package", true /* excludeTests */)

// Detect banned constructs
violations, err := analyzer.DetectBannedSource("main.go", sourceCode)

// Load manifest and compare
manifest, err := analyzer.LoadManifest("required_capabilities.json")
result := analyzer.CompareCapabilities(caps, manifest)
fmt.Println(result.Summary())
```

### As a CLI

```bash
# Build
cd cmd/ca-capability-analyzer && go build -o ca-capability-analyzer .

# Detect capabilities
./ca-capability-analyzer detect ./path/to/package
./ca-capability-analyzer detect --json ./path/to/file.go

# Check against manifest
./ca-capability-analyzer check --manifest required_capabilities.json ./path/to/package

# Scan for banned constructs
./ca-capability-analyzer banned ./path/to/package
```

## Detection rules

| Pattern | Capability | Example |
|---------|-----------|---------|
| `import "os"` | fs:\*:\* | Broad filesystem access |
| `import "net"` | net:\*:\* | Broad network access |
| `import "os/exec"` | proc:exec:\* | Process execution |
| `import "unsafe"` | ffi:\*:\* | Foreign function interface |
| `os.Open("x")` | fs:read:x | Read specific file |
| `os.Create("x")` | fs:write:x | Write specific file |
| `os.Getenv("K")` | env:read:K | Read env var |
| `exec.Command("c")` | proc:exec:c | Run specific command |

## Where it fits

```
CI Pipeline
    |
    +---> Capability Analyzer <-- you are here
    |       (detect, check, banned)
    |
    +---> required_capabilities.json
            (declared capabilities)
```

## Testing

```bash
go test ./... -v -cover
```

## Implementations

| Language | Location |
|----------|----------|
| Python | `code/packages/python/ca-capability-analyzer/` |
| **Go** | `code/packages/go/ca-capability-analyzer/` |
