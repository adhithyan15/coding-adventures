package record

// The build tool's results (`-emit-results`, KILN04 section 4.6, 4.8)
// ===================================================================
//
// The build tool writes, after the run and even when a package failed,
//
//	{"schema":"kiln-legacy-results/1","packages":[
//	  {"package":"code/packages/go/demo","status":"built","steps_run":3,"steps_total":3}]}
//
// `collect` reads it for exactly two facts about each action that the traces
// cannot give: the executor's own STATUS (`built`, `failed`, `dep-skipped`,
// ...), and how many steps the executor believes it STARTED. The second is a
// cross-check, not a source: the number of sidecars the supervisor wrote must
// agree with it, because an action whose step the executor launched but the
// supervisor never accounted for (a refused wrapper, a supervisor killed
// mid-step) has a write set nobody can vouch for ("the traced step count
// differs from kiln-legacy-results", section 7, unknown write sets).
//
// The file is the build tool's own output, written OUTSIDE the sandbox, but it
// is still read like everything else `collect` reads: through a non-following
// handle, a regular file only, bounded, and decoded strictly (no unknown key,
// no duplicate or case-variant key, no trailing data, packages strictly
// sorted so a repeat cannot hide one). A results file that is wrong in any way
// means `collect` cannot say what the build did, so it refuses and writes
// nothing; there is no "partial" reading of it.

import (
	"fmt"
	"io"

	"github.com/adhithyan15/coding-adventures/code/programs/go/kiln/internal/legacy"
	"github.com/adhithyan15/coding-adventures/code/programs/go/kiln/internal/recfmt"
)

// ResultsSchema is the only results schema `collect` understands.
const ResultsSchema = "kiln-legacy-results/1"

// MaxResultsBytes bounds the results file. A real one is a few hundred bytes
// per package; 16 MiB is generous for tens of thousands of packages.
const MaxResultsBytes = 16 << 20

// ResultsPackage is one package of the results.
type ResultsPackage struct {
	Package    string `json:"package"`
	Status     string `json:"status"`
	StepsRun   int    `json:"steps_run"`
	StepsTotal int    `json:"steps_total"`
}

// Results is a validated results document.
type Results struct {
	Schema   string           `json:"schema"`
	Packages []ResultsPackage `json:"packages"`
}

// ReadResults reads and decodes the results file at path (absolute): a
// non-following open, a regular file, at most MaxResultsBytes.
func ReadResults(path string) (*Results, error) {
	f, err := openNoFollow(path)
	if err != nil {
		return nil, fmt.Errorf("--results: %v", err)
	}
	defer f.Close()
	st, err := f.Stat()
	if err != nil {
		return nil, fmt.Errorf("--results: %v", err)
	}
	if !st.Mode().IsRegular() {
		return nil, fmt.Errorf("--results is not a regular file")
	}
	if st.Size() > MaxResultsBytes {
		return nil, fmt.Errorf("--results is %d bytes, over %d", st.Size(), MaxResultsBytes)
	}
	data, err := io.ReadAll(io.LimitReader(f, MaxResultsBytes+1))
	if err != nil {
		return nil, fmt.Errorf("--results: %v", err)
	}
	if len(data) > MaxResultsBytes {
		return nil, fmt.Errorf("--results grew past %d bytes while it was read", MaxResultsBytes)
	}
	return DecodeResults(data)
}

// DecodeResults decodes a results document strictly.
func DecodeResults(data []byte) (*Results, error) {
	var r Results
	if err := legacy.PolicyDecode(data, &r); err != nil {
		return nil, fmt.Errorf("results: %w", err)
	}
	if r.Schema != ResultsSchema {
		return nil, fmt.Errorf("results: schema %q, want %q", printable(r.Schema), ResultsSchema)
	}
	if r.Packages == nil {
		return nil, fmt.Errorf("results: packages is null")
	}
	for i, p := range r.Packages {
		if err := recfmt.CheckPath(p.Package); err != nil {
			return nil, fmt.Errorf("results: package %q: %v", printable(p.Package), err)
		}
		if i > 0 && r.Packages[i-1].Package >= p.Package {
			return nil, fmt.Errorf("results: packages are not strictly sorted at %q", printable(p.Package))
		}
		// The status becomes a shard record's `status`, which the format holds
		// to "a short lowercase word" (recfmt statusOK). A status that is not
		// one would make the record unencodable, so it is refused here, where
		// the cause can be named.
		if !statusWord(p.Status) {
			return nil, fmt.Errorf("results: package %q has the status %q, which is not a lowercase word", printable(p.Package), printable(p.Status))
		}
		if p.StepsRun < 0 || p.StepsTotal < 0 || p.StepsRun > p.StepsTotal {
			return nil, fmt.Errorf("results: package %q ran %d of %d steps", printable(p.Package), p.StepsRun, p.StepsTotal)
		}
	}
	return &r, nil
}

// statusWord is recfmt's rule for a status: lowercase letters and '-', 1 to 32
// bytes.
func statusWord(s string) bool {
	if s == "" || len(s) > 32 {
		return false
	}
	for i := 0; i < len(s); i++ {
		if c := s[i]; !(c >= 'a' && c <= 'z' || c == '-') {
			return false
		}
	}
	return true
}

// ByPackage indexes the document by package path.
func (r *Results) ByPackage() map[string]ResultsPackage {
	m := make(map[string]ResultsPackage, len(r.Packages))
	for _, p := range r.Packages {
		m[p.Package] = p
	}
	return m
}
