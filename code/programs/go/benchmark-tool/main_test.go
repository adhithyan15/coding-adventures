package main

import (
	"bufio"
	"bytes"
	"encoding/json"
	"errors"
	"math"
	"net"
	"os"
	"os/exec"
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

	c10k, err := LoadManifest("../../../benchmarks/mini-redis/c10k-hold.toml")
	if err != nil {
		t.Fatalf("load mini redis c10k manifest: %v", err)
	}
	if err := ValidateManifest(c10k); err != nil {
		t.Fatalf("mini redis c10k manifest should validate: %v", err)
	}
	if c10k.Workloads[len(c10k.Workloads)-1].Mode != respModeHold {
		t.Fatalf("c10k workload mode = %q", c10k.Workloads[len(c10k.Workloads)-1].Mode)
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

func TestValidateRejectsUnsafeTCPRESPShapes(t *testing.T) {
	manifest := validTinyManifest()
	manifest.Subjects[0].Kind = "service"
	manifest.Subjects[0].ReadyCheck = "tcp-connect"
	manifest.Workloads[0] = Workload{
		Name:                  "bad-resp",
		Driver:                respDriver,
		Mode:                  "warp-speed",
		ReadMode:              respReadMode,
		Request:               strings.Repeat("x", maxTCPPayloadBytes+1),
		Expect:                "+PONG\r\n",
		Connections:           maxTCPConnections + 1,
		Concurrency:           maxTCPConcurrency + 1,
		RequestsPerConnection: maxTCPRequestsPerConn + 1,
		HoldMS:                maxTCPHoldMS + 1,
	}

	err := ValidateManifest(manifest)
	if err == nil {
		t.Fatal("expected validation error")
	}
	for _, want := range []string{"mode", "connections", "concurrency", "requests_per_connection", "request", "hold_ms"} {
		if !strings.Contains(err.Error(), want) {
			t.Fatalf("validation error should mention %s: %v", want, err)
		}
	}

	manifest.Workloads[0] = Workload{
		Name:      "idle",
		Driver:    respDriver,
		Mode:      respModeIdle,
		ReadMode:  respReadMode,
		TimeoutMS: 10,
	}
	if err := ValidateManifest(manifest); err != nil {
		t.Fatalf("idle RESP workload should not require expect: %v", err)
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

func TestRunTCPRESPWorkloadsAgainstLocalServer(t *testing.T) {
	address := startRESPTestServer(t)
	dir := t.TempDir()
	samplesPath := filepath.Join(dir, "samples.jsonl")
	trialsPath := filepath.Join(dir, "trials.jsonl")
	samplesFile, err := os.Create(samplesPath)
	if err != nil {
		t.Fatalf("create samples: %v", err)
	}
	defer samplesFile.Close()
	trialsFile, err := os.Create(trialsPath)
	if err != nil {
		t.Fatalf("create trials: %v", err)
	}
	defer trialsFile.Close()
	subject := PreparedSubject{Subject: Subject{Name: "resp-server"}}
	defaults := Defaults{WarmupTrials: 0, MeasurementTrials: 1, CooldownMS: 0, FailFast: true}

	workloads := []Workload{
		{
			Name:        "one-shot",
			Driver:      respDriver,
			Mode:        respModeOneShot,
			ReadMode:    respReadMode,
			Request:     "*1\r\n$4\r\nPING\r\n",
			Expect:      "+PONG\r\n",
			Connections: 16,
			Concurrency: 8,
			TimeoutMS:   2000,
		},
		{
			Name:        "preconnect",
			Driver:      respDriver,
			Mode:        respModePreconnectThenFire,
			ReadMode:    respReadMode,
			Request:     "*1\r\n$4\r\nPING\r\n",
			Expect:      "+PONG\r\n",
			Connections: 16,
			Concurrency: 8,
			TimeoutMS:   2000,
		},
		{
			Name:                  "pipeline",
			Driver:                respDriver,
			Mode:                  respModePipeline,
			ReadMode:              respReadMode,
			Request:               "*3\r\n$3\r\nSET\r\n$3\r\nkey\r\n$5\r\nvalue\r\n*2\r\n$3\r\nGET\r\n$3\r\nkey\r\n",
			Expect:                "+OK\r\n$5\r\nvalue\r\n",
			Connections:           4,
			Concurrency:           2,
			RequestsPerConnection: 5,
			TimeoutMS:             2000,
		},
		{
			Name:        "hold",
			Driver:      respDriver,
			Mode:        respModeHold,
			ReadMode:    respReadMode,
			Request:     "*1\r\n$4\r\nPING\r\n",
			Expect:      "+PONG\r\n",
			Connections: 6,
			Concurrency: 3,
			TimeoutMS:   2000,
			HoldMS:      20,
		},
	}
	for _, workload := range workloads {
		trials, err := runTCPRESPWorkload(subject, workload, defaults, address, samplesFile, trialsFile)
		if err != nil {
			t.Fatalf("run %s: %v", workload.Name, err)
		}
		if len(trials) != 1 {
			t.Fatalf("%s trial count = %d", workload.Name, len(trials))
		}
		if !trials[0].OK {
			t.Fatalf("%s trial failed: %#v", workload.Name, trials[0])
		}
		if trials[0].Metrics["ops_per_second"] <= 0 {
			t.Fatalf("%s should report throughput: %#v", workload.Name, trials[0].Metrics)
		}
		if workload.Mode == respModeHold && trials[0].Metrics["connected_before_hold"] != 6 {
			t.Fatalf("hold should report connected sockets: %#v", trials[0].Metrics)
		}
	}
	if lines := readLinesForTest(t, samplesPath); len(lines) < 42 {
		t.Fatalf("expected per-connection samples, got %d", len(lines))
	}
}

func TestTCPRESPWorkloadFailsWrongResponsesAndSupportsIdle(t *testing.T) {
	address := startRESPTestServer(t)
	dir := t.TempDir()
	samplesFile, err := os.Create(filepath.Join(dir, "samples.jsonl"))
	if err != nil {
		t.Fatalf("create samples: %v", err)
	}
	defer samplesFile.Close()
	trialsFile, err := os.Create(filepath.Join(dir, "trials.jsonl"))
	if err != nil {
		t.Fatalf("create trials: %v", err)
	}
	defer trialsFile.Close()
	subject := PreparedSubject{Subject: Subject{Name: "resp-server"}}
	defaults := Defaults{WarmupTrials: 0, MeasurementTrials: 1, CooldownMS: 0, FailFast: true}

	wrong := Workload{
		Name:        "wrong",
		Driver:      respDriver,
		Mode:        respModeOneShot,
		ReadMode:    respReadMode,
		Request:     "*1\r\n$4\r\nPING\r\n",
		Expect:      "+NOPE\r\n",
		Connections: 2,
		Concurrency: 2,
		TimeoutMS:   2000,
	}
	trials, err := runTCPRESPWorkload(subject, wrong, defaults, address, samplesFile, trialsFile)
	if err != nil {
		t.Fatalf("wrong response run should produce failed trial, not harness error: %v", err)
	}
	if len(trials) != 1 || trials[0].OK {
		t.Fatalf("expected failed correctness trial: %#v", trials)
	}
	if trials[0].Metrics["failed_operations"] == 0 {
		t.Fatalf("expected failed operation metric: %#v", trials[0].Metrics)
	}

	idle := Workload{
		Name:        "idle",
		Driver:      respDriver,
		Mode:        respModeIdle,
		ReadMode:    respReadMode,
		Connections: 2,
		Concurrency: 2,
		TimeoutMS:   20,
	}
	trials, err = runTCPRESPWorkload(subject, idle, defaults, address, samplesFile, trialsFile)
	if err != nil {
		t.Fatalf("idle run: %v", err)
	}
	if len(trials) != 1 || !trials[0].OK {
		t.Fatalf("expected successful idle trial: %#v", trials)
	}
}

func TestRunManifestStartsTCPServiceSubject(t *testing.T) {
	if _, err := exec.LookPath("go"); err != nil {
		t.Skip("go is required for service lifecycle tests")
	}
	dir := t.TempDir()
	resultDir := filepath.Join(dir, "results")
	writeFileForTest(t, filepath.Join(dir, "resp_server.go"), respServerProgramForTest())
	writeFileForTest(t, filepath.Join(dir, "benchmark.toml"), tcpServiceManifestText(dir))
	manifestPath := filepath.Join(dir, "benchmark.toml")
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
	summary := readSummaryForTest(t, filepath.Join(resultDir, "summary.json"))
	if !hasSummaryMetric(summary, "ops_per_second") {
		t.Fatalf("expected TCP throughput summary, got %#v", summary.Summaries)
	}
	if !hasSummaryMetric(summary, "first_byte_ms") {
		t.Fatalf("expected TCP latency summary, got %#v", summary.Summaries)
	}
	if _, err := os.Stat(filepath.Join(resultDir, "subjects", "resp-service", "service.log")); err != nil {
		t.Fatalf("expected service log: %v", err)
	}
}

func TestRESPFrameParsingHandlesNestedFramesAndLimits(t *testing.T) {
	frames, err := splitRESPFrames([]byte("+OK\r\n$5\r\nvalue\r\n*2\r\n$3\r\nGET\r\n$3\r\nkey\r\n"))
	if err != nil {
		t.Fatalf("split RESP frames: %v", err)
	}
	if len(frames) != 3 {
		t.Fatalf("frame count = %d", len(frames))
	}
	if string(frames[2]) != "*2\r\n$3\r\nGET\r\n$3\r\nkey\r\n" {
		t.Fatalf("nested frame = %q", string(frames[2]))
	}
	_, err = readRESPFrame(bufioReaderForTest("$20\r\nshort\r\n"), 8)
	if err == nil {
		t.Fatal("expected max frame size error")
	}
	_, err = readRESPFrame(bufioReaderForTest("+missing-lf"), maxTCPFrameBytes)
	if err == nil {
		t.Fatal("expected malformed frame error")
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
	if code := runCLI(cliSpec, []string{"benchmark-tool", "run", manifestPath, "--out", leftDir, "--warmup", "0", "--trials", "1", "--subjects", "print", "--workloads", "print-once"}, &stdout, &stderr); code != 0 {
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

func TestCompareResultDirsWritesVerdictArtifacts(t *testing.T) {
	dir := t.TempDir()
	baseDir := filepath.Join(dir, "base")
	candidateDir := filepath.Join(dir, "candidate")
	writeTrialResultDirForTest(t, baseDir, []float64{100, 101, 99}, nil, "elapsed_ms", "svc", "ping")
	writeTrialResultDirForTest(t, candidateDir, []float64{90, 91, 89}, nil, "elapsed_ms", "svc", "ping")
	var stdout bytes.Buffer

	if err := compareResultDirs(baseDir, candidateDir, "elapsed_ms", &stdout); err != nil {
		t.Fatalf("compare result dirs: %v", err)
	}
	if !strings.Contains(stdout.String(), "improvement") {
		t.Fatalf("stdout should include verdict: %s", stdout.String())
	}
	comparison := readComparisonForTest(t, filepath.Join(candidateDir, "comparison.json"))
	if len(comparison.Comparisons) != 1 {
		t.Fatalf("comparison count = %d", len(comparison.Comparisons))
	}
	row := comparison.Comparisons[0]
	if row.Verdict != "improvement" {
		t.Fatalf("verdict = %q row=%#v", row.Verdict, row)
	}
	if row.Direction != "lower_is_better" {
		t.Fatalf("latency direction = %q", row.Direction)
	}
	if row.RelativeDifferencePercent >= 0 {
		t.Fatalf("expected candidate latency to be lower: %#v", row)
	}
	if _, err := os.Stat(filepath.Join(candidateDir, "comparison.md")); err != nil {
		t.Fatalf("expected comparison markdown: %v", err)
	}
}

func TestCompareVerdictsRespectThroughputDirectionAndCorrectness(t *testing.T) {
	throughput := compareTrialSets(
		"base",
		"candidate",
		"ops_per_second",
		trialsForTest([]float64{1000, 1010, 990}, nil, "ops_per_second", "svc", "ping"),
		trialsForTest([]float64{900, 910, 890}, nil, "ops_per_second", "svc", "ping"),
	)
	if got := throughput.Comparisons[0].Verdict; got != "regression" {
		t.Fatalf("throughput verdict = %q row=%#v", got, throughput.Comparisons[0])
	}
	if got := throughput.Comparisons[0].Direction; got != "higher_is_better" {
		t.Fatalf("throughput direction = %q", got)
	}

	correctness := compareTrialSets(
		"base",
		"candidate",
		"elapsed_ms",
		trialsForTest([]float64{100, 101, 99}, nil, "elapsed_ms", "svc", "ping"),
		trialsForTest([]float64{50, 51, 49}, map[int]string{1: "wrong response"}, "elapsed_ms", "svc", "ping"),
	)
	if got := correctness.Comparisons[0].Verdict; got != "correctness_failed" {
		t.Fatalf("correctness verdict = %q row=%#v", got, correctness.Comparisons[0])
	}
	if got := relativeDifferencePercent(0, 3); got != 300 {
		t.Fatalf("zero-baseline relative difference should stay JSON-safe, got %f", got)
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

func TestRunOptionsApplySubjectOverridesAndFilters(t *testing.T) {
	manifest := validTinyManifest()
	manifest.Subjects = append(manifest.Subjects, Subject{
		Name:     "baseline",
		Kind:     "command",
		Prebuilt: true,
		Command:  "printf baseline",
	})
	manifest.Workloads = append(manifest.Workloads, Workload{
		Name:      "other",
		Driver:    "command",
		TimeoutMS: 1000,
	})

	updated, err := applyRunOptions(manifest, RunOptions{
		MeasurementTrials: 7,
		WarmupTrials:      2,
		SubjectOverrides:  map[string]string{"baseline": "origin/main"},
		SubjectFilter:     []string{"baseline"},
		WorkloadFilter:    []string{"other"},
	})
	if err != nil {
		t.Fatalf("apply run options: %v", err)
	}
	if updated.Defaults.MeasurementTrials != 7 || updated.Defaults.WarmupTrials != 2 {
		t.Fatalf("trial overrides were not applied: %#v", updated.Defaults)
	}
	if len(updated.Subjects) != 1 || updated.Subjects[0].Name != "baseline" || updated.Subjects[0].Checkout != "origin/main" {
		t.Fatalf("subject filter/override mismatch: %#v", updated.Subjects)
	}
	if len(updated.Workloads) != 1 || updated.Workloads[0].Name != "other" {
		t.Fatalf("workload filter mismatch: %#v", updated.Workloads)
	}
}

func TestParseSubjectOverridesRejectsMalformedValues(t *testing.T) {
	if _, err := parseSubjectOverrides([]string{"current=HEAD", "baseline=origin/main"}); err != nil {
		t.Fatalf("valid overrides should parse: %v", err)
	}
	if _, err := parseSubjectOverrides([]string{"missing-equals"}); err == nil {
		t.Fatal("expected malformed subject override to fail")
	}
}

func TestRunManifestPreparesGitWorktreeForCheckoutSubjects(t *testing.T) {
	if _, err := exec.LookPath("git"); err != nil {
		t.Skip("git is required for worktree tests")
	}
	if err := gitUsableForTest(); err != nil {
		t.Skipf("git is not usable in this environment: %v", err)
	}
	dir := t.TempDir()
	repo := filepath.Join(dir, "repo")
	resultDir := filepath.Join(dir, "results")
	if err := os.MkdirAll(repo, 0o755); err != nil {
		t.Fatalf("mkdir repo: %v", err)
	}
	runGitForTest(t, repo, "init")
	runGitForTest(t, repo, "config", "user.email", "bench@example.com")
	runGitForTest(t, repo, "config", "user.name", "Benchmark Test")
	writeFileForTest(t, filepath.Join(repo, "print_marker.go"), `package main

import (
	"fmt"
	"os"
	"strings"
)

func main() {
	data, err := os.ReadFile("marker.txt")
	if err != nil {
		panic(err)
	}
	fmt.Print(strings.TrimSpace(string(data)))
}
`)
	writeFileForTest(t, filepath.Join(repo, "benchmark.toml"), gitWorktreeManifestText())
	writeFileForTest(t, filepath.Join(repo, "marker.txt"), "baseline\n")
	runGitForTest(t, repo, "add", ".")
	runGitForTest(t, repo, "commit", "-m", "baseline")
	runGitForTest(t, repo, "branch", "baseline-ref")
	writeFileForTest(t, filepath.Join(repo, "marker.txt"), "current\n")
	runGitForTest(t, repo, "add", "marker.txt")
	runGitForTest(t, repo, "commit", "-m", "current")
	runGitForTest(t, repo, "branch", "current-ref")

	manifestPath := filepath.Join(repo, "benchmark.toml")
	manifest, err := LoadManifest(manifestPath)
	if err != nil {
		t.Fatalf("load manifest: %v", err)
	}
	if err := RunManifest(manifestPath, manifest, resultDir); err != nil {
		t.Fatalf("run manifest: %v", err)
	}

	assertBuildLogContainsForTest(t, resultDir, "baseline", "baseline")
	assertBuildLogContainsForTest(t, resultDir, "current", "current")
	metadata := readSubjectMetadataForTest(t, filepath.Join(resultDir, "subjects", "baseline", "subject.json"))
	if metadata.Commit == "" {
		t.Fatalf("expected pinned commit metadata: %#v", metadata)
	}
	if metadata.WorktreePath == "" {
		t.Fatalf("expected worktree metadata: %#v", metadata)
	}
	if _, err := os.Stat(metadata.WorktreePath); !errors.Is(err, os.ErrNotExist) {
		t.Fatalf("worktree should be cleaned up after the run, stat err=%v", err)
	}
}

func TestRunBuildWritesLogsAndPropagatesFailures(t *testing.T) {
	dir := t.TempDir()
	okSubject := PreparedSubject{
		Subject: Subject{
			Name:             "builder",
			WorkingDirectory: dir,
			Build:            "printf build-ok",
		},
		BenchmarkRoot: dir,
	}
	if err := runBuild(okSubject, dir); err != nil {
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
	badSubject.Subject.Build = shellExitCommand(7)
	if err := runBuild(badSubject, dir); err == nil {
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

func gitWorktreeManifestText() string {
	return `name = "git-worktree-benchmark"

[defaults]
warmup_trials = 0
measurement_trials = 1
cooldown_ms = 0
fail_fast = true

[[subjects]]
name = "baseline"
kind = "command"
checkout = "baseline-ref"
working_directory = "."
build = "go run print_marker.go"
command = "go version"

[[subjects]]
name = "current"
kind = "command"
checkout = "current-ref"
working_directory = "."
build = "go run print_marker.go"
command = "go version"

[[workloads]]
name = "go-version"
driver = "command"
operations = 1
timeout_ms = 10000
`
}

func tcpServiceManifestText(dir string) string {
	return `name = "tcp-service-benchmark"

[defaults]
warmup_trials = 0
measurement_trials = 1
cooldown_ms = 0
fail_fast = true

[[subjects]]
name = "resp-service"
kind = "service"
prebuilt = true
working_directory = "` + escapeManifestStringForTest(dir) + `"
command = "go run resp_server.go --port {port}"
ready_check = "tcp-connect"

[[workloads]]
name = "ping"
driver = "tcp-resp"
mode = "one-shot"
read_mode = "resp-frame"
request = "*1\r\n$4\r\nPING\r\n"
expect = "+PONG\r\n"
connections = 8
concurrency = 4
timeout_ms = 5000
`
}

func escapeManifestStringForTest(value string) string {
	value = strings.ReplaceAll(value, `\`, `\\`)
	return strings.ReplaceAll(value, `"`, `\"`)
}

func startRESPTestServer(t *testing.T) string {
	t.Helper()
	listener, err := net.Listen("tcp", "127.0.0.1:0")
	if err != nil {
		t.Fatalf("listen: %v", err)
	}
	t.Cleanup(func() { _ = listener.Close() })
	go func() {
		for {
			conn, err := listener.Accept()
			if err != nil {
				return
			}
			go handleRESPTestConn(conn)
		}
	}()
	return listener.Addr().String()
}

func handleRESPTestConn(conn net.Conn) {
	defer conn.Close()
	reader := bufio.NewReader(conn)
	for {
		frame, err := readRESPFrame(reader, maxTCPFrameBytes)
		if err != nil {
			return
		}
		text := string(frame)
		switch {
		case strings.Contains(text, "$4\r\nPING\r\n"):
			_, _ = conn.Write([]byte("+PONG\r\n"))
		case strings.Contains(text, "$3\r\nSET\r\n"):
			_, _ = conn.Write([]byte("+OK\r\n"))
		case strings.Contains(text, "$3\r\nGET\r\n"):
			_, _ = conn.Write([]byte("$5\r\nvalue\r\n"))
		default:
			_, _ = conn.Write([]byte("-ERR unknown command\r\n"))
		}
	}
}

func bufioReaderForTest(input string) *bufio.Reader {
	return bufio.NewReader(strings.NewReader(input))
}

func respServerProgramForTest() string {
	return `package main

import (
	"bufio"
	"flag"
	"io"
	"net"
	"strconv"
	"strings"
)

func main() {
	port := flag.String("port", "0", "port")
	flag.Parse()
	listener, err := net.Listen("tcp", "127.0.0.1:"+*port)
	if err != nil {
		panic(err)
	}
	for {
		conn, err := listener.Accept()
		if err != nil {
			return
		}
		go handle(conn)
	}
}

func handle(conn net.Conn) {
	defer conn.Close()
	reader := bufio.NewReader(conn)
	for {
		frame, err := readFrame(reader)
		if err != nil {
			return
		}
		if strings.Contains(string(frame), "$4\r\nPING\r\n") {
			_, _ = conn.Write([]byte("+PONG\r\n"))
		} else {
			_, _ = conn.Write([]byte("-ERR unknown command\r\n"))
		}
	}
}

func readFrame(reader *bufio.Reader) ([]byte, error) {
	var b strings.Builder
	prefix, err := reader.ReadByte()
	if err != nil {
		return nil, err
	}
	b.WriteByte(prefix)
	switch prefix {
	case '+', '-', ':':
		line, err := reader.ReadString('\n')
		if err != nil {
			return nil, err
		}
		b.WriteString(line)
	case '$':
		line, err := reader.ReadString('\n')
		if err != nil {
			return nil, err
		}
		b.WriteString(line)
		length, err := strconv.Atoi(strings.TrimSuffix(line, "\r\n"))
		if err != nil || length < 0 {
			return []byte(b.String()), err
		}
		body := make([]byte, length+2)
		_, err = io.ReadFull(reader, body)
		b.WriteString(string(body))
		return []byte(b.String()), err
	case '*':
		line, err := reader.ReadString('\n')
		if err != nil {
			return nil, err
		}
		b.WriteString(line)
		count, err := strconv.Atoi(strings.TrimSuffix(line, "\r\n"))
		if err != nil {
			return nil, err
		}
		for i := 0; i < count; i++ {
			nested, err := readFrame(reader)
			if err != nil {
				return nil, err
			}
			b.WriteString(string(nested))
		}
	}
	return []byte(b.String()), nil
}
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

func readComparisonForTest(t *testing.T, path string) ComparisonFile {
	t.Helper()
	data, err := os.ReadFile(path)
	if err != nil {
		t.Fatalf("read comparison: %v", err)
	}
	var comparison ComparisonFile
	if err := json.Unmarshal(data, &comparison); err != nil {
		t.Fatalf("decode comparison: %v", err)
	}
	return comparison
}

func writeTrialResultDirForTest(t *testing.T, dir string, values []float64, failures map[int]string, metric string, subject string, workload string) {
	t.Helper()
	if err := os.MkdirAll(dir, 0o755); err != nil {
		t.Fatalf("mkdir result dir: %v", err)
	}
	file, err := os.Create(filepath.Join(dir, "trials.jsonl"))
	if err != nil {
		t.Fatalf("create trials: %v", err)
	}
	defer file.Close()
	for _, trial := range trialsForTest(values, failures, metric, subject, workload) {
		if err := writeJSONLine(file, trial); err != nil {
			t.Fatalf("write trial: %v", err)
		}
	}
}

func trialsForTest(values []float64, failures map[int]string, metric string, subject string, workload string) []Trial {
	trials := make([]Trial, 0, len(values))
	for i, value := range values {
		trialNumber := i + 1
		errText := failures[trialNumber]
		trials = append(trials, Trial{
			Subject:  subject,
			Workload: workload,
			Trial:    trialNumber,
			Phase:    "measurement",
			OK:       errText == "",
			Metrics:  map[string]float64{metric: value},
			Error:    errText,
		})
	}
	return trials
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

func runGitForTest(t *testing.T, dir string, args ...string) {
	t.Helper()
	cmd := exec.Command("git", args...)
	cmd.Dir = dir
	out, err := cmd.CombinedOutput()
	if err != nil {
		t.Fatalf("git %s failed: %v\n%s", strings.Join(args, " "), err, string(out))
	}
}

func gitUsableForTest() error {
	cmd := exec.Command("git", "--version")
	return cmd.Run()
}

func writeFileForTest(t *testing.T, path string, content string) {
	t.Helper()
	if err := os.WriteFile(path, []byte(content), 0o644); err != nil {
		t.Fatalf("write %s: %v", path, err)
	}
}

func assertBuildLogContainsForTest(t *testing.T, resultDir string, subject string, want string) {
	t.Helper()
	data, err := os.ReadFile(filepath.Join(resultDir, "subjects", subject, "build.log"))
	if err != nil {
		t.Fatalf("read build log for %s: %v", subject, err)
	}
	if !strings.Contains(string(data), want) {
		t.Fatalf("build log for %s should contain %q:\n%s", subject, want, string(data))
	}
}

func readSubjectMetadataForTest(t *testing.T, path string) SubjectMetadata {
	t.Helper()
	data, err := os.ReadFile(path)
	if err != nil {
		t.Fatalf("read subject metadata: %v", err)
	}
	var metadata SubjectMetadata
	if err := json.Unmarshal(data, &metadata); err != nil {
		t.Fatalf("decode subject metadata: %v", err)
	}
	return metadata
}
