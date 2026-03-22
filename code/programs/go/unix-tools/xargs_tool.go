// =========================================================================
// xargs — Build and Execute Command Lines from Standard Input
// =========================================================================
//
// The `xargs` utility reads items from standard input (delimited by
// whitespace or newlines by default), and executes a specified command
// with those items as arguments.
//
// # Why xargs exists
//
// Many Unix commands accept arguments on the command line, but not from
// stdin. xargs bridges this gap:
//
//   find . -name "*.txt" | xargs wc -l
//
// Without xargs, you'd need a loop. With xargs, the output of find
// becomes arguments to wc.
//
// # How it works
//
//   1. Read items from stdin (split by whitespace/newlines)
//   2. Group items into batches (controlled by -n)
//   3. For each batch, execute: COMMAND [initial-args] [items...]
//   4. Collect exit codes
//
// # Key flags
//
//   -0, --null        Input items are NUL-separated (for filenames with spaces)
//   -d, --delimiter   Custom delimiter character
//   -n, --max-args    Max items per command invocation
//   -I, --replace     Replace string (like {} in find -exec)
//   -t, --verbose     Print command before executing
//   -r, --no-run-if-empty  Don't run if stdin is empty
//   -P, --max-procs   Run commands in parallel
//
// # Architecture
//
//   xargs.json (spec)            xargs_tool.go (this file)
//   ┌──────────────────┐       ┌──────────────────────────────────┐
//   │ flags: -0,-d,-n  │       │ read items from stdin            │
//   │ -I,-t,-r,-P      │──────>│ batch items per -n               │
//   │ arg: COMMAND...   │       │ exec COMMAND with item args      │
//   └──────────────────┘       └──────────────────────────────────┘

package main

import (
	"bufio"
	"fmt"
	"io"
	"os"
	"os/exec"
	"strings"
	"sync"

	clibuilder "github.com/adhithyan15/coding-adventures/code/packages/go/cli-builder"
)

// =========================================================================
// XargsOptions — configuration for the xargs operation
// =========================================================================

type XargsOptions struct {
	NullDelimiter  bool   // -0: items separated by NUL
	Delimiter      string // -d: custom delimiter
	MaxArgs        int    // -n: max args per command invocation
	ReplaceStr     string // -I: replacement string
	Verbose        bool   // -t: print command before executing
	NoRunIfEmpty   bool   // -r: don't run if input is empty
	MaxProcs       int    // -P: max parallel processes
}

// =========================================================================
// xargsReadItems — read items from a reader using the specified delimiter
// =========================================================================
//
// Items are read and split according to the delimiter:
//   - Default: split on whitespace (spaces, tabs, newlines)
//   - -0: split on NUL characters
//   - -d X: split on character X
//
// This function returns a slice of non-empty items.

func xargsReadItems(reader io.Reader, opts XargsOptions) ([]string, error) {
	if opts.NullDelimiter {
		// Read everything and split on NUL.
		data, err := io.ReadAll(reader)
		if err != nil {
			return nil, err
		}
		parts := strings.Split(string(data), "\x00")
		var items []string
		for _, p := range parts {
			if p != "" {
				items = append(items, p)
			}
		}
		return items, nil
	}

	if opts.Delimiter != "" {
		// Custom delimiter.
		data, err := io.ReadAll(reader)
		if err != nil {
			return nil, err
		}
		parts := strings.Split(string(data), opts.Delimiter)
		var items []string
		for _, p := range parts {
			p = strings.TrimSpace(p)
			if p != "" {
				items = append(items, p)
			}
		}
		return items, nil
	}

	// Default: split on whitespace, line by line.
	scanner := bufio.NewScanner(reader)
	var items []string
	for scanner.Scan() {
		line := scanner.Text()
		fields := strings.Fields(line)
		items = append(items, fields...)
	}
	return items, scanner.Err()
}

// =========================================================================
// xargsBatchItems — split items into batches of at most maxArgs
// =========================================================================
//
// If maxArgs is 0 or negative, all items go in one batch.
//
// Example with maxArgs=2:
//   Input:  [a, b, c, d, e]
//   Output: [[a, b], [c, d], [e]]

func xargsBatchItems(items []string, maxArgs int) [][]string {
	if maxArgs <= 0 || len(items) == 0 {
		return [][]string{items}
	}

	var batches [][]string
	for i := 0; i < len(items); i += maxArgs {
		end := i + maxArgs
		if end > len(items) {
			end = len(items)
		}
		batches = append(batches, items[i:end])
	}
	return batches
}

// =========================================================================
// xargsExecFunc — function type for executing commands
// =========================================================================
//
// We use a function variable so tests can inject a mock executor
// instead of actually running system commands.

type xargsExecFunc func(name string, args []string, stdout, stderr io.Writer) int

// xargsRealExec executes a real system command.
func xargsRealExec(name string, args []string, stdout, stderr io.Writer) int {
	cmd := exec.Command(name, args...)
	cmd.Stdout = stdout
	cmd.Stderr = stderr
	err := cmd.Run()
	if err != nil {
		if exitErr, ok := err.(*exec.ExitError); ok {
			return exitErr.ExitCode()
		}
		return 1
	}
	return 0
}

// =========================================================================
// xargsExecute — execute commands with the given items
// =========================================================================
//
// This function handles:
//   1. Building command lines from items
//   2. Handling -I (replace string) mode
//   3. Batching with -n
//   4. Parallel execution with -P

func xargsExecute(cmdParts []string, items []string, opts XargsOptions,
	stdout, stderr io.Writer, execFn xargsExecFunc) int {

	// Determine the command name and initial args.
	cmdName := "/bin/echo"
	var initialArgs []string
	if len(cmdParts) > 0 {
		cmdName = cmdParts[0]
		initialArgs = cmdParts[1:]
	}

	exitCode := 0

	if opts.ReplaceStr != "" {
		// Replace mode: for each item, replace occurrences of the replace
		// string in the initial args, then execute.
		for _, item := range items {
			var args []string
			for _, arg := range initialArgs {
				args = append(args, strings.ReplaceAll(arg, opts.ReplaceStr, item))
			}
			if opts.Verbose {
				fmt.Fprintf(stderr, "%s %s\n", cmdName, strings.Join(args, " "))
			}
			rc := execFn(cmdName, args, stdout, stderr)
			if rc != 0 {
				exitCode = rc
			}
		}
		return exitCode
	}

	// Batch mode: split items into batches and execute each batch.
	batches := xargsBatchItems(items, opts.MaxArgs)

	if opts.MaxProcs > 1 {
		// Parallel execution.
		var mu sync.Mutex
		var wg sync.WaitGroup
		sem := make(chan struct{}, opts.MaxProcs)

		for _, batch := range batches {
			wg.Add(1)
			sem <- struct{}{}
			go func(b []string) {
				defer wg.Done()
				defer func() { <-sem }()

				args := append(append([]string{}, initialArgs...), b...)
				if opts.Verbose {
					mu.Lock()
					fmt.Fprintf(stderr, "%s %s\n", cmdName, strings.Join(args, " "))
					mu.Unlock()
				}
				rc := execFn(cmdName, args, stdout, stderr)
				if rc != 0 {
					mu.Lock()
					exitCode = rc
					mu.Unlock()
				}
			}(batch)
		}
		wg.Wait()
	} else {
		// Sequential execution.
		for _, batch := range batches {
			args := append(append([]string{}, initialArgs...), batch...)
			if opts.Verbose {
				fmt.Fprintf(stderr, "%s %s\n", cmdName, strings.Join(args, " "))
			}
			rc := execFn(cmdName, args, stdout, stderr)
			if rc != 0 {
				exitCode = rc
			}
		}
	}

	return exitCode
}

// =========================================================================
// runXargs — the testable core of the xargs tool
// =========================================================================

func runXargs(specPath string, argv []string, stdout io.Writer, stderr io.Writer) int {
	return runXargsWithStdin(specPath, argv, stdout, stderr, os.Stdin, xargsRealExec)
}

func runXargsWithStdin(specPath string, argv []string, stdout io.Writer, stderr io.Writer,
	stdin io.Reader, execFn xargsExecFunc) int {

	// Step 1: Create the parser from the spec.
	parser, err := clibuilder.NewParser(specPath, argv)
	if err != nil {
		fmt.Fprintf(stderr, "xargs: %s\n", err)
		return 1
	}

	// Step 2: Parse the arguments.
	result, err := parser.Parse()
	if err != nil {
		fmt.Fprintf(stderr, "%s\n", err)
		return 1
	}

	// Step 3: Handle the result.
	switch r := result.(type) {

	case *clibuilder.HelpResult:
		fmt.Fprintln(stdout, r.Text)
		return 0

	case *clibuilder.VersionResult:
		fmt.Fprintln(stdout, r.Version)
		return 0

	case *clibuilder.ParseResult:
		opts := XargsOptions{
			NullDelimiter: getBool(r.Flags, "null"),
			NoRunIfEmpty:  getBool(r.Flags, "no_run_if_empty"),
			Verbose:       getBool(r.Flags, "verbose"),
			MaxProcs:      1,
		}

		if d, ok := r.Flags["delimiter"].(string); ok {
			opts.Delimiter = d
		}
		if n, ok := getInt(r.Flags, "max_args"); ok {
			opts.MaxArgs = n
		}
		if s, ok := r.Flags["replace"].(string); ok {
			opts.ReplaceStr = s
		}
		if p, ok := getInt(r.Flags, "max_procs"); ok && p > 0 {
			opts.MaxProcs = p
		}

		// Determine input source.
		inputReader := stdin
		if argFile, ok := r.Flags["arg_file"].(string); ok && argFile != "" {
			f, err := os.Open(argFile)
			if err != nil {
				fmt.Fprintf(stderr, "xargs: %s: %s\n", argFile, err)
				return 1
			}
			defer f.Close()
			inputReader = f
		}

		// Read items from input.
		items, err := xargsReadItems(inputReader, opts)
		if err != nil {
			fmt.Fprintf(stderr, "xargs: %s\n", err)
			return 1
		}

		// If no items and -r is set, exit without running.
		if len(items) == 0 && opts.NoRunIfEmpty {
			return 0
		}

		// Get the command parts (COMMAND [ARGS...]).
		cmdParts := getStringSlice(r.Arguments, "command")

		return xargsExecute(cmdParts, items, opts, stdout, stderr, execFn)

	default:
		fmt.Fprintf(stderr, "xargs: unexpected result type: %T\n", result)
		return 1
	}
}
