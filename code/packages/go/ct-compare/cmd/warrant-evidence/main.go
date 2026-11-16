// Command warrant-evidence runs the language-neutral Warrant corpus against this
// package (VER04i section 5.3, the Go lane; WD-103).
//
// It is the consumer registry's `runner_entry` for `go/ct-compare`. Go cannot
// import a library by name at run time, so this program is where the library is
// compiled in: it builds the registry of ADAPTERS that the shared runner,
// `go/warrant-corpus`, needs, fixes the four package-specific paths, and forwards
// the command. Run it from the package directory:
//
//	go run ./cmd/warrant-evidence run --out evidence/test-manifest.json   # regenerate
//	go run ./cmd/warrant-evidence check evidence/test-manifest.json       # CI drift check
//	go run ./cmd/warrant-evidence sabotage                                # prove it can fail
//	go run ./cmd/warrant-evidence validate-manifest evidence/test-manifest.json
//
// `run --out` regenerates the committed manifest from a real execution; `check`
// regenerates it in memory and fails if the committed file drifted; `sabotage`
// proves a deliberately broken export makes the runner print `not ok` and exit
// non-zero, once per negative control.
//
// # What an adapter may do
//
// An adapter translates VALUES and ERRORS across the boundary and nothing else:
// it holds no computed answer, no retry and no fallback. It lives inside this
// package, so the manifest's source_sha256 digests it together with the library.
// There are exactly two translations, both forced by the library's own Go API:
//
//   - CTSelectBytes takes its selection bit as an int (a Go bool cannot become an
//     integer without a branch on the secret, CT01), while the Warrant `choice`
//     is a Bool. The adapter turns true into 1 and false into 0.
//   - CTSelectBytes reports a length mismatch by panicking with a fixed message,
//     while the Warrant model returns Err(CT_SELECT_LENGTH_MISMATCH). The adapter
//     recovers exactly that message and returns the ErrLengthMismatch sentinel;
//     any other panic is re-raised, so the runner records it as an unexpected
//     error instead of swallowing it.
package main

import (
	"errors"
	"os"
	"path/filepath"

	ctcompare "github.com/adhithyan15/coding-adventures/code/packages/go/ct-compare"
	warrantcorpus "github.com/adhithyan15/coding-adventures/code/packages/go/warrant-corpus"
)

// ErrLengthMismatch is the sentinel `evidence.bindings` maps to the declared
// Warrant error CT_SELECT_LENGTH_MISMATCH.
var ErrLengthMismatch = errors.New("ctselectbytes: slices differ in length")

// lengthMismatchPanic is the exact message CTSelectBytes panics with for slices of
// different lengths. It is compared whole, so the other panic (a choice that is
// not 0 or 1) is not mistaken for it.
const lengthMismatchPanic = "ctselectbytes requires equal-length slices"

func bytesPair(args []any) ([]byte, []byte) { return args[0].([]byte), args[1].([]byte) }

func eq(args []any) (any, error) {
	left, right := bytesPair(args)
	return ctcompare.CTEq(left, right), nil
}

func eqFixed(args []any) (any, error) {
	left, right := bytesPair(args)
	return ctcompare.CTEqFixed(left, right), nil
}

func eqU64(args []any) (any, error) {
	return ctcompare.CTEqU64(args[0].(uint64), args[1].(uint64)), nil
}

func selectBytes(args []any) (result any, err error) {
	defer func() {
		if recovered := recover(); recovered != nil {
			if recovered == lengthMismatchPanic {
				result, err = nil, ErrLengthMismatch
				return
			}
			panic(recovered)
		}
	}()
	left, right := bytesPair(args)
	choice := 0
	if args[2].(bool) {
		choice = 1
	}
	return ctcompare.CTSelectBytes(left, right, choice), nil
}

// registry is the table of compiled-in exports. Impl is the real library
// function; the runner never calls it, it only asks the Go runtime where it is
// defined.
func registry() *warrantcorpus.Registry {
	return &warrantcorpus.Registry{
		Exports: map[string]warrantcorpus.Export{
			"CTEq":          {Call: eq, Impl: ctcompare.CTEq},
			"CTEqFixed":     {Call: eqFixed, Impl: ctcompare.CTEqFixed},
			"CTSelectBytes": {Call: selectBytes, Impl: ctcompare.CTSelectBytes},
			"CTEqU64":       {Call: eqU64, Impl: ctcompare.CTEqU64},
		},
		Errors: map[string]error{"ErrLengthMismatch": ErrLengthMismatch},
	}
}

// forward adds this package's paths to a command: argv is `<command> [options]`.
// packageDir is the package directory (the working directory when run as above).
//
// `sabotage` re-executes this very binary once per control, passing the full
// flag list the runner built, so a command that already names `--repo-root` is
// forwarded unchanged instead of having the paths added twice.
func forward(packageDir string, argv []string) []string {
	if len(argv) == 0 {
		return argv
	}
	for _, token := range argv {
		if token == "--repo-root" {
			return argv
		}
	}
	repoRoot := filepath.Join(packageDir, "..", "..", "..", "..")
	corpus := filepath.Join(repoRoot, "code", "specs", "fixtures", "ct-compare-warrant-v1")
	forwarded := []string{
		argv[0],
		"--repo-root", repoRoot,
		"--corpus", corpus,
		"--binding", filepath.Join(packageDir, "evidence.bindings"),
		"--package-dir", packageDir,
	}
	return append(forwarded, argv[1:]...)
}

func run(packageDir string, argv []string) int {
	return warrantcorpus.Main(forward(packageDir, argv), warrantcorpus.ProcessIO(), registry())
}

func main() {
	packageDir, err := os.Getwd()
	if err != nil {
		os.Stderr.WriteString("error: cannot find the working directory\n")
		os.Exit(2)
	}
	os.Exit(run(packageDir, os.Args[1:]))
}
