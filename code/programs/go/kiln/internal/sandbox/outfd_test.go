package sandbox

// Tests of S2 (KILN04 section 4.7): --stdout-fd and --stderr-fd.  The parsing
// and the descriptor-type decision are pure, so they run on every OS.

import (
	"reflect"
	"strings"
	"testing"
)

func TestParseExecArgsOutputFDs(t *testing.T) {
	sh := []string{"sh", "-c", "make"}
	for name, tc := range map[string]struct {
		args           []string
		stdout, stderr int
	}{
		"neither":    {[]string{"--config", "/c", "--", "sh", "-c", "make"}, NoFD, NoFD},
		"both":       {[]string{"--config", "/c", "--stdout-fd", "3", "--stderr-fd", "4", "--", "sh", "-c", "make"}, 3, 4},
		"same":       {[]string{"--config", "/c", "--stdout-fd", "5", "--stderr-fd", "5", "--", "sh", "-c", "make"}, 5, 5},
		"big":        {[]string{"--config", "/c", "--stdout-fd", "1048576", "--stderr-fd", "3", "--", "sh", "-c", "make"}, 1 << 20, 3},
		"leading 0s": {[]string{"--config", "/c", "--stdout-fd", "007", "--stderr-fd", "8", "--", "sh", "-c", "make"}, 7, 8},
	} {
		a, err := ParseExecArgs(tc.args)
		if err != nil {
			t.Errorf("%s: %v", name, err)
			continue
		}
		if a.StdoutFD != tc.stdout || a.StderrFD != tc.stderr || a.Config != "/c" || !reflect.DeepEqual(a.Argv, sh) {
			t.Errorf("%s: %+v", name, a)
		}
	}
	bad := map[string][]string{
		"stdout only (L6)":  {"--config", "/c", "--stdout-fd", "3", "--", "sh"},
		"stderr only (L6)":  {"--config", "/c", "--stderr-fd", "4", "--", "sh"},
		"stderr first":      {"--config", "/c", "--stderr-fd", "4", "--stdout-fd", "3", "--", "sh"},
		"twice":             {"--config", "/c", "--stdout-fd", "3", "--stdout-fd", "4", "--stderr-fd", "5", "--", "sh"},
		"no number":         {"--config", "/c", "--stdout-fd", "--", "sh"},
		"empty":             {"--config", "/c", "--stdout-fd", "", "--", "sh"},
		"signed":            {"--config", "/c", "--stdout-fd", "+3", "--", "sh"},
		"negative":          {"--config", "/c", "--stdout-fd", "-3", "--", "sh"},
		"hex":               {"--config", "/c", "--stdout-fd", "0x3", "--", "sh"},
		"space":             {"--config", "/c", "--stdout-fd", " 3", "--", "sh"},
		"stdin":             {"--config", "/c", "--stdout-fd", "0", "--", "sh"},
		"stdout itself":     {"--config", "/c", "--stdout-fd", "1", "--", "sh"},
		"stderr itself":     {"--config", "/c", "--stderr-fd", "2", "--", "sh"},
		"too big":           {"--config", "/c", "--stdout-fd", "1048577", "--", "sh"},
		"too long":          {"--config", "/c", "--stdout-fd", "99999999", "--", "sh"},
		"no argv":           {"--config", "/c", "--stdout-fd", "3", "--"},
		"no separator":      {"--config", "/c", "--stdout-fd", "3", "sh", "x"},
		"flag after --":     {"--config", "/c", "--", "--stdout-fd", "3"}, // the step's own argv: accepted, see below
		"flag, no argv":     {"--config", "/c", "--stdout-fd", "3"},
		"config after flag": {"--stdout-fd", "3", "--config", "/c", "--", "sh"},
	}
	for name, args := range bad {
		a, err := ParseExecArgs(args)
		if name == "flag after --" {
			// Everything after the first "--" is the step's argv, even text
			// that looks like a flag: it is never parsed as one.
			if err != nil || a.StdoutFD != NoFD || !reflect.DeepEqual(a.Argv, []string{"--stdout-fd", "3"}) {
				t.Errorf("%s: %+v, %v", name, a, err)
			}
			continue
		}
		if err == nil {
			t.Errorf("%s: accepted: %+v", name, a)
		}
	}
}

func TestCheckOutputFD(t *testing.T) {
	const (
		fifo = 0o010000 | 0o600
		reg  = 0o100000 | 0o644
		dir  = 0o040000 | 0o755
		chr  = 0o020000 | 0o620
		sock = 0o140000 | 0o755
		blk  = 0o060000 | 0o660
		lnk  = 0o120000 | 0o777
	)
	if err := CheckOutputFD("fd", fifo, oWronly); err != nil {
		t.Errorf("a write end: %v", err)
	}
	if err := CheckOutputFD("fd", fifo, oWronly|0o2000|0o4000); err != nil { // O_APPEND, O_NONBLOCK are fine
		t.Errorf("a write end with other flags: %v", err)
	}
	for name, tc := range map[string]struct {
		mode  uint32
		flags int
		want  string
	}{
		"read end":             {fifo, 0, "not open write-only"},
		"read-write fifo (L4)": {fifo, 2, "not open write-only"},
		"regular file":         {reg, oWronly, "a regular file"},
		"directory":            {dir, 0, "a directory"},
		"terminal":             {chr, 2, "character device"},
		"socket":               {sock, 2, "a socket"},
		"block device":         {blk, 2, "a block device"},
		"symlink":              {lnk, oWronly, "a symbolic link"},
		"no type bits":         {0, oWronly, "not a pipe"},
		"access mode 3":        {fifo, 3, "not open write-only"},
	} {
		err := CheckOutputFD("--stdout-fd 3", tc.mode, tc.flags)
		if err == nil || !strings.Contains(err.Error(), tc.want) || !strings.Contains(err.Error(), "--stdout-fd 3") {
			t.Errorf("%s: %v, want an error containing %q", name, err, tc.want)
		}
	}
}
