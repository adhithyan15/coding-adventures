// benchmark-tool is the repository's language-neutral benchmark runner.
//
// The first slices deliberately keep the hot path simple:
//   - read the benchmark manifest format from code/specs/benchmarking-tools.md
//   - validate ambiguous benchmark definitions before they become folklore
//   - run command-style subjects with warmup and measured trials
//   - start local TCP service subjects and drive RESP-framed workloads
//   - write raw samples, trial rows, environment metadata, summaries, and a
//     Markdown report into one self-contained result directory
package main

import (
	"bufio"
	"bytes"
	"context"
	"crypto/sha256"
	"encoding/json"
	"errors"
	"fmt"
	"io"
	"math"
	"math/rand"
	"net"
	"os"
	"os/exec"
	"path/filepath"
	"runtime"
	"sort"
	"strconv"
	"strings"
	"sync"
	"time"

	_ "embed"

	clibuilder "github.com/adhithyan15/coding-adventures/code/packages/go/cli-builder"
)

const version = "0.5.0"

const (
	defaultTCPTimeout          = 10 * time.Second
	serviceReadyTimeout        = 10 * time.Second
	serviceReadyProbeTimeout   = 100 * time.Millisecond
	maxTCPConnections          = 10000
	maxTCPConcurrency          = 10000
	maxTCPRequestsPerConn      = 10000
	maxTCPFrameBytes           = 16 * 1024 * 1024
	maxTCPPayloadBytes         = 16 * 1024 * 1024
	respDriver                 = "tcp-resp"
	respReadMode               = "resp-frame"
	respModeOneShot            = "one-shot"
	respModePreconnectThenFire = "preconnect-then-fire"
	respModePipeline           = "pipeline"
	respModeIdle               = "idle"
	respModeHold               = "hold"
	maxTCPHoldMS               = 10 * 60 * 1000
)

//go:embed benchmark-tool.json
var cliSpec []byte

type Manifest struct {
	Name        string     `json:"name"`
	Description string     `json:"description,omitempty"`
	Defaults    Defaults   `json:"defaults"`
	Subjects    []Subject  `json:"subjects"`
	Workloads   []Workload `json:"workloads"`
}

type Defaults struct {
	WarmupTrials          int  `json:"warmup_trials"`
	MeasurementTrials     int  `json:"measurement_trials"`
	CooldownMS            int  `json:"cooldown_ms"`
	RandomizeSubjectOrder bool `json:"randomize_subject_order"`
	FailFast              bool `json:"fail_fast"`
}

type Subject struct {
	Name             string `json:"name"`
	Kind             string `json:"kind"`
	Checkout         string `json:"checkout,omitempty"`
	WorkingDirectory string `json:"working_directory,omitempty"`
	Build            string `json:"build,omitempty"`
	Command          string `json:"command"`
	ReadyCheck       string `json:"ready_check,omitempty"`
	Prebuilt         bool   `json:"prebuilt,omitempty"`
}

type Workload struct {
	Name                        string `json:"name"`
	Driver                      string `json:"driver"`
	Mode                        string `json:"mode,omitempty"`
	Command                     string `json:"command,omitempty"`
	ReadMode                    string `json:"read_mode,omitempty"`
	Request                     string `json:"request,omitempty"`
	Expect                      string `json:"expect,omitempty"`
	ProtocolClosesAfterResponse bool   `json:"protocol_closes_after_response,omitempty"`
	Connections                 int    `json:"connections,omitempty"`
	Concurrency                 int    `json:"concurrency,omitempty"`
	RequestsPerConnection       int    `json:"requests_per_connection,omitempty"`
	Operations                  int    `json:"operations,omitempty"`
	TimeoutMS                   int    `json:"timeout_ms,omitempty"`
	HoldMS                      int    `json:"hold_ms,omitempty"`
}

type Environment struct {
	GeneratedAt      string            `json:"generated_at"`
	Hostname         string            `json:"hostname"`
	OS               string            `json:"os"`
	Arch             string            `json:"arch"`
	CPUs             int               `json:"cpus"`
	WorkingDirectory string            `json:"working_directory"`
	GitCommit        string            `json:"git_commit,omitempty"`
	GitBranch        string            `json:"git_branch,omitempty"`
	GitDirty         bool              `json:"git_dirty"`
	GitRemote        string            `json:"git_remote,omitempty"`
	GoVersion        string            `json:"go_version,omitempty"`
	Kernel           string            `json:"kernel,omitempty"`
	ULimitOpenFiles  string            `json:"ulimit_open_files,omitempty"`
	Extra            map[string]string `json:"extra,omitempty"`
}

type Sample struct {
	SampleKind string             `json:"sample_kind"`
	Subject    string             `json:"subject"`
	Workload   string             `json:"workload"`
	Trial      int                `json:"trial"`
	Phase      string             `json:"phase"`
	OK         bool               `json:"ok"`
	Metrics    map[string]float64 `json:"metrics"`
	Error      string             `json:"error,omitempty"`
}

type Trial struct {
	Subject  string             `json:"subject"`
	Workload string             `json:"workload"`
	Trial    int                `json:"trial"`
	Phase    string             `json:"phase"`
	OK       bool               `json:"ok"`
	Metrics  map[string]float64 `json:"metrics"`
	Error    string             `json:"error,omitempty"`
}

type Summary struct {
	Subject      string  `json:"subject"`
	Workload     string  `json:"workload"`
	Metric       string  `json:"metric"`
	Count        int     `json:"count"`
	Min          float64 `json:"min"`
	Max          float64 `json:"max"`
	Mean         float64 `json:"mean"`
	Median       float64 `json:"median"`
	Stddev       float64 `json:"stddev"`
	MAD          float64 `json:"mad"`
	P50          float64 `json:"p50"`
	P90          float64 `json:"p90"`
	P95          float64 `json:"p95"`
	P99          float64 `json:"p99"`
	MeanCILow    float64 `json:"mean_ci_low"`
	MeanCIHigh   float64 `json:"mean_ci_high"`
	MedianCILow  float64 `json:"median_ci_low"`
	MedianCIHigh float64 `json:"median_ci_high"`
}

type SummaryFile struct {
	ManifestName string    `json:"manifest_name"`
	GeneratedAt  string    `json:"generated_at"`
	Summaries    []Summary `json:"summaries"`
}

type ComparisonFile struct {
	GeneratedAt  string       `json:"generated_at"`
	BaseDir      string       `json:"base_dir"`
	CandidateDir string       `json:"candidate_dir"`
	Metric       string       `json:"metric"`
	Comparisons  []Comparison `json:"comparisons"`
}

type Comparison struct {
	Subject                   string  `json:"subject"`
	Workload                  string  `json:"workload"`
	Metric                    string  `json:"metric"`
	Direction                 string  `json:"direction"`
	BaseCount                 int     `json:"base_count"`
	CandidateCount            int     `json:"candidate_count"`
	BaseMedian                float64 `json:"base_median"`
	CandidateMedian           float64 `json:"candidate_median"`
	AbsoluteDifference        float64 `json:"absolute_difference"`
	RelativeDifferencePercent float64 `json:"relative_difference_percent"`
	RelativeCILowPercent      float64 `json:"relative_ci_low_percent"`
	RelativeCIHighPercent     float64 `json:"relative_ci_high_percent"`
	CliffsDelta               float64 `json:"cliffs_delta"`
	PracticalThresholdPercent float64 `json:"practical_threshold_percent"`
	BaseCorrect               bool    `json:"base_correct"`
	CandidateCorrect          bool    `json:"candidate_correct"`
	Verdict                   string  `json:"verdict"`
	Reason                    string  `json:"reason"`
}

type RunOptions struct {
	MeasurementTrials int
	WarmupTrials      int
	SubjectOverrides  map[string]string
	SubjectFilter     []string
	WorkloadFilter    []string
}

type PreparedSubject struct {
	Subject       Subject         `json:"subject"`
	BenchmarkRoot string          `json:"benchmark_root"`
	Metadata      SubjectMetadata `json:"metadata"`
	Cleanup       func() error    `json:"-"`
}

type SubjectMetadata struct {
	Name             string `json:"name"`
	Checkout         string `json:"checkout,omitempty"`
	Commit           string `json:"commit,omitempty"`
	Branch           string `json:"branch,omitempty"`
	Dirty            bool   `json:"dirty"`
	Prebuilt         bool   `json:"prebuilt"`
	BenchmarkRoot    string `json:"benchmark_root"`
	WorkingDirectory string `json:"working_directory"`
	WorktreePath     string `json:"worktree_path,omitempty"`
}

func main() {
	os.Exit(runCLI(cliSpec, os.Args, os.Stdout, os.Stderr))
}

func runCLI(specJSON []byte, argv []string, stdout, stderr io.Writer) int {
	parser, err := clibuilder.NewParserFromBytes(specJSON, argv)
	if err != nil {
		fmt.Fprintf(stderr, "benchmark-tool: %s\n", err)
		return 1
	}
	result, err := parser.Parse()
	if err != nil {
		fmt.Fprintf(stderr, "%s\n", err)
		return 1
	}
	switch r := result.(type) {
	case *clibuilder.HelpResult:
		fmt.Fprintln(stdout, r.Text)
		return 0
	case *clibuilder.VersionResult:
		fmt.Fprintln(stdout, r.Version)
		return 0
	case *clibuilder.ParseResult:
		if err := dispatchCLI(r, stdout); err != nil {
			fmt.Fprintln(stderr, "Error:", err)
			return 1
		}
		return 0
	default:
		fmt.Fprintf(stderr, "benchmark-tool: unexpected parse result %T\n", result)
		return 1
	}
}

func dispatchCLI(result *clibuilder.ParseResult, stdout io.Writer) error {
	if len(result.CommandPath) < 2 {
		return errors.New("expected a command; run benchmark-tool --help")
	}
	command := result.CommandPath[len(result.CommandPath)-1]
	switch command {
	case "doctor":
		return doctor(stdout)
	case "validate":
		return validateManifestFile(stringArgument(result, "manifest"), stdout)
	case "run":
		options, err := runOptionsFromCLI(result)
		if err != nil {
			return err
		}
		return runManifestFile(
			stringArgument(result, "manifest"),
			stringFlag(result, "out"),
			options,
		)
	case "report":
		return reportResultDir(stringArgument(result, "result-dir"), stdout)
	case "compare":
		return compareResultDirs(
			stringArgument(result, "base-dir"),
			stringArgument(result, "new-dir"),
			stringFlagDefault(result, "metric", "elapsed_ms"),
			stdout,
		)
	default:
		return fmt.Errorf("unknown command %q", command)
	}
}

func runOptionsFromCLI(result *clibuilder.ParseResult) (RunOptions, error) {
	overrides, err := parseSubjectOverrides(stringSliceFlag(result, "subject"))
	if err != nil {
		return RunOptions{}, err
	}
	return RunOptions{
		MeasurementTrials: intFlag(result, "trials", -1),
		WarmupTrials:      intFlag(result, "warmup", -1),
		SubjectOverrides:  overrides,
		SubjectFilter:     splitCSVFlag(stringFlag(result, "subjects")),
		WorkloadFilter:    splitCSVFlag(stringFlag(result, "workloads")),
	}, nil
}

func stringArgument(result *clibuilder.ParseResult, id string) string {
	value, _ := result.Arguments[id].(string)
	return value
}

func stringFlag(result *clibuilder.ParseResult, id string) string {
	value, _ := result.Flags[id].(string)
	return value
}

func stringSliceFlag(result *clibuilder.ParseResult, id string) []string {
	switch value := result.Flags[id].(type) {
	case nil:
		return nil
	case string:
		if value == "" {
			return nil
		}
		return []string{value}
	case []string:
		return append([]string(nil), value...)
	case []any:
		values := make([]string, 0, len(value))
		for _, item := range value {
			if text, ok := item.(string); ok && text != "" {
				values = append(values, text)
			}
		}
		return values
	default:
		return nil
	}
}

func stringFlagDefault(result *clibuilder.ParseResult, id string, fallback string) string {
	if value := stringFlag(result, id); value != "" {
		return value
	}
	return fallback
}

func intFlag(result *clibuilder.ParseResult, id string, fallback int) int {
	switch value := result.Flags[id].(type) {
	case int:
		return value
	case int64:
		converted, ok := intFromInt64(value)
		if !ok {
			return fallback
		}
		return converted
	case float64:
		converted, ok := intFromFloat64(value)
		if !ok {
			return fallback
		}
		return converted
	default:
		return fallback
	}
}

func intFromInt64(value int64) (int, bool) {
	converted, err := strconv.Atoi(strconv.FormatInt(value, 10))
	if err != nil {
		return 0, false
	}
	return converted, true
}

func intFromFloat64(value float64) (int, bool) {
	if math.IsNaN(value) || math.IsInf(value, 0) || math.Trunc(value) != value {
		return 0, false
	}
	converted, err := strconv.Atoi(strconv.FormatFloat(value, 'f', 0, 64))
	if err != nil {
		return 0, false
	}
	return converted, true
}

func doctor(w io.Writer) error {
	env := captureEnvironment(".")
	fmt.Fprintf(w, "benchmark-tool %s\n", version)
	fmt.Fprintf(w, "host: %s %s/%s cpus=%d\n", env.Hostname, env.OS, env.Arch, env.CPUs)
	fmt.Fprintf(w, "go: %s\n", env.GoVersion)
	if env.Kernel != "" {
		fmt.Fprintf(w, "kernel: %s\n", env.Kernel)
	}
	if env.ULimitOpenFiles != "" {
		fmt.Fprintf(w, "ulimit_open_files: %s\n", env.ULimitOpenFiles)
	}
	fmt.Fprintln(w, "doctor: ok")
	return nil
}

func validateManifestFile(path string, stdout io.Writer) error {
	manifest, err := LoadManifest(path)
	if err != nil {
		return err
	}
	if err := ValidateManifest(manifest); err != nil {
		return err
	}
	fmt.Fprintf(stdout, "manifest %q is valid (%d subject(s), %d workload(s))\n", manifest.Name, len(manifest.Subjects), len(manifest.Workloads))
	return nil
}

func runManifestFile(manifestPath string, outDir string, options RunOptions) error {
	manifest, err := LoadManifest(manifestPath)
	if err != nil {
		return err
	}
	manifest, err = applyRunOptions(manifest, options)
	if err != nil {
		return err
	}
	if err := ValidateManifest(manifest); err != nil {
		return err
	}
	if outDir == "" {
		outDir = defaultResultDir(manifest.Name)
	}
	return RunManifest(manifestPath, manifest, outDir)
}

func applyRunOptions(manifest Manifest, options RunOptions) (Manifest, error) {
	if options.MeasurementTrials >= 0 {
		manifest.Defaults.MeasurementTrials = options.MeasurementTrials
	}
	if options.WarmupTrials >= 0 {
		manifest.Defaults.WarmupTrials = options.WarmupTrials
	}
	for name, checkout := range options.SubjectOverrides {
		found := false
		for i := range manifest.Subjects {
			if manifest.Subjects[i].Name == name {
				manifest.Subjects[i].Checkout = checkout
				found = true
			}
		}
		if !found {
			return Manifest{}, fmt.Errorf("subject override references unknown subject %q", name)
		}
	}
	if len(options.SubjectFilter) > 0 {
		filtered, err := filterSubjects(manifest.Subjects, options.SubjectFilter)
		if err != nil {
			return Manifest{}, err
		}
		manifest.Subjects = filtered
	}
	if len(options.WorkloadFilter) > 0 {
		filtered, err := filterWorkloads(manifest.Workloads, options.WorkloadFilter)
		if err != nil {
			return Manifest{}, err
		}
		manifest.Workloads = filtered
	}
	return manifest, nil
}

func parseSubjectOverrides(raw []string) (map[string]string, error) {
	overrides := map[string]string{}
	for _, item := range raw {
		name, ref, ok := strings.Cut(item, "=")
		name = strings.TrimSpace(name)
		ref = strings.TrimSpace(ref)
		if !ok || name == "" || ref == "" {
			return nil, fmt.Errorf("subject override %q must use name=ref", item)
		}
		overrides[name] = ref
	}
	if len(overrides) == 0 {
		return nil, nil
	}
	return overrides, nil
}

func splitCSVFlag(raw string) []string {
	if strings.TrimSpace(raw) == "" {
		return nil
	}
	parts := strings.Split(raw, ",")
	values := make([]string, 0, len(parts))
	for _, part := range parts {
		value := strings.TrimSpace(part)
		if value != "" {
			values = append(values, value)
		}
	}
	return values
}

func filterSubjects(subjects []Subject, names []string) ([]Subject, error) {
	wanted := nameSet(names)
	filtered := make([]Subject, 0, len(subjects))
	seen := map[string]bool{}
	for _, subject := range subjects {
		if wanted[subject.Name] {
			filtered = append(filtered, subject)
			seen[subject.Name] = true
		}
	}
	if missing := missingNames(names, seen); len(missing) > 0 {
		return nil, fmt.Errorf("unknown subject filter(s): %s", strings.Join(missing, ", "))
	}
	return filtered, nil
}

func filterWorkloads(workloads []Workload, names []string) ([]Workload, error) {
	wanted := nameSet(names)
	filtered := make([]Workload, 0, len(workloads))
	seen := map[string]bool{}
	for _, workload := range workloads {
		if wanted[workload.Name] {
			filtered = append(filtered, workload)
			seen[workload.Name] = true
		}
	}
	if missing := missingNames(names, seen); len(missing) > 0 {
		return nil, fmt.Errorf("unknown workload filter(s): %s", strings.Join(missing, ", "))
	}
	return filtered, nil
}

func nameSet(names []string) map[string]bool {
	set := map[string]bool{}
	for _, name := range names {
		set[name] = true
	}
	return set
}

func missingNames(names []string, seen map[string]bool) []string {
	var missing []string
	for _, name := range names {
		if !seen[name] {
			missing = append(missing, name)
		}
	}
	return missing
}

func reportResultDir(dir string, stdout io.Writer) error {
	summary, err := readSummary(filepath.Join(dir, "summary.json"))
	if err != nil {
		return err
	}
	report := renderReport(summary)
	if err := os.WriteFile(filepath.Join(dir, "report.md"), []byte(report), 0o644); err != nil {
		return err
	}
	fmt.Fprintln(stdout, filepath.Join(dir, "report.md"))
	return nil
}

func compareResultDirs(leftDir string, rightDir string, metric string, stdout io.Writer) error {
	leftTrials, err := readTrials(filepath.Join(leftDir, "trials.jsonl"))
	if err != nil {
		return err
	}
	rightTrials, err := readTrials(filepath.Join(rightDir, "trials.jsonl"))
	if err != nil {
		return err
	}
	comparison := compareTrialSets(leftDir, rightDir, metric, leftTrials, rightTrials)
	if len(comparison.Comparisons) == 0 {
		return fmt.Errorf("no common %q trial metrics found", metric)
	}
	if err := writeJSON(filepath.Join(rightDir, "comparison.json"), comparison); err != nil {
		return err
	}
	report := renderComparisonReport(comparison)
	if err := os.WriteFile(filepath.Join(rightDir, "comparison.md"), []byte(report), 0o644); err != nil {
		return err
	}
	for _, row := range comparison.Comparisons {
		fmt.Fprintf(
			stdout,
			"%s/%s %s: %s (%+.2f%%, CI %.2f..%.2f, Cliff's delta %.3f)\n",
			row.Subject,
			row.Workload,
			row.Metric,
			row.Verdict,
			row.RelativeDifferencePercent,
			row.RelativeCILowPercent,
			row.RelativeCIHighPercent,
			row.CliffsDelta,
		)
	}
	fmt.Fprintf(stdout, "wrote %s\n", filepath.Join(rightDir, "comparison.json"))
	fmt.Fprintf(stdout, "wrote %s\n", filepath.Join(rightDir, "comparison.md"))
	return nil
}

func compareTrialSets(baseDir string, candidateDir string, metric string, baseTrials []Trial, candidateTrials []Trial) ComparisonFile {
	base := trialValuesByKey(baseTrials, metric)
	candidate := trialValuesByKey(candidateTrials, metric)
	keys := sortedCommonValueKeys(base, candidate)
	rows := make([]Comparison, 0, len(keys))
	for _, key := range keys {
		baseSet := base[key]
		candidateSet := candidate[key]
		baseValues := append([]float64(nil), baseSet.Values...)
		candidateValues := append([]float64(nil), candidateSet.Values...)
		sort.Float64s(baseValues)
		sort.Float64s(candidateValues)
		row := compareMetricValues(key, metric, baseValues, candidateValues, baseSet.Correct, candidateSet.Correct)
		rows = append(rows, row)
	}
	return ComparisonFile{
		GeneratedAt:  time.Now().UTC().Format(time.RFC3339),
		BaseDir:      baseDir,
		CandidateDir: candidateDir,
		Metric:       metric,
		Comparisons:  rows,
	}
}

type trialMetricSet struct {
	Values  []float64
	Correct bool
	Seen    bool
}

func trialValuesByKey(trials []Trial, metric string) map[string]trialMetricSet {
	out := map[string]trialMetricSet{}
	for _, trial := range trials {
		if trial.Phase != "measurement" {
			continue
		}
		key := trial.Subject + "/" + trial.Workload
		entry := out[key]
		if !entry.Seen {
			entry.Correct = true
			entry.Seen = true
		}
		if !trial.OK {
			entry.Correct = false
		}
		if value, ok := trial.Metrics[metric]; ok && trial.OK {
			entry.Values = append(entry.Values, value)
		}
		out[key] = entry
	}
	return out
}

func compareMetricValues(key string, metric string, baseValues []float64, candidateValues []float64, baseCorrect bool, candidateCorrect bool) Comparison {
	subject, workload, _ := strings.Cut(key, "/")
	baseMedian := percentile(baseValues, 50)
	candidateMedian := percentile(candidateValues, 50)
	absoluteDiff := candidateMedian - baseMedian
	relativeDiff := relativeDifferencePercent(baseMedian, candidateMedian)
	ciLow, ciHigh := bootstrapRelativeDifferenceCI(baseValues, candidateValues)
	direction := metricDirection(metric)
	threshold := practicalThresholdPercent(metric)
	row := Comparison{
		Subject:                   subject,
		Workload:                  workload,
		Metric:                    metric,
		Direction:                 direction,
		BaseCount:                 len(baseValues),
		CandidateCount:            len(candidateValues),
		BaseMedian:                baseMedian,
		CandidateMedian:           candidateMedian,
		AbsoluteDifference:        absoluteDiff,
		RelativeDifferencePercent: relativeDiff,
		RelativeCILowPercent:      ciLow,
		RelativeCIHighPercent:     ciHigh,
		CliffsDelta:               cliffsDelta(baseValues, candidateValues),
		PracticalThresholdPercent: threshold,
		BaseCorrect:               baseCorrect,
		CandidateCorrect:          candidateCorrect,
	}
	row.Verdict, row.Reason = comparisonVerdict(row)
	return row
}

func sortedCommonValueKeys(left, right map[string]trialMetricSet) []string {
	var keys []string
	for key := range left {
		if _, ok := right[key]; ok {
			keys = append(keys, key)
		}
	}
	sort.Strings(keys)
	return keys
}

func comparisonVerdict(row Comparison) (string, string) {
	if !row.BaseCorrect || !row.CandidateCorrect {
		return "correctness_failed", "performance verdict suppressed because at least one side had failed measurement trials"
	}
	if row.BaseCount < 2 || row.CandidateCount < 2 {
		return "inconclusive", "at least two successful measurement trials per side are required for a confidence interval verdict"
	}
	if row.RelativeCILowPercent <= 0 && row.RelativeCIHighPercent >= 0 {
		return "inconclusive", "relative-difference confidence interval crosses zero"
	}
	if math.Abs(row.RelativeDifferencePercent) < row.PracticalThresholdPercent {
		return "no_clear_change", "observed median change is below the practical threshold"
	}
	if row.Direction == "higher_is_better" {
		if row.RelativeDifferencePercent > 0 && row.RelativeCILowPercent > 0 {
			return "improvement", "candidate is statistically and practically higher"
		}
		if row.RelativeDifferencePercent < 0 && row.RelativeCIHighPercent < 0 {
			return "regression", "candidate is statistically and practically lower"
		}
	} else {
		if row.RelativeDifferencePercent < 0 && row.RelativeCIHighPercent < 0 {
			return "improvement", "candidate is statistically and practically lower"
		}
		if row.RelativeDifferencePercent > 0 && row.RelativeCILowPercent > 0 {
			return "regression", "candidate is statistically and practically higher"
		}
	}
	return "inconclusive", "statistical and practical signals disagree"
}

func relativeDifferencePercent(baseMedian float64, candidateMedian float64) float64 {
	if baseMedian == 0 {
		if candidateMedian == 0 {
			return 0
		}
		return candidateMedian * 100.0
	}
	return (candidateMedian - baseMedian) / math.Abs(baseMedian) * 100.0
}

func bootstrapRelativeDifferenceCI(baseValues []float64, candidateValues []float64) (float64, float64) {
	if len(baseValues) == 0 || len(candidateValues) == 0 {
		return 0, 0
	}
	if len(baseValues) == 1 && len(candidateValues) == 1 {
		value := relativeDifferencePercent(baseValues[0], candidateValues[0])
		return value, value
	}
	seedBytes := sha256.Sum256([]byte(fmt.Sprintf("%v/%v", baseValues, candidateValues)))
	seed := int64(0)
	for i := 0; i < 8; i++ {
		seed = seed<<8 + int64(seedBytes[i])
	}
	rng := rand.New(rand.NewSource(seed))
	const rounds = 1000
	estimates := make([]float64, rounds)
	baseSample := make([]float64, len(baseValues))
	candidateSample := make([]float64, len(candidateValues))
	for i := 0; i < rounds; i++ {
		for j := range baseSample {
			baseSample[j] = baseValues[rng.Intn(len(baseValues))]
		}
		for j := range candidateSample {
			candidateSample[j] = candidateValues[rng.Intn(len(candidateValues))]
		}
		sort.Float64s(baseSample)
		sort.Float64s(candidateSample)
		estimates[i] = relativeDifferencePercent(percentile(baseSample, 50), percentile(candidateSample, 50))
	}
	sort.Float64s(estimates)
	return percentile(estimates, 2.5), percentile(estimates, 97.5)
}

func cliffsDelta(baseValues []float64, candidateValues []float64) float64 {
	if len(baseValues) == 0 || len(candidateValues) == 0 {
		return 0
	}
	var greater float64
	var lesser float64
	for _, candidate := range candidateValues {
		for _, base := range baseValues {
			switch {
			case candidate > base:
				greater++
			case candidate < base:
				lesser++
			}
		}
	}
	return (greater - lesser) / float64(len(baseValues)*len(candidateValues))
}

func metricDirection(metric string) string {
	lower := strings.ToLower(metric)
	if strings.Contains(lower, "ops_per_second") || strings.Contains(lower, "bytes_per_second") || strings.Contains(lower, "throughput") {
		return "higher_is_better"
	}
	return "lower_is_better"
}

func practicalThresholdPercent(metric string) float64 {
	lower := strings.ToLower(metric)
	if strings.Contains(lower, "startup") {
		return 10
	}
	return 5
}

func renderComparisonReport(comparison ComparisonFile) string {
	var b strings.Builder
	fmt.Fprintf(&b, "# Benchmark Comparison\n\n")
	fmt.Fprintf(&b, "Generated: %s\n\n", comparison.GeneratedAt)
	fmt.Fprintf(&b, "Metric: `%s`\n\n", comparison.Metric)
	if len(comparison.Comparisons) == 0 {
		b.WriteString("No comparable measurement trials were found.\n")
		return b.String()
	}
	b.WriteString("| Subject | Workload | Verdict | Base Median | Candidate Median | Relative Diff | 95% CI | Cliff's Delta | Reason |\n")
	b.WriteString("|---|---|---|---:|---:|---:|---:|---:|---|\n")
	for _, row := range comparison.Comparisons {
		fmt.Fprintf(
			&b,
			"| %s | %s | %s | %.3f | %.3f | %+.2f%% | %.2f..%.2f%% | %.3f | %s |\n",
			row.Subject,
			row.Workload,
			row.Verdict,
			row.BaseMedian,
			row.CandidateMedian,
			row.RelativeDifferencePercent,
			row.RelativeCILowPercent,
			row.RelativeCIHighPercent,
			row.CliffsDelta,
			row.Reason,
		)
	}
	return b.String()
}

func readTrials(path string) ([]Trial, error) {
	file, err := os.Open(path)
	if err != nil {
		return nil, err
	}
	defer file.Close()
	var trials []Trial
	scanner := bufio.NewScanner(file)
	for scanner.Scan() {
		line := strings.TrimSpace(scanner.Text())
		if line == "" {
			continue
		}
		var trial Trial
		if err := json.Unmarshal([]byte(line), &trial); err != nil {
			return nil, err
		}
		trials = append(trials, trial)
	}
	if err := scanner.Err(); err != nil {
		return nil, err
	}
	return trials, nil
}

func LoadManifest(path string) (Manifest, error) {
	file, err := os.Open(path)
	if err != nil {
		return Manifest{}, err
	}
	defer file.Close()
	return parseManifest(file)
}

func parseManifest(r io.Reader) (Manifest, error) {
	manifest := Manifest{
		Defaults: Defaults{
			WarmupTrials:      3,
			MeasurementTrials: 30,
			CooldownMS:        250,
		},
	}
	section := "root"
	subjectIndex := -1
	workloadIndex := -1
	scanner := bufio.NewScanner(r)
	lineNo := 0
	for scanner.Scan() {
		lineNo++
		line := stripComment(scanner.Text())
		line = strings.TrimSpace(line)
		if line == "" {
			continue
		}
		switch line {
		case "[defaults]":
			section = "defaults"
			continue
		case "[[subjects]]":
			manifest.Subjects = append(manifest.Subjects, Subject{})
			subjectIndex = len(manifest.Subjects) - 1
			section = "subject"
			continue
		case "[[workloads]]":
			manifest.Workloads = append(manifest.Workloads, Workload{})
			workloadIndex = len(manifest.Workloads) - 1
			section = "workload"
			continue
		}
		if strings.HasPrefix(line, "[") {
			return Manifest{}, fmt.Errorf("line %d: unsupported section %s", lineNo, line)
		}
		key, raw, ok := strings.Cut(line, "=")
		if !ok {
			return Manifest{}, fmt.Errorf("line %d: expected key = value", lineNo)
		}
		key = strings.TrimSpace(key)
		value, err := parseValue(strings.TrimSpace(raw))
		if err != nil {
			return Manifest{}, fmt.Errorf("line %d: %w", lineNo, err)
		}
		if err := assignManifestValue(&manifest, section, subjectIndex, workloadIndex, key, value); err != nil {
			return Manifest{}, fmt.Errorf("line %d: %w", lineNo, err)
		}
	}
	if err := scanner.Err(); err != nil {
		return Manifest{}, err
	}
	return manifest, nil
}

func stripComment(line string) string {
	inString := false
	escaped := false
	for i, r := range line {
		if escaped {
			escaped = false
			continue
		}
		if r == '\\' && inString {
			escaped = true
			continue
		}
		if r == '"' {
			inString = !inString
			continue
		}
		if r == '#' && !inString {
			return line[:i]
		}
	}
	return line
}

func parseValue(raw string) (any, error) {
	if raw == "" {
		return nil, errors.New("empty value")
	}
	if strings.HasPrefix(raw, "\"") {
		return strconv.Unquote(raw)
	}
	if raw == "true" {
		return true, nil
	}
	if raw == "false" {
		return false, nil
	}
	if strings.HasPrefix(raw, "[") && strings.HasSuffix(raw, "]") {
		inner := strings.TrimSpace(strings.TrimSuffix(strings.TrimPrefix(raw, "["), "]"))
		if inner == "" {
			return []any{}, nil
		}
		parts := splitArray(inner)
		values := make([]any, 0, len(parts))
		for _, part := range parts {
			value, err := parseValue(strings.TrimSpace(part))
			if err != nil {
				return nil, err
			}
			values = append(values, value)
		}
		return values, nil
	}
	if strings.Contains(raw, ".") {
		f, err := strconv.ParseFloat(raw, 64)
		if err == nil {
			return f, nil
		}
	}
	i, err := strconv.Atoi(raw)
	if err == nil {
		return i, nil
	}
	return nil, fmt.Errorf("unsupported value %q", raw)
}

func splitArray(raw string) []string {
	var parts []string
	start := 0
	inString := false
	escaped := false
	for i, r := range raw {
		if escaped {
			escaped = false
			continue
		}
		if r == '\\' && inString {
			escaped = true
			continue
		}
		if r == '"' {
			inString = !inString
			continue
		}
		if r == ',' && !inString {
			parts = append(parts, raw[start:i])
			start = i + 1
		}
	}
	parts = append(parts, raw[start:])
	return parts
}

func assignManifestValue(m *Manifest, section string, subjectIndex, workloadIndex int, key string, value any) error {
	switch section {
	case "root":
		switch key {
		case "name":
			m.Name = asString(value)
		case "description":
			m.Description = asString(value)
		default:
			return fmt.Errorf("unknown root key %q", key)
		}
	case "defaults":
		switch key {
		case "warmup_trials":
			m.Defaults.WarmupTrials = asInt(value)
		case "measurement_trials":
			m.Defaults.MeasurementTrials = asInt(value)
		case "cooldown_ms":
			m.Defaults.CooldownMS = asInt(value)
		case "randomize_subject_order":
			m.Defaults.RandomizeSubjectOrder = asBool(value)
		case "fail_fast":
			m.Defaults.FailFast = asBool(value)
		default:
			return fmt.Errorf("unknown defaults key %q", key)
		}
	case "subject":
		if subjectIndex < 0 {
			return errors.New("subject key outside subject section")
		}
		s := &m.Subjects[subjectIndex]
		switch key {
		case "name":
			s.Name = asString(value)
		case "kind":
			s.Kind = asString(value)
		case "checkout":
			s.Checkout = asString(value)
		case "working_directory":
			s.WorkingDirectory = asString(value)
		case "build":
			s.Build = asString(value)
		case "command":
			s.Command = asString(value)
		case "ready_check":
			s.ReadyCheck = asString(value)
		case "prebuilt":
			s.Prebuilt = asBool(value)
		default:
			return fmt.Errorf("unknown subject key %q", key)
		}
	case "workload":
		if workloadIndex < 0 {
			return errors.New("workload key outside workload section")
		}
		w := &m.Workloads[workloadIndex]
		switch key {
		case "name":
			w.Name = asString(value)
		case "driver":
			w.Driver = asString(value)
		case "mode":
			w.Mode = asString(value)
		case "command":
			w.Command = asString(value)
		case "read_mode":
			w.ReadMode = asString(value)
		case "request":
			w.Request = asString(value)
		case "expect":
			w.Expect = asString(value)
		case "protocol_closes_after_response":
			w.ProtocolClosesAfterResponse = asBool(value)
		case "connections":
			w.Connections = asInt(value)
		case "concurrency":
			w.Concurrency = asInt(value)
		case "requests_per_connection":
			w.RequestsPerConnection = asInt(value)
		case "operations":
			w.Operations = asInt(value)
		case "timeout_ms":
			w.TimeoutMS = asInt(value)
		case "hold_ms":
			w.HoldMS = asInt(value)
		default:
			return fmt.Errorf("unknown workload key %q", key)
		}
	default:
		return fmt.Errorf("unknown section %q", section)
	}
	return nil
}

func asString(value any) string {
	if s, ok := value.(string); ok {
		return s
	}
	return fmt.Sprint(value)
}

func asInt(value any) int {
	switch v := value.(type) {
	case int:
		return v
	case float64:
		return int(v)
	default:
		return 0
	}
}

func asBool(value any) bool {
	v, _ := value.(bool)
	return v
}

func ValidateManifest(m Manifest) error {
	var problems []string
	if strings.TrimSpace(m.Name) == "" {
		problems = append(problems, "manifest name is required")
	}
	if m.Defaults.MeasurementTrials < 1 {
		problems = append(problems, "defaults.measurement_trials must be at least 1")
	}
	if m.Defaults.WarmupTrials < 0 {
		problems = append(problems, "defaults.warmup_trials must be non-negative")
	}
	if len(m.Subjects) == 0 {
		problems = append(problems, "at least one subject is required")
	}
	if len(m.Workloads) == 0 {
		problems = append(problems, "at least one workload is required")
	}
	names := map[string]string{}
	for i, s := range m.Subjects {
		where := fmt.Sprintf("subjects[%d]", i)
		if s.Name == "" {
			problems = append(problems, where+".name is required")
		}
		if previous := names["subject:"+s.Name]; s.Name != "" && previous != "" {
			problems = append(problems, where+".name duplicates "+previous)
		}
		names["subject:"+s.Name] = where
		if s.Kind != "command" && s.Kind != "service" {
			problems = append(problems, where+".kind must be command or service")
		}
		if strings.TrimSpace(s.Command) == "" {
			problems = append(problems, where+".command is required")
		}
		if s.Kind == "service" && s.ReadyCheck == "" {
			problems = append(problems, where+".ready_check is required for service subjects")
		}
		if !s.Prebuilt && s.Build == "" && s.Checkout == "" {
			problems = append(problems, where+" should declare build, checkout, or prebuilt = true")
		}
	}
	for i, w := range m.Workloads {
		where := fmt.Sprintf("workloads[%d]", i)
		if w.Name == "" {
			problems = append(problems, where+".name is required")
		}
		if previous := names["workload:"+w.Name]; w.Name != "" && previous != "" {
			problems = append(problems, where+".name duplicates "+previous)
		}
		names["workload:"+w.Name] = where
		if w.Driver == "" {
			problems = append(problems, where+".driver is required")
		}
		if w.Driver == "command" && w.ReadMode == "eof" && !w.ProtocolClosesAfterResponse {
			problems = append(problems, where+" uses EOF reads without protocol_closes_after_response = true")
		}
		if w.Driver == respDriver {
			mode := tcpWorkloadMode(w)
			if w.ReadMode != respReadMode {
				problems = append(problems, where+" must use read_mode = \"resp-frame\" for RESP workloads")
			}
			if mode != respModeIdle && w.Expect == "" {
				problems = append(problems, where+".expect is required for RESP workloads")
			}
			if !isSupportedTCPRESPMode(mode) {
				problems = append(problems, where+".mode must be one-shot, preconnect-then-fire, pipeline, idle, or hold")
			}
			if w.Connections < 0 {
				problems = append(problems, where+".connections must be non-negative")
			}
			if w.Connections > maxTCPConnections {
				problems = append(problems, fmt.Sprintf("%s.connections must be <= %d", where, maxTCPConnections))
			}
			if w.Concurrency < 0 {
				problems = append(problems, where+".concurrency must be non-negative")
			}
			if w.Concurrency > maxTCPConcurrency {
				problems = append(problems, fmt.Sprintf("%s.concurrency must be <= %d", where, maxTCPConcurrency))
			}
			if w.RequestsPerConnection < 0 {
				problems = append(problems, where+".requests_per_connection must be non-negative")
			}
			if w.RequestsPerConnection > maxTCPRequestsPerConn {
				problems = append(problems, fmt.Sprintf("%s.requests_per_connection must be <= %d", where, maxTCPRequestsPerConn))
			}
			if len(w.Request) > maxTCPPayloadBytes {
				problems = append(problems, fmt.Sprintf("%s.request must be <= %d bytes", where, maxTCPPayloadBytes))
			}
			if len(w.Expect) > maxTCPPayloadBytes {
				problems = append(problems, fmt.Sprintf("%s.expect must be <= %d bytes", where, maxTCPPayloadBytes))
			}
			requestRepeats := tcpRequestsPerConnection(w)
			if len(w.Request) > 0 && requestRepeats > maxTCPPayloadBytes/len(w.Request) {
				problems = append(problems, fmt.Sprintf("%s repeated request payload must be <= %d bytes", where, maxTCPPayloadBytes))
			}
		} else if strings.Contains(w.Driver, "resp") && w.ReadMode != respReadMode {
			problems = append(problems, where+" must use read_mode = \"resp-frame\" for RESP workloads")
		}
		if w.TimeoutMS < 0 {
			problems = append(problems, where+".timeout_ms must be non-negative")
		}
		if w.HoldMS < 0 {
			problems = append(problems, where+".hold_ms must be non-negative")
		}
		if w.HoldMS > maxTCPHoldMS {
			problems = append(problems, fmt.Sprintf("%s.hold_ms must be <= %d", where, maxTCPHoldMS))
		}
	}
	if len(problems) > 0 {
		return errors.New(strings.Join(problems, "\n"))
	}
	return nil
}

func RunManifest(manifestPath string, manifest Manifest, outDir string) error {
	benchmarkRoot := benchmarkRootForManifest(manifestPath)
	if err := os.MkdirAll(outDir, 0o755); err != nil {
		return err
	}
	if err := copyFile(manifestPath, filepath.Join(outDir, "manifest.toml")); err != nil {
		return err
	}
	env := captureEnvironment(benchmarkRoot)
	if err := writeJSON(filepath.Join(outDir, "environment.json"), env); err != nil {
		return err
	}
	if err := os.MkdirAll(filepath.Join(outDir, "subjects"), 0o755); err != nil {
		return err
	}
	samplesPath := filepath.Join(outDir, "samples.jsonl")
	trialsPath := filepath.Join(outDir, "trials.jsonl")
	samplesFile, err := os.Create(samplesPath)
	if err != nil {
		return err
	}
	defer samplesFile.Close()
	trialsFile, err := os.Create(trialsPath)
	if err != nil {
		return err
	}
	defer trialsFile.Close()

	var measured []Trial
	for _, subject := range manifest.Subjects {
		subjectDir := filepath.Join(outDir, "subjects", safeName(subject.Name))
		if err := os.MkdirAll(subjectDir, 0o755); err != nil {
			return err
		}
		prepared, err := prepareSubject(subject, subjectDir, benchmarkRoot)
		if err != nil {
			if manifest.Defaults.FailFast {
				return err
			}
			fmt.Fprintln(os.Stderr, "Subject preparation error:", err)
			continue
		}
		if prepared.Cleanup != nil {
			defer func(cleanup func() error) {
				if err := cleanup(); err != nil {
					fmt.Fprintln(os.Stderr, "Subject cleanup error:", err)
				}
			}(prepared.Cleanup)
		}
		if err := writeJSON(filepath.Join(subjectDir, "subject.json"), prepared.Metadata); err != nil {
			return err
		}
		if prepared.Subject.Build != "" {
			if err := runBuild(prepared, subjectDir); err != nil {
				if manifest.Defaults.FailFast {
					return err
				}
				fmt.Fprintln(os.Stderr, "Build error:", err)
			}
		}
		var service *RunningService
		if subjectNeedsTCPService(prepared.Subject, manifest.Workloads) {
			var err error
			service, err = startServiceSubject(prepared, subjectDir)
			if err != nil {
				if manifest.Defaults.FailFast {
					return err
				}
				fmt.Fprintln(os.Stderr, "Service error:", err)
				continue
			}
		}
		for _, workload := range manifest.Workloads {
			var allTrials []Trial
			var err error
			switch workload.Driver {
			case "command":
				allTrials, err = runCommandWorkload(prepared, workload, manifest.Defaults, samplesFile, trialsFile)
			case respDriver:
				if service == nil {
					err = fmt.Errorf("workload %s requires a running service subject", workload.Name)
				} else {
					allTrials, err = runTCPRESPWorkload(prepared, workload, manifest.Defaults, service.Address, samplesFile, trialsFile)
				}
			default:
				fmt.Fprintf(os.Stderr, "Skipping workload %s: driver %s is not implemented\n", workload.Name, workload.Driver)
				continue
			}
			if err != nil {
				if manifest.Defaults.FailFast {
					if service != nil {
						_ = service.Stop()
					}
					return err
				}
				fmt.Fprintln(os.Stderr, "Workload error:", err)
			}
			for _, trial := range allTrials {
				if trial.Phase == "measurement" && trial.OK {
					measured = append(measured, trial)
				}
			}
		}
		if service != nil {
			if err := service.Stop(); err != nil && manifest.Defaults.FailFast {
				return err
			} else if err != nil {
				fmt.Fprintln(os.Stderr, "Service cleanup error:", err)
			}
		}
	}
	summary := SummaryFile{
		ManifestName: manifest.Name,
		GeneratedAt:  time.Now().UTC().Format(time.RFC3339),
		Summaries:    summarizeTrials(measured),
	}
	if err := writeJSON(filepath.Join(outDir, "summary.json"), summary); err != nil {
		return err
	}
	if err := os.WriteFile(filepath.Join(outDir, "report.md"), []byte(renderReport(summary)), 0o644); err != nil {
		return err
	}
	fmt.Println(outDir)
	return nil
}

func benchmarkRootForManifest(manifestPath string) string {
	abs, err := filepath.Abs(manifestPath)
	if err != nil {
		return "."
	}
	manifestDir := filepath.Dir(abs)
	if root := commandOutput(manifestDir, "git", "rev-parse", "--show-toplevel"); root != "" {
		return root
	}
	return manifestDir
}

func resolveSubjectWorkingDirectory(subject Subject, benchmarkRoot string) string {
	if subject.WorkingDirectory == "" {
		return benchmarkRoot
	}
	if filepath.IsAbs(subject.WorkingDirectory) {
		return subject.WorkingDirectory
	}
	return filepath.Join(benchmarkRoot, subject.WorkingDirectory)
}

func prepareSubject(subject Subject, subjectDir string, benchmarkRoot string) (PreparedSubject, error) {
	prepared := PreparedSubject{
		Subject:       subject,
		BenchmarkRoot: benchmarkRoot,
		Metadata: SubjectMetadata{
			Name:             subject.Name,
			Checkout:         subject.Checkout,
			Prebuilt:         subject.Prebuilt,
			BenchmarkRoot:    benchmarkRoot,
			WorkingDirectory: resolveSubjectWorkingDirectory(subject, benchmarkRoot),
			Branch:           commandOutput(benchmarkRoot, "git", "rev-parse", "--abbrev-ref", "HEAD"),
			Commit:           commandOutput(benchmarkRoot, "git", "rev-parse", "HEAD"),
			Dirty:            commandOutput(benchmarkRoot, "git", "status", "--short") != "",
		},
	}
	if subject.Checkout == "" {
		return prepared, nil
	}
	tempParent, err := os.MkdirTemp("", "benchmark-tool-worktrees-")
	if err != nil {
		return PreparedSubject{}, err
	}
	worktreePath := filepath.Join(tempParent, safeName(subject.Name))
	result := runProcess(benchmarkRoot, 30*time.Second, "git", "worktree", "add", "--detach", worktreePath, subject.Checkout)
	log := fmt.Sprintf("command: git worktree add --detach %s %s\nok: %t\nelapsed_ms: %.3f\n\nstdout:\n%s\n\nstderr:\n%s\n", worktreePath, subject.Checkout, result.OK, result.ElapsedMS, result.Stdout, result.Stderr)
	if err := os.WriteFile(filepath.Join(subjectDir, "prepare.log"), []byte(log), 0o644); err != nil {
		_ = os.RemoveAll(tempParent)
		return PreparedSubject{}, err
	}
	if !result.OK {
		_ = os.RemoveAll(tempParent)
		return PreparedSubject{}, fmt.Errorf("prepare failed for subject %s: %s", subject.Name, result.Error)
	}
	prepared.BenchmarkRoot = worktreePath
	prepared.Metadata.BenchmarkRoot = worktreePath
	prepared.Metadata.WorktreePath = worktreePath
	prepared.Metadata.WorkingDirectory = resolveSubjectWorkingDirectory(subject, worktreePath)
	prepared.Metadata.Branch = commandOutput(worktreePath, "git", "rev-parse", "--abbrev-ref", "HEAD")
	prepared.Metadata.Commit = commandOutput(worktreePath, "git", "rev-parse", "HEAD")
	prepared.Metadata.Dirty = commandOutput(worktreePath, "git", "status", "--short") != ""
	prepared.Cleanup = func() error {
		remove := runProcess(benchmarkRoot, 30*time.Second, "git", "worktree", "remove", "--force", worktreePath)
		if !remove.OK {
			return fmt.Errorf("remove git worktree %s: %s", worktreePath, remove.Error)
		}
		if err := os.RemoveAll(tempParent); err != nil {
			return err
		}
		return nil
	}
	if prepared.Metadata.Dirty {
		if prepared.Cleanup != nil {
			_ = prepared.Cleanup()
		}
		return PreparedSubject{}, fmt.Errorf("subject %s checkout %s is dirty after preparation", subject.Name, subject.Checkout)
	}
	return prepared, nil
}

func runBuild(subject PreparedSubject, subjectDir string) error {
	workingDir := resolveSubjectWorkingDirectory(subject.Subject, subject.BenchmarkRoot)
	result := runShell(subject.Subject.Build, workingDir, 0)
	log := fmt.Sprintf("command: %s\nok: %t\nelapsed_ms: %.3f\nworking_directory: %s\ncommit: %s\n\nstdout:\n%s\n\nstderr:\n%s\n", subject.Subject.Build, result.OK, result.ElapsedMS, workingDir, subject.Metadata.Commit, result.Stdout, result.Stderr)
	if err := os.WriteFile(filepath.Join(subjectDir, "build.log"), []byte(log), 0o644); err != nil {
		return err
	}
	if !result.OK {
		return fmt.Errorf("build failed for subject %s: %s", subject.Subject.Name, result.Error)
	}
	return nil
}

type RunningService struct {
	Command string
	Address string
	Port    int
	cmd     *exec.Cmd
	done    chan error
	log     *os.File
}

func subjectNeedsTCPService(subject Subject, workloads []Workload) bool {
	for _, workload := range workloads {
		if workload.Driver == respDriver {
			return subject.Kind == "service"
		}
	}
	return false
}

func startServiceSubject(subject PreparedSubject, subjectDir string) (*RunningService, error) {
	if subject.Subject.Kind != "service" {
		return nil, fmt.Errorf("subject %s must be kind = service for TCP workloads", subject.Subject.Name)
	}
	if subject.Subject.ReadyCheck != "tcp-connect" {
		return nil, fmt.Errorf("subject %s uses unsupported ready_check %q", subject.Subject.Name, subject.Subject.ReadyCheck)
	}
	port, err := allocateTCPPort()
	if err != nil {
		return nil, err
	}
	address := net.JoinHostPort("127.0.0.1", strconv.Itoa(port))
	command := strings.ReplaceAll(subject.Subject.Command, "{port}", strconv.Itoa(port))
	workingDir := resolveSubjectWorkingDirectory(subject.Subject, subject.BenchmarkRoot)
	logPath := filepath.Join(subjectDir, "service.log")
	logFile, err := os.Create(logPath)
	if err != nil {
		return nil, err
	}
	fmt.Fprintf(logFile, "command: %s\nworking_directory: %s\naddress: %s\n\n", command, workingDir, address)
	cmd := shellCommand(command)
	cmd.Dir = workingDir
	cmd.Stdout = logFile
	cmd.Stderr = logFile
	if err := cmd.Start(); err != nil {
		_ = logFile.Close()
		return nil, err
	}
	service := &RunningService{
		Command: command,
		Address: address,
		Port:    port,
		cmd:     cmd,
		done:    make(chan error, 1),
		log:     logFile,
	}
	go func() {
		service.done <- cmd.Wait()
	}()
	if err := waitForTCPReady(address, serviceReadyTimeout); err != nil {
		_ = service.Stop()
		return nil, fmt.Errorf("service %s did not become ready: %w", subject.Subject.Name, err)
	}
	return service, nil
}

func (s *RunningService) Stop() error {
	if s == nil || s.cmd == nil {
		return nil
	}
	var waitErr error
	select {
	case waitErr = <-s.done:
	default:
		if s.cmd.Process != nil {
			if err := s.cmd.Process.Kill(); err != nil && !errors.Is(err, os.ErrProcessDone) {
				_ = s.log.Close()
				return err
			}
		}
		waitErr = <-s.done
	}
	if s.log != nil {
		_, _ = fmt.Fprintf(s.log, "\nservice_exit: %v\n", waitErr)
		if err := s.log.Close(); err != nil {
			return err
		}
	}
	return nil
}

func allocateTCPPort() (int, error) {
	listener, err := net.Listen("tcp", "127.0.0.1:0")
	if err != nil {
		return 0, err
	}
	defer listener.Close()
	addr, ok := listener.Addr().(*net.TCPAddr)
	if !ok {
		return 0, fmt.Errorf("unexpected TCP listener address %T", listener.Addr())
	}
	return addr.Port, nil
}

func waitForTCPReady(address string, timeout time.Duration) error {
	deadline := time.Now().Add(timeout)
	var lastErr error
	for time.Now().Before(deadline) {
		conn, err := net.DialTimeout("tcp", address, serviceReadyProbeTimeout)
		if err == nil {
			_ = conn.Close()
			return nil
		}
		lastErr = err
		time.Sleep(25 * time.Millisecond)
	}
	if lastErr != nil {
		return lastErr
	}
	return errors.New("readiness timeout")
}

func shellCommand(command string) *exec.Cmd {
	if runtime.GOOS == "windows" {
		return exec.Command("cmd", "/C", command)
	}
	return exec.Command("sh", "-c", command)
}

func runCommandWorkload(subject PreparedSubject, workload Workload, defaults Defaults, samplesFile, trialsFile *os.File) ([]Trial, error) {
	var trials []Trial
	total := defaults.WarmupTrials + defaults.MeasurementTrials
	for i := 0; i < total; i++ {
		phase := "measurement"
		trialNumber := i - defaults.WarmupTrials + 1
		if i < defaults.WarmupTrials {
			phase = "warmup"
			trialNumber = i + 1
		}
		command := subject.Subject.Command
		if workload.Command != "" {
			command = workload.Command
		}
		workingDir := resolveSubjectWorkingDirectory(subject.Subject, subject.BenchmarkRoot)
		result := runShell(command, workingDir, time.Duration(workload.TimeoutMS)*time.Millisecond)
		metrics := map[string]float64{"elapsed_ms": result.ElapsedMS}
		if workload.Operations > 0 && result.ElapsedMS > 0 {
			metrics["operations"] = float64(workload.Operations)
			metrics["ops_per_second"] = float64(workload.Operations) / (result.ElapsedMS / 1000.0)
		}
		sample := Sample{
			SampleKind: "operation_batch",
			Subject:    subject.Subject.Name,
			Workload:   workload.Name,
			Trial:      trialNumber,
			Phase:      phase,
			OK:         result.OK,
			Metrics:    metrics,
			Error:      result.Error,
		}
		trial := Trial{
			Subject:  subject.Subject.Name,
			Workload: workload.Name,
			Trial:    trialNumber,
			Phase:    phase,
			OK:       result.OK,
			Metrics:  metrics,
			Error:    result.Error,
		}
		if err := writeJSONLine(samplesFile, sample); err != nil {
			return trials, err
		}
		if err := writeJSONLine(trialsFile, trial); err != nil {
			return trials, err
		}
		trials = append(trials, trial)
		if defaults.CooldownMS > 0 {
			time.Sleep(time.Duration(defaults.CooldownMS) * time.Millisecond)
		}
	}
	return trials, nil
}

type tcpRequestResult struct {
	OK      bool
	Metrics map[string]float64
	Error   string
}

func runTCPRESPWorkload(subject PreparedSubject, workload Workload, defaults Defaults, address string, samplesFile, trialsFile *os.File) ([]Trial, error) {
	expectedFrames, err := splitRESPFrames([]byte(workload.Expect))
	if err != nil && tcpWorkloadMode(workload) != respModeIdle {
		return nil, fmt.Errorf("parse expected RESP frames for workload %s: %w", workload.Name, err)
	}
	total := defaults.WarmupTrials + defaults.MeasurementTrials
	trials := make([]Trial, 0, total)
	for i := 0; i < total; i++ {
		phase := "measurement"
		trialNumber := i - defaults.WarmupTrials + 1
		if i < defaults.WarmupTrials {
			phase = "warmup"
			trialNumber = i + 1
		}
		start := time.Now()
		results := executeTCPRESPTrial(address, workload, expectedFrames)
		elapsedMS := float64(time.Since(start).Microseconds()) / 1000.0
		samples := tcpSamplesFromResults(subject.Subject.Name, workload.Name, trialNumber, phase, results)
		trial := tcpTrialFromSamples(subject.Subject.Name, workload, trialNumber, phase, samples, elapsedMS)
		for _, sample := range samples {
			if err := writeJSONLine(samplesFile, sample); err != nil {
				return trials, err
			}
		}
		if err := writeJSONLine(trialsFile, trial); err != nil {
			return trials, err
		}
		trials = append(trials, trial)
		if defaults.CooldownMS > 0 {
			time.Sleep(time.Duration(defaults.CooldownMS) * time.Millisecond)
		}
	}
	return trials, nil
}

func executeTCPRESPTrial(address string, workload Workload, expectedFrames [][]byte) []tcpRequestResult {
	mode := tcpWorkloadMode(workload)
	timeout := tcpTimeout(workload)
	connections := tcpConnections(workload)
	concurrency := tcpConcurrency(workload, connections)
	request := []byte(workload.Request)
	switch mode {
	case respModeOneShot:
		return runTCPRESPParallel(connections, concurrency, func(_ int) tcpRequestResult {
			return executeRESPDialExchange(address, request, expectedFrames, 1, timeout)
		})
	case respModePreconnectThenFire:
		return executeRESPPreconnectThenFire(address, request, expectedFrames, connections, concurrency, timeout)
	case respModePipeline:
		repeats := tcpRequestsPerConnection(workload)
		return runTCPRESPParallel(connections, concurrency, func(_ int) tcpRequestResult {
			return executeRESPDialExchange(address, request, expectedFrames, repeats, timeout)
		})
	case respModeIdle:
		return runTCPRESPParallel(connections, concurrency, func(_ int) tcpRequestResult {
			return executeTCPIdle(address, timeout)
		})
	case respModeHold:
		return executeRESPHold(address, request, expectedFrames, connections, concurrency, timeout, tcpHoldDuration(workload))
	default:
		return []tcpRequestResult{{
			OK:      false,
			Metrics: map[string]float64{},
			Error:   "unsupported tcp-resp mode " + mode,
		}}
	}
}

func runTCPRESPParallel(total int, concurrency int, fn func(int) tcpRequestResult) []tcpRequestResult {
	if total <= 0 {
		return nil
	}
	if concurrency <= 0 || concurrency > total {
		concurrency = total
	}
	results := make([]tcpRequestResult, total)
	sem := make(chan struct{}, concurrency)
	var wg sync.WaitGroup
	for i := 0; i < total; i++ {
		sem <- struct{}{}
		wg.Add(1)
		go func(index int) {
			defer wg.Done()
			defer func() { <-sem }()
			results[index] = fn(index)
		}(i)
	}
	wg.Wait()
	return results
}

type preconnectedRESPConn struct {
	conn      net.Conn
	connectMS float64
	err       error
}

type heldRESPConn struct {
	conn      net.Conn
	connectMS float64
}

func executeRESPPreconnectThenFire(address string, request []byte, expectedFrames [][]byte, connections int, concurrency int, timeout time.Duration) []tcpRequestResult {
	preconnected := make([]preconnectedRESPConn, connections)
	connectResults := runTCPRESPParallel(connections, concurrency, func(index int) tcpRequestResult {
		conn, connectMS, err := dialTCP(address, timeout)
		if err != nil {
			return tcpRequestResult{
				OK:      false,
				Metrics: map[string]float64{"connect_ms": connectMS},
				Error:   err.Error(),
			}
		}
		preconnected[index] = preconnectedRESPConn{conn: conn, connectMS: connectMS}
		return tcpRequestResult{OK: true, Metrics: map[string]float64{"connect_ms": connectMS}}
	})
	results := runTCPRESPParallel(connections, concurrency, func(index int) tcpRequestResult {
		entry := preconnected[index]
		if entry.conn == nil {
			return connectResults[index]
		}
		defer entry.conn.Close()
		return executeRESPOnConn(entry.conn, request, expectedFrames, 1, timeout, entry.connectMS, time.Now())
	})
	return results
}

func executeRESPHold(address string, request []byte, expectedFrames [][]byte, connections int, concurrency int, timeout time.Duration, hold time.Duration) []tcpRequestResult {
	trialStart := time.Now()
	held := make([]heldRESPConn, connections)
	connectResults := runTCPRESPParallel(connections, concurrency, func(index int) tcpRequestResult {
		conn, connectMS, err := dialTCP(address, timeout)
		if err != nil {
			return tcpRequestResult{
				OK:      false,
				Metrics: map[string]float64{"connect_ms": connectMS, "total_ms": elapsedMillis(trialStart), "hold_ms": 0},
				Error:   err.Error(),
			}
		}
		held[index] = heldRESPConn{conn: conn, connectMS: connectMS}
		return tcpRequestResult{
			OK:      true,
			Metrics: map[string]float64{"connect_ms": connectMS, "total_ms": elapsedMillis(trialStart), "hold_ms": 0, "operations": 0},
		}
	})

	connected := 0
	for _, entry := range held {
		if entry.conn != nil {
			connected++
		}
	}
	if connected > 0 && hold > 0 {
		time.Sleep(hold)
	}

	results := runTCPRESPParallel(connections, concurrency, func(index int) tcpRequestResult {
		entry := held[index]
		if entry.conn == nil {
			return connectResults[index]
		}
		defer entry.conn.Close()

		result := executeRESPOnConn(entry.conn, request, expectedFrames, 1, timeout, entry.connectMS, trialStart)
		if result.Metrics == nil {
			result.Metrics = map[string]float64{}
		}
		result.Metrics["hold_ms"] = float64(hold.Milliseconds())
		result.Metrics["connected_before_hold"] = 1
		return result
	})
	return results
}

func executeRESPDialExchange(address string, request []byte, expectedFrames [][]byte, repeats int, timeout time.Duration) tcpRequestResult {
	start := time.Now()
	conn, connectMS, err := dialTCP(address, timeout)
	if err != nil {
		return tcpRequestResult{
			OK:      false,
			Metrics: map[string]float64{"connect_ms": connectMS, "total_ms": elapsedMillis(start)},
			Error:   err.Error(),
		}
	}
	defer conn.Close()
	return executeRESPOnConn(conn, request, expectedFrames, repeats, timeout, connectMS, start)
}

func executeRESPOnConn(conn net.Conn, request []byte, expectedFrames [][]byte, repeats int, timeout time.Duration, connectMS float64, start time.Time) tcpRequestResult {
	if repeats <= 0 {
		repeats = 1
	}
	if err := conn.SetDeadline(time.Now().Add(timeout)); err != nil {
		return tcpRequestResult{
			OK:      false,
			Metrics: map[string]float64{"connect_ms": connectMS, "total_ms": elapsedMillis(start)},
			Error:   err.Error(),
		}
	}
	payload := bytes.Repeat(request, repeats)
	writeStart := time.Now()
	if err := writeFull(conn, payload); err != nil {
		return tcpRequestResult{
			OK:      false,
			Metrics: map[string]float64{"connect_ms": connectMS, "write_ms": elapsedMillis(writeStart), "total_ms": elapsedMillis(start)},
			Error:   err.Error(),
		}
	}
	writeMS := elapsedMillis(writeStart)
	reader := bufio.NewReader(conn)
	expectedCount := len(expectedFrames) * repeats
	readStart := time.Now()
	var firstByteMS float64
	var frameMS float64
	for i := 0; i < expectedCount; i++ {
		if _, err := reader.Peek(1); err != nil {
			return tcpRequestResult{
				OK:      false,
				Metrics: tcpMetrics(connectMS, writeMS, firstByteMS, frameMS, elapsedMillis(start), i),
				Error:   err.Error(),
			}
		}
		if i == 0 {
			firstByteMS = elapsedMillis(readStart)
		}
		frameStart := time.Now()
		frame, err := readRESPFrame(reader, maxTCPFrameBytes)
		frameMS += elapsedMillis(frameStart)
		if err != nil {
			return tcpRequestResult{
				OK:      false,
				Metrics: tcpMetrics(connectMS, writeMS, firstByteMS, frameMS, elapsedMillis(start), i),
				Error:   err.Error(),
			}
		}
		expected := expectedFrames[i%len(expectedFrames)]
		if !bytes.Equal(frame, expected) {
			return tcpRequestResult{
				OK:      false,
				Metrics: tcpMetrics(connectMS, writeMS, firstByteMS, frameMS, elapsedMillis(start), i),
				Error:   fmt.Sprintf("unexpected RESP frame %d: got %q want %q", i, string(frame), string(expected)),
			}
		}
	}
	operations := expectedCount
	if operations == 0 {
		operations = repeats
	}
	return tcpRequestResult{
		OK:      true,
		Metrics: tcpMetrics(connectMS, writeMS, firstByteMS, frameMS, elapsedMillis(start), operations),
	}
}

func executeTCPIdle(address string, timeout time.Duration) tcpRequestResult {
	start := time.Now()
	conn, connectMS, err := dialTCP(address, timeout)
	if err != nil {
		return tcpRequestResult{
			OK:      false,
			Metrics: map[string]float64{"connect_ms": connectMS, "total_ms": elapsedMillis(start)},
			Error:   err.Error(),
		}
	}
	defer conn.Close()
	hold := timeout
	if hold > 100*time.Millisecond {
		hold = 100 * time.Millisecond
	}
	time.Sleep(hold)
	return tcpRequestResult{
		OK:      true,
		Metrics: map[string]float64{"connect_ms": connectMS, "total_ms": elapsedMillis(start), "operations": 1},
	}
}

func dialTCP(address string, timeout time.Duration) (net.Conn, float64, error) {
	start := time.Now()
	conn, err := net.DialTimeout("tcp", address, timeout)
	return conn, elapsedMillis(start), err
}

func writeFull(w io.Writer, data []byte) error {
	for len(data) > 0 {
		n, err := w.Write(data)
		if err != nil {
			return err
		}
		if n == 0 {
			return io.ErrShortWrite
		}
		data = data[n:]
	}
	return nil
}

func tcpSamplesFromResults(subject string, workload string, trialNumber int, phase string, results []tcpRequestResult) []Sample {
	samples := make([]Sample, 0, len(results))
	for _, result := range results {
		samples = append(samples, Sample{
			SampleKind: "tcp_request",
			Subject:    subject,
			Workload:   workload,
			Trial:      trialNumber,
			Phase:      phase,
			OK:         result.OK,
			Metrics:    result.Metrics,
			Error:      result.Error,
		})
	}
	return samples
}

func tcpTrialFromSamples(subject string, workload Workload, trialNumber int, phase string, samples []Sample, elapsedMS float64) Trial {
	metrics := map[string]float64{
		"elapsed_ms":  elapsedMS,
		"connections": float64(tcpConnections(workload)),
		"concurrency": float64(tcpConcurrency(workload, tcpConnections(workload))),
	}
	ok := true
	var firstError string
	var operations float64
	var failed float64
	var connectedBeforeHold float64
	values := map[string][]float64{}
	for _, sample := range samples {
		if !sample.OK {
			ok = false
			failed++
			if firstError == "" {
				firstError = sample.Error
			}
		}
		if sample.Metrics != nil {
			operations += sample.Metrics["operations"]
			for metric, value := range sample.Metrics {
				if metric == "operations" {
					continue
				}
				if metric == "connected_before_hold" {
					connectedBeforeHold += value
					continue
				}
				values[metric] = append(values[metric], value)
			}
		}
	}
	metrics["operations"] = operations
	metrics["failed_operations"] = failed
	if connectedBeforeHold > 0 {
		metrics["connected_before_hold"] = connectedBeforeHold
	}
	if elapsedMS > 0 {
		metrics["ops_per_second"] = operations / (elapsedMS / 1000.0)
	}
	for metric, metricValues := range values {
		sort.Float64s(metricValues)
		metrics[metric] = percentile(metricValues, 50)
	}
	return Trial{
		Subject:  subject,
		Workload: workload.Name,
		Trial:    trialNumber,
		Phase:    phase,
		OK:       ok,
		Metrics:  metrics,
		Error:    firstError,
	}
}

func tcpMetrics(connectMS, writeMS, firstByteMS, frameMS, totalMS float64, operations int) map[string]float64 {
	return map[string]float64{
		"connect_ms":    connectMS,
		"write_ms":      writeMS,
		"first_byte_ms": firstByteMS,
		"frame_ms":      frameMS,
		"total_ms":      totalMS,
		"operations":    float64(operations),
	}
}

func tcpWorkloadMode(workload Workload) string {
	if workload.Mode == "" {
		return respModeOneShot
	}
	return workload.Mode
}

func isSupportedTCPRESPMode(mode string) bool {
	switch mode {
	case respModeOneShot, respModePreconnectThenFire, respModePipeline, respModeIdle, respModeHold:
		return true
	default:
		return false
	}
}

func tcpConnections(workload Workload) int {
	if workload.Connections > 0 {
		return workload.Connections
	}
	if workload.Operations > 0 {
		return workload.Operations
	}
	return 1
}

func tcpConcurrency(workload Workload, connections int) int {
	if workload.Concurrency > 0 {
		if workload.Concurrency > connections {
			return connections
		}
		return workload.Concurrency
	}
	return connections
}

func tcpRequestsPerConnection(workload Workload) int {
	if workload.RequestsPerConnection > 0 {
		return workload.RequestsPerConnection
	}
	return 1
}

func tcpTimeout(workload Workload) time.Duration {
	if workload.TimeoutMS > 0 {
		return time.Duration(workload.TimeoutMS) * time.Millisecond
	}
	return defaultTCPTimeout
}

func tcpHoldDuration(workload Workload) time.Duration {
	if workload.HoldMS > 0 {
		return time.Duration(workload.HoldMS) * time.Millisecond
	}
	return tcpTimeout(workload)
}

func elapsedMillis(start time.Time) float64 {
	return float64(time.Since(start).Microseconds()) / 1000.0
}

type respAccumulator struct {
	bytes.Buffer
	max int
}

func (a *respAccumulator) addByte(b byte) error {
	if a.Len()+1 > a.max {
		return errors.New("RESP frame exceeds maximum size")
	}
	return a.WriteByte(b)
}

func (a *respAccumulator) addBytes(data []byte) error {
	if a.Len()+len(data) > a.max {
		return errors.New("RESP frame exceeds maximum size")
	}
	_, err := a.Write(data)
	return err
}

func readRESPFrame(reader *bufio.Reader, maxBytes int) ([]byte, error) {
	acc := &respAccumulator{max: maxBytes}
	if err := readRESPValue(reader, acc); err != nil {
		return nil, err
	}
	return acc.Bytes(), nil
}

func readRESPValue(reader *bufio.Reader, acc *respAccumulator) error {
	prefix, err := reader.ReadByte()
	if err != nil {
		return err
	}
	if err := acc.addByte(prefix); err != nil {
		return err
	}
	switch prefix {
	case '+', '-', ':':
		_, err := readRESPLine(reader, acc)
		return err
	case '$':
		line, err := readRESPLine(reader, acc)
		if err != nil {
			return err
		}
		length, err := strconv.Atoi(line)
		if err != nil {
			return fmt.Errorf("invalid RESP bulk string length %q", line)
		}
		if length < -1 {
			return fmt.Errorf("invalid RESP bulk string length %d", length)
		}
		if length == -1 {
			return nil
		}
		if length > acc.max-acc.Len()-2 {
			return errors.New("RESP frame exceeds maximum size")
		}
		body := make([]byte, length+2)
		if _, err := io.ReadFull(reader, body); err != nil {
			return err
		}
		if len(body) < 2 || body[len(body)-2] != '\r' || body[len(body)-1] != '\n' {
			return errors.New("RESP bulk string missing CRLF terminator")
		}
		return acc.addBytes(body)
	case '*':
		line, err := readRESPLine(reader, acc)
		if err != nil {
			return err
		}
		count, err := strconv.Atoi(line)
		if err != nil {
			return fmt.Errorf("invalid RESP array length %q", line)
		}
		if count < -1 {
			return fmt.Errorf("invalid RESP array length %d", count)
		}
		if count > acc.max/3 {
			return errors.New("RESP array exceeds maximum size")
		}
		for i := 0; i < count; i++ {
			if err := readRESPValue(reader, acc); err != nil {
				return err
			}
		}
		return nil
	default:
		return fmt.Errorf("unknown RESP prefix %q", prefix)
	}
}

func readRESPLine(reader *bufio.Reader, acc *respAccumulator) (string, error) {
	var line []byte
	for {
		fragment, err := reader.ReadSlice('\n')
		line = append(line, fragment...)
		if acc.Len()+len(line) > acc.max {
			return "", errors.New("RESP frame exceeds maximum size")
		}
		if err == nil {
			break
		}
		if !errors.Is(err, bufio.ErrBufferFull) {
			return "", err
		}
	}
	if len(line) < 2 || line[len(line)-2] != '\r' || line[len(line)-1] != '\n' {
		return "", errors.New("RESP line missing CRLF terminator")
	}
	if err := acc.addBytes(line); err != nil {
		return "", err
	}
	return string(line[:len(line)-2]), nil
}

func splitRESPFrames(data []byte) ([][]byte, error) {
	if len(data) == 0 {
		return nil, nil
	}
	reader := bufio.NewReader(bytes.NewReader(data))
	var frames [][]byte
	for {
		_, err := reader.Peek(1)
		if errors.Is(err, io.EOF) {
			return frames, nil
		}
		if err != nil {
			return nil, err
		}
		frame, err := readRESPFrame(reader, maxTCPFrameBytes)
		if err != nil {
			return nil, err
		}
		frames = append(frames, frame)
	}
}

type commandResult struct {
	OK        bool
	ElapsedMS float64
	Stdout    string
	Stderr    string
	Error     string
}

func runShell(command, dir string, timeout time.Duration) commandResult {
	if timeout <= 0 {
		timeout = 60 * time.Second
	}
	ctx, cancel := context.WithTimeout(context.Background(), timeout)
	defer cancel()
	start := time.Now()
	var cmd *exec.Cmd
	if runtime.GOOS == "windows" {
		cmd = exec.CommandContext(ctx, "cmd", "/C", command)
	} else {
		cmd = exec.CommandContext(ctx, "sh", "-c", command)
	}
	cmd.Dir = dir
	out, err := cmd.CombinedOutput()
	elapsed := float64(time.Since(start).Microseconds()) / 1000.0
	result := commandResult{
		OK:        err == nil,
		ElapsedMS: elapsed,
		Stdout:    string(out),
	}
	if ctx.Err() == context.DeadlineExceeded {
		result.Error = "timeout"
		result.OK = false
	} else if err != nil {
		result.Error = err.Error()
	}
	return result
}

func runProcess(dir string, timeout time.Duration, name string, args ...string) commandResult {
	if timeout <= 0 {
		timeout = 60 * time.Second
	}
	ctx, cancel := context.WithTimeout(context.Background(), timeout)
	defer cancel()
	start := time.Now()
	cmd := exec.CommandContext(ctx, name, args...)
	cmd.Dir = dir
	var stdout bytes.Buffer
	var stderr bytes.Buffer
	cmd.Stdout = &stdout
	cmd.Stderr = &stderr
	err := cmd.Run()
	elapsed := float64(time.Since(start).Microseconds()) / 1000.0
	result := commandResult{
		OK:        err == nil,
		ElapsedMS: elapsed,
		Stdout:    stdout.String(),
		Stderr:    stderr.String(),
	}
	if ctx.Err() == context.DeadlineExceeded {
		result.Error = "timeout"
		result.OK = false
	} else if err != nil {
		result.Error = err.Error()
	}
	return result
}

func summarizeTrials(trials []Trial) []Summary {
	values := map[string][]float64{}
	for _, trial := range trials {
		for metric, value := range trial.Metrics {
			key := trial.Subject + "\x00" + trial.Workload + "\x00" + metric
			values[key] = append(values[key], value)
		}
	}
	keys := make([]string, 0, len(values))
	for key := range values {
		keys = append(keys, key)
	}
	sort.Strings(keys)
	summaries := make([]Summary, 0, len(keys))
	for _, key := range keys {
		parts := strings.Split(key, "\x00")
		stats := computeStats(values[key])
		stats.Subject = parts[0]
		stats.Workload = parts[1]
		stats.Metric = parts[2]
		summaries = append(summaries, stats)
	}
	return summaries
}

func computeStats(values []float64) Summary {
	clean := append([]float64(nil), values...)
	sort.Float64s(clean)
	n := len(clean)
	if n == 0 {
		return Summary{}
	}
	mean := average(clean)
	median := percentile(clean, 50)
	deviations := make([]float64, n)
	var variance float64
	for i, value := range clean {
		delta := value - mean
		variance += delta * delta
		deviations[i] = math.Abs(value - median)
	}
	sort.Float64s(deviations)
	meanLow, meanHigh := bootstrapCI(clean, average)
	medianLow, medianHigh := bootstrapCI(clean, func(xs []float64) float64 {
		sort.Float64s(xs)
		return percentile(xs, 50)
	})
	return Summary{
		Count:        n,
		Min:          clean[0],
		Max:          clean[n-1],
		Mean:         mean,
		Median:       median,
		Stddev:       math.Sqrt(variance / float64(n)),
		MAD:          percentile(deviations, 50),
		P50:          median,
		P90:          percentile(clean, 90),
		P95:          percentile(clean, 95),
		P99:          percentile(clean, 99),
		MeanCILow:    meanLow,
		MeanCIHigh:   meanHigh,
		MedianCILow:  medianLow,
		MedianCIHigh: medianHigh,
	}
}

func average(values []float64) float64 {
	if len(values) == 0 {
		return 0
	}
	var total float64
	for _, value := range values {
		total += value
	}
	return total / float64(len(values))
}

func percentile(sorted []float64, p float64) float64 {
	if len(sorted) == 0 {
		return 0
	}
	if len(sorted) == 1 {
		return sorted[0]
	}
	pos := (p / 100.0) * float64(len(sorted)-1)
	lower := int(math.Floor(pos))
	upper := int(math.Ceil(pos))
	if lower == upper {
		return sorted[lower]
	}
	weight := pos - float64(lower)
	return sorted[lower]*(1-weight) + sorted[upper]*weight
}

func bootstrapCI(values []float64, estimator func([]float64) float64) (float64, float64) {
	if len(values) == 0 {
		return 0, 0
	}
	if len(values) == 1 {
		return values[0], values[0]
	}
	seedBytes := sha256.Sum256([]byte(fmt.Sprint(values)))
	seed := int64(0)
	for i := 0; i < 8; i++ {
		seed = seed<<8 + int64(seedBytes[i])
	}
	rng := rand.New(rand.NewSource(seed))
	const rounds = 1000
	estimates := make([]float64, rounds)
	sample := make([]float64, len(values))
	for i := 0; i < rounds; i++ {
		for j := range sample {
			sample[j] = values[rng.Intn(len(values))]
		}
		cp := append([]float64(nil), sample...)
		estimates[i] = estimator(cp)
	}
	sort.Float64s(estimates)
	return percentile(estimates, 2.5), percentile(estimates, 97.5)
}

func renderReport(summary SummaryFile) string {
	var b strings.Builder
	fmt.Fprintf(&b, "# Benchmark Report: %s\n\n", summary.ManifestName)
	fmt.Fprintf(&b, "Generated: %s\n\n", summary.GeneratedAt)
	if len(summary.Summaries) == 0 {
		b.WriteString("No measured samples were recorded.\n")
		return b.String()
	}
	b.WriteString("| Subject | Workload | Metric | Count | Median | p90 | p99 | Mean 95% CI |\n")
	b.WriteString("|---|---|---|---:|---:|---:|---:|---:|\n")
	for _, s := range summary.Summaries {
		fmt.Fprintf(
			&b,
			"| %s | %s | %s | %d | %.3f | %.3f | %.3f | %.3f..%.3f |\n",
			s.Subject,
			s.Workload,
			s.Metric,
			s.Count,
			s.Median,
			s.P90,
			s.P99,
			s.MeanCILow,
			s.MeanCIHigh,
		)
	}
	return b.String()
}

func captureEnvironment(root string) Environment {
	hostname, _ := os.Hostname()
	wd, _ := os.Getwd()
	return Environment{
		GeneratedAt:      time.Now().UTC().Format(time.RFC3339),
		Hostname:         hostname,
		OS:               runtime.GOOS,
		Arch:             runtime.GOARCH,
		CPUs:             runtime.NumCPU(),
		WorkingDirectory: wd,
		GitCommit:        commandOutput(root, "git", "rev-parse", "HEAD"),
		GitBranch:        commandOutput(root, "git", "rev-parse", "--abbrev-ref", "HEAD"),
		GitDirty:         commandOutput(root, "git", "status", "--short") != "",
		GitRemote:        commandOutput(root, "git", "remote", "get-url", "origin"),
		GoVersion:        runtime.Version(),
		Kernel:           kernelDescription(),
		ULimitOpenFiles:  ulimitOpenFiles(),
	}
}

func commandOutput(dir string, name string, args ...string) string {
	ctx, cancel := context.WithTimeout(context.Background(), 2*time.Second)
	defer cancel()
	cmd := exec.CommandContext(ctx, name, args...)
	cmd.Dir = dir
	out, err := cmd.Output()
	if err != nil {
		return ""
	}
	return strings.TrimSpace(string(out))
}

func kernelDescription() string {
	if runtime.GOOS == "windows" {
		return commandOutput(".", "cmd", "/C", "ver")
	}
	return commandOutput(".", "uname", "-a")
}

func ulimitOpenFiles() string {
	if runtime.GOOS == "windows" {
		return ""
	}
	return commandOutput(".", "sh", "-c", "ulimit -n")
}

func defaultResultDir(name string) string {
	stamp := time.Now().UTC().Format("20060102T150405Z")
	return filepath.Join("benchmark-results", stamp+"-"+safeName(name))
}

func safeName(name string) string {
	var b strings.Builder
	for _, r := range strings.ToLower(name) {
		if (r >= 'a' && r <= 'z') || (r >= '0' && r <= '9') {
			b.WriteRune(r)
		} else if r == '-' || r == '_' || r == '.' {
			b.WriteRune(r)
		} else {
			b.WriteRune('-')
		}
	}
	out := strings.Trim(b.String(), "-")
	if out == "" {
		return "unnamed"
	}
	return out
}

func copyFile(src, dst string) error {
	in, err := os.Open(src)
	if err != nil {
		return err
	}
	defer in.Close()
	out, err := os.Create(dst)
	if err != nil {
		return err
	}
	defer out.Close()
	if _, err := io.Copy(out, in); err != nil {
		return err
	}
	return out.Close()
}

func writeJSON(path string, value any) error {
	data, err := json.MarshalIndent(value, "", "  ")
	if err != nil {
		return err
	}
	data = append(data, '\n')
	return os.WriteFile(path, data, 0o644)
}

func writeJSONLine(w io.Writer, value any) error {
	data, err := json.Marshal(value)
	if err != nil {
		return err
	}
	_, err = w.Write(append(data, '\n'))
	return err
}

func readSummary(path string) (SummaryFile, error) {
	var summary SummaryFile
	data, err := os.ReadFile(path)
	if err != nil {
		return summary, err
	}
	if err := json.Unmarshal(data, &summary); err != nil {
		return summary, err
	}
	return summary, nil
}

func summaryByKey(summaries []Summary, metric string) map[string]Summary {
	out := map[string]Summary{}
	for _, summary := range summaries {
		if summary.Metric == metric {
			out[summary.Subject+"/"+summary.Workload] = summary
		}
	}
	return out
}

func sortedCommonKeys(left, right map[string]Summary) []string {
	var keys []string
	for key := range left {
		if _, ok := right[key]; ok {
			keys = append(keys, key)
		}
	}
	sort.Strings(keys)
	return keys
}
