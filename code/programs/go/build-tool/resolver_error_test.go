package main

import (
	"errors"
	"flag"
	"io"
	"os"
	"path/filepath"
	"strings"
	"testing"

	"github.com/adhithyan15/coding-adventures/code/programs/go/build-tool/internal/resolver"
)

func TestFormatResolverErrorUsesStableMetadataDiagnostic(t *testing.T) {
	message, exitCode := formatResolverError(&resolver.MetadataEncodingError{
		Code:     "METADATA_INVALID_UTF8",
		Package:  "lua/pkg",
		Manifest: "code/packages/lua/pkg/coding-adventures-pkg-0.1.0-1.rockspec",
		Encoding: "UTF-8",
	})

	if exitCode != 2 {
		t.Fatalf("metadata input error exit code = %d, want 2", exitCode)
	}
	want := "METADATA_INVALID_UTF8: package=lua/pkg manifest=code/packages/lua/pkg/coding-adventures-pkg-0.1.0-1.rockspec encoding=UTF-8"
	if message != want {
		t.Fatalf("metadata diagnostic = %q, want %q", message, want)
	}
	if strings.Contains(message, `C:\private-checkout`) {
		t.Fatalf("metadata diagnostic leaked a host path: %s", message)
	}
}

func TestFormatResolverErrorKeepsOperationalFailuresDistinct(t *testing.T) {
	message, exitCode := formatResolverError(errors.New("resolver failed"))
	if exitCode != 1 || message != "Error: resolver failed" {
		t.Fatalf("operational failure = (%q, %d), want (%q, 1)", message, exitCode, "Error: resolver failed")
	}
}

func TestRunFailsClosedOnInvalidRockspecUTF8(t *testing.T) {
	root := t.TempDir()
	packageDir := filepath.Join(root, "code", "packages", "lua", "pkg")
	if err := os.MkdirAll(packageDir, 0755); err != nil {
		t.Fatal(err)
	}
	if err := os.WriteFile(filepath.Join(packageDir, "BUILD"), []byte("echo building\n"), 0644); err != nil {
		t.Fatal(err)
	}
	rockspec := filepath.Join(packageDir, "coding-adventures-pkg-0.1.0-1.rockspec")
	invalid := append([]byte("package = \"coding-adventures-pkg\"\n-- invalid: "), 0x97)
	if err := os.WriteFile(rockspec, invalid, 0644); err != nil {
		t.Fatal(err)
	}

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
	os.Args = []string{
		"build-tool",
		"-root", root,
		"-force",
		"-dry-run",
		"-validate-build-files=false",
	}
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
	want := "METADATA_INVALID_UTF8: package=lua/pkg manifest=code/packages/lua/pkg/coding-adventures-pkg-0.1.0-1.rockspec encoding=UTF-8\n"
	if string(stderr) != want {
		t.Fatalf("stderr = %q, want %q", stderr, want)
	}
	if strings.Contains(string(stderr), root) {
		t.Fatalf("front door leaked the checkout path: %s", stderr)
	}
}
