// =========================================================================
// uname — Tests
// =========================================================================
//
// These tests verify the uname tool's behavior:
//
//   1. Business logic: getSystemInfo(), formatUname(), mapping functions
//   2. Integration: runUname() with various flags
//   3. Help/version flags
//   4. Error handling

package main

import (
	"bytes"
	"runtime"
	"strings"
	"testing"
)

// =========================================================================
// Business logic tests
// =========================================================================

// TestGetSystemInfo verifies that getSystemInfo returns non-empty fields.
func TestGetSystemInfo(t *testing.T) {
	info := getSystemInfo()

	if info.KernelName == "" {
		t.Error("KernelName is empty")
	}
	if info.Nodename == "" {
		t.Error("Nodename is empty")
	}
	if info.Machine == "" {
		t.Error("Machine is empty")
	}
	if info.OperatingSystem == "" {
		t.Error("OperatingSystem is empty")
	}
}

// TestMapKernelName verifies the GOOS-to-kernel-name mapping.
func TestMapKernelName(t *testing.T) {
	tests := []struct {
		goos string
		want string
	}{
		{"darwin", "Darwin"},
		{"linux", "Linux"},
		{"windows", "Windows_NT"},
		{"freebsd", "FreeBSD"},
		{"openbsd", "OpenBSD"},
		{"netbsd", "NetBSD"},
		{"plan9", "Plan9"},
	}

	for _, tt := range tests {
		got := mapKernelName(tt.goos)
		if got != tt.want {
			t.Errorf("mapKernelName(%q) = %q, want %q", tt.goos, got, tt.want)
		}
	}
}

// TestMapMachineName verifies the GOARCH-to-machine-name mapping.
func TestMapMachineName(t *testing.T) {
	tests := []struct {
		goarch string
		want   string
	}{
		{"amd64", "x86_64"},
		{"arm64", "aarch64"},
		{"386", "i686"},
		{"arm", "armv7l"},
		{"mips", "mips"},
	}

	for _, tt := range tests {
		got := mapMachineName(tt.goarch)
		if got != tt.want {
			t.Errorf("mapMachineName(%q) = %q, want %q", tt.goarch, got, tt.want)
		}
	}
}

// TestMapOSName verifies the GOOS-to-OS-name mapping.
func TestMapOSName(t *testing.T) {
	tests := []struct {
		goos string
		want string
	}{
		{"linux", "GNU/Linux"},
		{"darwin", "Darwin"},
		{"windows", "Windows"},
	}

	for _, tt := range tests {
		got := mapOSName(tt.goos)
		if got != tt.want {
			t.Errorf("mapOSName(%q) = %q, want %q", tt.goos, got, tt.want)
		}
	}
}

// TestFormatUnameDefault verifies that no flags prints kernel name.
func TestFormatUnameDefault(t *testing.T) {
	info := UnameInfo{
		KernelName:       "Linux",
		Nodename:         "myhost",
		KernelRelease:    "6.1.0",
		KernelVersion:    "#1 SMP",
		Machine:          "x86_64",
		Processor:        "x86_64",
		HardwarePlatform: "x86_64",
		OperatingSystem:  "GNU/Linux",
	}

	got := formatUname(info, false, false, false, false, false, false, false, false, false)
	if got != "Linux" {
		t.Errorf("formatUname(no flags) = %q, want %q", got, "Linux")
	}
}

// TestFormatUnameAll verifies that -a prints all fields.
func TestFormatUnameAll(t *testing.T) {
	info := UnameInfo{
		KernelName:       "Linux",
		Nodename:         "myhost",
		KernelRelease:    "6.1.0",
		KernelVersion:    "#1 SMP",
		Machine:          "x86_64",
		Processor:        "x86_64",
		HardwarePlatform: "x86_64",
		OperatingSystem:  "GNU/Linux",
	}

	got := formatUname(info, true, false, false, false, false, false, false, false, false)
	want := "Linux myhost 6.1.0 #1 SMP x86_64 x86_64 x86_64 GNU/Linux"
	if got != want {
		t.Errorf("formatUname(-a) = %q, want %q", got, want)
	}
}

// TestFormatUnameSelectiveFlags tests individual flag selection.
func TestFormatUnameSelectiveFlags(t *testing.T) {
	info := UnameInfo{
		KernelName:       "Darwin",
		Nodename:         "mac.local",
		KernelRelease:    "23.1.0",
		KernelVersion:    "Darwin Kernel",
		Machine:          "arm64",
		Processor:        "arm64",
		HardwarePlatform: "arm64",
		OperatingSystem:  "Darwin",
	}

	// -s -n should print kernel name and nodename.
	got := formatUname(info, false, true, true, false, false, false, false, false, false)
	if got != "Darwin mac.local" {
		t.Errorf("formatUname(-s -n) = %q, want %q", got, "Darwin mac.local")
	}

	// -m only should print machine.
	got = formatUname(info, false, false, false, false, false, true, false, false, false)
	if got != "arm64" {
		t.Errorf("formatUname(-m) = %q, want %q", got, "arm64")
	}
}

// =========================================================================
// runUname integration tests
// =========================================================================

// TestRunUnameDefault verifies default output (kernel name only).
func TestRunUnameDefault(t *testing.T) {
	var stdout, stderr bytes.Buffer
	code := runUname(toolSpecPath(t, "uname"), []string{"uname"}, &stdout, &stderr)

	if code != 0 {
		t.Errorf("runUname() returned %d, want 0. stderr: %s", code, stderr.String())
	}

	output := strings.TrimSpace(stdout.String())
	expected := mapKernelName(runtime.GOOS)
	if output != expected {
		t.Errorf("runUname() = %q, want %q", output, expected)
	}
}

// TestRunUnameHelp verifies --help flag.
func TestRunUnameHelp(t *testing.T) {
	var stdout, stderr bytes.Buffer
	code := runUname(toolSpecPath(t, "uname"), []string{"uname", "--help"}, &stdout, &stderr)

	if code != 0 {
		t.Errorf("runUname(--help) returned %d, want 0", code)
	}
	if stdout.Len() == 0 {
		t.Error("runUname(--help) produced no output")
	}
}

// TestRunUnameVersion verifies --version flag.
func TestRunUnameVersion(t *testing.T) {
	var stdout, stderr bytes.Buffer
	code := runUname(toolSpecPath(t, "uname"), []string{"uname", "--version"}, &stdout, &stderr)

	if code != 0 {
		t.Errorf("runUname(--version) returned %d, want 0", code)
	}
	output := strings.TrimSpace(stdout.String())
	if output != "1.0.0" {
		t.Errorf("runUname(--version) = %q, want %q", output, "1.0.0")
	}
}

// TestRunUnameInvalidSpec verifies error handling for bad spec path.
func TestRunUnameInvalidSpec(t *testing.T) {
	var stdout, stderr bytes.Buffer
	code := runUname("/nonexistent/uname.json", []string{"uname"}, &stdout, &stderr)
	if code != 1 {
		t.Errorf("runUname(bad spec) returned %d, want 1", code)
	}
}
