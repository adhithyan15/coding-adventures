package clibuilder

// =========================================================================
// Result types
// =========================================================================
//
// Parser.Parse() returns one of three result types depending on what it
// encountered in argv:
//
//   - *ParseResult  — normal successful parse: flags, arguments, command path
//   - *HelpResult   — the user passed --help or -h; contains rendered help text
//   - *VersionResult — the user passed --version; contains the version string
//
// Returning an interface{} (any) lets the caller type-switch cleanly:
//
//	switch r := result.(type) {
//	case *ParseResult:   // normal operation
//	case *HelpResult:    // fmt.Println(r.Text); os.Exit(0)
//	case *VersionResult: // fmt.Println(r.Version); os.Exit(0)
//	}

// ParseResult is the normal successful result of parsing argv.
//
// All flags in scope are included in Flags — absent optional boolean flags
// are false, absent optional non-boolean flags are nil (or the declared
// `default` value if one was set). Variadic arguments produce []any slices.
//
// CommandPath always contains at least one element: the program name.
// For root-level invocations it is just ["program-name"]. For subcommands
// it is the full path: ["git", "remote", "add"].
type ParseResult struct {
	// Program is argv[0]: the program name as invoked.
	Program string

	// CommandPath is the full path from the root to the resolved command.
	// Example: ["git", "remote", "add"]
	CommandPath []string

	// Flags maps flag `id` to the parsed and type-coerced value.
	// All flags in scope are present; absent optional flags use their
	// default value (false for booleans, nil for others, 0 for count).
	Flags map[string]any

	// Arguments maps argument `id` to the parsed and type-coerced value.
	// Variadic arguments map to []any.
	Arguments map[string]any

	// ExplicitFlags lists the IDs of flags that were explicitly set by
	// the user in argv. This distinguishes "user typed --verbose" from
	// "verbose was filled with its default value".
	//
	// The slice preserves insertion order: the first flag consumed from
	// argv appears first. A flag ID may appear multiple times if the flag
	// is repeatable or is a count type encountered multiple times.
	//
	// Use case: a program might want to know whether --color was
	// explicitly passed or silently defaulted. ExplicitFlags makes
	// that distinction possible without comparing against defaults.
	ExplicitFlags []string
}

// HelpResult is returned when the user passes --help or -h.
//
// The library generates the help text for the deepest resolved command
// (so `git remote --help` shows help for the `remote` subcommand, not
// the root `git` help).
//
// The caller should print Text to stdout and exit 0.
type HelpResult struct {
	// Text is the fully rendered help message, ready to print.
	Text string

	// CommandPath is the command for which help was generated.
	// Useful for logging or testing.
	CommandPath []string
}

// VersionResult is returned when the user passes --version.
//
// The caller should print Version to stdout and exit 0.
type VersionResult struct {
	// Version is the version string from the spec's `version` field.
	Version string
}
