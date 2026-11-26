// microbenchmark-experiments retains measurements and the assumptions needed
// to interpret them. It measures elapsed wall time, never simulated CPU cycles.
package main

import (
	"crypto/sha256"
	"encoding/hex"
	"encoding/json"
	"flag"
	"fmt"
	"io"
	"os"
	"path/filepath"
	"regexp"
	"runtime"
	"runtime/debug"
	"time"

	mb "github.com/adhithyan15/coding-adventures/code/packages/go/microbenchmark"
)

type coreReport struct {
	mb.Report
	Workload   string   `json:"workload"`
	Invocation []string `json:"invocation"`
	Revision   string   `json:"revision"`
	Runtime    string   `json:"runtime"`
	OS         string   `json:"os"`
	Arch       string   `json:"arch"`
	Profile    string   `json:"profile"`
}
type caseRecord struct {
	ID              string      `json:"id"`
	Report          string      `json:"report"`
	SHA256          string      `json:"sha256"`
	Seed            int64       `json:"layout_and_schedule_seed"`
	Baseline        string      `json:"baseline"`
	Candidate       string      `json:"candidate"`
	RequestedBytes  int         `json:"requested_bytes_per_variant,omitempty"`
	ActualBytes     int         `json:"actual_data_bytes_per_variant,omitempty"`
	NodeBytes       int         `json:"node_bytes,omitempty"`
	Visits          int         `json:"visits_or_insertions_per_operation"`
	BaselineCounts  *workCounts `json:"baseline_growth_counts,omitempty"`
	CandidateCounts *workCounts `json:"candidate_growth_counts,omitempty"`
}
type manifest struct {
	Version          int          `json:"version"`
	PlanSHA256       string       `json:"plan_sha256"`
	SourceRevision   string       `json:"source_revision_supplied"`
	ExecutableSHA256 string       `json:"executable_sha256"`
	BuildInfo        string       `json:"build_info"`
	Runtime          string       `json:"runtime"`
	OS               string       `json:"os"`
	Arch             string       `json:"arch"`
	LogicalCPUs      int          `json:"logical_cpus"`
	Started          string       `json:"started_utc"`
	Finished         string       `json:"finished_utc"`
	Plan             Plan         `json:"plan"`
	Notes            []string     `json:"notes"`
	Cases            []caseRecord `json:"cases"`
}

func digest(data []byte) string { sum := sha256.Sum256(data); return hex.EncodeToString(sum[:]) }
func fileDigest(path string) (string, error) {
	f, err := os.Open(path)
	if err != nil {
		return "", err
	}
	defer f.Close()
	h := sha256.New()
	if _, err := io.Copy(h, f); err != nil {
		return "", err
	}
	return hex.EncodeToString(h.Sum(nil)), nil
}
func writeNew(path string, data []byte) error {
	f, err := os.OpenFile(path, os.O_WRONLY|os.O_CREATE|os.O_EXCL, 0600)
	if err != nil {
		return err
	}
	return writeClosed(f, data)
}

func writeClosed(f io.WriteCloser, data []byte) error {
	n, writeErr := f.Write(data)
	closeErr := f.Close()
	if writeErr == nil && n != len(data) {
		writeErr = io.ErrShortWrite
	}
	if writeErr != nil {
		return writeErr
	}
	return closeErr
}

// Presence means completion, so never expose this name while writing. A hard
// link publishes the fully closed file atomically and refuses an existing name.
func publishManifest(directory string, data []byte) error {
	f, err := os.CreateTemp(directory, ".manifest-*.tmp")
	if err != nil {
		return err
	}
	temporary := f.Name()
	defer os.Remove(temporary)
	if err := writeClosed(f, data); err != nil {
		return err
	}
	return os.Link(temporary, filepath.Join(directory, "manifest.json"))
}

func execute(planPath, out, revision string, now mb.Clock, stdout io.Writer) error {
	if out == "" || !regexp.MustCompile(`^[0-9a-f]{40}$`).MatchString(revision) {
		return fmt.Errorf("new output directory and full lowercase source revision are required")
	}
	f, err := os.Open(planPath)
	if err != nil {
		return err
	}
	raw, readErr := io.ReadAll(io.LimitReader(f, 65537))
	closeErr := f.Close()
	if readErr != nil {
		return readErr
	}
	if closeErr != nil {
		return closeErr
	}
	p, err := parsePlan(raw)
	if err != nil {
		return err
	}
	exe, err := os.Executable()
	if err != nil {
		return err
	}
	binaryHash, err := fileDigest(exe)
	if err != nil {
		return err
	}
	if err := os.Mkdir(out, 0700); err != nil {
		return err
	}
	build := "unavailable"
	if info, ok := debug.ReadBuildInfo(); ok {
		build = info.String()
	}
	m := manifest{Version: 1, PlanSHA256: digest(raw), SourceRevision: revision, ExecutableSHA256: binaryHash, BuildInfo: build, Runtime: runtime.Version(), OS: runtime.GOOS, Arch: runtime.GOARCH, LogicalCPUs: runtime.NumCPU(), Started: time.Now().UTC().Format(time.RFC3339Nano), Plan: p, Notes: []string{
		"Warm adjacent paired wall-clock measurements; no cold-cache protocol or simulated cycles.",
		"Working-set bytes exclude slice headers, allocator overhead, permutation setup and garbage collector metadata; link cases retain two arenas.",
		"Allocation and cycle construction/validation are outside traversal timing; growth timing includes allocation, insertion, copy and final summation.",
		"Growth counts are deterministic software counts from an untimed policy simulation, not measured hardware events.",
		"Source revision is supplied by the reproducer; executable and plan digests identify the actual inputs.",
		"Affinity and frequency are uncontrolled; concurrent work, prefetch, caches and TLB can affect results. No PMU events measured.",
		"Sizes/seeds are predeclared exploratory comparisons; no family-wise confidence or performance threshold is claimed.",
	}}
	for _, seed := range p.Seeds {
		for _, kind := range []string{"indexed", "links", "growth"} {
			sizes := p.MemoryBytes
			if kind == "growth" {
				sizes = p.GrowthLengths
			}
			for _, size := range sizes {
				c, err := prepare(kind, size, seed)
				if err != nil {
					return err
				}
				runtime.GC()
				config := p.Config
				config.Seed = seed
				report, err := mb.Collect(mb.CollectOptions{Config: config, ExpectedChecksum: c.Expected, Baseline: c.Baseline, Candidate: c.Candidate, Now: now})
				if err != nil {
					return fmt.Errorf("%s: %w", c.ID, err)
				}
				core := coreReport{report, c.Workload, os.Args, revision, runtime.Version(), runtime.GOOS, runtime.GOARCH, "native-go"}
				data, err := json.MarshalIndent(core, "", "  ")
				if err != nil {
					return err
				}
				data = append(data, '\n')
				name := c.ID + ".json"
				if err := writeNew(filepath.Join(out, name), data); err != nil {
					return err
				}
				m.Cases = append(m.Cases, caseRecord{c.ID, name, digest(data), seed, c.BaselineLabel, c.CandidateLabel, c.RequestedBytes, c.ActualBytes, c.NodeBytes, c.Visits, c.BaselineCounts, c.CandidateCounts})
			}
		}
	}
	m.Finished = time.Now().UTC().Format(time.RFC3339Nano)
	data, err := json.MarshalIndent(m, "", "  ")
	if err != nil {
		return err
	}
	data = append(data, '\n')
	if err := publishManifest(out, data); err != nil {
		return err
	}
	message := fmt.Sprintf("Wrote %d paired reports and manifest.json to %s\n", len(m.Cases), out)
	n, err := io.WriteString(stdout, message)
	if err == nil && n != len(message) {
		return io.ErrShortWrite
	}
	return err
}

func cli(args []string, stdout, stderr io.Writer) int {
	f := flag.NewFlagSet("microbenchmark-experiments", flag.ContinueOnError)
	f.SetOutput(stderr)
	plan := f.String("plan", "", "predeclared JSON plan")
	out := f.String("out", "", "new output directory")
	revision := f.String("revision", "", "full source commit supplied by reproducer")
	if err := f.Parse(args); err != nil {
		if err == flag.ErrHelp {
			return 0
		}
		return 2
	}
	if f.NArg() != 0 {
		fmt.Fprintln(stderr, "unexpected positional arguments")
		return 2
	}
	if err := execute(*plan, *out, *revision, mb.MonotonicClock(), stdout); err != nil {
		fmt.Fprintln(stderr, err)
		return 1
	}
	return 0
}
func main() { os.Exit(cli(os.Args[1:], os.Stdout, os.Stderr)) }
