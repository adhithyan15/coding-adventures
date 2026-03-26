// =========================================================================
// env — Tests
// =========================================================================
//
// These tests verify the env tool's behavior, covering:
//
//   1. Spec loading
//   2. Print current environment (no args)
//   3. Set variables (NAME=VALUE)
//   4. Empty environment (-i)
//   5. Unset variables (-u)
//   6. Null-terminated output (-0)
//   7. Running a command with modified environment
//   8. Change directory (-C)
//   9. Environment building logic
//  10. Error handling

package main

import (
	"bytes"
	"io"
	"os"
	"strings"
	"testing"

	clibuilder "github.com/adhithyan15/coding-adventures/code/packages/go/cli-builder"
)

// =========================================================================
// Spec loading test
// =========================================================================

func TestEnvSpecLoads(t *testing.T) {
	parser, err := clibuilder.NewParser(toolSpecPath(t, "env"), []string{"env"})
	if err != nil {
		t.Fatalf("failed to load env.json spec: %v", err)
	}
	if parser == nil {
		t.Fatal("NewParser returned nil parser without error")
	}
}

// =========================================================================
// envBuildEnviron tests
// =========================================================================

func TestEnvBuildEnvironEmpty(t *testing.T) {
	env := envBuildEnviron(nil, EnvOptions{IgnoreEnvironment: true})
	if len(env) != 0 {
		t.Errorf("with -i and no assignments, env should be empty, got %d entries", len(env))
	}
}

func TestEnvBuildEnvironWithAssignments(t *testing.T) {
	env := envBuildEnviron(
		[]string{"FOO=bar", "BAZ=qux"},
		EnvOptions{IgnoreEnvironment: true},
	)
	if len(env) != 2 {
		t.Fatalf("expected 2 entries, got %d: %v", len(env), env)
	}
	if env[0] != "FOO=bar" {
		t.Errorf("env[0] = %q, want FOO=bar", env[0])
	}
	if env[1] != "BAZ=qux" {
		t.Errorf("env[1] = %q, want BAZ=qux", env[1])
	}
}

func TestEnvBuildEnvironUnset(t *testing.T) {
	// Set a known variable first.
	os.Setenv("ENV_TEST_UNSET_VAR", "should_be_removed")
	defer os.Unsetenv("ENV_TEST_UNSET_VAR")

	env := envBuildEnviron(nil, EnvOptions{
		IgnoreEnvironment: false,
		Unset:             []string{"ENV_TEST_UNSET_VAR"},
	})

	for _, e := range env {
		if strings.HasPrefix(e, "ENV_TEST_UNSET_VAR=") {
			t.Error("ENV_TEST_UNSET_VAR should have been removed from environment")
		}
	}
}

func TestEnvBuildEnvironOverwrite(t *testing.T) {
	// When assigning a variable that already exists, it should be overwritten.
	os.Setenv("ENV_TEST_OVERWRITE", "old_value")
	defer os.Unsetenv("ENV_TEST_OVERWRITE")

	env := envBuildEnviron(
		[]string{"ENV_TEST_OVERWRITE=new_value"},
		EnvOptions{IgnoreEnvironment: false},
	)

	found := false
	for _, e := range env {
		if e == "ENV_TEST_OVERWRITE=new_value" {
			found = true
		}
		if e == "ENV_TEST_OVERWRITE=old_value" {
			t.Error("old value should have been replaced")
		}
	}
	if !found {
		t.Error("new value should be present in environment")
	}
}

// =========================================================================
// Mock executor for env tests
// =========================================================================

type envExecRecord struct {
	env  []string
	dir  string
	name string
	args []string
}

func makeMockEnvExec(records *[]envExecRecord) envExecFunc {
	return func(env []string, dir string, name string, args []string,
		stdout, stderr io.Writer) int {
		envCopy := make([]string, len(env))
		copy(envCopy, env)
		argsCopy := make([]string, len(args))
		copy(argsCopy, args)
		*records = append(*records, envExecRecord{envCopy, dir, name, argsCopy})
		return 0
	}
}

// =========================================================================
// Integration tests via runEnvWithExec
// =========================================================================

func TestEnvPrintEnvironment(t *testing.T) {
	os.Setenv("ENV_TEST_PRINT", "visible")
	defer os.Unsetenv("ENV_TEST_PRINT")

	var stdout, stderr bytes.Buffer
	rc := runEnvWithExec(toolSpecPath(t, "env"),
		[]string{"env"},
		&stdout, &stderr,
		func(env []string, dir string, name string, args []string,
			stdout, stderr io.Writer) int {
			return 0
		})

	if rc != 0 {
		t.Errorf("exit code = %d, want 0", rc)
	}

	output := stdout.String()
	if !strings.Contains(output, "ENV_TEST_PRINT=visible") {
		t.Errorf("output should contain ENV_TEST_PRINT=visible, got:\n%s", output)
	}
}

func TestEnvEmptyEnvironment(t *testing.T) {
	var stdout, stderr bytes.Buffer
	rc := runEnvWithExec(toolSpecPath(t, "env"),
		[]string{"env", "-i"},
		&stdout, &stderr,
		func(env []string, dir string, name string, args []string,
			stdout, stderr io.Writer) int {
			return 0
		})

	if rc != 0 {
		t.Errorf("exit code = %d, want 0", rc)
	}

	// With -i and no assignments or command, output should be empty.
	if stdout.Len() != 0 {
		t.Errorf("with -i and no assignments, output should be empty, got %q", stdout.String())
	}
}

func TestEnvRunCommandWithModifiedEnv(t *testing.T) {
	var records []envExecRecord
	mockExec := makeMockEnvExec(&records)

	var stdout, stderr bytes.Buffer
	rc := runEnvWithExec(toolSpecPath(t, "env"),
		[]string{"env", "-i", "FOO=bar", "mycommand", "arg1"},
		&stdout, &stderr, mockExec)

	if rc != 0 {
		t.Errorf("exit code = %d, want 0", rc)
	}
	if len(records) != 1 {
		t.Fatalf("executed %d commands, want 1", len(records))
	}
	if records[0].name != "mycommand" {
		t.Errorf("command = %q, want mycommand", records[0].name)
	}
	// Check the environment contains FOO=bar.
	found := false
	for _, e := range records[0].env {
		if e == "FOO=bar" {
			found = true
		}
	}
	if !found {
		t.Error("environment should contain FOO=bar")
	}
}

func TestEnvNullTerminated(t *testing.T) {
	var stdout, stderr bytes.Buffer
	rc := runEnvWithExec(toolSpecPath(t, "env"),
		[]string{"env", "-i", "-0", "A=1", "B=2"},
		&stdout, &stderr,
		func(env []string, dir string, name string, args []string,
			stdout, stderr io.Writer) int {
			return 0
		})

	if rc != 0 {
		t.Errorf("exit code = %d, want 0", rc)
	}

	output := stdout.String()
	if !strings.Contains(output, "\x00") {
		t.Errorf("null-terminated output should contain NUL bytes, got %q", output)
	}
	if strings.Contains(output, "\n") {
		t.Errorf("null-terminated output should not contain newlines, got %q", output)
	}
}

func TestEnvInvalidSpec(t *testing.T) {
	var stdout, stderr bytes.Buffer
	rc := runEnv("/nonexistent/env.json", []string{"env"}, &stdout, &stderr)
	if rc != 1 {
		t.Errorf("exit code = %d, want 1", rc)
	}
}
