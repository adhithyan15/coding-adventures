// =========================================================================
// chmod — Change File Mode Bits
// =========================================================================
//
// The `chmod` utility changes the file permission bits of files and
// directories. File permissions are one of Unix's core security
// mechanisms, controlling who can read, write, or execute a file.
//
// # How Unix permissions work
//
// Every file has three sets of permissions, one for each class of user:
//
//   Class    Letter   Who
//   ───────  ──────   ─────────────────────────
//   User     u        The file's owner
//   Group    g        Members of the file's group
//   Other    o        Everyone else
//   All      a        All three classes (u+g+o)
//
// Each class has three permission bits:
//
//   Bit      Letter   Octal   Meaning
//   ───────  ──────   ─────   ────────────────────
//   Read     r        4       Can read file contents
//   Write    w        2       Can modify file contents
//   Execute  x        1       Can run as a program
//
// # Specifying permissions
//
// Two notations are supported:
//
//   Octal:    chmod 755 file    → rwxr-xr-x
//   Symbolic: chmod u+rwx,go+rx file
//
// Symbolic notation is more flexible:
//   chmod u+x file      Add execute for owner
//   chmod go-w file      Remove write for group and others
//   chmod a=r file       Set read-only for everyone
//
// # Architecture
//
//   chmod.json (spec)            chmod_tool.go (this file)
//   ┌──────────────────┐       ┌──────────────────────────────────┐
//   │ flags: -R,-v,-c  │       │ parse mode (octal or symbolic)   │
//   │ -f,--reference   │──────>│ for each file:                   │
//   │ args: MODE FILES │       │   get current mode               │
//   │                  │       │   compute new mode               │
//   └──────────────────┘       │   os.Chmod(file, newMode)        │
//                              └──────────────────────────────────┘

package main

import (
	"fmt"
	"io"
	"os"
	"path/filepath"
	"strconv"
	"strings"

	clibuilder "github.com/adhithyan15/coding-adventures/code/packages/go/cli-builder"
)

// =========================================================================
// ChmodOptions — configuration for the chmod operation
// =========================================================================

type ChmodOptions struct {
	Recursive bool // -R: change files and directories recursively
	Verbose   bool // -v: output a diagnostic for every file processed
	Changes   bool // -c: like verbose but report only when a change is made
	Silent    bool // -f: suppress most error messages
}

// =========================================================================
// parseChmodOctal — parse an octal mode string like "755"
// =========================================================================
//
// Octal notation represents permissions as a 3 or 4-digit number.
// Each digit is the sum of permission bits:
//
//   Digit   Permissions
//   ─────   ───────────
//   7       rwx (4+2+1)
//   6       rw- (4+2)
//   5       r-x (4+1)
//   4       r-- (4)
//   3       -wx (2+1)
//   2       -w- (2)
//   1       --x (1)
//   0       --- (0)
//
// Returns the file mode and true if valid, or 0 and false otherwise.

func parseChmodOctal(mode string) (os.FileMode, bool) {
	// Must be 3 or 4 digits, all in range 0-7.
	if len(mode) < 3 || len(mode) > 4 {
		return 0, false
	}
	for _, c := range mode {
		if c < '0' || c > '7' {
			return 0, false
		}
	}
	val, err := strconv.ParseUint(mode, 8, 32)
	if err != nil {
		return 0, false
	}
	return os.FileMode(val), true
}

// =========================================================================
// parseChmodSymbolic — parse a symbolic mode string like "u+rwx,go+rx"
// =========================================================================
//
// Symbolic mode syntax: [ugoa...][[-+=][perms...]...],...
//
// The grammar:
//   clause    = who op perms
//   who       = [ugoa]* (default: a, if omask applied)
//   op        = + | - | =
//   perms     = [rwxXst]*
//
// Multiple clauses are separated by commas:
//   u+rwx,go+rx
//
// This function takes the current mode and returns the new mode.

func parseChmodSymbolic(spec string, current os.FileMode) (os.FileMode, error) {
	mode := current

	// Split on commas for multiple clauses.
	clauses := strings.Split(spec, ",")

	for _, clause := range clauses {
		if clause == "" {
			continue
		}

		// Parse the "who" part (u, g, o, a).
		i := 0
		var who []byte
		for i < len(clause) && (clause[i] == 'u' || clause[i] == 'g' ||
			clause[i] == 'o' || clause[i] == 'a') {
			who = append(who, clause[i])
			i++
		}

		// Default to 'a' if no who specified.
		if len(who) == 0 {
			who = []byte{'a'}
		}

		// Parse operator and permissions (may repeat: u+rw-x).
		for i < len(clause) {
			if i >= len(clause) {
				break
			}

			op := clause[i]
			if op != '+' && op != '-' && op != '=' {
				return 0, fmt.Errorf("invalid mode: %s", spec)
			}
			i++

			// Parse permission characters.
			var perms []byte
			for i < len(clause) && (clause[i] == 'r' || clause[i] == 'w' ||
				clause[i] == 'x' || clause[i] == 'X' || clause[i] == 's' ||
				clause[i] == 't') {
				perms = append(perms, clause[i])
				i++
			}

			// Apply the operation.
			mode = applyChmodSymbolicOp(mode, who, op, perms)
		}
	}

	return mode, nil
}

// =========================================================================
// applyChmodSymbolicOp — apply a single symbolic permission operation
// =========================================================================
//
// Given the current mode, who chars, operator, and permission chars,
// compute the new mode.
//
// The mapping from symbolic to octal bits:
//
//   Who   Bit shift   r      w      x
//   ────  ─────────   ────   ────   ────
//   u     6           0400   0200   0100
//   g     3           0040   0020   0010
//   o     0           0004   0002   0001

func applyChmodSymbolicOp(mode os.FileMode, who []byte, op byte, perms []byte) os.FileMode {
	// Build the permission mask.
	var mask os.FileMode

	// Determine which classes are affected.
	affectsUser := false
	affectsGroup := false
	affectsOther := false

	for _, w := range who {
		switch w {
		case 'u':
			affectsUser = true
		case 'g':
			affectsGroup = true
		case 'o':
			affectsOther = true
		case 'a':
			affectsUser = true
			affectsGroup = true
			affectsOther = true
		}
	}

	// Build the bit mask from permission characters.
	for _, p := range perms {
		switch p {
		case 'r':
			if affectsUser {
				mask |= 0400
			}
			if affectsGroup {
				mask |= 0040
			}
			if affectsOther {
				mask |= 0004
			}
		case 'w':
			if affectsUser {
				mask |= 0200
			}
			if affectsGroup {
				mask |= 0020
			}
			if affectsOther {
				mask |= 0002
			}
		case 'x':
			if affectsUser {
				mask |= 0100
			}
			if affectsGroup {
				mask |= 0010
			}
			if affectsOther {
				mask |= 0001
			}
		case 'X':
			// X sets execute only if the file is a directory or already
			// has execute permission for some user.
			if mode&0111 != 0 || mode.IsDir() {
				if affectsUser {
					mask |= 0100
				}
				if affectsGroup {
					mask |= 0010
				}
				if affectsOther {
					mask |= 0001
				}
			}
		case 's':
			// Set-user-ID and set-group-ID bits.
			if affectsUser {
				mask |= os.ModeSetuid
			}
			if affectsGroup {
				mask |= os.ModeSetgid
			}
		case 't':
			// Sticky bit.
			mask |= os.ModeSticky
		}
	}

	// Apply the operation.
	switch op {
	case '+':
		mode |= mask
	case '-':
		mode &^= mask
	case '=':
		// Clear all bits for the affected classes, then set the new ones.
		var clearMask os.FileMode
		if affectsUser {
			clearMask |= 0700
		}
		if affectsGroup {
			clearMask |= 0070
		}
		if affectsOther {
			clearMask |= 0007
		}
		mode = (mode &^ clearMask) | mask
	}

	return mode
}

// =========================================================================
// chmodApplyMode — compute the new mode for a file
// =========================================================================
//
// This function takes a mode specification (octal or symbolic) and the
// file's current mode, and returns the new mode to apply.

func chmodApplyMode(modeSpec string, currentMode os.FileMode) (os.FileMode, error) {
	// Try octal first.
	if newMode, ok := parseChmodOctal(modeSpec); ok {
		return newMode, nil
	}

	// Try symbolic.
	return parseChmodSymbolic(modeSpec, currentMode)
}

// =========================================================================
// chmodFile — change the mode of a single file
// =========================================================================

func chmodFile(path, modeSpec string, opts ChmodOptions, stdout, stderr io.Writer) int {
	info, err := os.Lstat(path)
	if err != nil {
		if !opts.Silent {
			fmt.Fprintf(stderr, "chmod: cannot access '%s': %s\n", path, err)
		}
		return 1
	}

	oldMode := info.Mode().Perm()
	newMode, err := chmodApplyMode(modeSpec, info.Mode())
	if err != nil {
		if !opts.Silent {
			fmt.Fprintf(stderr, "chmod: invalid mode: '%s'\n", modeSpec)
		}
		return 1
	}

	err = os.Chmod(path, newMode)
	if err != nil {
		if !opts.Silent {
			fmt.Fprintf(stderr, "chmod: changing permissions of '%s': %s\n", path, err)
		}
		return 1
	}

	// Report changes based on verbosity flags.
	if opts.Verbose {
		fmt.Fprintf(stdout, "mode of '%s' changed from %04o to %04o\n",
			path, uint32(oldMode), uint32(newMode.Perm()))
	} else if opts.Changes && oldMode != newMode.Perm() {
		fmt.Fprintf(stdout, "mode of '%s' changed from %04o to %04o\n",
			path, uint32(oldMode), uint32(newMode.Perm()))
	}

	return 0
}

// =========================================================================
// chmodRecursive — recursively change permissions
// =========================================================================

func chmodRecursive(path, modeSpec string, opts ChmodOptions, stdout, stderr io.Writer) int {
	exitCode := 0

	err := filepath.Walk(path, func(p string, info os.FileInfo, err error) error {
		if err != nil {
			if !opts.Silent {
				fmt.Fprintf(stderr, "chmod: cannot access '%s': %s\n", p, err)
			}
			exitCode = 1
			return nil
		}

		rc := chmodFile(p, modeSpec, opts, stdout, stderr)
		if rc != 0 {
			exitCode = rc
		}
		return nil
	})

	if err != nil && !opts.Silent {
		fmt.Fprintf(stderr, "chmod: %s\n", err)
		return 1
	}

	return exitCode
}

// =========================================================================
// runChmod — the testable core of the chmod tool
// =========================================================================

func runChmod(specPath string, argv []string, stdout io.Writer, stderr io.Writer) int {
	// Step 1: Create the parser from the spec.
	parser, err := clibuilder.NewParser(specPath, argv)
	if err != nil {
		fmt.Fprintf(stderr, "chmod: %s\n", err)
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
		opts := ChmodOptions{
			Recursive: getBool(r.Flags, "recursive"),
			Verbose:   getBool(r.Flags, "verbose"),
			Changes:   getBool(r.Flags, "changes"),
			Silent:    getBool(r.Flags, "silent"),
		}

		// If --reference is specified, get mode from reference file.
		var modeSpec string
		if refFile, ok := r.Flags["reference"].(string); ok && refFile != "" {
			refInfo, err := os.Stat(refFile)
			if err != nil {
				fmt.Fprintf(stderr, "chmod: cannot stat reference file '%s': %s\n", refFile, err)
				return 1
			}
			modeSpec = fmt.Sprintf("%o", refInfo.Mode().Perm())
		} else {
			modeSpec, _ = r.Arguments["mode"].(string)
		}

		if modeSpec == "" {
			fmt.Fprintf(stderr, "chmod: missing operand\n")
			return 1
		}

		// Get file list.
		files := getStringSlice(r.Arguments, "files")
		if len(files) == 0 {
			fmt.Fprintf(stderr, "chmod: missing operand after '%s'\n", modeSpec)
			return 1
		}

		exitCode := 0
		for _, file := range files {
			var rc int
			if opts.Recursive {
				rc = chmodRecursive(file, modeSpec, opts, stdout, stderr)
			} else {
				rc = chmodFile(file, modeSpec, opts, stdout, stderr)
			}
			if rc != 0 {
				exitCode = rc
			}
		}

		return exitCode

	default:
		fmt.Fprintf(stderr, "chmod: unexpected result type: %T\n", result)
		return 1
	}
}
