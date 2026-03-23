// =========================================================================
// chmod — Tests
// =========================================================================
//
// These tests verify the chmod tool's behavior, covering:
//
//   1. Spec loading
//   2. Octal mode parsing (755, 644, etc.)
//   3. Symbolic mode parsing (u+rwx, go-w, a=r, etc.)
//   4. Applying octal modes to files
//   5. Applying symbolic modes to files
//   6. Recursive mode (-R)
//   7. Verbose mode (-v)
//   8. Changes-only mode (-c)
//   9. Silent mode (-f)
//  10. Error handling (missing files, invalid modes)

package main

import (
	"bytes"
	"os"
	"path/filepath"
	"runtime"
	"strings"
	"testing"

	clibuilder "github.com/adhithyan15/coding-adventures/code/packages/go/cli-builder"
)

// =========================================================================
// Spec loading test
// =========================================================================

func TestChmodSpecLoads(t *testing.T) {
	parser, err := clibuilder.NewParser(toolSpecPath(t, "chmod"), []string{"chmod", "755", "file"})
	if err != nil {
		t.Fatalf("failed to load chmod.json spec: %v", err)
	}
	if parser == nil {
		t.Fatal("NewParser returned nil parser without error")
	}
}

// =========================================================================
// Octal parsing tests
// =========================================================================

func TestParseChmodOctalValid(t *testing.T) {
	tests := []struct {
		input    string
		expected os.FileMode
	}{
		{"755", 0755},
		{"644", 0644},
		{"777", 0777},
		{"000", 0000},
		{"700", 0700},
		{"0755", 0755},
	}

	for _, tt := range tests {
		t.Run(tt.input, func(t *testing.T) {
			mode, ok := parseChmodOctal(tt.input)
			if !ok {
				t.Fatalf("parseChmodOctal(%q) returned not ok", tt.input)
			}
			if mode != tt.expected {
				t.Errorf("parseChmodOctal(%q) = %04o, want %04o", tt.input, mode, tt.expected)
			}
		})
	}
}

func TestParseChmodOctalInvalid(t *testing.T) {
	invalids := []string{"abc", "89", "12", "7777777", ""}
	for _, s := range invalids {
		t.Run(s, func(t *testing.T) {
			_, ok := parseChmodOctal(s)
			if ok {
				t.Errorf("parseChmodOctal(%q) should return not ok", s)
			}
		})
	}
}

// =========================================================================
// Symbolic parsing tests
// =========================================================================

func TestParseChmodSymbolic(t *testing.T) {
	tests := []struct {
		name     string
		spec     string
		current  os.FileMode
		expected os.FileMode
	}{
		{"user add execute", "u+x", 0644, 0744},
		{"group add read", "g+r", 0700, 0740},
		{"other add read", "o+r", 0700, 0704},
		{"all add execute", "a+x", 0644, 0755},
		{"user remove write", "u-w", 0755, 0555},
		{"set exact", "u=rwx", 0000, 0700},
		{"multiple clauses", "u+rwx,go+rx", 0000, 0755},
		{"default who is all", "+x", 0644, 0755},
		{"remove all", "a-rwx", 0777, 0000},
		{"set group exact", "g=rx", 0700, 0750},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			result, err := parseChmodSymbolic(tt.spec, tt.current)
			if err != nil {
				t.Fatalf("parseChmodSymbolic(%q, %04o) error: %v", tt.spec, tt.current, err)
			}
			if result.Perm() != tt.expected {
				t.Errorf("parseChmodSymbolic(%q, %04o) = %04o, want %04o",
					tt.spec, tt.current, result.Perm(), tt.expected)
			}
		})
	}
}

func TestParseChmodSymbolicInvalid(t *testing.T) {
	_, err := parseChmodSymbolic("u?x", 0644)
	if err == nil {
		t.Error("parseChmodSymbolic with invalid operator should return error")
	}
}

// =========================================================================
// applyChmodSymbolicOp — comprehensive tests
// =========================================================================

func TestApplyChmodSymbolicOpGroupWrite(t *testing.T) {
	// Test group write add.
	mode := applyChmodSymbolicOp(0700, []byte{'g'}, '+', []byte{'w'})
	if mode.Perm() != 0720 {
		t.Errorf("g+w on 0700 = %04o, want 0720", mode.Perm())
	}
}

func TestApplyChmodSymbolicOpOtherRead(t *testing.T) {
	mode := applyChmodSymbolicOp(0700, []byte{'o'}, '+', []byte{'r'})
	if mode.Perm() != 0704 {
		t.Errorf("o+r on 0700 = %04o, want 0704", mode.Perm())
	}
}

func TestApplyChmodSymbolicOpRemove(t *testing.T) {
	mode := applyChmodSymbolicOp(0777, []byte{'a'}, '-', []byte{'w'})
	if mode.Perm() != 0555 {
		t.Errorf("a-w on 0777 = %04o, want 0555", mode.Perm())
	}
}

func TestApplyChmodSymbolicOpEquals(t *testing.T) {
	mode := applyChmodSymbolicOp(0777, []byte{'u'}, '=', []byte{'r'})
	if mode.Perm() != 0477 {
		t.Errorf("u=r on 0777 = %04o, want 0477", mode.Perm())
	}
}

func TestApplyChmodSymbolicOpCapitalX(t *testing.T) {
	// X only sets execute if file already has execute somewhere.
	mode := applyChmodSymbolicOp(0644, []byte{'a'}, '+', []byte{'X'})
	if mode.Perm() != 0644 {
		t.Errorf("a+X on 0644 (no exec) = %04o, want 0644", mode.Perm())
	}

	mode = applyChmodSymbolicOp(0744, []byte{'a'}, '+', []byte{'X'})
	if mode.Perm() != 0755 {
		t.Errorf("a+X on 0744 (has exec) = %04o, want 0755", mode.Perm())
	}
}

func TestApplyChmodSymbolicOpStickyBit(t *testing.T) {
	mode := applyChmodSymbolicOp(0755, []byte{'a'}, '+', []byte{'t'})
	if mode&os.ModeSticky == 0 {
		t.Error("a+t should set sticky bit")
	}
}

func TestApplyChmodSymbolicOpSetuid(t *testing.T) {
	mode := applyChmodSymbolicOp(0755, []byte{'u'}, '+', []byte{'s'})
	if mode&os.ModeSetuid == 0 {
		t.Error("u+s should set setuid bit")
	}
}

func TestApplyChmodSymbolicOpSetgid(t *testing.T) {
	mode := applyChmodSymbolicOp(0755, []byte{'g'}, '+', []byte{'s'})
	if mode&os.ModeSetgid == 0 {
		t.Error("g+s should set setgid bit")
	}
}

func TestApplyChmodSymbolicOpAllRWX(t *testing.T) {
	mode := applyChmodSymbolicOp(0000, []byte{'a'}, '+', []byte{'r', 'w', 'x'})
	if mode.Perm() != 0777 {
		t.Errorf("a+rwx on 0000 = %04o, want 0777", mode.Perm())
	}
}

func TestApplyChmodSymbolicOpUserGroupOther(t *testing.T) {
	// Test each who class individually for read.
	tests := []struct {
		who      byte
		expected os.FileMode
	}{
		{'u', 0400},
		{'g', 0040},
		{'o', 0004},
	}
	for _, tt := range tests {
		mode := applyChmodSymbolicOp(0000, []byte{tt.who}, '+', []byte{'r'})
		if mode.Perm() != tt.expected {
			t.Errorf("%c+r on 0000 = %04o, want %04o", tt.who, mode.Perm(), tt.expected)
		}
	}

	// Test write.
	for _, tt := range []struct {
		who      byte
		expected os.FileMode
	}{
		{'u', 0200},
		{'g', 0020},
		{'o', 0002},
	} {
		mode := applyChmodSymbolicOp(0000, []byte{tt.who}, '+', []byte{'w'})
		if mode.Perm() != tt.expected {
			t.Errorf("%c+w on 0000 = %04o, want %04o", tt.who, mode.Perm(), tt.expected)
		}
	}

	// Test execute.
	for _, tt := range []struct {
		who      byte
		expected os.FileMode
	}{
		{'u', 0100},
		{'g', 0010},
		{'o', 0001},
	} {
		mode := applyChmodSymbolicOp(0000, []byte{tt.who}, '+', []byte{'x'})
		if mode.Perm() != tt.expected {
			t.Errorf("%c+x on 0000 = %04o, want %04o", tt.who, mode.Perm(), tt.expected)
		}
	}
}

func TestApplyChmodSymbolicOpEqualsClears(t *testing.T) {
	// = should clear bits for the affected class first.
	mode := applyChmodSymbolicOp(0777, []byte{'g'}, '=', []byte{'r'})
	if mode.Perm() != 0747 {
		t.Errorf("g=r on 0777 = %04o, want 0747", mode.Perm())
	}
	mode = applyChmodSymbolicOp(0777, []byte{'o'}, '=', []byte{})
	if mode.Perm() != 0770 {
		t.Errorf("o= on 0777 = %04o, want 0770", mode.Perm())
	}
}

func TestChmodSilentMode(t *testing.T) {
	var stdout, stderr bytes.Buffer
	rc := runChmod(toolSpecPath(t, "chmod"),
		[]string{"chmod", "-f", "755", "/nonexistent/file"},
		&stdout, &stderr)

	if rc != 1 {
		t.Errorf("exit code = %d, want 1", rc)
	}
	// With -f, stderr should be suppressed.
	if stderr.Len() != 0 {
		t.Errorf("silent mode should suppress errors, got %q", stderr.String())
	}
}

// =========================================================================
// chmodApplyMode tests
// =========================================================================

func TestChmodApplyModeOctal(t *testing.T) {
	mode, err := chmodApplyMode("755", 0644)
	if err != nil {
		t.Fatalf("chmodApplyMode error: %v", err)
	}
	if mode != 0755 {
		t.Errorf("mode = %04o, want 0755", mode)
	}
}

func TestChmodApplyModeSymbolic(t *testing.T) {
	mode, err := chmodApplyMode("u+x", 0644)
	if err != nil {
		t.Fatalf("chmodApplyMode error: %v", err)
	}
	if mode.Perm() != 0744 {
		t.Errorf("mode = %04o, want 0744", mode.Perm())
	}
}

// =========================================================================
// Integration tests — applying modes to actual files
// =========================================================================

func TestChmodOctalOnFile(t *testing.T) {
	if runtime.GOOS == "windows" {
		t.Skip("chmod not supported on Windows")
	}
	dir := t.TempDir()
	file := filepath.Join(dir, "test.txt")
	os.WriteFile(file, []byte("data"), 0644)

	var stdout, stderr bytes.Buffer
	rc := runChmod(toolSpecPath(t, "chmod"),
		[]string{"chmod", "755", file},
		&stdout, &stderr)

	if rc != 0 {
		t.Errorf("exit code = %d, want 0, stderr: %s", rc, stderr.String())
	}

	info, _ := os.Stat(file)
	if info.Mode().Perm() != 0755 {
		t.Errorf("file mode = %04o, want 0755", info.Mode().Perm())
	}
}

func TestChmodSymbolicOnFile(t *testing.T) {
	if runtime.GOOS == "windows" {
		t.Skip("chmod not supported on Windows")
	}
	dir := t.TempDir()
	file := filepath.Join(dir, "test.txt")
	os.WriteFile(file, []byte("data"), 0644)

	var stdout, stderr bytes.Buffer
	rc := runChmod(toolSpecPath(t, "chmod"),
		[]string{"chmod", "u+x", file},
		&stdout, &stderr)

	if rc != 0 {
		t.Errorf("exit code = %d, want 0, stderr: %s", rc, stderr.String())
	}

	info, _ := os.Stat(file)
	if info.Mode().Perm() != 0744 {
		t.Errorf("file mode = %04o, want 0744", info.Mode().Perm())
	}
}

func TestChmodRecursive(t *testing.T) {
	if runtime.GOOS == "windows" {
		t.Skip("chmod not supported on Windows")
	}
	dir := t.TempDir()
	subdir := filepath.Join(dir, "sub")
	os.MkdirAll(subdir, 0755)
	os.WriteFile(filepath.Join(subdir, "a.txt"), []byte("a"), 0644)
	os.WriteFile(filepath.Join(subdir, "b.txt"), []byte("b"), 0644)

	var stdout, stderr bytes.Buffer
	rc := runChmod(toolSpecPath(t, "chmod"),
		[]string{"chmod", "-R", "755", dir},
		&stdout, &stderr)

	if rc != 0 {
		t.Errorf("exit code = %d, want 0, stderr: %s", rc, stderr.String())
	}

	// Check that files in subdirectory got the new permissions.
	info, _ := os.Stat(filepath.Join(subdir, "a.txt"))
	if info.Mode().Perm() != 0755 {
		t.Errorf("sub/a.txt mode = %04o, want 0755", info.Mode().Perm())
	}
}

func TestChmodVerbose(t *testing.T) {
	dir := t.TempDir()
	file := filepath.Join(dir, "test.txt")
	os.WriteFile(file, []byte("data"), 0644)

	var stdout, stderr bytes.Buffer
	rc := runChmod(toolSpecPath(t, "chmod"),
		[]string{"chmod", "-v", "755", file},
		&stdout, &stderr)

	if rc != 0 {
		t.Errorf("exit code = %d, want 0", rc)
	}

	if !strings.Contains(stdout.String(), "mode of") {
		t.Errorf("verbose output should contain 'mode of', got %q", stdout.String())
	}
}

func TestChmodChangesOnly(t *testing.T) {
	if runtime.GOOS == "windows" {
		t.Skip("chmod not supported on Windows")
	}
	dir := t.TempDir()
	file := filepath.Join(dir, "test.txt")
	os.WriteFile(file, []byte("data"), 0755)

	var stdout, stderr bytes.Buffer
	// Setting same mode — should not report.
	rc := runChmod(toolSpecPath(t, "chmod"),
		[]string{"chmod", "-c", "755", file},
		&stdout, &stderr)

	if rc != 0 {
		t.Errorf("exit code = %d, want 0", rc)
	}

	if stdout.Len() != 0 {
		t.Errorf("changes-only should produce no output when mode unchanged, got %q", stdout.String())
	}
}

// =========================================================================
// Error handling
// =========================================================================

func TestChmodMissingFile(t *testing.T) {
	var stdout, stderr bytes.Buffer
	rc := runChmod(toolSpecPath(t, "chmod"),
		[]string{"chmod", "755", "/nonexistent/file"},
		&stdout, &stderr)

	if rc != 1 {
		t.Errorf("exit code = %d, want 1", rc)
	}
}

func TestChmodInvalidSpec(t *testing.T) {
	var stdout, stderr bytes.Buffer
	rc := runChmod("/nonexistent/chmod.json", []string{"chmod", "755", "file"}, &stdout, &stderr)
	if rc != 1 {
		t.Errorf("exit code = %d, want 1", rc)
	}
}

func TestChmodMultipleFiles(t *testing.T) {
	if runtime.GOOS == "windows" {
		t.Skip("chmod not supported on Windows")
	}
	dir := t.TempDir()
	file1 := filepath.Join(dir, "a.txt")
	file2 := filepath.Join(dir, "b.txt")
	os.WriteFile(file1, []byte("a"), 0644)
	os.WriteFile(file2, []byte("b"), 0644)

	var stdout, stderr bytes.Buffer
	rc := runChmod(toolSpecPath(t, "chmod"),
		[]string{"chmod", "700", file1, file2},
		&stdout, &stderr)

	if rc != 0 {
		t.Errorf("exit code = %d, want 0, stderr: %s", rc, stderr.String())
	}

	info1, _ := os.Stat(file1)
	info2, _ := os.Stat(file2)
	if info1.Mode().Perm() != 0700 {
		t.Errorf("file1 mode = %04o, want 0700", info1.Mode().Perm())
	}
	if info2.Mode().Perm() != 0700 {
		t.Errorf("file2 mode = %04o, want 0700", info2.Mode().Perm())
	}
}
