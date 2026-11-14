package sandbox

import (
	"strings"
	"testing"
)

// --config-sha256 (KILN04 section 4.7): optional, exactly 64 lowercase hex
// digits, before the output descriptors.
func TestParseExecArgsConfigSHA256(t *testing.T) {
	h := strings.Repeat("ab", 32)
	a, err := ParseExecArgs([]string{"--config", "/c", "--config-sha256", h, "--stdout-fd", "3", "--stderr-fd", "4", "--", "sh", "-c", "x"})
	if err != nil || a.ConfigSHA256 != h || a.StdoutFD != 3 || a.StderrFD != 4 || len(a.Argv) != 3 {
		t.Fatalf("%+v %v", a, err)
	}
	a, err = ParseExecArgs([]string{"--config", "/c", "--", "sh"})
	if err != nil || a.ConfigSHA256 != "" {
		t.Fatalf("%+v %v", a, err)
	}
	for name, d := range map[string]string{"short": h[:63], "long": h + "a", "upper": strings.ToUpper(h), "not hex": strings.Repeat("g", 64), "empty": ""} {
		if _, err := ParseExecArgs([]string{"--config", "/c", "--config-sha256", d, "--", "sh"}); err == nil {
			t.Errorf("%s: accepted", name)
		}
	}
	// After the output flags it is the step's own argument list, not ours.
	if _, err := ParseExecArgs([]string{"--config", "/c", "--stdout-fd", "3", "--stderr-fd", "4", "--config-sha256", h, "--", "sh"}); err == nil {
		t.Error("the flag was accepted out of order")
	}
}
