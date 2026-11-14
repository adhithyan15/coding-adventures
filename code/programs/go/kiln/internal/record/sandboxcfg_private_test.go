package record

import (
	"strings"
	"testing"

	"github.com/adhithyan15/coding-adventures/code/programs/go/kiln/internal/legacy"
)

// M1: a state root is writable from the sandbox, so it may overlap none of the
// places the recorder keeps to itself (the proof of concept of the review).

func privateInput(extra ...legacy.HostRoot) ConfigInput {
	f := runnerFS()
	in := baseInput(f)
	in.Roots = hostRoots(append([]legacy.HostRoot{{Path: "/usr", Kind: legacy.RootImage}}, extra...)...)
	in.Private = []Region{
		{"the record directory", "/home/runner/work/_temp/kiln-record"}, {"RUNNER_TEMP", "/home/runner/work/_temp"},
		{"the kiln binary", "/home/runner/work/_temp/bin/kiln"}, {"the directory of GITHUB_ENV", "/home/runner/work/_temp/_runner_file_commands"},
	}
	in.PrivateState = []Region{{"the strace binary", "/usr/bin/strace"}, {"the git binary", "/usr/bin/git"},
		{"a directory of the recorder's PATH", "/home/runner/.local/bin"}, {"a directory of the recorder's PATH", "/usr/bin"}}
	return in
}

func TestStateRootsMayNotOverlapPrivateRegions(t *testing.T) {
	if _, err := BuildSandboxConfig(privateInput()); err != nil {
		t.Fatalf("the real runner configuration was refused: %v", err)
	}
	for _, p := range []string{
		"$HOME/work/_temp/kiln-record/traces", // inside the record directory
		"$HOME/work/_temp/kiln-record/post",
		"$HOME/work/_temp/newdir", // inside RUNNER_TEMP
		"$HOME/work/_temp",        // over RUNNER_TEMP
		"$HOME/work",              // over everything beneath it
		"$HOME/work/_temp/bin",    // over the kiln binary's directory
		"$HOME/work/_temp/bin/kiln",
		"$HOME/work/_temp/_runner_file_commands/x",
		"$HOME/.local/bin", // on the recorder's own PATH
		"$HOME/.local",     // over a PATH directory
		"/usr/bin/git",     // the git binary
	} {
		_, err := BuildSandboxConfig(privateInput(legacy.HostRoot{Path: p, Kind: legacy.RootState, Ecosystem: "x"}))
		if err == nil {
			t.Errorf("state root %s was accepted", p)
		}
	}
}

// /usr is an ro root and holds the image's strace and git; only a WRITABLE root
// over them is refused.
func TestStraceAndGitInAReadOnlyRootAreFine(t *testing.T) {
	cfg, err := BuildSandboxConfig(privateInput())
	if err != nil {
		t.Fatal(err)
	}
	found := false
	for _, r := range cfg.RoRoots {
		found = found || r == "/usr"
	}
	if !found {
		t.Fatalf("%v", cfg.RoRoots)
	}
}

func TestXDGConfigHomeMustBeAbsolute(t *testing.T) {
	in := baseInput(runnerFS())
	in.XDGConfigHome = "relative/config"
	if _, err := BuildSandboxConfig(in); err == nil || !strings.Contains(err.Error(), "XDG_CONFIG_HOME") {
		t.Fatalf("%v", err)
	}
}
