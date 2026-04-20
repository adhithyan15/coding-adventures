package main

import (
	"bytes"
	"encoding/json"
	"math"
	"os"
	"path/filepath"
	"runtime"
	"strconv"
	"strings"
	"testing"
	"time"

	clibuilder "github.com/adhithyan15/coding-adventures/code/packages/go/cli-builder"
)

func TestParseAndValidateCommandManifest(t *testing.T) {
	manifest, err := LoadManifest("../../../benchmarks/examples/command/benchmark.toml")
	if err != nil {
		t.Fatalf("load command manifest: %v", err)
	}
	if err := ValidateManifest(manifest); err != nil {
		t.Fatalf("command manifest should validate: %v", err)
	}
	if manifest.Name != "benchmark-tool-command-smoke" {
		t.Fatalf("manifest name = %q", manifest.Name)
	}
	if got := manifest.Defaults.MeasurementTrials; got != 3 {
		t.Fatalf("measurement trials = %d", got)
	}
	if got := manifest.Subjects[0].Command; got != "go version" {
		t.Fatalf("subject command = %q", got)
	}
}

func TestMiniRedisManifestValidatesRespWorkloads(t *testing.T) {
	manifest, err := LoadManifest("../../../benchmarks/mini-redis/benchmark.toml")
	if err != nil {
		t.Fatalf("load mini redis manifest: %v", err)
	}
	if err := ValidateManifest(manifest); err != nil {
		t.Fatalf("mini redis manifest should validate: %v", err)
	}
	if manifest.Subjects[0].Kind != "service" {
		t.Fatalf("mini redis subject kind = %q", manifest.Subjects[0].Kind)
	}
	if manifest.Workloads[0].Driver != "tcp-resp" {
		t.Fatalf("mini redis workload driver = %q", manifest.Workloads[0].Driver)
	}
	if manifest.Workloads[0].Request != "*1\r\n$4\r\nPING\r\n" {
		t.Fatalf("RESP request was not unescaped correctly: %q", manifest.Workloads[0].Request)
	}
}

func TestValidateRejectsServiceWithoutReadyCheck(t *testing.T) {
	manifest := validTinyManifest()
	manifest.Subjects[0].Kind = "service"
	manifest.Subjects[0].ReadyCheck = ""

	err := ValidateManifest(manifest)
	if err == nil {
		t.Fatal("expected validation error")
	}
	if !strings.Contains(err.Error(), "ready_check") {
		t.Fatalf("validation error should mention ready_check: %v", err)
	}
}

func TestValidateRejectsRespWithoutFrameReads(t *testing.T) {
	manifest := validTinyManifest()
	manifest.Workloads[0].Driver = "tcp-resp"
	manifest.Workloads[0].ReadMode = "eof"
	manifest.Workloads[0].Expect = "+PONG\r\n"

	err := ValidateManifest(manifest)
	if err == nil {
		t.Fatal("expected validation error")
	}
	if !strings.Contains(err.Error(), "resp-frame") {
		t.Fatalf("validation error should mention resp-frame: %v", err)
	}
}

func TestValidateRejectsEOFReadsWithoutProtocolClose(t *testing.T) {
	manifest := validTinyManifest()
	manifest.Workloads[0].ReadMode = "eof"
	manifest.Workloads[0].ProtocolClosesAfterResponse = false

	err := ValidateManifest(manifest)
	if err == nil {
		t.Fatal("expected validation error")
	}
	if !strings.Contains(err.Error(), "protocol_closes_after_response") {
		t.Fatalf("validation error should mention protocol close: %v", err)
	}
}

func TestRunCommandManifestWritesBenchmarkArtifacts(t *testing.T) {
	dir := t.TempDir()
	manifestPath := filepath.Join(dir, "benchmark.toml")
	resultDir := filepath.Join(dir, "results")
	if err := os.WriteFile(manifestPath, []byte(commandManifestText("unit-command-benchmark")), 0o644); err != nil {
		t.Fatalf("write manifest: %v", err)
	}
	manifest, err := LoadManifest(manifestPath)
	if err != nil {
		t.Fatalf("load manifest: %v", err)
	}
	if err := ValidateManifest(manifest); err != nil {
		t.Fatalf("manifest should validate: %v", err)
	}
	if err := RunManifest(manifestPath, manifest, resultDir); err != nil {
		t.Fatalf("run manifest: %v", err)
	}

	for _, name := range []string{"manifest.toml", "environment.json", "samples.jsonl", "trials.jsonl", "summary.json", "report.md"} {
		path := filepath.Join(resultDir, name)
		if _, err := os.Stat(path); err != nil {
			t.Fatalf("expected artifact %s: %v", name, err)
		}
	}
	summary := readSummaryForTest(t, filepath.Join(resultDir, "summary.json"))
	if len(summary.Summaries) == 0 {
		t.Fatal("expected summary rows")
	}
	if !hasSummaryMetric(summary, "elapsed_ms") {
		t.Fatalf("expected elapsed_ms summary, got %#v", summary.Summaries)
	}
	if !hasSummaryMetric(summary, "ops_per_second") {
		t.Fatalf("expected ops_per_second summary, got %#v", summary.Summaries)
	}
	trials := readLinesForTest(t, filepath.Join(resultDir, "trials.jsonl"))
	if len(trials) != 3 {
		t.Fatalf("expected 3 trials including warmup, got %d", len(trials))
	}
}

func TestCLICommandFunctionsExerciseHappyPaths(t *testing.T) {
	dir := t.TempDir()
	manifestPath := filepath.Join(dir, "benchmark.toml")
	leftDir := filepath.Join(dir, "left")
	rightDir := filepath.Join(dir, "right")
	if err := os.WriteFile(manifestPath, []byte(commandManifestText("cli-command-benchmark")), 0o644); err != nil {
		t.Fatalf("write manifest: %v", err)
	}
	var stdout bytes.Buffer
	var stderr bytes.Buffer

	if code := runCLI(cliSpec, []string{"benchmark-tool", "validate", manifestPath}, &stdout, &stderr); code != 0 {
		t.Fatalf("validate command exit=%d stderr=%s", code, stderr.String())
	}
	if code := runCLI(cliSpec, []string{"benchmark-tool", "run", manifestPath, "--out", leftDir, "--warmup", "0", "--trials", "1"}, &stdout, &stderr); code != 0 {
		t.Fatalf("run command left exit=%d stderr=%s", code, stderr.String())
	}
	if code := runCLI(cliSpec, []string{"benchmark-tool", "run", "--out", rightDir, "--warmup=0", "--trials=1", manifestPath}, &stdout, &stderr); code != 0 {
		t.Fatalf("run command right exit=%d stderr=%s", code, stderr.String())
	}
	if code := runCLI(cliSpec, []string{"benchmark-tool", "report", leftDir}, &stdout, &stderr); code != 0 {
		t.Fatalf("report command exit=%d stderr=%s", code, stderr.String())
	}
	if code := runCLI(cliSpec, []string{"benchmark-tool", "compare", leftDir, "--metric", "elapsed_ms", rightDir}, &stdout, &stderr); code != 0 {
		t.Fatalf("compare command exit=%d stderr=%s", code, stderr.String())
	}
	if !strings.Contains(stdout.String(), "manifest \"cli-command-benchmark\" is valid") {
		t.Fatalf("stdout should include validation success: %s", stdout.String())
	}
}

func TestCLICommandFunctionsRejectBadArguments(t *testing.T) {
	var stdout bytes.Buffer
	var stderr bytes.Buffer
	if code := runCLI(cliSpec, []string{"benchmark-tool", "validate"}, &stdout, &stderr); code == 0 {
		t.Fatal("validate without args should fail")
	}
	if code := runCLI(cliSpec, []string{"benchmark-tool", "run"}, &stdout, &stderr); code == 0 {
		t.Fatal("run without args should fail")
	}
	if code := runCLI(cliSpec, []string{"benchmark-tool", "report"}, &stdout, &stderr); code == 0 {
		t.Fatal("report without args should fail")
	}
	if code := runCLI(cliSpec, []string{"benchmark-tool", "compare"}, &stdout, &stderr); code == 0 {
		t.Fatal("compare without args should fail")
	}
	if code := runCLI(cliSpec, []string{"benchmark-tool", "compare", t.TempDir(), t.TempDir()}, &stdout, &stderr); code == 0 {
		t.Fatal("compare without summaries should fail")
	}
}

func TestDoctorAndUsageAreAvailable(t *testing.T) {
	var b bytes.Buffer
	if err := doctor(&b); err != nil {
		t.Fatalf("doctor: %v", err)
	}
	if !strings.Contains(b.String(), "doctor: ok") {
		t.Fatalf("doctor output = %q", b.String())
	}
	var stdout bytes.Buffer
	var stderr bytes.Buffer
	if code := runCLI(cliSpec, []string{"benchmark-tool", "--help"}, &stdout, &stderr); code != 0 {
		t.Fatalf("help exit=%d stderr=%s", code, stderr.String())
	}
	if !strings.Contains(stdout.String(), "benchmark-tool") {
		t.Fatalf("help did not describe the CLI: %q", stdout.String())
	}
}

func TestIntFlagConversionsAreBoundsChecked(t *testing.T) {
	result := &clibuilder.ParseResult{
		Flags: map[string]any{
			"int64-ok":      int64(7),
			"float-ok":      float64(9),
			"float-not-int": float64(9.5),
		},
	}
	if got := intFlag(result, "int64-ok", -1); got != 7 {
		t.Fatalf("int64 flag = %d", got)
	}
	if got := intFlag(result, "float-ok", -1); got != 9 {
		t.Fatalf("float64 flag = %d", got)
	}
	if got := intFlag(result, "float-not-int", -1); got != -1 {
		t.Fatalf("non-integer float flag = %d", got)
	}

	_, ok := intFromFloat64(math.Inf(1))
	if ok {
		t.Fatal("infinite float should not convert to int")
	}
}

func TestRunBuildWritesLogsAndPropagatesFailures(t *testing.T) {
	dir := t.TempDir()
	okSubject := Subject{
		Name:             "builder",
		WorkingDirectory: dir,
		Build:            "printf build-ok",
	}
	if err := runBuild(okSubject, dir, dir); err != nil {
		t.Fatalf("run build: %v", err)
	}
	logData, err := os.ReadFile(filepath.Join(dir, "build.log"))
	if err != nil {
		t.Fatalf("read build log: %v", err)
	}
	if !strings.Contains(string(logData), "build-ok") {
		t.Fatalf("build log should include stdout: %q", string(logData))
	}

	badSubject := okSubject
	badSubject.Build = shellExitCommand(7)
	if err := runBuild(badSubject, dir, dir); err == nil {
		t.Fatal("expected failing build to return an error")
	}
}

func TestSubjectWorkingDirectoryResolvesFromBenchmarkRoot(t *testing.T) {
	root := t.TempDir()
	subject := Subject{WorkingDirectory: "code/programs/rust/mini-redis"}
	got := resolveSubjectWorkingDirectory(subject, root)
	want := filepath.Join(root, "code/programs/rust/mini-redis")
	if got != want {
		t.Fatalf("working directory = %q, want %q", got, want)
	}

	absolute := Subject{WorkingDirectory: root}
	if got := resolveSubjectWorkingDirectory(absolute, "/elsewhere"); got != root {
		t.Fatalf("absolute working directory changed to %q", got)
	}
}

func TestRunShellReportsFailureAndTimeout(t *testing.T) {
	failed := runShell(shellExitCommand(9), ".", time.Second)
	if failed.OK {
		t.Fatal("expected shell failure")
	}
	if failed.Error == "" {
		t.Fatal("expected shell failure to include an error")
	}

	timedOut := runShell(shellSleepCommand(), ".", time.Millisecond)
	if timedOut.OK {
		t.Fatal("expected timeout")
	}
	if timedOut.Error != "timeout" {
		t.Fatalf("timeout error = %q", timedOut.Error)
	}
}

func TestParserRejectsMalformedManifests(t *testing.T) {
	cases := []string{
		`[unknown]`,
		`name "missing equals"`,
		`name = `,
		`unknown = "value"`,
		`name = { nope = true }`,
	}
	for _, input := range cases {
		if _, err := parseManifest(strings.NewReader(input)); err == nil {
			t.Fatalf("expected parse error for %q", input)
		}
	}
}

func TestParserHandlesCommentsArraysAndPrimitiveValues(t *testing.T) {
	value, err := parseValue(`["alpha", "bravo, charlie", 3, true]`)
	if err != nil {
		t.Fatalf("parse array: %v", err)
	}
	parts := value.([]any)
	if len(parts) != 4 {
		t.Fatalf("array length = %d", len(parts))
	}
	if got := stripComment(`name = "hash # inside" # outside`); got != `name = "hash # inside" ` {
		t.Fatalf("strip comment = %q", got)
	}
}

func TestReportHelpersHandleEmptyAndMissingCommonData(t *testing.T) {
	report := renderReport(SummaryFile{ManifestName: "empty", GeneratedAt: "now"})
	if !strings.Contains(report, "No measured samples") {
		t.Fatalf("empty report = %q", report)
	}
	left := summaryByKey([]Summary{{Subject: "a", Workload: "w", Metric: "elapsed_ms"}}, "ops_per_second")
	right := summaryByKey([]Summary{{Subject: "a", Workload: "w", Metric: "elapsed_ms"}}, "ops_per_second")
	if keys := sortedCommonKeys(left, right); len(keys) != 0 {
		t.Fatalf("expected no common keys, got %#v", keys)
	}
}

func TestNamesAndDefaultResultDirsAreFilesystemFriendly(t *testing.T) {
	if got := safeName("Hello, TCP/IP World!"); got != "hello--tcp-ip-world" {
		t.Fatalf("safe name = %q", got)
	}
	if got := safeName("!!!"); got != "unnamed" {
		t.Fatalf("empty safe name = %q", got)
	}
	if got := defaultResultDir("Hello World"); !strings.Contains(got, "benchmark-results") || !strings.Contains(got, "hello-world") {
		t.Fatalf("default result dir = %q", got)
	}
}

func TestCompareUsesCommonSummaryKeys(t *testing.T) {
	left := map[string]Summary{
		"alpha/work": {Subject: "alpha", Workload: "work", Metric: "elapsed_ms", Median: 10},
		"beta/work":  {Subject: "beta", Workload: "work", Metric: "elapsed_ms", Median: 20},
	}
	right := map[string]Summary{
		"alpha/work": {Subject: "alpha", Workload: "work", Metric: "elapsed_ms", Median: 11},
		"zeta/work":  {Subject: "zeta", Workload: "work", Metric: "elapsed_ms", Median: 3},
	}

	keys := sortedCommonKeys(left, right)
	if len(keys) != 1 || keys[0] != "alpha/work" {
		t.Fatalf("common keys = %#v", keys)
	}
}

func TestComputeStatsReportsDistributionShape(t *testing.T) {
	stats := computeStats([]float64{1, 2, 3, 4, 100})
	if stats.Count != 5 {
		t.Fatalf("count = %d", stats.Count)
	}
	if stats.Median != 3 {
		t.Fatalf("median = %f", stats.Median)
	}
	if stats.P90 <= stats.Median {
		t.Fatalf("p90 should exceed median: %#v", stats)
	}
	if stats.MeanCILow > stats.MeanCIHigh {
		t.Fatalf("invalid mean confidence interval: %#v", stats)
	}
}

func commandManifestText(name string) string {
	return `name = "` + name + `"

[defaults]
warmup_trials = 1
measurement_trials = 2
cooldown_ms = 0
fail_fast = true

[[subjects]]
name = "print"
kind = "command"
prebuilt = true
command = "printf benchmark-tool"

[[workloads]]
name = "print-once"
driver = "command"
operations = 1
timeout_ms = 10000
`
}

func shellExitCommand(code int) string {
	if runtime.GOOS == "windows" {
		return "exit " + strconv.Itoa(code)
	}
	return "exit " + strconv.Itoa(code)
}

func shellSleepCommand() string {
	if runtime.GOOS == "windows" {
		return "ping -n 2 127.0.0.1 >NUL"
	}
	return "sleep 1"
}

func validTinyManifest() Manifest {
	return Manifest{
		Name: "tiny",
		Defaults: Defaults{
			WarmupTrials:      0,
			MeasurementTrials: 1,
		},
		Subjects: []Subject{{
			Name:     "subject",
			Kind:     "command",
			Prebuilt: true,
			Command:  "printf ok",
		}},
		Workloads: []Workload{{
			Name:      "workload",
			Driver:    "command",
			TimeoutMS: 1000,
		}},
	}
}

func readSummaryForTest(t *testing.T, path string) SummaryFile {
	t.Helper()
	data, err := os.ReadFile(path)
	if err != nil {
		t.Fatalf("read summary: %v", err)
	}
	var summary SummaryFile
	if err := json.Unmarshal(data, &summary); err != nil {
		t.Fatalf("decode summary: %v", err)
	}
	return summary
}

func hasSummaryMetric(summary SummaryFile, metric string) bool {
	for _, row := range summary.Summaries {
		if row.Metric == metric {
			return true
		}
	}
	return false
}

func readLinesForTest(t *testing.T, path string) []string {
	t.Helper()
	data, err := os.ReadFile(path)
	if err != nil {
		t.Fatalf("read lines: %v", err)
	}
	text := strings.TrimSpace(string(data))
	if text == "" {
		return nil
	}
	return strings.Split(text, "\n")
}
