package main

import (
	"encoding/json"
	"errors"
	"flag"
	"io"
	"os"
	"path/filepath"
	"runtime"
	"slices"
	"strings"
	"testing"

	"github.com/adhithyan15/coding-adventures/code/programs/go/build-tool/internal/discovery"
)

type duplicateIdentityFixture struct {
	Workspace struct {
		Files []struct {
			Path        string `json:"path"`
			ContentUTF8 string `json:"content_utf8"`
		} `json:"files"`
	} `json:"workspace"`
	Expected struct {
		Diagnostics []struct {
			Code    string `json:"code"`
			Package string `json:"package"`
			Details struct {
				Paths []string `json:"paths"`
			} `json:"details"`
		} `json:"diagnostics"`
	} `json:"expected"`
}

func materializeDuplicateIdentityFixture(t *testing.T) (string, duplicateIdentityFixture) {
	t.Helper()
	_, sourceFile, _, ok := runtime.Caller(0)
	if !ok {
		t.Fatal("could not locate main-package test source")
	}
	repoRoot := filepath.Clean(filepath.Join(filepath.Dir(sourceFile), "..", "..", "..", ".."))
	data, err := os.ReadFile(filepath.Join(repoRoot, "code", "specs", "fixtures", "build-tool-v1", "cases", "discovery-duplicate-identity.json"))
	if err != nil {
		t.Fatal(err)
	}
	var fixture duplicateIdentityFixture
	if err := json.Unmarshal(data, &fixture); err != nil {
		t.Fatal(err)
	}
	root := t.TempDir()
	for _, file := range fixture.Workspace.Files {
		path := filepath.Join(root, filepath.FromSlash(file.Path))
		if err := os.MkdirAll(filepath.Dir(path), 0o755); err != nil {
			t.Fatal(err)
		}
		if err := os.WriteFile(path, []byte(file.ContentUTF8), 0o644); err != nil {
			t.Fatal(err)
		}
	}
	return root, fixture
}

func TestFormatDiscoveryErrorUsesStableDuplicateDiagnostic(t *testing.T) {
	err := &discovery.DuplicatePackageIdentityError{
		Code:    "DUPLICATE_PACKAGE_IDENTITY",
		Package: "unknown/demo",
		Paths:   []string{"code/packages/alpha/demo", "code/packages/beta/demo"},
	}
	message, exitCode := formatDiscoveryError(err)
	if exitCode != 2 || message != err.Error() {
		t.Fatalf("duplicate diagnostic = (%q, %d), want (%q, 2)", message, exitCode, err.Error())
	}
}

func TestFormatDiscoveryErrorKeepsOperationalFailuresDistinct(t *testing.T) {
	message, exitCode := formatDiscoveryError(errors.New("discovery failed"))
	if exitCode != 1 || message != "Error: discovery failed" {
		t.Fatalf("operational failure = (%q, %d), want (%q, 1)", message, exitCode, "Error: discovery failed")
	}
}

func TestRunFailsClosedOnDuplicatePackageIdentity(t *testing.T) {
	root, fixture := materializeDuplicateIdentityFixture(t)
	diagnostic := fixture.Expected.Diagnostics[0]

	originalArgs := os.Args
	originalFlags := flag.CommandLine
	originalStderr := os.Stderr
	reader, writer, err := os.Pipe()
	if err != nil {
		t.Fatal(err)
	}
	t.Cleanup(func() {
		os.Args = originalArgs
		flag.CommandLine = originalFlags
		os.Stderr = originalStderr
		reader.Close()
		writer.Close()
	})
	flag.CommandLine = flag.NewFlagSet("build-tool-test", flag.ContinueOnError)
	os.Args = []string{"build-tool", "-root", root, "-force", "-dry-run", "-validate-build-files=false"}
	os.Stderr = writer

	exitCode := run()
	if err := writer.Close(); err != nil {
		t.Fatal(err)
	}
	os.Stderr = originalStderr
	stderr, err := io.ReadAll(reader)
	if err != nil {
		t.Fatal(err)
	}

	if exitCode != 2 {
		t.Fatalf("run exit code = %d, want 2; stderr=%s", exitCode, stderr)
	}
	wantPaths := diagnostic.Details.Paths
	want := diagnostic.Code + ": package=" + diagnostic.Package + " paths=" + strings.Join(wantPaths, ",") + "\n"
	if string(stderr) != want {
		t.Fatalf("stderr = %q, want %q", stderr, want)
	}
	if strings.Contains(string(stderr), root) {
		t.Fatalf("front door leaked workspace root: %s", stderr)
	}
	if !slices.IsSorted(wantPaths) {
		t.Fatalf("fixture paths must be sorted: %v", wantPaths)
	}
}
