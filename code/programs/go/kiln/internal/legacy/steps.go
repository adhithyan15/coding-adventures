package legacy

// The expected-steps export (kiln-legacy-steps/1), KILN04 sections 4.6 and 4.8
// ============================================================================
//
// The recorder runs every BUILD step behind a wrapper, and afterwards asks
// whether the trace shows the lines the package was SUPPOSED to run (the
// recipe check). The supposed lines are not the BUILD file's text: a Starlark
// BUILD has none until it is evaluated, and the executor rewrites a few lines
// (the Swift native-build-system pin, the Windows inline-environment prefix).
// Kiln is language-blind and cannot redo any of that, so the build tool writes
// the answer, `executor.EffectiveStepLines`, per package, in the same
// invocation and from the same discovery pass as the legacy graph:
//
//	{"schema":"kiln-legacy-steps/1","os":"linux",
//	 "packages":[{"package":"code/packages/go/x","steps":["go build ./..."]}]}
//
// This file is the Kiln-side reader. It is STRICT, because what it returns
// becomes `traced.Input.Recipe`, the yardstick a trace is held against:
//
//   - unknown keys, duplicate keys, keys spelled in another case and trailing
//     data are refused (policyDecode),
//   - null is not an empty list: a missing "steps" must not read as "this
//     package runs nothing",
//   - the packages are strictly sorted, so a repeat or a reordering is an
//     error rather than a silent "last one wins",
//   - a package path must be a valid Kiln path (the same rule as the graph),
//   - a line may not contain NUL (a C string would end there, so two different
//     lines could compare equal) and is size-capped.
//
// An empty "steps" list is legal: a package with no commands. Whether that is
// acceptable for a particular package is the caller's decision.

import (
	"fmt"
	"io"
	"os"
	"sort"
	"strings"

	"github.com/adhithyan15/coding-adventures/code/programs/go/kiln/internal/key"
)

// StepsSchema is the only steps-export schema this loader understands.
const StepsSchema = "kiln-legacy-steps/1"

// Limits that keep a hostile or broken export from exhausting memory.
const (
	MaxStepsBytes   = 64 << 20 // 64 MiB of JSON
	MaxStepsPerPkg  = 4096     // lines of one package
	MaxStepLineSize = 1 << 20  // bytes of one line
)

// RecordTmpdir is the TMPDIR the recorder derives keys under (KILN04 section
// 4.8, G13). The sandbox requires a tmpdir and hosted runners do not set one;
// the value is keyed, so stage 5b lookups must use the same one.
const RecordTmpdir = "/tmp/kiln-tmp"

// StepsPackage is one entry of the export.
type StepsPackage struct {
	Package string   `json:"package"`
	Steps   []string `json:"steps"`
}

// Steps is a validated export.
type Steps struct {
	Schema   string         `json:"schema"`
	OS       string         `json:"os"`
	Packages []StepsPackage `json:"packages"`
}

// ReadStepsFile loads and validates the export at path. The file must be a
// regular file no larger than MaxStepsBytes.
func ReadStepsFile(path string) (*Steps, error) {
	f, err := os.Open(path)
	if err != nil {
		return nil, err
	}
	defer f.Close()
	fi, err := f.Stat()
	if err != nil {
		return nil, err
	}
	if !fi.Mode().IsRegular() {
		return nil, fmt.Errorf("steps %s: not a regular file", path)
	}
	data, err := io.ReadAll(io.LimitReader(f, MaxStepsBytes+1))
	if err != nil {
		return nil, err
	}
	return DecodeSteps(data)
}

// DecodeSteps strictly decodes and validates an export.
func DecodeSteps(data []byte) (*Steps, error) {
	if len(data) > MaxStepsBytes {
		return nil, fmt.Errorf("steps: exceeds the %d byte limit", MaxStepsBytes)
	}
	var s Steps
	if err := policyDecode(data, &s); err != nil {
		return nil, fmt.Errorf("steps: %w", err)
	}
	if s.Schema != StepsSchema {
		return nil, fmt.Errorf("steps: schema %q, want %q", short(s.Schema), StepsSchema)
	}
	known := false
	for _, o := range OSes {
		known = known || o == s.OS
	}
	if !known {
		return nil, fmt.Errorf("steps: os %q is not one of %v", short(s.OS), OSes)
	}
	if s.Packages == nil {
		return nil, fmt.Errorf("steps: packages is null")
	}
	if len(s.Packages) > packageLimit {
		return nil, fmt.Errorf("steps: %d packages, limit %d", len(s.Packages), packageLimit)
	}
	for i, p := range s.Packages {
		if err := key.ValidatePath(p.Package); err != nil {
			return nil, fmt.Errorf("steps: package %q: %v", short(p.Package), err)
		}
		if i > 0 && s.Packages[i-1].Package >= p.Package {
			return nil, fmt.Errorf("steps: packages are not sorted and unique at %q", short(p.Package))
		}
		if p.Steps == nil {
			return nil, fmt.Errorf("steps: package %q: steps is null", short(p.Package))
		}
		if len(p.Steps) > MaxStepsPerPkg {
			return nil, fmt.Errorf("steps: package %q: %d steps, limit %d", short(p.Package), len(p.Steps), MaxStepsPerPkg)
		}
		for j, l := range p.Steps {
			if len(l) > MaxStepLineSize {
				return nil, fmt.Errorf("steps: package %q: step %d is %d bytes, limit %d", short(p.Package), j, len(l), MaxStepLineSize)
			}
			if strings.ContainsRune(l, 0) {
				return nil, fmt.Errorf("steps: package %q: step %d contains NUL", short(p.Package), j)
			}
		}
	}
	return &s, nil
}

// Lines returns the expected lines of the package at relPath (the graph's
// rel_path), and whether the export names it. The result is a copy.
func (s *Steps) Lines(relPath string) ([]string, bool) {
	i := sort.Search(len(s.Packages), func(i int) bool { return s.Packages[i].Package >= relPath })
	if i == len(s.Packages) || s.Packages[i].Package != relPath {
		return nil, false
	}
	return append([]string{}, s.Packages[i].Steps...), true
}
