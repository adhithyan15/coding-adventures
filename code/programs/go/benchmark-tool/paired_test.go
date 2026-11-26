package main

import (
	"bytes"
	"crypto/sha256"
	"encoding/hex"
	"encoding/json"
	"fmt"
	"io"
	"os"
	"path/filepath"
	"strings"
	"testing"
)

// Equal durations independently imply six ratios of one and an interval [1,1].
// The wide integer tokens deliberately exceed binary64's exact integer range.
func pairedTestReport() []byte {
	var rows []string
	for i := 0; i < 6; i++ {
		order := "baseline-first"
		if i%2 != 0 {
			order = "candidate-first"
		}
		rows = append(rows, fmt.Sprintf(`{"baseline_ns":9223372036854775807,"candidate_ns":9223372036854775807,"order":%q}`, order))
	}
	return []byte(`{"config":{"pairs":6,"confidence_percent":95,"minimum_factor":2,"seed":-9223372036854775808,"target_ns":9223372036854775807,"max_batch":9223372036854775807},"batch_size":9007199254740993,"pairs":[` + strings.Join(rows, ",") + `],"analysis":{"ratio":1,"interval":[1,1],"verdict":"equivalent"},"invocation":["native-driver"],"runtime":"test","profile":"release"}`)
}

func TestPairedReplayLosslessIntegersAndExport(t *testing.T) {
	data := pairedTestReport()
	summary, err := readPairedReport(data)
	if err != nil {
		t.Fatal(err)
	}
	if summary.Config.Seed != -9223372036854775808 || summary.Config.MaxBatch != 9223372036854775807 || summary.Batch != 9007199254740993 || summary.Analysis.Ratio != 1 || summary.Analysis.Verdict != "equivalent" {
		t.Fatalf("incorrect replay: %+v", summary)
	}
	sum := sha256.Sum256(data)
	if summary.SHA256 != hex.EncodeToString(sum[:]) {
		t.Fatal("incorrect input digest")
	}
	dir := t.TempDir()
	input := filepath.Join(dir, "input.json")
	out := filepath.Join(dir, "evidence")
	if err := os.WriteFile(input, data, 0o600); err != nil {
		t.Fatal(err)
	}
	var stdout, stderr bytes.Buffer
	if code := runCLI(cliSpec, []string{"benchmark-tool", "paired-replay", input, "--out", out}, &stdout, &stderr); code != 0 {
		t.Fatalf("CLI %d: %s", code, &stderr)
	}
	exported, err := os.ReadFile(filepath.Join(out, "paired-report.json"))
	if err != nil || !bytes.Equal(exported, data) {
		t.Fatalf("raw bytes changed: %v", err)
	}
	var decoded pairedSummary
	if err := json.Unmarshal(stdout.Bytes(), &decoded); err != nil || decoded.SHA256 != summary.SHA256 {
		t.Fatalf("invalid CLI summary: %v", err)
	}
	for _, name := range []string{"paired-summary.json", "paired-report.md"} {
		if info, err := os.Stat(filepath.Join(out, name)); err != nil || info.Size() == 0 {
			t.Fatalf("missing %s: %v", name, err)
		}
	}
	if err := pairedReplayFile(input, out, io.Discard); err == nil {
		t.Fatal("overwrote existing output")
	}
	again, _ := os.ReadFile(filepath.Join(out, "paired-report.json"))
	if !bytes.Equal(again, data) {
		t.Fatal("existing evidence changed")
	}
}

func TestPairedReplayInvalidReportsProduceNoSuccess(t *testing.T) {
	base := string(pairedTestReport())
	cases := map[string]string{
		"missing seed":          strings.Replace(base, `"seed":-9223372036854775808,`, "", 1),
		"null seed":             strings.Replace(base, `"seed":-9223372036854775808`, `"seed":null`, 1),
		"seed overflow":         strings.Replace(base, `-9223372036854775808`, `9223372036854775808`, 1),
		"fractional integer":    strings.Replace(base, `"pairs":6`, `"pairs":6.0`, 1),
		"unknown root":          strings.Replace(base, `{"config"`, `{"machine":"secret","config"`, 1),
		"unknown config":        strings.Replace(base, `"config":{`, `"config":{"extra":1,`, 1),
		"unknown pair":          strings.Replace(base, `"baseline_ns":`, `"extra":1,"baseline_ns":`, 1),
		"unknown analysis":      strings.Replace(base, `"analysis":{`, `"analysis":{"extra":1,`, 1),
		"zero target":           strings.Replace(base, `"target_ns":9223372036854775807`, `"target_ns":0`, 1),
		"zero batch":            strings.Replace(base, `"batch_size":9007199254740993`, `"batch_size":0`, 1),
		"batch exceeds maximum": strings.Replace(base, `"max_batch":9223372036854775807`, `"max_batch":1`, 1),
		"zero pairs":            strings.Replace(base, `"pairs":6`, `"pairs":0`, 1),
		"too many pairs":        strings.Replace(base, `"pairs":6`, `"pairs":10001`, 1),
		"confidence":            strings.Replace(base, `"confidence_percent":95`, `"confidence_percent":100`, 1),
		"factor one":            strings.Replace(base, `"minimum_factor":2`, `"minimum_factor":1`, 1),
		"factor overflow":       strings.Replace(base, `"minimum_factor":2`, `"minimum_factor":1e999`, 1),
		"zero duration":         strings.Replace(base, `"baseline_ns":9223372036854775807`, `"baseline_ns":0`, 1),
		"bad order":             strings.Replace(base, `baseline-first`, `other`, 1),
		"unbalanced":            strings.Replace(base, `candidate-first`, `baseline-first`, 1),
		"wrong ratio":           strings.Replace(base, `"ratio":1`, `"ratio":2`, 1),
		"wrong verdict":         strings.Replace(base, `"verdict":"equivalent"`, `"verdict":"faster"`, 1),
		"wrong interval":        strings.Replace(base, `[1,1]`, `[1,2]`, 1),
		"long interval":         strings.Replace(base, `[1,1]`, `[1,1,1]`, 1),
		"short interval":        strings.Replace(base, `[1,1]`, `[1]`, 1),
		"null endpoint":         strings.Replace(base, `[1,1]`, `[1,null]`, 1),
		"absent interval":       strings.Replace(base, `"interval":[1,1],`, "", 1),
		"null interval":         strings.Replace(base, `[1,1]`, `null`, 1),
		"blank metadata":        strings.Replace(base, `"runtime":"test"`, `"runtime":" "`, 1),
		"null metadata":         strings.Replace(base, `"runtime":"test"`, `"runtime":null`, 1),
		"null invocation":       strings.Replace(base, `["native-driver"]`, `null`, 1),
		"bad invocation entry":  strings.Replace(base, `["native-driver"]`, `[null]`, 1),
		"duplicate root":        strings.Replace(base, `{"config":`, `{"config":{},"config":`, 1),
		"duplicate config":      strings.Replace(base, `"pairs":6`, `"pairs":6,"pa\u0069rs":6`, 1),
		"duplicate pair":        strings.Replace(base, `"baseline_ns":`, `"order":"candidate-first","baseline_ns":`, 1),
		"duplicate analysis":    strings.Replace(base, `"ratio":1`, `"ratio":1,"ratio":1`, 1),
		"duplicate metadata":    strings.Replace(base, `"runtime":"test"`, `"runtime":"test","runtime":"test"`, 1),
		"trailing value":        base + ` {}`,
		"malformed":             base[:len(base)-1],
		"not object":            `[]`,
		"null object":           `null`,
		"deep":                  strings.Repeat("[", 17) + "0" + strings.Repeat("]", 17),
		"large":                 strings.Repeat(" ", maxPairedBytes+1),
		"invalid utf8":          string([]byte{'"', 255, '"'}),
	}
	for name, data := range cases {
		t.Run(name, func(t *testing.T) {
			if _, err := readPairedReport([]byte(data)); err == nil {
				t.Fatal("accepted invalid report")
			}
		})
	}
}

func TestPairedReplayEmptyArgumentVector(t *testing.T) {
	// A producer that records argv[1:] can legitimately run with no arguments.
	data := bytes.Replace(pairedTestReport(), []byte(`["native-driver"]`), []byte(`[]`), 1)
	if _, err := readPairedReport(data); err != nil {
		t.Fatal(err)
	}
}

func TestPairedReplayGenericWorkloadRejectsEscapedNUL(t *testing.T) {
	base := string(pairedTestReport())
	valid := strings.Replace(base, `"invocation":`, `"workload":"dependent-link-order","invocation":`, 1)
	if _, err := readPairedReport([]byte(valid)); err != nil {
		t.Fatalf("valid generic workload rejected: %v", err)
	}
	unicodeBlank := strings.Replace(valid, `"workload":"dependent-link-order"`, `"workload":"\u2003"`, 1)
	if _, err := readPairedReport([]byte(unicodeBlank)); err != nil {
		t.Fatalf("Unicode whitespace is a valid workload identifier: %v", err)
	}
	for name, token := range map[string]string{
		"astral surrogate pair": `"\ud83d\ude00"`,
		"replacement character": `"\ufffd"`,
		"literal backslash-u":   `"\\uD800"`,
	} {
		t.Run(name, func(t *testing.T) {
			mutated := strings.Replace(valid, `"workload":"dependent-link-order"`, `"workload":`+token, 1)
			if _, err := readPairedReport([]byte(mutated)); err != nil {
				t.Fatalf("valid Unicode workload rejected: %v", err)
			}
		})
	}
	invalidTokens := map[string]string{
		"empty":               `""`,
		"ASCII blank":         `" \t\n\r\f\u000b"`,
		"NUL only":            `"\u0000"`,
		"embedded NUL":        `"dependent\u0000link-order"`,
		"lone high surrogate": `"\ud800"`,
		"lone low surrogate":  `"\udfff"`,
		"null":                `null`,
		"number":              `17`,
		"boolean":             `true`,
		"array":               `[]`,
	}
	for name, token := range invalidTokens {
		t.Run(name, func(t *testing.T) {
			mutated := strings.Replace(valid, `"workload":"dependent-link-order"`, `"workload":`+token, 1)
			if _, err := readPairedReport([]byte(mutated)); err == nil {
				t.Fatal("invalid workload identifier accepted")
			}
		})
	}
}

func TestPairedReplayPortableCases(t *testing.T) {
	var fixture struct {
		Confidence int64   `json:"confidence_percent"`
		Factor     float64 `json:"minimum_factor"`
		Cases      []struct {
			Name      string      `json:"name"`
			Baseline  int64       `json:"baseline_ns"`
			Candidate []int64     `json:"candidate_ns"`
			Ratio     float64     `json:"ratio"`
			Interval  *[2]float64 `json:"interval"`
			Verdict   string      `json:"verdict"`
		} `json:"cases"`
	}
	data, err := os.ReadFile("../../../specs/fixtures/microbenchmark-pairs-v1.json")
	if err != nil {
		t.Fatal(err)
	}
	if err := json.Unmarshal(data, &fixture); err != nil {
		t.Fatal(err)
	}
	for _, c := range fixture.Cases {
		t.Run(c.Name, func(t *testing.T) {
			pairs := make([]map[string]any, len(c.Candidate))
			for i, n := range c.Candidate {
				order := "candidate-first"
				if i < (len(pairs)+1)/2 {
					order = "baseline-first"
				}
				pairs[i] = map[string]any{"baseline_ns": c.Baseline, "candidate_ns": n, "order": order}
			}
			report := map[string]any{"config": pairedConfig{int64(len(pairs)), fixture.Confidence, fixture.Factor, 0, 100000, 65536}, "batch_size": 1, "pairs": pairs, "analysis": map[string]any{"ratio": c.Ratio, "interval": c.Interval, "verdict": c.Verdict}}
			raw, err := json.Marshal(report)
			if err != nil {
				t.Fatal(err)
			}
			if _, err := readPairedReport(raw); err != nil {
				t.Fatal(err)
			}
		})
	}
}

func TestPairedReplayMaximumAndIngressBudgets(t *testing.T) {
	var report map[string]json.RawMessage
	if err := json.Unmarshal(pairedTestReport(), &report); err != nil {
		t.Fatal(err)
	}
	var config pairedConfig
	if err := json.Unmarshal(report["config"], &config); err != nil {
		t.Fatal(err)
	}
	config.Pairs = 10000
	var pairs []json.RawMessage
	if err := json.Unmarshal(report["pairs"], &pairs); err != nil {
		t.Fatal(err)
	}
	large := make([]json.RawMessage, 10000)
	for i := range large {
		large[i] = pairs[i%2]
	}
	report["config"], _ = json.Marshal(config)
	report["pairs"], _ = json.Marshal(large)
	data, err := json.Marshal(report)
	if err != nil {
		t.Fatal(err)
	}
	if summary, err := readPairedReport(data); err != nil || summary.Pairs != 10000 {
		t.Fatalf("valid maximum rejected: %v", err)
	}
	if err := uniquePairedJSON([]byte("[" + strings.Repeat("0,", 200000) + "0]")); err == nil {
		t.Fatal("value budget not enforced")
	}
	if err := uniquePairedJSON([]byte(`{"s":"[\\\"{}]","again":"[\\\"{}]"}`)); err != nil {
		t.Fatalf("quoted punctuation counted as nesting: %v", err)
	}
}

type pairedShortWriter struct{}

func (pairedShortWriter) Write(p []byte) (int, error) { return 0, nil }
func TestPairedReplayIOFailures(t *testing.T) {
	dir := t.TempDir()
	path := filepath.Join(dir, "input.json")
	if err := os.WriteFile(path, pairedTestReport(), 0600); err != nil {
		t.Fatal(err)
	}
	if err := pairedReplayFile(path, "", pairedShortWriter{}); err != io.ErrShortWrite {
		t.Fatalf("short write: %v", err)
	}
	if err := pairedReplayFile(filepath.Join(dir, "absent"), "", io.Discard); err == nil {
		t.Fatal("accepted missing input")
	}
	if err := pairedReplayFile(path, filepath.Join(dir, "missing", "out"), io.Discard); err == nil {
		t.Fatal("accepted missing parent")
	}
	bad := filepath.Join(dir, "bad.json")
	if err := os.WriteFile(bad, []byte(`{"config":null}`), 0600); err != nil {
		t.Fatal(err)
	}
	var stdout, stderr bytes.Buffer
	if code := runCLI(cliSpec, []string{"benchmark-tool", "paired-replay", bad}, &stdout, &stderr); code == 0 || stdout.Len() != 0 || stderr.Len() == 0 {
		t.Fatalf("invalid input produced success: %d %s %s", code, &stdout, &stderr)
	}
}

func FuzzPairedReportIngress(f *testing.F) {
	f.Add(pairedTestReport())
	f.Add([]byte(`{"x":1,"x":2}`))
	f.Add([]byte{255})
	f.Add([]byte(`null`))
	f.Fuzz(func(t *testing.T, data []byte) {
		if len(data) > maxPairedBytes {
			t.Skip()
		}
		summary, err := readPairedReport(data)
		if err == nil && (summary.Pairs < 1 || summary.Pairs > 10000 || summary.Analysis.Ratio <= 0) {
			t.Fatal("successful replay violated report bounds")
		}
	})
}
