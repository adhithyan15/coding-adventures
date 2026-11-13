// Command kiln is the Kiln build system (see code/specs/KILN00-kiln-build-system.md).
//
// At this stage it only contains the canonical key encoder of KILN02 and a
// way to check it against the golden vectors:
//
//	kiln version                   print the version and key schema
//	kiln verify-vectors <path>     check every golden vector; non-zero on mismatch
//	kiln legacy-plan ...           compute legacy keys for every package (KILN03)
//	kiln explain ...               print one package's record from a plan
package main

import (
	"fmt"
	"io"
	"os"

	"github.com/adhithyan15/coding-adventures/code/programs/go/kiln/internal/key"
)

// Version is the program version.
const Version = "0.1.0"

func main() {
	os.Exit(run(os.Args[1:], os.Stdout, os.Stderr))
}

// run executes the CLI and returns the process exit code.
func run(args []string, stdout, stderr io.Writer) int {
	if len(args) == 0 {
		fmt.Fprintln(stderr, "usage: kiln <version|verify-vectors <path>|legacy-plan|explain>")
		return 2
	}
	switch args[0] {
	case "version":
		fmt.Fprintf(stdout, "kiln %s (key schema %d)\n", Version, key.SchemaVersion)
		return 0
	case "verify-vectors":
		if len(args) != 2 {
			fmt.Fprintln(stderr, "usage: kiln verify-vectors <path>")
			return 2
		}
		data, err := key.ReadVectorFile(args[1])
		if err != nil {
			fmt.Fprintf(stderr, "kiln: %v\n", err)
			return 1
		}
		vf, err := key.ParseVectors(data)
		if err != nil {
			fmt.Fprintf(stderr, "kiln: %v\n", err)
			return 1
		}
		if fails := vf.Verify(); len(fails) > 0 {
			for _, f := range fails {
				fmt.Fprintln(stderr, "FAIL", f)
			}
			return 1
		}
		fmt.Fprintf(stdout, "ok: %d valid, %d invalid vectors; %d digests, %d strong-consumed, %d refused entries\n",
			len(vf.Valid), len(vf.Invalid), len(vf.Digests), len(vf.StrongConsumed), len(vf.RefusedEntries))
		return 0
	case "legacy-plan":
		return runLegacyPlan(args[1:], stdout, stderr)
	case "explain":
		return runExplain(args[1:], stdout, stderr)
	}
	fmt.Fprintf(stderr, "kiln: unknown command %q\n", args[0])
	return 2
}
