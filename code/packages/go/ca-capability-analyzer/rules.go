package main

// rules.go defines the complete mapping from Go stdlib imports and function
// calls to capability strings. This is the knowledge base of the analyzer —
// everything "what does os.ReadFile mean in terms of capabilities?" lives here.
//
// There are two kinds of rules:
//
//   - ImportRule: fires when a specific import path is present, regardless of
//     which functions from that package are called. Used for packages where
//     any usage implies the capability (net, net/http, os/exec).
//
//   - CallRule: fires when a specific function from a specific import is called.
//     Used for packages like "os" where only certain functions imply a capability
//     (os.ReadFile implies fs:read:*, but os.Exit does not imply any tracked cap).
//
// Design rationale for import-level vs call-level split:
//
//   - "net" and "net/http": any import implies net:*:* because there is no
//     meaningful way to import these packages without doing network I/O.
//   - "os/exec": any import implies proc:exec:* — there is no non-I/O use.
//   - "os": NOT at import level because os.Exit, os.Args, os.Getpid etc. are
//     common in every CLI and don't imply filesystem or env capabilities. Only
//     specific functions (ReadFile, Create, Getenv…) are tracked.
//   - "time": NOT at import level because time.Duration, time.Second etc. are
//     pure constants with no OS access. Only time.Now and time.Sleep are tracked.
//   - "fmt": flagged only for Print/Println/Printf because they write to stdout.
//     fmt.Sprintf and fmt.Errorf are pure string operations.

// importCapabilityRule maps a bare import path to a capability that is implied
// by the mere presence of that import.
type importCapabilityRule struct {
	// ImportPath is the full Go import string, e.g., "net/http".
	ImportPath string

	// Capability is what the import implies.
	Capability CapabilityString

	// Evidence is the human-readable description used in violation messages.
	Evidence string
}

// callCapabilityRule maps a qualified function call (by import path + function name)
// to a capability.
type callCapabilityRule struct {
	// ImportPath is the import that must be present for this rule to apply.
	// Must match the string in the source's import statement.
	ImportPath string

	// FunctionName is the unqualified name, e.g., "ReadFile".
	FunctionName string

	// Capability is the inferred capability.
	Capability CapabilityString

	// Evidence is the human-readable description, e.g., "os.ReadFile call".
	Evidence string
}

// ImportRules is the table of import-level rules. Exported so tests can inspect
// it and verify completeness.
//
// Each entry fires as soon as the given import path appears in the file's
// import list, regardless of which functions are called.
var ImportRules = []importCapabilityRule{
	// ── Network ──────────────────────────────────────────────────────────────
	// "net" is the low-level networking package. Any file that imports it is
	// doing socket-level I/O.
	{ImportPath: "net", Capability: "net:*:*", Evidence: `import "net"`},

	// "net/http" is the most common Go networking package. Importing it
	// implies the package makes or serves HTTP requests.
	{ImportPath: "net/http", Capability: "net:*:*", Evidence: `import "net/http"`},

	// ── Process execution ─────────────────────────────────────────────────
	// "os/exec" is the only stdlib package for spawning subprocesses. Any
	// import implies proc:exec:*.
	{ImportPath: "os/exec", Capability: "proc:exec:*", Evidence: `import "os/exec"`},
}

// CallRules is the table of call-level rules. Exported so tests can inspect it.
//
// Each entry fires when a specific function from a specific import is called.
// The analyzer checks both that the import is active AND that the function is
// called; neither condition alone is sufficient for call-level rules.
//
// Rules are grouped by capability category for readability.
var CallRules = []callCapabilityRule{
	// ── Filesystem reads (fs:read:*) ──────────────────────────────────────
	// os.Open opens a file for reading. It returns an *os.File which can
	// subsequently be read with Read/ReadAt/ReadDir. The open itself is
	// the capability-requiring operation.
	{ImportPath: "os", FunctionName: "Open", Capability: "fs:read:*", Evidence: "os.Open call"},

	// os.ReadFile reads an entire file into memory in one call. The most
	// common file-read pattern in modern Go.
	{ImportPath: "os", FunctionName: "ReadFile", Capability: "fs:read:*", Evidence: "os.ReadFile call"},

	// os.OpenFile is the generalized open — can open for reading, writing,
	// or both. We tag it fs:read:* conservatively; if it is also used for
	// writing the fs:write:* rule below will fire too (both are reported).
	{ImportPath: "os", FunctionName: "OpenFile", Capability: "fs:read:*", Evidence: "os.OpenFile call"},

	// ── Filesystem writes (fs:write:*) ────────────────────────────────────
	// os.Create creates or truncates a file for writing.
	{ImportPath: "os", FunctionName: "Create", Capability: "fs:write:*", Evidence: "os.Create call"},

	// os.WriteFile writes a byte slice to a file (creating it if necessary).
	{ImportPath: "os", FunctionName: "WriteFile", Capability: "fs:write:*", Evidence: "os.WriteFile call"},

	// os.Mkdir creates a directory.
	{ImportPath: "os", FunctionName: "Mkdir", Capability: "fs:write:*", Evidence: "os.Mkdir call"},

	// os.MkdirAll creates a directory and all missing parents.
	{ImportPath: "os", FunctionName: "MkdirAll", Capability: "fs:write:*", Evidence: "os.MkdirAll call"},

	// os.Rename moves or renames a file or directory.
	{ImportPath: "os", FunctionName: "Rename", Capability: "fs:write:*", Evidence: "os.Rename call"},

	// ── Filesystem deletes (fs:delete:*) ──────────────────────────────────
	// os.Remove deletes a single file or empty directory.
	{ImportPath: "os", FunctionName: "Remove", Capability: "fs:delete:*", Evidence: "os.Remove call"},

	// os.RemoveAll deletes a path and everything under it. More dangerous
	// than Remove; tracked separately for clarity.
	{ImportPath: "os", FunctionName: "RemoveAll", Capability: "fs:delete:*", Evidence: "os.RemoveAll call"},

	// ── Filesystem listing (fs:list:*) ────────────────────────────────────
	// os.ReadDir lists the entries of a directory.
	{ImportPath: "os", FunctionName: "ReadDir", Capability: "fs:list:*", Evidence: "os.ReadDir call"},

	// os.Stat returns file metadata without opening it. Technically read-only
	// but it probes the filesystem, so we categorize it as fs:list:*.
	{ImportPath: "os", FunctionName: "Stat", Capability: "fs:list:*", Evidence: "os.Stat call"},

	// os.Lstat is like Stat but for symlinks.
	{ImportPath: "os", FunctionName: "Lstat", Capability: "fs:list:*", Evidence: "os.Lstat call"},

	// ── Environment variables (env:read:*) ────────────────────────────────
	// os.Getenv reads a single environment variable by name.
	{ImportPath: "os", FunctionName: "Getenv", Capability: "env:read:*", Evidence: "os.Getenv call"},

	// os.Environ returns all environment variables as a slice of "KEY=VALUE".
	{ImportPath: "os", FunctionName: "Environ", Capability: "env:read:*", Evidence: "os.Environ call"},

	// os.LookupEnv reads a variable and reports whether it was set.
	{ImportPath: "os", FunctionName: "LookupEnv", Capability: "env:read:*", Evidence: "os.LookupEnv call"},

	// ── Time (time:read:*) ────────────────────────────────────────────────
	// time.Now reads the current wall-clock time from the OS.
	{ImportPath: "time", FunctionName: "Now", Capability: "time:read:*", Evidence: "time.Now call"},

	// time.Sleep suspends the goroutine for a duration. It interacts with
	// the OS scheduler and thus requires time capability.
	{ImportPath: "time", FunctionName: "Sleep", Capability: "time:read:*", Evidence: "time.Sleep call"},

	// ── Standard output (stdout:write:*) ──────────────────────────────────
	// fmt.Println writes its arguments followed by a newline to os.Stdout.
	{ImportPath: "fmt", FunctionName: "Println", Capability: "stdout:write:*", Evidence: "fmt.Println call"},

	// fmt.Print writes its arguments to os.Stdout without a newline.
	{ImportPath: "fmt", FunctionName: "Print", Capability: "stdout:write:*", Evidence: "fmt.Print call"},

	// fmt.Printf writes a formatted string to os.Stdout.
	{ImportPath: "fmt", FunctionName: "Printf", Capability: "stdout:write:*", Evidence: "fmt.Printf call"},
}
