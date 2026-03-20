package analyzer

import (
	"strings"
	"testing"
)

// ── Helper functions ─────────────────────────────────────────────────
//
// These helpers make tests more readable by reducing boilerplate.

// mustAnalyze parses source and returns detected capabilities, failing the
// test if parsing fails.
func mustAnalyze(t *testing.T, source string) []DetectedCapability {
	t.Helper()
	caps, err := AnalyzeSource("test.go", source)
	if err != nil {
		t.Fatalf("failed to parse source: %v", err)
	}
	return caps
}

// assertCapabilityFound checks that at least one detected capability matches
// the given category and action.
func assertCapabilityFound(t *testing.T, caps []DetectedCapability, category, action string) {
	t.Helper()
	for _, cap := range caps {
		if cap.Category == category && cap.Action == action {
			return
		}
	}
	t.Errorf("expected capability %s:%s:* but not found in %v", category, action, caps)
}

// assertCapabilityWithTarget checks that at least one detected capability
// matches the given category, action, and target.
func assertCapabilityWithTarget(t *testing.T, caps []DetectedCapability, category, action, target string) {
	t.Helper()
	for _, cap := range caps {
		if cap.Category == category && cap.Action == action && cap.Target == target {
			return
		}
	}
	t.Errorf("expected capability %s:%s:%s but not found in %v", category, action, target, caps)
}

// assertNoCapabilities checks that no capabilities were detected.
func assertNoCapabilities(t *testing.T, caps []DetectedCapability) {
	t.Helper()
	if len(caps) != 0 {
		t.Errorf("expected no capabilities but found %d: %v", len(caps), caps)
	}
}

// ── Import detection tests ──────────────────────────────────────────
//
// These tests verify that importing specific packages triggers the correct
// capability detection. Each Go standard library package maps to a known
// capability category.

func TestImportOS(t *testing.T) {
	// The "os" package provides filesystem operations (Open, Create, Remove, etc.)
	// Importing it indicates potential filesystem access.
	caps := mustAnalyze(t, `package main
import "os"
func main() { _ = os.Args }
`)
	assertCapabilityFound(t, caps, "fs", "*")
}

func TestImportIO(t *testing.T) {
	// The "io" package provides Reader/Writer interfaces used for file I/O.
	caps := mustAnalyze(t, `package main
import "io"
var _ io.Reader
`)
	assertCapabilityFound(t, caps, "fs", "*")
}

func TestImportIOIoutil(t *testing.T) {
	// io/ioutil is the deprecated but still-common package for ReadFile, etc.
	caps := mustAnalyze(t, `package main
import "io/ioutil"
var _ = ioutil.Discard
`)
	assertCapabilityFound(t, caps, "fs", "*")
}

func TestImportNet(t *testing.T) {
	// The "net" package provides low-level networking (Dial, Listen, etc.)
	caps := mustAnalyze(t, `package main
import "net"
var _ net.Conn
`)
	assertCapabilityFound(t, caps, "net", "*")
}

func TestImportNetHTTP(t *testing.T) {
	// net/http provides HTTP client and server functionality.
	// We map it to net:connect because http.Get makes outbound connections.
	caps := mustAnalyze(t, `package main
import "net/http"
var _ = http.StatusOK
`)
	assertCapabilityFound(t, caps, "net", "connect")
}

func TestImportOSExec(t *testing.T) {
	// os/exec provides Command() for running external programs.
	caps := mustAnalyze(t, `package main
import "os/exec"
var _ = exec.ErrNotFound
`)
	assertCapabilityFound(t, caps, "proc", "exec")
}

func TestImportSyscall(t *testing.T) {
	// syscall provides raw system call access — very powerful.
	caps := mustAnalyze(t, `package main
import "syscall"
var _ = syscall.SIGINT
`)
	assertCapabilityFound(t, caps, "proc", "*")
}

func TestImportUnsafe(t *testing.T) {
	// unsafe breaks Go's type system — used for FFI and low-level hacks.
	caps := mustAnalyze(t, `package main
import "unsafe"
var _ = unsafe.Sizeof(0)
`)
	assertCapabilityFound(t, caps, "ffi", "*")
}

func TestImportPlugin(t *testing.T) {
	// plugin loads shared libraries at runtime.
	caps := mustAnalyze(t, `package main
import "plugin"
var _ plugin.Plugin
`)
	assertCapabilityFound(t, caps, "ffi", "load")
}

func TestImportReflect(t *testing.T) {
	// reflect enables dynamic dispatch, bypassing static analysis.
	caps := mustAnalyze(t, `package main
import "reflect"
var _ = reflect.TypeOf(0)
`)
	assertCapabilityFound(t, caps, "ffi", "*")
}

func TestImportCgo(t *testing.T) {
	// import "C" enables calling arbitrary C functions via cgo.
	// Note: This won't parse without actual cgo setup, so we test
	// the import detection logic directly.
	caps := mustAnalyze(t, `package main
import "C"
func main() {}
`)
	assertCapabilityFound(t, caps, "ffi", "*")
}

func TestMultipleImports(t *testing.T) {
	// A file can import multiple capability-bearing packages.
	caps := mustAnalyze(t, `package main
import (
	"os"
	"net/http"
	"os/exec"
)
func main() {}
`)
	assertCapabilityFound(t, caps, "fs", "*")
	assertCapabilityFound(t, caps, "net", "connect")
	assertCapabilityFound(t, caps, "proc", "exec")
	if len(caps) != 3 {
		t.Errorf("expected 3 capabilities, got %d", len(caps))
	}
}

func TestImportAlias(t *testing.T) {
	// Import aliases should still trigger detection. The import path
	// is what matters, not the local name.
	caps := mustAnalyze(t, `package main
import myos "os"
func main() { _ = myos.Args }
`)
	assertCapabilityFound(t, caps, "fs", "*")
}

func TestUnrelatedImport(t *testing.T) {
	// Importing packages not in our mapping should not trigger detection.
	caps := mustAnalyze(t, `package main
import "fmt"
func main() { fmt.Println("hello") }
`)
	assertNoCapabilities(t, caps)
}

func TestImportStrings(t *testing.T) {
	// "strings" is a pure utility package — no capability needed.
	caps := mustAnalyze(t, `package main
import "strings"
func main() { _ = strings.ToUpper("hi") }
`)
	assertNoCapabilities(t, caps)
}

// ── Function call detection tests ───────────────────────────────────
//
// These tests verify that specific function calls are detected with the
// correct category, action, and (where possible) target.

func TestOSOpen(t *testing.T) {
	// os.Open opens a file for reading. The first argument is the path.
	caps := mustAnalyze(t, `package main
import "os"
func main() { os.Open("config.txt") }
`)
	assertCapabilityWithTarget(t, caps, "fs", "read", "config.txt")
}

func TestOSReadFile(t *testing.T) {
	caps := mustAnalyze(t, `package main
import "os"
func main() { os.ReadFile("data.json") }
`)
	assertCapabilityWithTarget(t, caps, "fs", "read", "data.json")
}

func TestOSCreate(t *testing.T) {
	// os.Create creates a new file for writing.
	caps := mustAnalyze(t, `package main
import "os"
func main() { os.Create("output.txt") }
`)
	assertCapabilityWithTarget(t, caps, "fs", "write", "output.txt")
}

func TestOSWriteFile(t *testing.T) {
	caps := mustAnalyze(t, `package main
import "os"
func main() { os.WriteFile("out.txt", nil, 0644) }
`)
	assertCapabilityWithTarget(t, caps, "fs", "write", "out.txt")
}

func TestOSRemove(t *testing.T) {
	caps := mustAnalyze(t, `package main
import "os"
func main() { os.Remove("temp.txt") }
`)
	assertCapabilityWithTarget(t, caps, "fs", "delete", "temp.txt")
}

func TestOSRemoveAll(t *testing.T) {
	caps := mustAnalyze(t, `package main
import "os"
func main() { os.RemoveAll("tempdir") }
`)
	assertCapabilityWithTarget(t, caps, "fs", "delete", "tempdir")
}

func TestOSMkdir(t *testing.T) {
	caps := mustAnalyze(t, `package main
import "os"
func main() { os.Mkdir("newdir", 0755) }
`)
	assertCapabilityWithTarget(t, caps, "fs", "create", "newdir")
}

func TestOSMkdirAll(t *testing.T) {
	caps := mustAnalyze(t, `package main
import "os"
func main() { os.MkdirAll("a/b/c", 0755) }
`)
	assertCapabilityWithTarget(t, caps, "fs", "create", "a/b/c")
}

func TestOSReadDir(t *testing.T) {
	caps := mustAnalyze(t, `package main
import "os"
func main() { os.ReadDir(".") }
`)
	assertCapabilityWithTarget(t, caps, "fs", "list", ".")
}

func TestOSGetenv(t *testing.T) {
	// os.Getenv reads an environment variable. The first arg is the key.
	caps := mustAnalyze(t, `package main
import "os"
func main() { os.Getenv("HOME") }
`)
	assertCapabilityWithTarget(t, caps, "env", "read", "HOME")
}

func TestOSSetenv(t *testing.T) {
	caps := mustAnalyze(t, `package main
import "os"
func main() { os.Setenv("PATH", "/usr/bin") }
`)
	assertCapabilityWithTarget(t, caps, "env", "write", "PATH")
}

func TestExecCommand(t *testing.T) {
	// exec.Command runs an external program. First arg is the command.
	caps := mustAnalyze(t, `package main
import "os/exec"
func main() { exec.Command("ls") }
`)
	assertCapabilityWithTarget(t, caps, "proc", "exec", "ls")
}

func TestExecCommandVariable(t *testing.T) {
	// When the command is a variable, we can't determine it statically.
	// The target should be "*".
	caps := mustAnalyze(t, `package main
import "os/exec"
func main() {
	cmd := "ls"
	exec.Command(cmd)
}
`)
	assertCapabilityWithTarget(t, caps, "proc", "exec", "*")
}

func TestNetDial(t *testing.T) {
	caps := mustAnalyze(t, `package main
import "net"
func main() { net.Dial("tcp", "localhost:8080") }
`)
	assertCapabilityFound(t, caps, "net", "connect")
}

func TestHTTPGet(t *testing.T) {
	caps := mustAnalyze(t, `package main
import "net/http"
func main() { http.Get("https://example.com") }
`)
	assertCapabilityFound(t, caps, "net", "connect")
}

func TestHTTPPost(t *testing.T) {
	caps := mustAnalyze(t, `package main
import "net/http"
func main() { http.Post("https://example.com", "text/plain", nil) }
`)
	assertCapabilityFound(t, caps, "net", "connect")
}

func TestOSExit(t *testing.T) {
	caps := mustAnalyze(t, `package main
import "os"
func main() { os.Exit(1) }
`)
	assertCapabilityFound(t, caps, "proc", "signal")
}

func TestNetListen(t *testing.T) {
	caps := mustAnalyze(t, `package main
import "net"
func main() { net.Listen("tcp", ":8080") }
`)
	assertCapabilityFound(t, caps, "net", "listen")
}

// ── Pure code tests ─────────────────────────────────────────────────
//
// These tests verify that pure computation code (no imports or calls
// that indicate capability usage) produces zero detections.

func TestPureCode(t *testing.T) {
	caps := mustAnalyze(t, `package main

func add(a, b int) int { return a + b }
func main() { _ = add(1, 2) }
`)
	assertNoCapabilities(t, caps)
}

func TestPureCodeWithFmt(t *testing.T) {
	// fmt.Println writes to stdout, but fmt is not in our capability map
	// because stdout access is considered benign for most packages.
	caps := mustAnalyze(t, `package main
import "fmt"
func main() { fmt.Println("hello") }
`)
	assertNoCapabilities(t, caps)
}

func TestPureCodeWithMath(t *testing.T) {
	caps := mustAnalyze(t, `package main
import "math"
func main() { _ = math.Sqrt(4.0) }
`)
	assertNoCapabilities(t, caps)
}

func TestPureCodeWithSort(t *testing.T) {
	caps := mustAnalyze(t, `package main
import "sort"
func main() { sort.Ints([]int{3, 1, 2}) }
`)
	assertNoCapabilities(t, caps)
}

// ── Edge cases ──────────────────────────────────────────────────────

func TestImportAndCallBoth(t *testing.T) {
	// Importing "os" gives us fs:*:*, and os.Open gives us fs:read:config.txt.
	// Both should be detected.
	caps := mustAnalyze(t, `package main
import "os"
func main() { os.Open("config.txt") }
`)
	// Should have at least the import-level and call-level detections
	if len(caps) < 2 {
		t.Errorf("expected at least 2 capabilities (import + call), got %d", len(caps))
	}
	assertCapabilityFound(t, caps, "fs", "*")
	assertCapabilityWithTarget(t, caps, "fs", "read", "config.txt")
}

func TestBacktickStringLiteral(t *testing.T) {
	// Go raw string literals use backticks. We should extract them too.
	caps := mustAnalyze(t, "package main\nimport \"os\"\nfunc main() { os.Open(`raw_path.txt`) }\n")
	assertCapabilityWithTarget(t, caps, "fs", "read", "raw_path.txt")
}

func TestAliasedImportCall(t *testing.T) {
	// When a package is imported with an alias, calls should still be detected.
	caps := mustAnalyze(t, `package main
import myexec "os/exec"
func main() { myexec.Command("git") }
`)
	assertCapabilityWithTarget(t, caps, "proc", "exec", "git")
}

func TestEmptyFile(t *testing.T) {
	caps := mustAnalyze(t, `package main
`)
	assertNoCapabilities(t, caps)
}

func TestBlankImport(t *testing.T) {
	// Blank imports (_ "os") are still imports and should trigger detection.
	caps := mustAnalyze(t, `package main
import _ "os"
func main() {}
`)
	assertCapabilityFound(t, caps, "fs", "*")
}

func TestDetectedCapabilityString(t *testing.T) {
	cap := DetectedCapability{
		Category: "fs",
		Action:   "read",
		Target:   "file.txt",
		File:     "test.go",
		Line:     5,
		Evidence: "os.Open(\"file.txt\")",
	}
	expected := "fs:read:file.txt"
	if cap.String() != expected {
		t.Errorf("expected %q, got %q", expected, cap.String())
	}
}

func TestMultipleCalls(t *testing.T) {
	// Multiple different calls in the same file.
	caps := mustAnalyze(t, `package main
import (
	"os"
	"os/exec"
)
func main() {
	os.Open("input.txt")
	os.Create("output.txt")
	exec.Command("ls")
	os.Getenv("HOME")
}
`)
	assertCapabilityWithTarget(t, caps, "fs", "read", "input.txt")
	assertCapabilityWithTarget(t, caps, "fs", "write", "output.txt")
	assertCapabilityWithTarget(t, caps, "proc", "exec", "ls")
	assertCapabilityWithTarget(t, caps, "env", "read", "HOME")
}

func TestHTTPHead(t *testing.T) {
	caps := mustAnalyze(t, `package main
import "net/http"
func main() { http.Head("https://example.com") }
`)
	assertCapabilityFound(t, caps, "net", "connect")
}

// ── Evidence and line number tests ──────────────────────────────────

func TestEvidenceContainsCallInfo(t *testing.T) {
	caps := mustAnalyze(t, `package main
import "os"
func main() { os.Open("test.txt") }
`)
	for _, cap := range caps {
		if cap.Category == "fs" && cap.Action == "read" {
			if !strings.Contains(cap.Evidence, "os.Open") {
				t.Errorf("expected evidence to contain 'os.Open', got %q", cap.Evidence)
			}
			return
		}
	}
	t.Error("fs:read capability not found")
}

func TestLineNumbers(t *testing.T) {
	caps := mustAnalyze(t, `package main
import "os"
func main() {
	os.Open("first.txt")
	os.Create("second.txt")
}
`)
	for _, cap := range caps {
		if cap.Line <= 0 {
			t.Errorf("expected positive line number, got %d for %s", cap.Line, cap.String())
		}
	}
}

// ── AnalyzeSource error test ────────────────────────────────────────

func TestAnalyzeSourceParseError(t *testing.T) {
	// Invalid Go source should return an error, not panic.
	_, err := AnalyzeSource("bad.go", "this is not valid go code }{}{")
	if err == nil {
		t.Error("expected parse error for invalid Go source")
	}
}

// ── extractStringLiteral tests ──────────────────────────────────────

func TestExtractStringLiteralEdgeCases(t *testing.T) {
	// Test the extract function with non-string expressions
	result := extractStringLiteral(nil)
	if result != "*" {
		t.Errorf("expected * for nil, got %q", result)
	}
}
