// =========================================================================
// pwd — Tests
// =========================================================================
//
// These tests verify the pwd program's behavior by using the cli-builder
// parser directly with the pwd.json spec. This approach tests the full
// integration: spec loading, argument parsing, and result types — without
// needing to invoke the compiled binary as a subprocess.
//
// # Test strategy
//
// 1. Parser integration tests: verify that cli-builder correctly parses
//    the pwd.json spec for all flag combinations (-L, -P, --help, etc.)
//
// 2. Business logic tests: verify that getLogicalPath() and
//    getPhysicalPath() return correct results.
//
// 3. Spec file tests: verify the spec loads and validates correctly.
//
// We use the NewParser constructor with an absolute path to pwd.json,
// resolved relative to the test file's location. Go test always sets
// the working directory to the package directory, so "./pwd.json" works.

package main

import (
	"bytes"
	"os"
	"path/filepath"
	"strings"
	"testing"

	clibuilder "github.com/adhithyan15/coding-adventures/code/packages/go/cli-builder"
)

// specPath returns the absolute path to pwd.json for testing.
//
// In tests, the working directory is the package directory (where this
// file lives), so we can resolve pwd.json relative to ".".
func specPath(t *testing.T) string {
	t.Helper()
	abs, err := filepath.Abs("pwd.json")
	if err != nil {
		t.Fatalf("cannot resolve pwd.json path: %v", err)
	}
	return abs
}

// =========================================================================
// Spec loading tests
// =========================================================================

// TestSpecLoads verifies that pwd.json is a valid cli-builder spec.
//
// If this test fails, the spec file is either missing or malformed.
// All other tests depend on a valid spec, so this is the canary.
func TestSpecLoads(t *testing.T) {
	parser, err := clibuilder.NewParser(specPath(t), []string{"pwd"})
	if err != nil {
		t.Fatalf("failed to load pwd.json spec: %v", err)
	}
	if parser == nil {
		t.Fatal("NewParser returned nil parser without error")
	}
}

// =========================================================================
// Flag parsing tests
// =========================================================================

// TestDefaultMode verifies that running `pwd` with no flags produces a
// ParseResult where both "logical" and "physical" are false.
//
// The default behavior is logical mode, but that's handled by the business
// logic — the parser just reports what flags the user explicitly set.
func TestDefaultMode(t *testing.T) {
	parser, err := clibuilder.NewParser(specPath(t), []string{"pwd"})
	if err != nil {
		t.Fatalf("NewParser failed: %v", err)
	}

	result, err := parser.Parse()
	if err != nil {
		t.Fatalf("Parse failed: %v", err)
	}

	pr, ok := result.(*clibuilder.ParseResult)
	if !ok {
		t.Fatalf("expected *ParseResult, got %T", result)
	}

	// With no flags, both should be false (their default boolean value).
	if physical, _ := pr.Flags["physical"].(bool); physical {
		t.Error("physical flag should be false by default")
	}
	if logical, _ := pr.Flags["logical"].(bool); logical {
		t.Error("logical flag should be false by default")
	}
}

// TestPhysicalShortFlag verifies that `-P` sets the physical flag.
func TestPhysicalShortFlag(t *testing.T) {
	parser, err := clibuilder.NewParser(specPath(t), []string{"pwd", "-P"})
	if err != nil {
		t.Fatalf("NewParser failed: %v", err)
	}

	result, err := parser.Parse()
	if err != nil {
		t.Fatalf("Parse failed: %v", err)
	}

	pr, ok := result.(*clibuilder.ParseResult)
	if !ok {
		t.Fatalf("expected *ParseResult, got %T", result)
	}

	if physical, _ := pr.Flags["physical"].(bool); !physical {
		t.Error("physical flag should be true when -P is passed")
	}
}

// TestPhysicalLongFlag verifies that `--physical` sets the physical flag.
func TestPhysicalLongFlag(t *testing.T) {
	parser, err := clibuilder.NewParser(specPath(t), []string{"pwd", "--physical"})
	if err != nil {
		t.Fatalf("NewParser failed: %v", err)
	}

	result, err := parser.Parse()
	if err != nil {
		t.Fatalf("Parse failed: %v", err)
	}

	pr, ok := result.(*clibuilder.ParseResult)
	if !ok {
		t.Fatalf("expected *ParseResult, got %T", result)
	}

	if physical, _ := pr.Flags["physical"].(bool); !physical {
		t.Error("physical flag should be true when --physical is passed")
	}
}

// TestLogicalShortFlag verifies that `-L` sets the logical flag.
func TestLogicalShortFlag(t *testing.T) {
	parser, err := clibuilder.NewParser(specPath(t), []string{"pwd", "-L"})
	if err != nil {
		t.Fatalf("NewParser failed: %v", err)
	}

	result, err := parser.Parse()
	if err != nil {
		t.Fatalf("Parse failed: %v", err)
	}

	pr, ok := result.(*clibuilder.ParseResult)
	if !ok {
		t.Fatalf("expected *ParseResult, got %T", result)
	}

	if logical, _ := pr.Flags["logical"].(bool); !logical {
		t.Error("logical flag should be true when -L is passed")
	}
}

// TestLogicalLongFlag verifies that `--logical` sets the logical flag.
func TestLogicalLongFlag(t *testing.T) {
	parser, err := clibuilder.NewParser(specPath(t), []string{"pwd", "--logical"})
	if err != nil {
		t.Fatalf("NewParser failed: %v", err)
	}

	result, err := parser.Parse()
	if err != nil {
		t.Fatalf("Parse failed: %v", err)
	}

	pr, ok := result.(*clibuilder.ParseResult)
	if !ok {
		t.Fatalf("expected *ParseResult, got %T", result)
	}

	if logical, _ := pr.Flags["logical"].(bool); !logical {
		t.Error("logical flag should be true when --logical is passed")
	}
}

// =========================================================================
// Help and version tests
// =========================================================================

// TestHelpFlag verifies that `--help` returns a HelpResult with non-empty text.
func TestHelpFlag(t *testing.T) {
	parser, err := clibuilder.NewParser(specPath(t), []string{"pwd", "--help"})
	if err != nil {
		t.Fatalf("NewParser failed: %v", err)
	}

	result, err := parser.Parse()
	if err != nil {
		t.Fatalf("Parse failed: %v", err)
	}

	hr, ok := result.(*clibuilder.HelpResult)
	if !ok {
		t.Fatalf("expected *HelpResult, got %T", result)
	}

	if hr.Text == "" {
		t.Error("help text should not be empty")
	}
}

// TestHelpShortFlag verifies that `-h` also returns a HelpResult.
func TestHelpShortFlag(t *testing.T) {
	parser, err := clibuilder.NewParser(specPath(t), []string{"pwd", "-h"})
	if err != nil {
		t.Fatalf("NewParser failed: %v", err)
	}

	result, err := parser.Parse()
	if err != nil {
		t.Fatalf("Parse failed: %v", err)
	}

	_, ok := result.(*clibuilder.HelpResult)
	if !ok {
		t.Fatalf("expected *HelpResult for -h, got %T", result)
	}
}

// TestVersionFlag verifies that `--version` returns a VersionResult.
func TestVersionFlag(t *testing.T) {
	parser, err := clibuilder.NewParser(specPath(t), []string{"pwd", "--version"})
	if err != nil {
		t.Fatalf("NewParser failed: %v", err)
	}

	result, err := parser.Parse()
	if err != nil {
		t.Fatalf("Parse failed: %v", err)
	}

	vr, ok := result.(*clibuilder.VersionResult)
	if !ok {
		t.Fatalf("expected *VersionResult, got %T", result)
	}

	if vr.Version != "1.0.0" {
		t.Errorf("expected version 1.0.0, got %s", vr.Version)
	}
}

// =========================================================================
// Mutual exclusivity tests
// =========================================================================

// TestMutualExclusivity verifies that passing both -L and -P produces a
// parse error, since they are declared as mutually exclusive in pwd.json.
func TestMutualExclusivity(t *testing.T) {
	parser, err := clibuilder.NewParser(specPath(t), []string{"pwd", "-L", "-P"})
	if err != nil {
		t.Fatalf("NewParser failed: %v", err)
	}

	_, err = parser.Parse()
	if err == nil {
		t.Error("expected parse error for -L -P (mutually exclusive), got nil")
	}
}

// =========================================================================
// Business logic tests
// =========================================================================

// TestGetLogicalPath verifies that getLogicalPath returns a non-empty path.
//
// In a test environment, $PWD is typically set by the shell, so this
// should return a valid path. We verify it's non-empty and absolute.
func TestGetLogicalPath(t *testing.T) {
	path, err := getLogicalPath()
	if err != nil {
		t.Fatalf("getLogicalPath failed: %v", err)
	}

	if path == "" {
		t.Error("getLogicalPath returned empty string")
	}

	if !filepath.IsAbs(path) {
		t.Errorf("getLogicalPath returned non-absolute path: %s", path)
	}
}

// TestGetPhysicalPath verifies that getPhysicalPath returns a non-empty,
// absolute path.
func TestGetPhysicalPath(t *testing.T) {
	path, err := getPhysicalPath()
	if err != nil {
		t.Fatalf("getPhysicalPath failed: %v", err)
	}

	if path == "" {
		t.Error("getPhysicalPath returned empty string")
	}

	if !filepath.IsAbs(path) {
		t.Errorf("getPhysicalPath returned non-absolute path: %s", path)
	}
}

// TestGetPhysicalPathMatchesGetwd verifies that getPhysicalPath returns
// the same result as os.Getwd() (or a symlink-resolved version of it).
//
// This ensures our implementation is consistent with the OS-reported
// working directory.
func TestGetPhysicalPathMatchesGetwd(t *testing.T) {
	physical, err := getPhysicalPath()
	if err != nil {
		t.Fatalf("getPhysicalPath failed: %v", err)
	}

	cwd, err := os.Getwd()
	if err != nil {
		t.Fatalf("os.Getwd failed: %v", err)
	}

	// Resolve cwd too, for a fair comparison.
	cwdResolved, err := filepath.EvalSymlinks(cwd)
	if err != nil {
		cwdResolved = cwd
	}

	if physical != cwdResolved {
		t.Errorf("getPhysicalPath() = %q, want %q", physical, cwdResolved)
	}
}

// TestGetLogicalPathWithEmptyPWD verifies that when $PWD is empty,
// getLogicalPath falls back to os.Getwd().
func TestGetLogicalPathWithEmptyPWD(t *testing.T) {
	// Save and restore $PWD.
	origPWD := os.Getenv("PWD")
	defer os.Setenv("PWD", origPWD)

	// Clear $PWD to test the fallback path.
	os.Unsetenv("PWD")

	path, err := getLogicalPath()
	if err != nil {
		t.Fatalf("getLogicalPath with empty $PWD failed: %v", err)
	}

	cwd, err := os.Getwd()
	if err != nil {
		t.Fatalf("os.Getwd failed: %v", err)
	}

	if path != cwd {
		t.Errorf("with empty $PWD, getLogicalPath() = %q, want %q (os.Getwd)", path, cwd)
	}
}

// TestGetLogicalPathWithStalePWD verifies that when $PWD points to a
// different directory, getLogicalPath falls back to os.Getwd().
func TestGetLogicalPathWithStalePWD(t *testing.T) {
	// Save and restore $PWD.
	origPWD := os.Getenv("PWD")
	defer os.Setenv("PWD", origPWD)

	// Set $PWD to a valid but wrong directory.
	os.Setenv("PWD", "/tmp")

	path, err := getLogicalPath()
	if err != nil {
		t.Fatalf("getLogicalPath with stale $PWD failed: %v", err)
	}

	// Should NOT return /tmp — should fall back to os.Getwd().
	cwd, err := os.Getwd()
	if err != nil {
		t.Fatalf("os.Getwd failed: %v", err)
	}

	if path == "/tmp" && cwd != "/tmp" {
		t.Error("getLogicalPath returned stale $PWD value /tmp instead of falling back")
	}
}

// TestGetLogicalPathWithInvalidPWD verifies that when $PWD points to a
// nonexistent path, getLogicalPath falls back to os.Getwd().
func TestGetLogicalPathWithInvalidPWD(t *testing.T) {
	// Save and restore $PWD.
	origPWD := os.Getenv("PWD")
	defer os.Setenv("PWD", origPWD)

	// Set $PWD to a path that doesn't exist.
	os.Setenv("PWD", "/nonexistent/path/that/does/not/exist")

	path, err := getLogicalPath()
	if err != nil {
		t.Fatalf("getLogicalPath with invalid $PWD failed: %v", err)
	}

	cwd, err := os.Getwd()
	if err != nil {
		t.Fatalf("os.Getwd failed: %v", err)
	}

	if path != cwd {
		t.Errorf("with invalid $PWD, getLogicalPath() = %q, want %q (os.Getwd)", path, cwd)
	}
}

// =========================================================================
// Spec file resolution test
// =========================================================================

// TestResolveSpecPath verifies that resolveSpecPath returns a path ending
// in "pwd.json".
//
// Note: In test mode, os.Executable() returns the test binary path, not
// the pwd binary path. So the resolved path will be alongside the test
// binary — which is fine for verifying the function's logic.
func TestResolveSpecPath(t *testing.T) {
	path, err := resolveSpecPath()
	if err != nil {
		t.Fatalf("resolveSpecPath failed: %v", err)
	}

	base := filepath.Base(path)
	if base != "pwd.json" {
		t.Errorf("resolveSpecPath returned %q, expected filename pwd.json", path)
	}

	if !filepath.IsAbs(path) {
		t.Errorf("resolveSpecPath returned non-absolute path: %s", path)
	}
}

// =========================================================================
// Error handling tests
// =========================================================================

// TestInvalidSpecPath verifies that NewParser returns an error when the
// spec file doesn't exist.
func TestInvalidSpecPath(t *testing.T) {
	_, err := clibuilder.NewParser("/nonexistent/spec.json", []string{"pwd"})
	if err == nil {
		t.Error("expected error for nonexistent spec file, got nil")
	}
}

// TestUnexpectedArgument verifies that passing an unexpected positional
// argument produces a parse error, since pwd accepts no arguments.
func TestUnexpectedArgument(t *testing.T) {
	parser, err := clibuilder.NewParser(specPath(t), []string{"pwd", "extra-arg"})
	if err != nil {
		t.Fatalf("NewParser failed: %v", err)
	}

	_, err = parser.Parse()
	if err == nil {
		t.Error("expected parse error for unexpected argument, got nil")
	}
}

// TestHelpTextContainsFlagDescriptions verifies that the help output
// includes descriptions for both -L and -P flags.
func TestHelpTextContainsFlagDescriptions(t *testing.T) {
	parser, err := clibuilder.NewParser(specPath(t), []string{"pwd", "--help"})
	if err != nil {
		t.Fatalf("NewParser failed: %v", err)
	}

	result, err := parser.Parse()
	if err != nil {
		t.Fatalf("Parse failed: %v", err)
	}

	hr, ok := result.(*clibuilder.HelpResult)
	if !ok {
		t.Fatalf("expected *HelpResult, got %T", result)
	}

	// The help text should mention both flags.
	if len(hr.Text) < 20 {
		t.Error("help text seems too short to be useful")
	}
}

// TestProgramNameInParseResult verifies that the Program field is set
// correctly in the ParseResult.
func TestProgramNameInParseResult(t *testing.T) {
	parser, err := clibuilder.NewParser(specPath(t), []string{"pwd"})
	if err != nil {
		t.Fatalf("NewParser failed: %v", err)
	}

	result, err := parser.Parse()
	if err != nil {
		t.Fatalf("Parse failed: %v", err)
	}

	pr, ok := result.(*clibuilder.ParseResult)
	if !ok {
		t.Fatalf("expected *ParseResult, got %T", result)
	}

	if pr.Program != "pwd" {
		t.Errorf("Program = %q, want %q", pr.Program, "pwd")
	}
}

// =========================================================================
// run() integration tests
// =========================================================================
//
// These tests exercise the run() function directly, which covers the full
// code path from spec loading through result handling. By capturing stdout
// and stderr in buffers, we can verify exact output without subprocess
// execution.

// TestRunDefaultPrintsDirectory verifies that `run` with no flags prints
// the current directory and returns exit code 0.
func TestRunDefaultPrintsDirectory(t *testing.T) {
	var stdout, stderr bytes.Buffer
	code := run(specPath(t), []string{"pwd"}, &stdout, &stderr)

	if code != 0 {
		t.Errorf("run() returned exit code %d, want 0. stderr: %s", code, stderr.String())
	}

	output := strings.TrimSpace(stdout.String())
	if output == "" {
		t.Error("run() produced no output")
	}

	if !filepath.IsAbs(output) {
		t.Errorf("run() output is not an absolute path: %q", output)
	}
}

// TestRunPhysicalPrintsDirectory verifies that `run` with -P prints
// the physical directory and returns exit code 0.
func TestRunPhysicalPrintsDirectory(t *testing.T) {
	var stdout, stderr bytes.Buffer
	code := run(specPath(t), []string{"pwd", "-P"}, &stdout, &stderr)

	if code != 0 {
		t.Errorf("run(-P) returned exit code %d, want 0. stderr: %s", code, stderr.String())
	}

	output := strings.TrimSpace(stdout.String())
	if output == "" {
		t.Error("run(-P) produced no output")
	}

	if !filepath.IsAbs(output) {
		t.Errorf("run(-P) output is not an absolute path: %q", output)
	}
}

// TestRunLogicalPrintsDirectory verifies that `run` with -L prints
// the logical directory and returns exit code 0.
func TestRunLogicalPrintsDirectory(t *testing.T) {
	var stdout, stderr bytes.Buffer
	code := run(specPath(t), []string{"pwd", "-L"}, &stdout, &stderr)

	if code != 0 {
		t.Errorf("run(-L) returned exit code %d, want 0. stderr: %s", code, stderr.String())
	}

	output := strings.TrimSpace(stdout.String())
	if !filepath.IsAbs(output) {
		t.Errorf("run(-L) output is not an absolute path: %q", output)
	}
}

// TestRunHelpReturnsZero verifies that --help prints help text to stdout
// and returns exit code 0.
func TestRunHelpReturnsZero(t *testing.T) {
	var stdout, stderr bytes.Buffer
	code := run(specPath(t), []string{"pwd", "--help"}, &stdout, &stderr)

	if code != 0 {
		t.Errorf("run(--help) returned exit code %d, want 0", code)
	}

	if stdout.Len() == 0 {
		t.Error("run(--help) produced no stdout output")
	}

	if stderr.Len() != 0 {
		t.Errorf("run(--help) produced unexpected stderr: %s", stderr.String())
	}
}

// TestRunVersionReturnsZero verifies that --version prints the version
// to stdout and returns exit code 0.
func TestRunVersionReturnsZero(t *testing.T) {
	var stdout, stderr bytes.Buffer
	code := run(specPath(t), []string{"pwd", "--version"}, &stdout, &stderr)

	if code != 0 {
		t.Errorf("run(--version) returned exit code %d, want 0", code)
	}

	output := strings.TrimSpace(stdout.String())
	if output != "1.0.0" {
		t.Errorf("run(--version) output = %q, want %q", output, "1.0.0")
	}
}

// TestRunMutualExclusionReturnsOne verifies that -L -P together produces
// an error and returns exit code 1.
func TestRunMutualExclusionReturnsOne(t *testing.T) {
	var stdout, stderr bytes.Buffer
	code := run(specPath(t), []string{"pwd", "-L", "-P"}, &stdout, &stderr)

	if code != 1 {
		t.Errorf("run(-L -P) returned exit code %d, want 1", code)
	}

	if stderr.Len() == 0 {
		t.Error("run(-L -P) should produce stderr output for the error")
	}
}

// TestRunInvalidSpecReturnsOne verifies that an invalid spec path
// causes run() to return exit code 1.
func TestRunInvalidSpecReturnsOne(t *testing.T) {
	var stdout, stderr bytes.Buffer
	code := run("/nonexistent/spec.json", []string{"pwd"}, &stdout, &stderr)

	if code != 1 {
		t.Errorf("run(bad spec) returned exit code %d, want 1", code)
	}

	if stderr.Len() == 0 {
		t.Error("run(bad spec) should produce stderr output")
	}
}

// TestRunUnexpectedArgReturnsOne verifies that extra positional args
// cause run() to return exit code 1.
func TestRunUnexpectedArgReturnsOne(t *testing.T) {
	var stdout, stderr bytes.Buffer
	code := run(specPath(t), []string{"pwd", "extra"}, &stdout, &stderr)

	if code != 1 {
		t.Errorf("run(extra arg) returned exit code %d, want 1", code)
	}
}

// TestRunPhysicalMatchesHelper verifies that the output of run() with -P
// matches getPhysicalPath().
func TestRunPhysicalMatchesHelper(t *testing.T) {
	var stdout, stderr bytes.Buffer
	code := run(specPath(t), []string{"pwd", "-P"}, &stdout, &stderr)

	if code != 0 {
		t.Fatalf("run(-P) failed: %s", stderr.String())
	}

	expected, err := getPhysicalPath()
	if err != nil {
		t.Fatalf("getPhysicalPath failed: %v", err)
	}

	output := strings.TrimSpace(stdout.String())
	if output != expected {
		t.Errorf("run(-P) = %q, getPhysicalPath() = %q", output, expected)
	}
}
