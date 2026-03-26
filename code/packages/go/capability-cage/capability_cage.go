// Package capabilitycage provides a compile-time capability manifest system
// that enforces which OS-level resources a package is allowed to access.
//
// # The Problem
//
// Traditional security approaches declare capabilities in JSON files that are
// read at runtime. This creates an attack surface: if an adversary can write
// to the filesystem, they can escalate a package's permissions by editing the
// JSON file before the program reads it.
//
// # The Solution
//
// Capability Cage eliminates the runtime JSON read entirely. A code generator
// (capability-cage-generator) reads the JSON at build time and emits a Go
// source file (gen_capabilities.go) with the capabilities baked in as Go
// constants. The JSON file is a development-time artifact only — it is never
// shipped with or read by the compiled binary.
//
// # How It Works
//
//  1. Developer declares capabilities in required_capabilities.json.
//  2. Developer runs: capability-cage-generator --manifest=required_capabilities.json
//  3. Generator emits gen_capabilities.go with var Manifest = cage.NewManifest(...)
//  4. Package code calls cage.ReadFile(Manifest, path) instead of os.ReadFile(path).
//  5. At runtime, ReadFile checks the manifest before delegating to the OS.
//  6. If the operation is not declared, a CapabilityViolationError is returned.
//
// # Capability Categories
//
// Eight categories cover all OS-level resource types:
//
//   - fs:      Filesystem read/write/create/delete/list
//   - net:     Network connect/listen/dns
//   - proc:    Process exec/fork/signal
//   - env:     Environment variable read/write
//   - ffi:     Foreign function call/load
//   - time:    Time read/sleep
//   - stdin:   Standard input read
//   - stdout:  Standard output write
//
// # Target Matching
//
// Targets support glob matching:
//   - "*" alone matches any target (wildcard).
//   - "*.tokens" matches any file ending in .tokens within a single directory
//     level (does not cross "/" boundaries).
//   - Literal strings match exactly.
//
// # Example
//
//	// gen_capabilities.go (auto-generated, do not edit)
//	var Manifest = cage.NewManifest([]cage.Capability{
//	    {Category: cage.CategoryFS, Action: cage.ActionRead,
//	     Target: "*", Justification: "Reads grammar files at init time."},
//	})
//
//	// lexer.go
//	data, err := cage.ReadFile(Manifest, grammarPath)
//	if err != nil { return nil, err }
package capabilitycage

import "fmt"

// Category constants for compile-time checked capability categories.
// Using these constants instead of raw strings prevents typos and enables
// IDE autocompletion.
const (
	// CategoryFS represents filesystem operations (read, write, create,
	// delete, list). Use for any package that reads or writes files.
	CategoryFS = "fs"

	// CategoryNet represents network operations (connect, listen, dns).
	// Use for any package that makes outgoing or incoming network connections.
	CategoryNet = "net"

	// CategoryProc represents process operations (exec, fork, signal).
	// Use for any package that spawns subprocesses or sends signals.
	CategoryProc = "proc"

	// CategoryEnv represents environment variable operations (read, write).
	// Use for any package that reads os.Getenv or writes os.Setenv.
	CategoryEnv = "env"

	// CategoryFFI represents foreign function interface operations (call, load).
	// Use for any package that calls C libraries or loads shared objects.
	CategoryFFI = "ffi"

	// CategoryTime represents time operations (read, sleep).
	// Use for any package that reads the system clock or sleeps.
	CategoryTime = "time"

	// CategoryStdin represents reading from standard input.
	CategoryStdin = "stdin"

	// CategoryStdout represents writing to standard output.
	CategoryStdout = "stdout"
)

// Action constants for compile-time checked capability actions.
const (
	// ActionRead reads data from a resource (fs, env, time, stdin).
	ActionRead = "read"

	// ActionWrite writes data to a resource (fs, stdout).
	ActionWrite = "write"

	// ActionCreate creates a new resource (fs).
	ActionCreate = "create"

	// ActionDelete removes a resource (fs).
	ActionDelete = "delete"

	// ActionList enumerates resources in a container (fs directory listing).
	ActionList = "list"

	// ActionConnect opens an outgoing network connection.
	ActionConnect = "connect"

	// ActionListen binds and accepts incoming network connections.
	ActionListen = "listen"

	// ActionDNS performs a DNS lookup.
	ActionDNS = "dns"

	// ActionExec launches a subprocess.
	ActionExec = "exec"

	// ActionFork forks the current process.
	ActionFork = "fork"

	// ActionSignal sends a signal to a process.
	ActionSignal = "signal"

	// ActionCall invokes a foreign function (FFI).
	ActionCall = "call"

	// ActionLoad loads a shared library (FFI).
	ActionLoad = "load"

	// ActionSleep suspends execution for a duration.
	ActionSleep = "sleep"
)

// Capability declares a single OS-level permission.
//
// A capability grants the package permission to perform exactly one
// (category, action, target) operation. The Justification field is
// mandatory and should explain WHY the package needs this access — not
// just what it does. A reviewer should be able to decide whether the
// access is legitimate by reading the justification alone.
//
// Example:
//
//	cage.Capability{
//	    Category:      cage.CategoryFS,
//	    Action:        cage.ActionRead,
//	    Target:        "../../grammars/*.tokens",
//	    Justification: "Reads token grammar definitions to build the lexer DFA at startup.",
//	}
type Capability struct {
	// Category is the class of OS resource (fs, net, proc, env, ffi, time,
	// stdin, stdout). Use the Category* constants above.
	Category string

	// Action is the operation performed on the resource (read, write, etc.).
	// Use the Action* constants above.
	Action string

	// Target identifies the specific resource. For fs: a file path or glob
	// pattern. For net: a "host:port" string. For proc: a command name.
	// For env: a variable name. Use "*" to match any target (broad access,
	// use sparingly and with strong justification).
	Target string

	// Justification explains WHY this capability is needed. Must be specific
	// enough for a reviewer to decide whether the access is legitimate.
	// Minimum 10 characters.
	Justification string
}

// Manifest is an immutable, compile-time capability declaration for a package.
//
// Manifests are constructed via NewManifest, which accepts a slice of
// Capability values. Once created, a Manifest cannot be modified. This
// immutability guarantee means that the capabilities baked into the binary
// at compile time are the capabilities the binary will enforce at runtime —
// no file read, no network fetch, no injection vector.
//
// The zero value of Manifest (i.e., &Manifest{}) has an empty capability
// list and will deny all operations. Use EmptyManifest for pure computation
// packages instead of constructing a zero-value Manifest directly.
type Manifest struct {
	// capabilities is unexported to prevent mutation after construction.
	// The slice is copied in from the constructor, so callers cannot
	// modify the original slice and affect the manifest.
	capabilities []Capability
}

// NewManifest constructs an immutable Manifest from a slice of capabilities.
//
// This is the primary constructor, called from generated gen_capabilities.go
// files. Pass nil or an empty slice to declare a pure-computation package with
// zero OS access.
//
// Example:
//
//	var Manifest = cage.NewManifest([]cage.Capability{
//	    {Category: cage.CategoryFS, Action: cage.ActionRead,
//	     Target: "*", Justification: "Reads grammar files."},
//	})
func NewManifest(caps []Capability) *Manifest {
	// Copy the slice so that the caller cannot modify it later.
	// This preserves the immutability guarantee.
	copied := make([]Capability, len(caps))
	copy(copied, caps)
	return &Manifest{capabilities: copied}
}

// EmptyManifest is the pre-built zero-capability manifest for pure-computation
// packages. It declares no OS access. Any attempt to perform I/O through the
// cage wrappers will return a CapabilityViolationError.
//
// Use this as the Manifest for packages that do only in-memory computation:
// parsers, evaluators, data structures, algorithms.
var EmptyManifest = NewManifest(nil)

// Check returns a CapabilityViolationError if no declared capability covers
// the (category, action, target) triple. Returns nil if the operation is
// permitted.
//
// Target matching uses glob semantics:
//   - Exact string match: "foo.txt" matches only "foo.txt".
//   - Single-star glob: "*.tokens" matches "verilog.tokens" but not
//     "grammars/verilog.tokens" (star does not cross "/").
//   - Bare star: "*" matches any target.
//
// This method is called by all secure wrapper functions before delegating
// to the OS. It is also useful in tests to verify what a manifest allows.
func (m *Manifest) Check(category, action, target string) error {
	if m.Has(category, action, target) {
		return nil
	}
	return &CapabilityViolationError{
		Category: category,
		Action:   action,
		Target:   target,
		Message: fmt.Sprintf(
			"capability %s:%s:%s not declared; add it to required_capabilities.json then regenerate gen_capabilities.go",
			category, action, target,
		),
	}
}

// Has returns true if the manifest declares a capability that covers the
// (category, action, target) triple. Unlike Check, it does not return an
// error — useful for conditional logic without error handling overhead.
func (m *Manifest) Has(category, action, target string) bool {
	for _, cap := range m.capabilities {
		if cap.Category == category && cap.Action == action {
			if matchTarget(cap.Target, target) {
				return true
			}
		}
	}
	return false
}

// Capabilities returns a copy of the manifest's capability list.
// This is useful for introspection (e.g., generating documentation or
// performing static analysis), but not needed in normal usage.
func (m *Manifest) Capabilities() []Capability {
	result := make([]Capability, len(m.capabilities))
	copy(result, m.capabilities)
	return result
}

// CapabilityViolationError is returned when a package attempts an OS operation
// that was not declared in its manifest.
//
// The error message includes actionable instructions: it tells the developer
// exactly what to add to required_capabilities.json and reminds them to
// regenerate gen_capabilities.go so the capability is compiled into the binary.
//
// Example error message:
//
//	"capability fs:read:/etc/passwd not declared; add it to
//	 required_capabilities.json then regenerate gen_capabilities.go"
type CapabilityViolationError struct {
	// Category is the category of the denied operation (e.g., "fs").
	Category string

	// Action is the action that was denied (e.g., "read").
	Action string

	// Target is the specific resource that was denied (e.g., "/etc/passwd").
	Target string

	// Message is the human-readable explanation with remediation steps.
	Message string
}

// Error implements the error interface.
func (e *CapabilityViolationError) Error() string {
	return e.Message
}
