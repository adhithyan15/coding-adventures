package record

import (
	"encoding/json"
	"strings"
	"testing"

	"github.com/adhithyan15/coding-adventures/code/programs/go/kiln/internal/legacy"
)

// This test-only predicate preserves the original whole-sidecar secret/PID
// checks while excluding only valid SHA-256 values in the sidecar's digest
// fields. An unrelated opaque digest can legitimately contain fixture digits.
func hasRawStraceStderrMarker(serialized string) bool {
	var sidecar StepSidecar
	if err := legacy.PolicyDecode([]byte(serialized), &sidecar); err != nil {
		return true
	}
	for _, digest := range []*string{
		&sidecar.BindingID,
		&sidecar.LineSHA256,
		&sidecar.TraceSHA256,
		&sidecar.ExecHeadSHA256,
	} {
		if hexDigest(*digest) {
			*digest = ""
		}
	}
	remaining, err := json.Marshal(sidecar)
	if err != nil {
		return true
	}
	text := string(remaining)
	return strings.Contains(text, "secret") ||
		strings.Contains(text, "4242") ||
		strings.Contains(text, "strace: ")
}

func TestStraceStderrDuplicateMemberControls(t *testing.T) {
	opaqueBindingID := strings.Repeat("a", 32) + "4242" + strings.Repeat("b", 28)
	for name, serialized := range map[string]string{
		"top-level duplicate with last safe value": `{"action":"secret","action":"safe"}`,
		"escaped alias of digest key":              `{"binding_id":"secret","binding\u005fid":"` + opaqueBindingID + `"}`,
		"nested duplicate leak":                    `{"unknown":{"value":"safe","value":"secret"}}`,
	} {
		t.Run(name, func(t *testing.T) {
			if !hasRawStraceStderrMarker(serialized) {
				t.Fatal("duplicate or unknown serialized member was collapsed before privacy checks")
			}
		})
	}

	// Verify the existing strict decoder rejects duplicates recursively and
	// compares decoded key names, not just raw escape spellings.
	for name, raw := range map[string]string{
		"nested duplicate":     `{"outer":{"value":"safe","value":"secret"}}`,
		"nested escaped alias": `{"outer":{"value":"safe","valu\u0065":"secret"}}`,
	} {
		t.Run(name+" strict decoder", func(t *testing.T) {
			var nested map[string]map[string]string
			if err := legacy.PolicyDecode([]byte(raw), &nested); err == nil {
				t.Fatal("strict decoder accepted a nested duplicate object member")
			}
		})
	}
}

func TestStraceStderrPrivacyOracleControls(t *testing.T) {
	opaqueBindingID := strings.Repeat("a", 32) + "4242" + strings.Repeat("b", 28)
	if !hexDigest(opaqueBindingID) {
		t.Fatal("positive control must be a valid lowercase SHA-256")
	}
	for _, field := range []string{"binding_id", "line_sha256", "trace_sha256", "exechead_sha256"} {
		t.Run("valid digest field "+field, func(t *testing.T) {
			serialized, err := json.Marshal(map[string]string{field: opaqueBindingID})
			if err != nil {
				t.Fatal(err)
			}
			if hasRawStraceStderrMarker(string(serialized)) {
				t.Fatal("a valid opaque digest containing fixture PID digits was treated as stderr")
			}
		})
	}

	for name, serialized := range map[string]string{
		"partial secret":       `{"action":"secreting"}`,
		"PID in text":          `{"action":"worker pid=4242 failed"}`,
		"numeric PID":          `{"seq":4242}`,
		"escaped secret":       `{"action":"sec\u0072et"}`,
		"escaped PID":          `{"action":"pid=\u0034\u0032\u0034\u0032"}`,
		"stderr text":          `{"action":"strace: ptrace(PTRACE_LISTEN, 4242, NULL, 0): Input/output error"}`,
		"secret path":          `{"action":"boom-secret-/home/x"}`,
		"unknown opaque field": `{"digest":"` + opaqueBindingID + `"}`,
		"malformed hash field": `{"binding_id":"prefix-4242"}`,
		"malformed JSON":       `{"diagnostic":"secret"`,
		"trailing JSON value":  `{"diagnostic":"safe"} {"diagnostic":"secret"}`,
	} {
		t.Run(name, func(t *testing.T) {
			if !hasRawStraceStderrMarker(serialized) {
				t.Fatal("privacy control did not detect the forbidden serialized value")
			}
		})
	}
}
