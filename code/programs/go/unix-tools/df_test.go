// =========================================================================
// df — Tests
// =========================================================================

package main

import (
	"bytes"
	"runtime"
	"strings"
	"testing"
)

// =========================================================================
// getFilesystemInfo tests
// =========================================================================

// TestGetFilesystemInfoRoot verifies that we can stat the root filesystem.
func TestGetFilesystemInfoRoot(t *testing.T) {
	if runtime.GOOS == "windows" {
		t.Skip("df uses Unix-specific filesystem info")
	}
	info, err := getFilesystemInfo("/")
	if err != nil {
		t.Fatalf("getFilesystemInfo(\"/\") failed: %v", err)
	}

	if info.TotalBytes == 0 {
		t.Error("TotalBytes is 0")
	}
	if info.MountPoint != "/" {
		t.Errorf("MountPoint = %q, want %q", info.MountPoint, "/")
	}
	if info.UsePercent < 0 || info.UsePercent > 100 {
		t.Errorf("UsePercent = %d, want 0-100", info.UsePercent)
	}
}

// TestGetFilesystemInfoInvalidPath verifies error for nonexistent path.
func TestGetFilesystemInfoInvalidPath(t *testing.T) {
	_, err := getFilesystemInfo("/nonexistent_path_xyzzy_99999")
	if err == nil {
		t.Error("getFilesystemInfo(invalid) should have returned an error")
	}
}

// TestGetFilesystemInfoTempDir verifies statting a temp directory.
func TestGetFilesystemInfoTempDir(t *testing.T) {
	if runtime.GOOS == "windows" {
		t.Skip("df uses Unix-specific filesystem info")
	}
	dir := t.TempDir()
	info, err := getFilesystemInfo(dir)
	if err != nil {
		t.Fatalf("getFilesystemInfo(tempdir) failed: %v", err)
	}

	// Available + used should not exceed total (sanity check).
	if info.AvailBytes > info.TotalBytes {
		t.Errorf("AvailBytes (%d) > TotalBytes (%d)", info.AvailBytes, info.TotalBytes)
	}
}

// =========================================================================
// formatSize tests
// =========================================================================

// TestFormatSizeDefault verifies default (1K-blocks) formatting.
func TestFormatSizeDefault(t *testing.T) {
	got := formatSize(1048576, false, false) // 1 MB = 1024 1K-blocks
	if got != "1024" {
		t.Errorf("formatSize(1MB, default) = %q, want %q", got, "1024")
	}
}

// TestFormatSizeHumanReadable verifies -h formatting.
func TestFormatSizeHumanReadable(t *testing.T) {
	tests := []struct {
		bytes uint64
		want  string
	}{
		{0, "0B"},
		{500, "500B"},
		{1024, "1.0K"},
		{1048576, "1.0M"},
		{1073741824, "1.0G"},
		{10737418240, "10G"},
	}

	for _, tt := range tests {
		got := formatSize(tt.bytes, true, false)
		if got != tt.want {
			t.Errorf("formatSize(%d, -h) = %q, want %q", tt.bytes, got, tt.want)
		}
	}
}

// TestFormatSizeSI verifies -H (SI / powers of 1000) formatting.
func TestFormatSizeSI(t *testing.T) {
	got := formatSize(1000000, false, true) // 1 MB in SI
	if got != "1.0MB" {
		t.Errorf("formatSize(1000000, -H) = %q, want %q", got, "1.0MB")
	}
}

// =========================================================================
// runDf integration tests
// =========================================================================

// TestRunDfDefault verifies default output includes a header.
func TestRunDfDefault(t *testing.T) {
	if runtime.GOOS == "windows" {
		t.Skip("df uses Unix-specific filesystem info")
	}
	var stdout, stderr bytes.Buffer
	code := runDf(toolSpecPath(t, "df"), []string{"df"}, &stdout, &stderr)

	if code != 0 {
		t.Errorf("runDf() returned %d, want 0. stderr: %s", code, stderr.String())
	}

	output := stdout.String()
	if !strings.Contains(output, "Filesystem") {
		t.Errorf("output missing header 'Filesystem': %q", output)
	}
}

// TestRunDfHelp verifies --help flag.
func TestRunDfHelp(t *testing.T) {
	var stdout, stderr bytes.Buffer
	code := runDf(toolSpecPath(t, "df"), []string{"df", "--help"}, &stdout, &stderr)
	if code != 0 {
		t.Errorf("runDf(--help) returned %d, want 0", code)
	}
	if stdout.Len() == 0 {
		t.Error("runDf(--help) produced no output")
	}
}

// TestRunDfVersion verifies --version flag.
func TestRunDfVersion(t *testing.T) {
	var stdout, stderr bytes.Buffer
	code := runDf(toolSpecPath(t, "df"), []string{"df", "--version"}, &stdout, &stderr)
	if code != 0 {
		t.Errorf("runDf(--version) returned %d, want 0", code)
	}
	output := strings.TrimSpace(stdout.String())
	if output != "1.0.0" {
		t.Errorf("runDf(--version) = %q, want %q", output, "1.0.0")
	}
}

// TestRunDfInvalidSpec verifies error handling.
func TestRunDfInvalidSpec(t *testing.T) {
	var stdout, stderr bytes.Buffer
	code := runDf("/nonexistent/df.json", []string{"df"}, &stdout, &stderr)
	if code != 1 {
		t.Errorf("runDf(bad spec) returned %d, want 1", code)
	}
}
