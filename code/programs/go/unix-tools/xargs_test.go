// =========================================================================
// xargs — Tests
// =========================================================================
//
// These tests verify the xargs tool's behavior, covering:
//
//   1. Spec loading
//   2. Reading items from stdin (whitespace-delimited)
//   3. Null delimiter (-0)
//   4. Custom delimiter (-d)
//   5. Max args batching (-n)
//   6. Replace string mode (-I)
//   7. Verbose mode (-t)
//   8. No-run-if-empty (-r)
//   9. Default echo command
//  10. Item batching logic

package main

import (
	"bytes"
	"fmt"
	"io"
	"strings"
	"testing"

	clibuilder "github.com/adhithyan15/coding-adventures/code/packages/go/cli-builder"
)

// =========================================================================
// Spec loading test
// =========================================================================

func TestXargsSpecLoads(t *testing.T) {
	parser, err := clibuilder.NewParser(toolSpecPath(t, "xargs"), []string{"xargs"})
	if err != nil {
		t.Fatalf("failed to load xargs.json spec: %v", err)
	}
	if parser == nil {
		t.Fatal("NewParser returned nil parser without error")
	}
}

// =========================================================================
// xargsReadItems tests
// =========================================================================

func TestXargsReadItemsDefault(t *testing.T) {
	input := strings.NewReader("hello world\nfoo bar\n")
	items, err := xargsReadItems(input, XargsOptions{})
	if err != nil {
		t.Fatalf("xargsReadItems failed: %v", err)
	}
	expected := []string{"hello", "world", "foo", "bar"}
	if len(items) != len(expected) {
		t.Fatalf("got %d items, want %d", len(items), len(expected))
	}
	for i, item := range items {
		if item != expected[i] {
			t.Errorf("item[%d] = %q, want %q", i, item, expected[i])
		}
	}
}

func TestXargsReadItemsNullDelimiter(t *testing.T) {
	input := strings.NewReader("file one\x00file two\x00file three\x00")
	items, err := xargsReadItems(input, XargsOptions{NullDelimiter: true})
	if err != nil {
		t.Fatalf("xargsReadItems failed: %v", err)
	}
	expected := []string{"file one", "file two", "file three"}
	if len(items) != len(expected) {
		t.Fatalf("got %d items, want %d: %v", len(items), len(expected), items)
	}
	for i, item := range items {
		if item != expected[i] {
			t.Errorf("item[%d] = %q, want %q", i, item, expected[i])
		}
	}
}

func TestXargsReadItemsCustomDelimiter(t *testing.T) {
	input := strings.NewReader("apple,banana,cherry")
	items, err := xargsReadItems(input, XargsOptions{Delimiter: ","})
	if err != nil {
		t.Fatalf("xargsReadItems failed: %v", err)
	}
	expected := []string{"apple", "banana", "cherry"}
	if len(items) != len(expected) {
		t.Fatalf("got %d items, want %d", len(items), len(expected))
	}
	for i, item := range items {
		if item != expected[i] {
			t.Errorf("item[%d] = %q, want %q", i, item, expected[i])
		}
	}
}

func TestXargsReadItemsEmpty(t *testing.T) {
	input := strings.NewReader("")
	items, err := xargsReadItems(input, XargsOptions{})
	if err != nil {
		t.Fatalf("xargsReadItems failed: %v", err)
	}
	if len(items) != 0 {
		t.Errorf("got %d items from empty input, want 0", len(items))
	}
}

// =========================================================================
// xargsBatchItems tests
// =========================================================================

func TestXargsBatchItemsNoBatch(t *testing.T) {
	items := []string{"a", "b", "c", "d"}
	batches := xargsBatchItems(items, 0)
	if len(batches) != 1 {
		t.Fatalf("got %d batches, want 1", len(batches))
	}
	if len(batches[0]) != 4 {
		t.Errorf("batch[0] has %d items, want 4", len(batches[0]))
	}
}

func TestXargsBatchItemsByTwo(t *testing.T) {
	items := []string{"a", "b", "c", "d", "e"}
	batches := xargsBatchItems(items, 2)
	if len(batches) != 3 {
		t.Fatalf("got %d batches, want 3", len(batches))
	}
	if len(batches[0]) != 2 || batches[0][0] != "a" || batches[0][1] != "b" {
		t.Errorf("batch[0] = %v, want [a b]", batches[0])
	}
	if len(batches[2]) != 1 || batches[2][0] != "e" {
		t.Errorf("batch[2] = %v, want [e]", batches[2])
	}
}

func TestXargsBatchItemsEmpty(t *testing.T) {
	batches := xargsBatchItems([]string{}, 5)
	if len(batches) != 1 {
		t.Fatalf("got %d batches, want 1 (empty batch)", len(batches))
	}
}

// =========================================================================
// xargsExecute tests (with mock executor)
// =========================================================================

// mockXargsExec records the commands that would be executed.
type xargsExecRecord struct {
	name string
	args []string
}

func makeMockXargsExec(records *[]xargsExecRecord) xargsExecFunc {
	return func(name string, args []string, stdout, stderr io.Writer) int {
		argsCopy := make([]string, len(args))
		copy(argsCopy, args)
		*records = append(*records, xargsExecRecord{name, argsCopy})
		// Simulate echo: write args to stdout.
		fmt.Fprintln(stdout, strings.Join(args, " "))
		return 0
	}
}

func TestXargsExecuteDefaultEcho(t *testing.T) {
	var records []xargsExecRecord
	mockExec := makeMockXargsExec(&records)

	var stdout, stderr bytes.Buffer
	rc := xargsExecute(nil, []string{"hello", "world"}, XargsOptions{},
		&stdout, &stderr, mockExec)

	if rc != 0 {
		t.Errorf("exit code = %d, want 0", rc)
	}
	if len(records) != 1 {
		t.Fatalf("executed %d commands, want 1", len(records))
	}
	if records[0].name != "/bin/echo" {
		t.Errorf("command = %q, want /bin/echo", records[0].name)
	}
}

func TestXargsExecuteWithMaxArgs(t *testing.T) {
	var records []xargsExecRecord
	mockExec := makeMockXargsExec(&records)

	var stdout, stderr bytes.Buffer
	rc := xargsExecute([]string{"myecho"}, []string{"a", "b", "c"},
		XargsOptions{MaxArgs: 2}, &stdout, &stderr, mockExec)

	if rc != 0 {
		t.Errorf("exit code = %d, want 0", rc)
	}
	if len(records) != 2 {
		t.Fatalf("executed %d commands, want 2", len(records))
	}
}

func TestXargsExecuteReplaceStr(t *testing.T) {
	var records []xargsExecRecord
	mockExec := makeMockXargsExec(&records)

	var stdout, stderr bytes.Buffer
	rc := xargsExecute([]string{"cp", "{}", "/dest/{}"},
		[]string{"file1.txt", "file2.txt"},
		XargsOptions{ReplaceStr: "{}"},
		&stdout, &stderr, mockExec)

	if rc != 0 {
		t.Errorf("exit code = %d, want 0", rc)
	}
	if len(records) != 2 {
		t.Fatalf("executed %d commands, want 2", len(records))
	}
	// First invocation should replace {} with file1.txt.
	if records[0].args[0] != "file1.txt" || records[0].args[1] != "/dest/file1.txt" {
		t.Errorf("first invocation args = %v, want [file1.txt /dest/file1.txt]", records[0].args)
	}
}

func TestXargsExecuteVerbose(t *testing.T) {
	var records []xargsExecRecord
	mockExec := makeMockXargsExec(&records)

	var stdout, stderr bytes.Buffer
	xargsExecute([]string{"echo"}, []string{"hello"},
		XargsOptions{Verbose: true}, &stdout, &stderr, mockExec)

	if !strings.Contains(stderr.String(), "echo") {
		t.Errorf("verbose mode should print command to stderr, got %q", stderr.String())
	}
}

// =========================================================================
// Integration test via runXargsWithStdin
// =========================================================================

func TestXargsNoRunIfEmpty(t *testing.T) {
	var records []xargsExecRecord
	mockExec := makeMockXargsExec(&records)

	var stdout, stderr bytes.Buffer
	rc := runXargsWithStdin(toolSpecPath(t, "xargs"),
		[]string{"xargs", "-r", "echo"},
		&stdout, &stderr,
		strings.NewReader(""),
		mockExec)

	if rc != 0 {
		t.Errorf("exit code = %d, want 0", rc)
	}
	if len(records) != 0 {
		t.Errorf("should not have executed any commands, executed %d", len(records))
	}
}

func TestXargsIntegrationBasic(t *testing.T) {
	var records []xargsExecRecord
	mockExec := makeMockXargsExec(&records)

	var stdout, stderr bytes.Buffer
	rc := runXargsWithStdin(toolSpecPath(t, "xargs"),
		[]string{"xargs", "echo"},
		&stdout, &stderr,
		strings.NewReader("hello world"),
		mockExec)

	if rc != 0 {
		t.Errorf("exit code = %d, want 0", rc)
	}
	if len(records) != 1 {
		t.Fatalf("executed %d commands, want 1", len(records))
	}
	if records[0].name != "echo" {
		t.Errorf("command = %q, want echo", records[0].name)
	}
}

func TestXargsParallelExecution(t *testing.T) {
	var records []xargsExecRecord
	mockExec := makeMockXargsExec(&records)

	var stdout, stderr bytes.Buffer
	rc := xargsExecute([]string{"echo"}, []string{"a", "b", "c", "d"},
		XargsOptions{MaxArgs: 2, MaxProcs: 2}, &stdout, &stderr, mockExec)

	if rc != 0 {
		t.Errorf("exit code = %d, want 0", rc)
	}
	if len(records) != 2 {
		t.Errorf("executed %d commands, want 2", len(records))
	}
}

func TestXargsExecuteWithInitialArgs(t *testing.T) {
	var records []xargsExecRecord
	mockExec := makeMockXargsExec(&records)

	var stdout, stderr bytes.Buffer
	rc := xargsExecute([]string{"grep", "-l", "pattern"}, []string{"file1", "file2"},
		XargsOptions{}, &stdout, &stderr, mockExec)

	if rc != 0 {
		t.Errorf("exit code = %d, want 0", rc)
	}
	if len(records) != 1 {
		t.Fatalf("executed %d commands, want 1", len(records))
	}
	// Args should be: -l pattern file1 file2
	if len(records[0].args) != 4 {
		t.Errorf("args = %v, want [-l pattern file1 file2]", records[0].args)
	}
}

func TestXargsInvalidSpec(t *testing.T) {
	var stdout, stderr bytes.Buffer
	rc := runXargs("/nonexistent/xargs.json", []string{"xargs"}, &stdout, &stderr)
	if rc != 1 {
		t.Errorf("exit code = %d, want 1", rc)
	}
}

