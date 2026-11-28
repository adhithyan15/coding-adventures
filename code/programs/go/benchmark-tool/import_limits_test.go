package main

import (
	"bytes"
	"encoding/json"
	"fmt"
	"os"
	"path/filepath"
	"strings"
	"testing"
)

func summaryDocumentForTest() []byte {
	row := Summary{
		Subject: "svc", Workload: "ping", Metric: "elapsed_ms", Count: 1,
		Min: 1, Max: 1, Mean: 1, Median: 1, P50: 1, P90: 1, P95: 1, P99: 1,
	}
	data, err := json.Marshal(SummaryFile{
		ManifestName: "test",
		GeneratedAt:  "2026-11-27T00:00:00Z",
		Summaries:    []Summary{row, row},
	})
	if err != nil {
		panic(err)
	}
	return data
}

func trialRecordForTest(subject string, trial int, phase string, ok bool, metricName string, metricValue float64) []byte {
	data, err := json.Marshal(Trial{
		Subject: subject, Workload: "ping", Trial: trial, Phase: phase, OK: ok,
		Metrics: map[string]float64{metricName: metricValue},
	})
	if err != nil {
		panic(err)
	}
	return data
}

func TestParseSummaryDocumentClosedUniqueAndComplete(t *testing.T) {
	valid := summaryDocumentForTest()
	parsed, err := parseSummaryDocument(valid)
	if err != nil {
		t.Fatalf("valid generated summary rejected: %v", err)
	}
	if len(parsed.Summaries) != 2 || parsed.Summaries[0].Subject != parsed.Summaries[1].Subject {
		t.Fatalf("separate summary rows with repeated values should remain valid: %#v", parsed.Summaries)
	}

	if !bytes.Contains(valid, []byte(`"subject":"svc"`)) {
		t.Fatal("test summary row has no subject field")
	}
	cases := []struct {
		name string
		data []byte
	}{
		{"duplicate root key", bytes.Replace(valid, []byte(`"manifest_name":"test"`), []byte(`"manifest_name":"first","manifest_name":"test"`), 1)},
		{"decoded duplicate row key", bytes.Replace(valid, []byte(`"subject":"svc"`), []byte(`"subject":"svc","\u0073ubject":"svc"`), 1)},
		{"unknown root field", bytes.Replace(valid, []byte(`"generated_at":`), []byte(`"future":true,"generated_at":`), 1)},
		{"unknown row field", bytes.Replace(valid, []byte(`"subject":"svc"`), []byte(`"subject":"svc","future":true`), 1)},
		{"missing row field", bytes.Replace(valid, []byte(`,"mean":1`), nil, 1)},
		{"wrong row type", bytes.Replace(valid, []byte(`"count":1`), []byte(`"count":1.5`), 1)},
		{"null rows", []byte(`{"manifest_name":"test","generated_at":"2026-11-27T00:00:00Z","summaries":null}`)},
		{"truncated", valid[:len(valid)-1]},
		{"trailing second value", append(append([]byte(nil), valid...), []byte(` {}`)...)},
	}
	for _, tc := range cases {
		t.Run(tc.name, func(t *testing.T) {
			if _, err := parseSummaryDocument(tc.data); err == nil {
				t.Fatalf("accepted malformed summary: %s", tc.data)
			}
		})
	}
}

func TestParseTrialRecordClosedUniqueAndComplete(t *testing.T) {
	valid := []byte(`{"subject":"svc","workload":"ping","trial":1,"phase":"measurement","ok":true,"metrics":{"elapsed_ms":1}}`)
	parsed, err := parseTrialRecord(valid)
	if err != nil || parsed.Metrics["elapsed_ms"] != 1 {
		t.Fatalf("valid producer row rejected: %#v, %v", parsed, err)
	}
	// A metric name in its nested map may equal a root member name. Duplicate
	// keys are rejected within one object, not across separate object scopes.
	repeatedAcrossScopes := []byte(`{"subject":"svc","workload":"ping","trial":1,"phase":"measurement","ok":true,"metrics":{"subject":1}}`)
	if _, err := parseTrialRecord(repeatedAcrossScopes); err != nil {
		t.Fatalf("same key in separate objects rejected: %v", err)
	}

	cases := []struct {
		name string
		data []byte
	}{
		{"decoded duplicate key", []byte(`{"subject":"svc","\u0073ubject":"svc","workload":"ping","trial":1,"phase":"measurement","ok":true,"metrics":{"elapsed_ms":1}}`)},
		{"unknown field", []byte(`{"subject":"svc","workload":"ping","trial":1,"phase":"measurement","ok":true,"metrics":{"elapsed_ms":1},"future":true}`)},
		{"missing field", []byte(`{"subject":"svc","workload":"ping","trial":1,"phase":"measurement","ok":true}`)},
		{"null optional error", []byte(`{"subject":"svc","workload":"ping","trial":1,"phase":"measurement","ok":true,"metrics":{},"error":null}`)},
		{"null metrics", []byte(`{"subject":"svc","workload":"ping","trial":1,"phase":"measurement","ok":true,"metrics":null}`)},
		{"null metric value", []byte(`{"subject":"svc","workload":"ping","trial":1,"phase":"measurement","ok":true,"metrics":{"elapsed_ms":null}}`)},
		{"wrong metric type", []byte(`{"subject":"svc","workload":"ping","trial":1,"phase":"measurement","ok":true,"metrics":{"elapsed_ms":"1"}}`)},
		{"truncated", valid[:len(valid)-1]},
		{"trailing second value", append(append([]byte(nil), valid...), []byte(` null`)...)},
		{"invalid UTF-8", append(append([]byte(nil), valid...), 0xff)},
	}
	for _, tc := range cases {
		t.Run(tc.name, func(t *testing.T) {
			if _, err := parseTrialRecord(tc.data); err == nil {
				t.Fatalf("accepted malformed trial row: %s", tc.data)
			}
		})
	}
}

func TestReadTrialsAggregatesOnlyMeasurementRows(t *testing.T) {
	path := filepath.Join(t.TempDir(), "trials.jsonl")
	data := bytes.Join([][]byte{
		trialRecordForTest("svc", 1, "warmup", true, "elapsed_ms", 100),
		trialRecordForTest("svc", 1, "measurement", true, "elapsed_ms", 10),
		trialRecordForTest("svc", 2, "measurement", false, "elapsed_ms", 20),
	}, []byte("\n"))
	if err := os.WriteFile(path, data, 0600); err != nil {
		t.Fatal(err)
	}
	values, err := readTrials(path, "elapsed_ms")
	if err != nil {
		t.Fatalf("read generated trial rows: %v", err)
	}
	group := values["svc/ping"]
	if !group.Seen || group.Correct || len(group.Values) != 1 || group.Values[0] != 10 {
		t.Fatalf("measurement projection = %#v", group)
	}
}

func TestReadTrialRowsRejectsOverBoundsWithoutReturningPartialData(t *testing.T) {
	first := trialRecordForTest("svc", 1, "measurement", true, "elapsed_ms", 10)
	second := trialRecordForTest("svc", 2, "measurement", true, "elapsed_ms", 11)
	data := append(append(append([]byte(nil), first...), '\n'), second...)

	values, err := readTrialMetrics(bytes.NewReader(data), "elapsed_ms", int64(len(data)), 2, maxTrialRecordBytes, maxImportedMeasurementGroups)
	if err != nil || len(values["svc/ping"].Values) != 2 {
		t.Fatalf("rows at maximum: values=%#v err=%v", values, err)
	}
	values, err = readTrialMetrics(bytes.NewReader(data), "elapsed_ms", int64(len(data)), 1, maxTrialRecordBytes, maxImportedMeasurementGroups)
	if err == nil || values != nil {
		t.Fatalf("row over-bound result = %#v, %v", values, err)
	}
	values, err = readTrialMetrics(bytes.NewReader(data), "elapsed_ms", int64(len(data)-1), 2, maxTrialRecordBytes, maxImportedMeasurementGroups)
	if err == nil || values != nil {
		t.Fatalf("byte over-bound result = %#v, %v", values, err)
	}

	otherGroup := trialRecordForTest("other", 1, "measurement", true, "elapsed_ms", 12)
	groups := append(append(append([]byte(nil), first...), '\n'), otherGroup...)
	values, err = readTrialMetrics(bytes.NewReader(groups), "elapsed_ms", int64(len(groups)), 2, maxTrialRecordBytes, 1)
	if err == nil || values != nil {
		t.Fatalf("measurement-group over-bound result = %#v, %v", values, err)
	}
}

func TestImportedResultBudgetsHaveExplicitBoundaries(t *testing.T) {
	if err := checkImportedSize("summary.json", maxImportedSummaryBytes, maxImportedSummaryBytes); err != nil {
		t.Fatalf("size at maximum rejected: %v", err)
	}
	if err := checkImportedSize("summary.json", maxImportedSummaryBytes+1, maxImportedSummaryBytes); err == nil {
		t.Fatal("accepted summary one byte above maximum")
	}
	if err := checkImportedRows("summary.json", maxImportedSummaryRows, maxImportedSummaryRows); err != nil {
		t.Fatalf("row count at maximum rejected: %v", err)
	}
	if err := checkImportedRows("summary.json", maxImportedSummaryRows+1, maxImportedSummaryRows); err == nil {
		t.Fatal("accepted summary one row above maximum")
	}

	line := trialRecordForTest("svc", 1, "measurement", true, "elapsed_ms", 1)
	line = append(line, bytes.Repeat([]byte{' '}, maxTrialRecordBytes-len(line))...)
	if _, err := parseTrialRecord(line); err != nil {
		t.Fatalf("record exactly at maximum rejected: %v", err)
	}
	if _, err := parseTrialRecord(append(line, ' ')); err == nil {
		t.Fatal("accepted trial record one byte above maximum")
	}

	array := "[" + strings.TrimSuffix(strings.Repeat("null,", maxImportedSummaryRows), ",") + "]"
	rows, err := decodeJSONArrayRaw([]byte(array), maxImportedSummaryRows)
	if err != nil || len(rows) != maxImportedSummaryRows {
		t.Fatalf("array at row maximum: rows=%d err=%v", len(rows), err)
	}
	over := "[" + strings.TrimSuffix(strings.Repeat("null,", maxImportedSummaryRows+1), ",") + "]"
	if _, err := decodeJSONArrayRaw([]byte(over), maxImportedSummaryRows); err == nil {
		t.Fatal("accepted summary array above row maximum")
	}
}

func TestComparisonWorkIsCheckedBeforeStatistics(t *testing.T) {
	if got, ok := comparisonBootstrapWork(0); !ok || got != 0 {
		t.Fatalf("empty group work = %d, %v", got, ok)
	}
	if got, ok := comparisonBootstrapWork(1); !ok || got != comparisonBootstrapRounds {
		t.Fatalf("one-sample group work = %d, %v", got, ok)
	}
	if _, ok := comparisonBootstrapWork(^uint64(0)); ok {
		t.Fatal("bootstrap work overflow was accepted")
	}

	base := make(map[string]trialMetricSet, maxComparisonGroups+1)
	candidate := make(map[string]trialMetricSet, maxComparisonGroups+1)
	for i := 0; i < maxComparisonGroups+1; i++ {
		key := fmt.Sprintf("s%d/w", i)
		base[key] = trialMetricSet{Values: []float64{1}}
		candidate[key] = trialMetricSet{Values: []float64{2}}
	}
	if err := validateComparisonWork(base, candidate); err == nil || !strings.Contains(err.Error(), "common groups") {
		t.Fatalf("group allocation budget result = %v", err)
	}

	withinGroupsBase := make(map[string]trialMetricSet, maxComparisonGroups)
	withinGroupsCandidate := make(map[string]trialMetricSet, maxComparisonGroups)
	for i := 0; i < maxComparisonGroups; i++ {
		key := fmt.Sprintf("s%d/w", i)
		withinGroupsBase[key] = trialMetricSet{Values: []float64{1}}
		withinGroupsCandidate[key] = trialMetricSet{Values: []float64{2}}
	}
	if err := validateComparisonWork(withinGroupsBase, withinGroupsCandidate); err != nil {
		t.Fatalf("exact maximum common groups rejected: %v", err)
	}
	if err := validateComparisonWork(
		map[string]trialMetricSet{"s/w": {Values: make([]float64, 3162)}},
		map[string]trialMetricSet{"s/w": {Values: make([]float64, 3162)}},
	); err != nil {
		t.Fatalf("pair work below maximum rejected: %v", err)
	}
	if err := validateComparisonWork(
		map[string]trialMetricSet{"s/w": {Values: make([]float64, 3163)}},
		map[string]trialMetricSet{"s/w": {Values: make([]float64, 3163)}},
	); err == nil || !strings.Contains(err.Error(), "Cliff's delta") {
		t.Fatalf("quadratic work budget result = %v", err)
	}
	if err := validateComparisonWork(
		map[string]trialMetricSet{"s/w": {Values: make([]float64, 7142)}},
		map[string]trialMetricSet{"s/w": {Values: []float64{1}}},
	); err != nil {
		t.Fatalf("bootstrap work below maximum rejected: %v", err)
	}
	if err := validateComparisonWork(
		map[string]trialMetricSet{"s/w": {Values: make([]float64, 7500)}},
		map[string]trialMetricSet{"s/w": {Values: []float64{1}}},
	); err == nil || !strings.Contains(err.Error(), "bootstrap work") {
		t.Fatalf("bootstrap work budget result = %v", err)
	}
}

func TestRESPNestingDepthBoundary(t *testing.T) {
	frameAt := func(depth int) []byte {
		return []byte(strings.Repeat("*1\r\n", depth-1) + "+PONG\r\n")
	}
	if _, err := readRESPFrame(bufioReaderForTest(string(frameAt(maxRESPDepth))), maxTCPFrameBytes); err != nil {
		t.Fatalf("RESP depth %d rejected: %v", maxRESPDepth, err)
	}
	if _, err := readRESPFrame(bufioReaderForTest(string(frameAt(maxRESPDepth+1))), maxTCPFrameBytes); err == nil {
		t.Fatalf("RESP depth %d accepted", maxRESPDepth+1)
	}
}

func TestTimeoutMillisecondsCannotOverflowDuration(t *testing.T) {
	if duration, err := timeoutFromMilliseconds(0, 0); err != nil || duration != 0 {
		t.Fatalf("command zero-timeout default = %v, %v", duration, err)
	}
	if duration, err := timeoutFromMilliseconds(0, defaultTCPTimeout); err != nil || duration != defaultTCPTimeout {
		t.Fatalf("TCP zero-timeout default = %v, %v", duration, err)
	}
	if duration, err := timeoutFromMilliseconds(int(maxTimeoutMS), 0); err != nil || duration <= 0 {
		t.Fatalf("maximum representable timeout = %v, %v", duration, err)
	}
	maxInt := int64(^uint(0) >> 1)
	if maxTimeoutMS < maxInt {
		if _, err := timeoutFromMilliseconds(int(maxTimeoutMS+1), 0); err == nil {
			t.Fatal("accepted timeout above duration representation")
		}
	}
}

func FuzzSummaryImport(f *testing.F) {
	f.Add(summaryDocumentForTest())
	f.Add([]byte(`{"summary":"truncated"`))
	f.Add([]byte(`{"manifest_name":"a","manifest_name":"b"}`))
	f.Fuzz(func(t *testing.T, data []byte) {
		if len(data) > int(maxImportedSummaryBytes) {
			t.Skip()
		}
		summary, err := parseSummaryDocument(data)
		if err == nil && len(summary.Summaries) > maxImportedSummaryRows {
			t.Fatal("successful parse exceeded summary row limit")
		}
	})
}

func FuzzTrialRecordIngress(f *testing.F) {
	f.Add(trialRecordForTest("svc", 1, "measurement", true, "elapsed_ms", 1))
	f.Add([]byte(`{"x":1,"x":2}`))
	f.Add([]byte(`null`))
	f.Fuzz(func(t *testing.T, data []byte) {
		if len(data) > maxTrialRecordBytes {
			t.Skip()
		}
		trial, err := parseTrialRecord(data)
		if err == nil && trial.Metrics == nil {
			t.Fatal("successful parse returned nil metrics")
		}
	})
}
