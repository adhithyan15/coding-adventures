// benchmark-tool is the repository's language-neutral benchmark runner.
//
// The first slice deliberately keeps the hot path simple:
//   - read the benchmark manifest format from code/specs/benchmarking-tools.md
//   - validate ambiguous benchmark definitions before they become folklore
//   - run command-style subjects with warmup and measured trials
//   - write raw samples, trial rows, environment metadata, summaries, and a
//     Markdown report into one self-contained result directory
//
// TCP/RESP load generation is intentionally left for the next program slice.
// This tool already validates those manifests so the contract can stabilize
// before we teach a load generator to execute them.
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
	"os"
	"os/exec"
	"path/filepath"
	"runtime"
	"sort"
	"strconv"
	"strings"
	"time"

	_ "embed"

	clibuilder "github.com/adhithyan15/coding-adventures/code/packages/go/cli-builder"
)

const version = "0.2.0"

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
	left, err := readSummary(filepath.Join(leftDir, "summary.json"))
	if err != nil {
		return err
	}
	right, err := readSummary(filepath.Join(rightDir, "summary.json"))
	if err != nil {
		return err
	}
	leftMap := summaryByKey(left.Summaries, metric)
	rightMap := summaryByKey(right.Summaries, metric)
	keys := sortedCommonKeys(leftMap, rightMap)
	if len(keys) == 0 {
		return fmt.Errorf("no common %q summaries found", metric)
	}
	for _, key := range keys {
		a := leftMap[key]
		b := rightMap[key]
		diff := b.Median - a.Median
		rel := 0.0
		if a.Median != 0 {
			rel = diff / a.Median * 100
		}
		fmt.Fprintf(stdout, "%s %s median: %.3f -> %.3f (%+.2f%%)\n", key, metric, a.Median, b.Median, rel)
	}
	return nil
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
		if strings.Contains(w.Driver, "resp") && w.ReadMode != "resp-frame" {
			problems = append(problems, where+" must use read_mode = \"resp-frame\" for RESP workloads")
		}
		if strings.Contains(w.Driver, "resp") && w.Expect == "" {
			problems = append(problems, where+".expect is required for RESP workloads")
		}
		if w.TimeoutMS < 0 {
			problems = append(problems, where+".timeout_ms must be non-negative")
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
		for _, workload := range manifest.Workloads {
			if workload.Driver != "command" {
				fmt.Fprintf(os.Stderr, "Skipping workload %s: driver %s is not implemented in phase one\n", workload.Name, workload.Driver)
				continue
			}
			allTrials, err := runCommandWorkload(prepared, workload, manifest.Defaults, samplesFile, trialsFile)
			if err != nil {
				if manifest.Defaults.FailFast {
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
