package main

// Paired reports already contain the observations needed for an exact sign
// interval. Replaying them must preserve integer timestamps before converting
// each operand to binary64. The command never folds these samples into the
// independent-result bootstrap comparison used elsewhere in this program.
import (
	"bytes"
	"crypto/sha256"
	"encoding/hex"
	"encoding/json"
	"fmt"
	"io"
	"math"
	"os"
	"path/filepath"
	"strings"
	"unicode/utf8"

	mb "github.com/adhithyan15/coding-adventures/code/packages/go/microbenchmark"
)

const maxPairedBytes = 8 << 20

type pairedConfig struct {
	Pairs      int64   `json:"pairs"`
	Confidence int64   `json:"confidence_percent"`
	Factor     float64 `json:"minimum_factor"`
	Seed       int64   `json:"seed"`
	Target     int64   `json:"target_ns"`
	MaxBatch   int64   `json:"max_batch"`
}

type pairedSummary struct {
	SHA256   string       `json:"input_sha256"`
	Pairs    int          `json:"pair_count"`
	Config   pairedConfig `json:"config"`
	Batch    int64        `json:"batch_size"`
	Analysis mb.Analysis  `json:"analysis"`
}

// The standard decoder handles syntax and escapes. This preliminary token walk
// adds duplicate-key and resource checks before maps can discard evidence.
func uniquePairedJSON(data []byte) error {
	return uniqueJSONDocument(data, maxPairedBytes, 200_000, 16, "report")
}

// uniqueJSONDocument performs bounded syntax, UTF-8, nesting, value-count, and
// decoded-key uniqueness checks before a typed decoder can discard evidence.
func uniqueJSONDocument(data []byte, maxBytes, maxValues, maxDepth int, label string) error {
	if maxBytes < 0 || len(data) > maxBytes || !utf8.Valid(data) {
		return fmt.Errorf("%s must be UTF-8 JSON of at most %d bytes", label, maxBytes)
	}
	d := json.NewDecoder(bytes.NewReader(data))
	d.UseNumber()
	budget := maxValues
	var value func(int) error
	value = func(depth int) error {
		budget--
		if budget < 0 {
			return fmt.Errorf("too many JSON values")
		}
		token, err := d.Token()
		if err != nil {
			return err
		}
		delim, container := token.(json.Delim)
		if !container {
			return nil
		}
		if depth >= maxDepth {
			return fmt.Errorf("%s JSON nesting exceeds %d containers", label, maxDepth)
		}
		switch delim {
		case '{':
			seen := map[string]bool{}
			for d.More() {
				key, err := d.Token()
				if err != nil {
					return err
				}
				name, ok := key.(string)
				if !ok {
					return fmt.Errorf("expected object key")
				}
				if seen[name] {
					return fmt.Errorf("duplicate JSON member %q", name)
				}
				seen[name] = true
				if err := value(depth + 1); err != nil {
					return err
				}
			}
		case '[':
			for d.More() {
				if err := value(depth + 1); err != nil {
					return err
				}
			}
		default:
			return fmt.Errorf("unexpected JSON delimiter")
		}
		_, err = d.Token()
		return err
	}
	if err := value(0); err != nil {
		return err
	}
	if _, err := d.Token(); err != io.EOF {
		return fmt.Errorf("%s must contain exactly one JSON value", label)
	}
	return nil
}

// encoding/json replaces unpaired escaped UTF-16 surrogates with U+FFFD.
// Validate the raw workload string first so malformed Unicode escapes cannot
// become indistinguishable from a valid replacement character.
func validWorkloadUnicodeEscapes(raw json.RawMessage) bool {
	raw = bytes.TrimSpace(raw)
	if len(raw) < 2 || raw[0] != '"' || raw[len(raw)-1] != '"' {
		return false
	}
	for i := 1; i < len(raw)-1; i++ {
		if raw[i] != '\\' {
			continue
		}
		if i+1 >= len(raw)-1 {
			return false
		}
		if raw[i+1] != 'u' {
			i++
			continue
		}
		unit, ok := jsonHexUnit(raw, i+2)
		if !ok {
			return false
		}
		i += 5
		if unit >= 0xd800 && unit <= 0xdbff {
			if i+6 >= len(raw) || raw[i+1] != '\\' || raw[i+2] != 'u' {
				return false
			}
			low, ok := jsonHexUnit(raw, i+3)
			if !ok || low < 0xdc00 || low > 0xdfff {
				return false
			}
			i += 6
		} else if unit >= 0xdc00 && unit <= 0xdfff {
			return false
		}
	}
	return true
}

func jsonHexUnit(raw []byte, start int) (uint16, bool) {
	if start < 0 || start+4 > len(raw) {
		return 0, false
	}
	var value uint16
	for _, digit := range raw[start : start+4] {
		value <<= 4
		switch {
		case digit >= '0' && digit <= '9':
			value |= uint16(digit - '0')
		case digit >= 'a' && digit <= 'f':
			value |= uint16(digit-'a') + 10
		case digit >= 'A' && digit <= 'F':
			value |= uint16(digit-'A') + 10
		default:
			return 0, false
		}
	}
	return value, true
}

// A field's presence differs from its zero value: seed zero is valid, but an
// omitted or null seed is not. Check exact names before decoding typed values.
func pairedObject(raw json.RawMessage, required, optional []string) (map[string]json.RawMessage, error) {
	if len(raw) == 0 || bytes.Equal(bytes.TrimSpace(raw), []byte("null")) {
		return nil, fmt.Errorf("expected object")
	}
	var object map[string]json.RawMessage
	if err := json.Unmarshal(raw, &object); err != nil || object == nil {
		return nil, fmt.Errorf("expected object")
	}
	allowed := map[string]bool{}
	for _, key := range required {
		allowed[key] = true
		v, ok := object[key]
		if !ok || bytes.Equal(bytes.TrimSpace(v), []byte("null")) {
			return nil, fmt.Errorf("missing or null %s", key)
		}
	}
	for _, key := range optional {
		allowed[key] = true
	}
	for key := range object {
		if !allowed[key] {
			return nil, fmt.Errorf("unknown field %q", key)
		}
	}
	return object, nil
}

func readPairedReport(data []byte) (pairedSummary, error) {
	var summary pairedSummary
	if err := uniquePairedJSON(data); err != nil {
		return summary, err
	}
	root, err := pairedObject(data, []string{"config", "batch_size", "pairs", "analysis"}, []string{"workload", "invocation", "revision", "runtime", "os", "arch", "profile"})
	if err != nil {
		return summary, err
	}
	if _, err := pairedObject(root["config"], []string{"pairs", "confidence_percent", "minimum_factor", "seed", "target_ns", "max_batch"}, nil); err != nil {
		return summary, fmt.Errorf("config: %w", err)
	}
	if err := json.Unmarshal(root["config"], &summary.Config); err != nil {
		return summary, fmt.Errorf("config: %w", err)
	}
	c := summary.Config
	if c.Pairs < 1 || c.Pairs > 10_000 || c.Confidence < 1 || c.Confidence > 99 || !finite(c.Factor) || c.Factor <= 1 || c.Target <= 0 || c.MaxBatch <= 0 {
		return summary, fmt.Errorf("invalid paired config")
	}
	if err := json.Unmarshal(root["batch_size"], &summary.Batch); err != nil || summary.Batch <= 0 || summary.Batch > c.MaxBatch {
		return summary, fmt.Errorf("invalid batch_size")
	}
	var rawPairs []json.RawMessage
	if err := json.Unmarshal(root["pairs"], &rawPairs); err != nil || int64(len(rawPairs)) != c.Pairs {
		return summary, fmt.Errorf("pair count does not match config")
	}
	pairs := make([]mb.Pair, len(rawPairs))
	baselineFirst := 0
	for i, raw := range rawPairs {
		if _, err := pairedObject(raw, []string{"baseline_ns", "candidate_ns", "order"}, nil); err != nil {
			return summary, fmt.Errorf("pair %d: %w", i, err)
		}
		if err := json.Unmarshal(raw, &pairs[i]); err != nil {
			return summary, fmt.Errorf("pair %d: %w", i, err)
		}
		if pairs[i].Order == "baseline-first" {
			baselineFirst++
		}
	}
	if baselineFirst != (len(pairs)+1)/2 {
		return summary, fmt.Errorf("unbalanced pair schedule")
	}
	computed, err := mb.Analyze(pairs, int(c.Confidence), c.Factor)
	if err != nil {
		return summary, err
	}
	storedFields, err := pairedObject(root["analysis"], []string{"ratio", "verdict"}, []string{"interval"})
	if err != nil {
		return summary, fmt.Errorf("analysis: %w", err)
	}
	if _, ok := storedFields["interval"]; !ok {
		return summary, fmt.Errorf("missing interval")
	}
	var stored mb.Analysis
	if err := json.Unmarshal(root["analysis"], &stored); err != nil {
		return summary, err
	}
	if stored.Ratio <= 0 || !closePaired(stored.Ratio, computed.Ratio) || stored.Verdict != computed.Verdict || (stored.Interval == nil) != (computed.Interval == nil) {
		return summary, fmt.Errorf("stored analysis does not match observations")
	}
	if stored.Interval != nil {
		var endpoints []json.RawMessage
		if err := json.Unmarshal(storedFields["interval"], &endpoints); err != nil || len(endpoints) != 2 {
			return summary, fmt.Errorf("interval must have two endpoints")
		}
		for i, endpoint := range endpoints {
			if bytes.Equal(bytes.TrimSpace(endpoint), []byte("null")) || stored.Interval[i] <= 0 || !closePaired(stored.Interval[i], computed.Interval[i]) {
				return summary, fmt.Errorf("stored interval does not match observations")
			}
		}
	}
	driverMetadata := []string{"workload", "invocation", "revision", "runtime", "os", "arch"}
	metadataPresent := 0
	for _, key := range driverMetadata {
		if _, ok := root[key]; ok {
			metadataPresent++
		}
	}
	if metadataPresent != 0 && metadataPresent != len(driverMetadata) {
		return summary, fmt.Errorf("driver metadata group must contain workload, invocation, revision, runtime, os, and arch")
	}
	if _, ok := root["profile"]; ok && metadataPresent != len(driverMetadata) {
		return summary, fmt.Errorf("profile requires the complete driver metadata group")
	}
	for _, key := range []string{"workload", "revision", "runtime", "os", "arch", "profile"} {
		if raw, ok := root[key]; ok {
			var s string
			err := json.Unmarshal(raw, &s)
			invalid := err != nil
			if key == "workload" {
				invalid = invalid || strings.Trim(s, " \t\n\r\v\f") == "" || strings.ContainsRune(s, '\x00') || !validWorkloadUnicodeEscapes(raw)
			} else {
				invalid = invalid || s == "" || strings.ContainsRune(s, '\x00')
			}
			if invalid {
				return summary, fmt.Errorf("invalid metadata %s", key)
			}
		}
	}
	if raw, ok := root["invocation"]; ok {
		var args []json.RawMessage
		if err := json.Unmarshal(raw, &args); err != nil || bytes.Equal(bytes.TrimSpace(raw), []byte("null")) {
			return summary, fmt.Errorf("invalid invocation")
		}
		for _, rawArg := range args {
			var arg *string
			if err := json.Unmarshal(rawArg, &arg); err != nil || arg == nil || strings.ContainsRune(*arg, '\x00') {
				return summary, fmt.Errorf("invalid invocation entry")
			}
		}
	}
	sum := sha256.Sum256(data)
	summary.SHA256 = hex.EncodeToString(sum[:])
	summary.Pairs = len(pairs)
	summary.Analysis = computed
	return summary, nil
}

func finite(x float64) bool { return !math.IsNaN(x) && !math.IsInf(x, 0) }
func closePaired(a, b float64) bool {
	return finite(a) && finite(b) && math.Abs(a-b) <= math.Max(1e-15, 1e-12*math.Max(math.Abs(a), math.Abs(b)))
}

func pairedReplayFile(path, out string, stdout io.Writer) error {
	f, err := os.Open(path)
	if err != nil {
		return err
	}
	defer f.Close()
	data, err := io.ReadAll(io.LimitReader(f, maxPairedBytes+1))
	if err != nil {
		return err
	}
	summary, err := readPairedReport(data)
	if err != nil {
		return err
	}
	encoded, err := json.MarshalIndent(summary, "", "  ")
	if err != nil {
		return err
	}
	encoded = append(encoded, '\n')
	if out != "" {
		if _, err := os.Lstat(out); err == nil {
			return fmt.Errorf("output already exists: %s", out)
		} else if !os.IsNotExist(err) {
			return err
		}
		stage, err := os.MkdirTemp(filepath.Dir(out), ".paired-report-")
		if err != nil {
			return err
		}
		defer os.RemoveAll(stage)
		markdown := fmt.Sprintf("# Paired benchmark replay\n\nInput SHA-256: `%s`\n\n%d adjacent pairs; candidate/baseline median ratio %.17g; verdict **%s**.\n\nThe exact median interval and original configuration are in paired-summary.json. Raw observations and producer metadata are preserved byte for byte in paired-report.json. These measurements describe the recorded workload and environment; they do not rank languages or identify a hardware cache by themselves.\n", summary.SHA256, summary.Pairs, summary.Analysis.Ratio, summary.Analysis.Verdict)
		for name, contents := range map[string][]byte{"paired-report.json": data, "paired-summary.json": encoded, "paired-report.md": []byte(markdown)} {
			if err := os.WriteFile(filepath.Join(stage, name), contents, 0o644); err != nil {
				return err
			}
		}
		// Mkdir reserves a previously absent name atomically. Renaming the stage
		// directory itself could replace a concurrently created empty directory
		// on Unix. Publish the summary last so readers have a completion record.
		if err := os.Mkdir(out, 0o700); err != nil {
			return err
		}
		complete := false
		defer func() {
			if !complete {
				_ = os.RemoveAll(out)
			}
		}()
		for _, name := range []string{"paired-report.json", "paired-report.md", "paired-summary.json"} {
			if err := os.Rename(filepath.Join(stage, name), filepath.Join(out, name)); err != nil {
				return err
			}
		}
		complete = true
	}
	n, err := stdout.Write(encoded)
	if err == nil && n != len(encoded) {
		return io.ErrShortWrite
	}
	return err
}
