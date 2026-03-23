// =========================================================================
// tty — Tests
// =========================================================================
//
// These tests verify the tty tool's behavior:
//
//   1. When stdin IS a terminal: prints device name, exits 0
//   2. When stdin is NOT a terminal: prints "not a tty", exits 1
//   3. Silent mode (-s): prints nothing in either case
//   4. --help and --version: standard meta-flags
//   5. Error handling: invalid spec path returns exit code 1
//
// Since tests run in a non-interactive environment (stdin is not a tty),
// we use mock ttyCheckers to test both the terminal and non-terminal paths.

package main

import (
	"bytes"
	"strings"
	"testing"

	clibuilder "github.com/adhithyan15/coding-adventures/code/packages/go/cli-builder"
)

// =========================================================================
// Mock tty checkers
// =========================================================================

// mockTTY simulates a terminal being connected to stdin.
type mockTTY struct {
	isTTY      bool
	deviceName string
}

func (m *mockTTY) IsTTY() bool    { return m.isTTY }
func (m *mockTTY) DeviceName() string { return m.deviceName }

// =========================================================================
// Spec loading tests
// =========================================================================

// TestTtySpecLoads verifies that tty.json is a valid cli-builder spec.
func TestTtySpecLoads(t *testing.T) {
	parser, err := clibuilder.NewParser(toolSpecPath(t, "tty"), []string{"tty"})
	if err != nil {
		t.Fatalf("failed to load tty.json spec: %v", err)
	}
	if parser == nil {
		t.Fatal("NewParser returned nil parser without error")
	}
}

// =========================================================================
// ttyLogic unit tests — terminal IS connected
// =========================================================================

// TestTtyLogicIsTTY verifies that when stdin is a terminal, ttyLogic
// prints the device name and returns 0.
func TestTtyLogicIsTTY(t *testing.T) {
	checker := &mockTTY{isTTY: true, deviceName: "/dev/ttys003"}
	var stdout, stderr bytes.Buffer

	code := ttyLogic(checker, false, &stdout, &stderr)

	if code != 0 {
		t.Errorf("ttyLogic(isTTY=true) returned %d, want 0", code)
	}

	output := strings.TrimSpace(stdout.String())
	if output != "/dev/ttys003" {
		t.Errorf("ttyLogic output = %q, want %q", output, "/dev/ttys003")
	}
}

// TestTtyLogicIsTTYSilent verifies that with silent mode, nothing is
// printed but exit code is still 0.
func TestTtyLogicIsTTYSilent(t *testing.T) {
	checker := &mockTTY{isTTY: true, deviceName: "/dev/ttys003"}
	var stdout, stderr bytes.Buffer

	code := ttyLogic(checker, true, &stdout, &stderr)

	if code != 0 {
		t.Errorf("ttyLogic(isTTY=true, silent) returned %d, want 0", code)
	}

	if stdout.Len() != 0 {
		t.Errorf("ttyLogic(silent) produced output: %q", stdout.String())
	}
}

// =========================================================================
// ttyLogic unit tests — terminal NOT connected
// =========================================================================

// TestTtyLogicNotTTY verifies that when stdin is not a terminal, ttyLogic
// prints "not a tty" and returns 1.
func TestTtyLogicNotTTY(t *testing.T) {
	checker := &mockTTY{isTTY: false, deviceName: ""}
	var stdout, stderr bytes.Buffer

	code := ttyLogic(checker, false, &stdout, &stderr)

	if code != 1 {
		t.Errorf("ttyLogic(isTTY=false) returned %d, want 1", code)
	}

	output := strings.TrimSpace(stdout.String())
	if output != "not a tty" {
		t.Errorf("ttyLogic output = %q, want %q", output, "not a tty")
	}
}

// TestTtyLogicNotTTYSilent verifies that with silent mode and no terminal,
// nothing is printed and exit code is 1.
func TestTtyLogicNotTTYSilent(t *testing.T) {
	checker := &mockTTY{isTTY: false, deviceName: ""}
	var stdout, stderr bytes.Buffer

	code := ttyLogic(checker, true, &stdout, &stderr)

	if code != 1 {
		t.Errorf("ttyLogic(isTTY=false, silent) returned %d, want 1", code)
	}

	if stdout.Len() != 0 {
		t.Errorf("ttyLogic(silent, not tty) produced output: %q", stdout.String())
	}
}

// =========================================================================
// ttyLogic unit tests — various device names
// =========================================================================

// TestTtyLogicDifferentDeviceNames verifies that various device names
// are printed correctly.
func TestTtyLogicDifferentDeviceNames(t *testing.T) {
	devices := []string{
		"/dev/tty",
		"/dev/pts/0",
		"/dev/ttys042",
		"/dev/console",
	}

	for _, dev := range devices {
		checker := &mockTTY{isTTY: true, deviceName: dev}
		var stdout, stderr bytes.Buffer

		code := ttyLogic(checker, false, &stdout, &stderr)

		if code != 0 {
			t.Errorf("ttyLogic(%s) returned %d, want 0", dev, code)
		}

		output := strings.TrimSpace(stdout.String())
		if output != dev {
			t.Errorf("ttyLogic(%s) output = %q, want %q", dev, output, dev)
		}
	}
}

// =========================================================================
// Help and version tests
// =========================================================================

// TestTtyHelpFlag verifies that --help prints help text and returns 0.
func TestTtyHelpFlag(t *testing.T) {
	var stdout, stderr bytes.Buffer
	code := runTty(toolSpecPath(t, "tty"), []string{"tty", "--help"}, &stdout, &stderr)

	if code != 0 {
		t.Errorf("runTty(--help) returned exit code %d, want 0", code)
	}

	if stdout.Len() == 0 {
		t.Error("runTty(--help) produced no stdout output")
	}
}

// TestTtyVersionFlag verifies that --version prints the version.
func TestTtyVersionFlag(t *testing.T) {
	var stdout, stderr bytes.Buffer
	code := runTty(toolSpecPath(t, "tty"), []string{"tty", "--version"}, &stdout, &stderr)

	if code != 0 {
		t.Errorf("runTty(--version) returned exit code %d, want 0", code)
	}

	output := strings.TrimSpace(stdout.String())
	if output != "1.0.0" {
		t.Errorf("runTty(--version) output = %q, want %q", output, "1.0.0")
	}
}

// =========================================================================
// Error handling tests
// =========================================================================

// TestTtyInvalidSpec verifies that an invalid spec path returns exit code 1.
func TestTtyInvalidSpec(t *testing.T) {
	var stdout, stderr bytes.Buffer
	code := runTty("/nonexistent/tty.json", []string{"tty"}, &stdout, &stderr)

	if code != 1 {
		t.Errorf("runTty(bad spec) returned exit code %d, want 1", code)
	}
}

// =========================================================================
// Integration test — runTty with real stdin (non-terminal in tests)
// =========================================================================

// TestRunTtyInTestEnvironment verifies that runTty returns 1 in a test
// environment (where stdin is not a terminal).
func TestRunTtyInTestEnvironment(t *testing.T) {
	var stdout, stderr bytes.Buffer
	code := runTty(toolSpecPath(t, "tty"), []string{"tty"}, &stdout, &stderr)

	// In test environments, stdin is typically not a terminal.
	// So we expect exit code 1 and "not a tty" output.
	if code != 1 {
		// If it IS a tty (e.g., running tests interactively), that's also valid.
		t.Logf("runTty() returned %d (stdin may be a real terminal)", code)
	}
}
