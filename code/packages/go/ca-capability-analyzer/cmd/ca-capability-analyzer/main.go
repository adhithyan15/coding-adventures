// Command ca-capability-analyzer is a static analyzer for Go source code
// that detects OS capability usage and banned constructs.
//
// # Usage
//
//	ca-capability-analyzer detect [--json] [--exclude-tests] <path>
//	ca-capability-analyzer check --manifest <path> <path>
//	ca-capability-analyzer banned [--json] <path>
//
// # Subcommands
//
// detect: Scan Go files and print all detected capabilities (imports,
// function calls that indicate filesystem, network, process, environment,
// or FFI usage).
//
// check: Compare detected capabilities against a required_capabilities.json
// manifest. Exits with code 0 if all capabilities are declared, 1 if
// there are undeclared capabilities.
//
// banned: Scan for banned constructs (reflect.Value.Call, plugin.Open,
// //go:linkname, unsafe.Pointer, cgo). These are forbidden regardless
// of capability declarations.
package main

import (
	"os"

	analyzer "github.com/adhithyan15/coding-adventures/code/packages/go/ca-capability-analyzer"
)

func main() {
	code := analyzer.RunCLI(os.Args[1:], os.Stdout)
	os.Exit(code)
}
