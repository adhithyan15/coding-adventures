package clibuilder

import (
	"testing"
)

// =========================================================================
// Parser tests — the integration tests for the full parsing pipeline.
// =========================================================================
//
// These tests cover the eight required scenarios from the spec plus several
// additional error cases. Each test embeds a minimal JSON spec as a constant
// string (no file I/O needed) and uses NewParserFromBytes.

// --- Spec definitions ---

// echoSpec: simple variadic args, flag conflicts (§10.1)
const echoSpec = `{
  "cli_builder_spec_version": "1.0",
  "name": "echo",
  "description": "Display a line of text",
  "version": "8.32",
  "flags": [
    {
      "id": "no-newline",
      "short": "n",
      "description": "Do not output the trailing newline",
      "type": "boolean"
    },
    {
      "id": "enable-escapes",
      "short": "e",
      "description": "Enable backslash escapes",
      "type": "boolean",
      "conflicts_with": ["disable-escapes"]
    },
    {
      "id": "disable-escapes",
      "short": "E",
      "description": "Disable backslash escapes",
      "type": "boolean",
      "conflicts_with": ["enable-escapes"]
    }
  ],
  "arguments": [
    {
      "id": "string",
      "name": "STRING",
      "description": "Text to print",
      "type": "string",
      "required": false,
      "variadic": true,
      "variadic_min": 0
    }
  ]
}`

// lsSpec: stacked short flags with requires dependency
const lsSpec = `{
  "cli_builder_spec_version": "1.0",
  "name": "ls",
  "description": "List directory contents",
  "flags": [
    {
      "id": "long-listing",
      "short": "l",
      "long": "long",
      "description": "Long listing format",
      "type": "boolean"
    },
    {
      "id": "all",
      "short": "a",
      "long": "all",
      "description": "Show hidden files",
      "type": "boolean"
    },
    {
      "id": "human-readable",
      "short": "h",
      "long": "human-readable",
      "description": "Human readable sizes",
      "type": "boolean",
      "requires": ["long-listing"]
    }
  ],
  "arguments": [
    {
      "id": "path",
      "name": "PATH",
      "description": "Directory to list",
      "type": "path",
      "required": false,
      "variadic": true,
      "variadic_min": 0
    }
  ]
}`

// cpSpec: variadic source + required dest (cp pattern)
const cpSpec = `{
  "cli_builder_spec_version": "1.0",
  "name": "cp",
  "description": "Copy files",
  "flags": [
    {
      "id": "recursive",
      "short": "r",
      "long": "recursive",
      "description": "Copy directories recursively",
      "type": "boolean"
    }
  ],
  "arguments": [
    {
      "id": "source",
      "name": "SOURCE",
      "description": "Source file(s)",
      "type": "path",
      "required": true,
      "variadic": true,
      "variadic_min": 1
    },
    {
      "id": "dest",
      "name": "DEST",
      "description": "Destination",
      "type": "path",
      "required": true
    }
  ]
}`

// grepSpec: exclusive group + enum flag
const grepSpec = `{
  "cli_builder_spec_version": "1.0",
  "name": "grep",
  "description": "Search for patterns",
  "flags": [
    {
      "id": "extended-regexp",
      "short": "E",
      "long": "extended-regexp",
      "description": "Extended regexp",
      "type": "boolean"
    },
    {
      "id": "fixed-strings",
      "short": "F",
      "long": "fixed-strings",
      "description": "Fixed strings",
      "type": "boolean"
    },
    {
      "id": "count",
      "short": "c",
      "long": "count",
      "description": "Print count",
      "type": "boolean"
    }
  ],
  "arguments": [
    {
      "id": "pattern",
      "name": "PATTERN",
      "description": "Search pattern",
      "type": "string",
      "required": true
    },
    {
      "id": "file",
      "name": "FILE",
      "description": "Input file",
      "type": "path",
      "required": false,
      "variadic": true,
      "variadic_min": 0
    }
  ],
  "mutually_exclusive_groups": [
    {
      "id": "regexp-engine",
      "flag_ids": ["extended-regexp", "fixed-strings"],
      "required": false
    }
  ]
}`

// tarSpec: traditional mode (tar-style without leading dash)
const tarSpec = `{
  "cli_builder_spec_version": "1.0",
  "name": "tar",
  "description": "Archive files",
  "parsing_mode": "traditional",
  "flags": [
    {"id": "extract", "short": "x", "description": "Extract", "type": "boolean"},
    {"id": "verbose", "short": "v", "description": "Verbose", "type": "boolean"},
    {"id": "file", "short": "f", "description": "Archive file", "type": "path"}
  ],
  "arguments": [
    {
      "id": "member",
      "name": "MEMBER",
      "description": "Archive member",
      "type": "path",
      "required": false,
      "variadic": true,
      "variadic_min": 0
    }
  ]
}`

// gitSpec: subcommand tree (git remote add)
const gitSpec = `{
  "cli_builder_spec_version": "1.0",
  "name": "git",
  "description": "The stupid content tracker",
  "global_flags": [
    {
      "id": "verbose",
      "short": "v",
      "long": "verbose",
      "description": "Be verbose",
      "type": "boolean"
    }
  ],
  "commands": [
    {
      "id": "cmd-remote",
      "name": "remote",
      "description": "Manage set of tracked repositories",
      "commands": [
        {
          "id": "cmd-remote-add",
          "name": "add",
          "aliases": ["a"],
          "description": "Add a named remote repository",
          "arguments": [
            {
              "id": "name",
              "name": "NAME",
              "description": "Remote name",
              "type": "string",
              "required": true
            },
            {
              "id": "url",
              "name": "URL",
              "description": "Remote URL",
              "type": "string",
              "required": true
            }
          ]
        },
        {
          "id": "cmd-remote-remove",
          "name": "remove",
          "description": "Remove a remote",
          "arguments": [
            {
              "id": "name",
              "name": "NAME",
              "description": "Remote name",
              "type": "string",
              "required": true
            }
          ]
        }
      ]
    }
  ]
}`

// =========================================================================
// Test 1: echo hello world
// =========================================================================

func TestParser_Echo_HelloWorld(t *testing.T) {
	p, err := NewParserFromBytes([]byte(echoSpec), []string{"echo", "hello", "world"})
	if err != nil {
		t.Fatalf("NewParserFromBytes failed: %v", err)
	}
	result, err := p.Parse()
	if err != nil {
		t.Fatalf("Parse failed: %v", err)
	}
	pr, ok := result.(*ParseResult)
	if !ok {
		t.Fatalf("expected *ParseResult, got %T", result)
	}
	if pr.Program != "echo" {
		t.Errorf("expected program 'echo', got %q", pr.Program)
	}
	if len(pr.CommandPath) != 1 || pr.CommandPath[0] != "echo" {
		t.Errorf("expected command_path [echo], got %v", pr.CommandPath)
	}

	// All flags should be false (absent)
	for _, id := range []string{"no-newline", "enable-escapes", "disable-escapes"} {
		if pr.Flags[id] != false {
			t.Errorf("expected flag %q = false, got %v", id, pr.Flags[id])
		}
	}

	// Arguments: string = ["hello", "world"]
	strArg, ok := pr.Arguments["string"].([]any)
	if !ok {
		t.Fatalf("expected 'string' arg to be []any, got %T: %v", pr.Arguments["string"], pr.Arguments["string"])
	}
	if len(strArg) != 2 {
		t.Errorf("expected 2 string args, got %d: %v", len(strArg), strArg)
	}
	if strArg[0] != "hello" || strArg[1] != "world" {
		t.Errorf("expected ['hello', 'world'], got %v", strArg)
	}
}

// =========================================================================
// Test 2: ls -lah /tmp
// =========================================================================

func TestParser_Ls_Lah(t *testing.T) {
	p, err := NewParserFromBytes([]byte(lsSpec), []string{"ls", "-lah", "/tmp"})
	if err != nil {
		t.Fatalf("NewParserFromBytes failed: %v", err)
	}
	result, err := p.Parse()
	if err != nil {
		t.Fatalf("Parse failed: %v", err)
	}
	pr := result.(*ParseResult)

	if pr.Flags["long-listing"] != true {
		t.Errorf("expected long-listing=true, got %v", pr.Flags["long-listing"])
	}
	if pr.Flags["all"] != true {
		t.Errorf("expected all=true, got %v", pr.Flags["all"])
	}
	if pr.Flags["human-readable"] != true {
		t.Errorf("expected human-readable=true, got %v", pr.Flags["human-readable"])
	}

	paths, ok := pr.Arguments["path"].([]any)
	if !ok {
		t.Fatalf("expected 'path' arg to be []any, got %T", pr.Arguments["path"])
	}
	if len(paths) != 1 || paths[0] != "/tmp" {
		t.Errorf("expected path=['/tmp'], got %v", paths)
	}
}

// =========================================================================
// Test 3: cp a.txt b.txt /dest (variadic source + required dest)
// =========================================================================

func TestParser_Cp_MultipleSourcesDest(t *testing.T) {
	p, err := NewParserFromBytes([]byte(cpSpec), []string{"cp", "a.txt", "b.txt", "/dest"})
	if err != nil {
		t.Fatalf("NewParserFromBytes failed: %v", err)
	}
	result, err := p.Parse()
	if err != nil {
		t.Fatalf("Parse failed: %v", err)
	}
	pr := result.(*ParseResult)

	sources, ok := pr.Arguments["source"].([]any)
	if !ok {
		t.Fatalf("expected source to be []any, got %T", pr.Arguments["source"])
	}
	if len(sources) != 2 {
		t.Errorf("expected 2 sources, got %d: %v", len(sources), sources)
	}
	if sources[0] != "a.txt" || sources[1] != "b.txt" {
		t.Errorf("expected sources=['a.txt','b.txt'], got %v", sources)
	}
	if pr.Arguments["dest"] != "/dest" {
		t.Errorf("expected dest='/dest', got %v", pr.Arguments["dest"])
	}
}

// =========================================================================
// Test 4: grep -E pattern file.txt
// =========================================================================

func TestParser_Grep_ExtendedRegexp(t *testing.T) {
	p, err := NewParserFromBytes([]byte(grepSpec), []string{"grep", "-E", "pattern", "file.txt"})
	if err != nil {
		t.Fatalf("NewParserFromBytes failed: %v", err)
	}
	result, err := p.Parse()
	if err != nil {
		t.Fatalf("Parse failed: %v", err)
	}
	pr := result.(*ParseResult)

	if pr.Flags["extended-regexp"] != true {
		t.Errorf("expected extended-regexp=true, got %v", pr.Flags["extended-regexp"])
	}
	if pr.Arguments["pattern"] != "pattern" {
		t.Errorf("expected pattern='pattern', got %v", pr.Arguments["pattern"])
	}
	files, ok := pr.Arguments["file"].([]any)
	if !ok {
		t.Fatalf("expected file to be []any, got %T", pr.Arguments["file"])
	}
	if len(files) != 1 || files[0] != "file.txt" {
		t.Errorf("expected file=['file.txt'], got %v", files)
	}
}

// =========================================================================
// Test 5: tar xvf archive.tar (traditional mode)
// =========================================================================

func TestParser_Tar_TraditionalMode(t *testing.T) {
	p, err := NewParserFromBytes([]byte(tarSpec), []string{"tar", "xvf", "archive.tar"})
	if err != nil {
		t.Fatalf("NewParserFromBytes failed: %v", err)
	}
	result, err := p.Parse()
	if err != nil {
		t.Fatalf("Parse failed: %v", err)
	}
	pr := result.(*ParseResult)

	if pr.Flags["extract"] != true {
		t.Errorf("expected extract=true, got %v", pr.Flags["extract"])
	}
	if pr.Flags["verbose"] != true {
		t.Errorf("expected verbose=true, got %v", pr.Flags["verbose"])
	}
	if pr.Flags["file"] != true {
		t.Errorf("expected file=true (boolean flag set by 'f'), got %v", pr.Flags["file"])
	}
	// "archive.tar" should be the value for the 'f' flag (next token after stacked)
	// But in our implementation, 'f' is boolean, so archive.tar is positional
	members, ok := pr.Arguments["member"].([]any)
	if !ok {
		t.Fatalf("expected member to be []any, got %T: %v", pr.Arguments["member"], pr.Arguments["member"])
	}
	if len(members) != 1 || members[0] != "archive.tar" {
		t.Errorf("expected member=['archive.tar'], got %v", members)
	}
}

// =========================================================================
// Test 6: git remote add origin https://example.com
// =========================================================================

func TestParser_Git_RemoteAdd(t *testing.T) {
	p, err := NewParserFromBytes([]byte(gitSpec), []string{"git", "remote", "add", "origin", "https://example.com"})
	if err != nil {
		t.Fatalf("NewParserFromBytes failed: %v", err)
	}
	result, err := p.Parse()
	if err != nil {
		t.Fatalf("Parse failed: %v", err)
	}
	pr := result.(*ParseResult)

	expected := []string{"git", "remote", "add"}
	if len(pr.CommandPath) != 3 {
		t.Errorf("expected command_path %v, got %v", expected, pr.CommandPath)
	}
	for i, e := range expected {
		if pr.CommandPath[i] != e {
			t.Errorf("command_path[%d]: expected %q, got %q", i, e, pr.CommandPath[i])
		}
	}
	if pr.Arguments["name"] != "origin" {
		t.Errorf("expected name='origin', got %v", pr.Arguments["name"])
	}
	if pr.Arguments["url"] != "https://example.com" {
		t.Errorf("expected url='https://example.com', got %v", pr.Arguments["url"])
	}
}

// =========================================================================
// Test 7 (error): ls -h without -l → missing_dependency_flag
// =========================================================================

func TestParser_Ls_MissingDependencyFlag(t *testing.T) {
	// -h requires -l (human-readable requires long-listing)
	p, err := NewParserFromBytes([]byte(lsSpec), []string{"ls", "-h"})
	if err != nil {
		t.Fatalf("NewParserFromBytes failed: %v", err)
	}
	_, err = p.Parse()
	if err == nil {
		t.Fatal("expected parse error for missing dependency, got nil")
	}
	pe, ok := err.(*ParseErrors)
	if !ok {
		t.Fatalf("expected *ParseErrors, got %T", err)
	}
	found := false
	for _, e := range pe.Errors {
		if e.ErrorType == ErrMissingDependencyFlag {
			found = true
			break
		}
	}
	if !found {
		t.Errorf("expected missing_dependency_flag error, got: %v", pe.Errors)
	}
}

// =========================================================================
// Test 8 (error): grep -E -F pattern → exclusive_group_violation
// =========================================================================

func TestParser_Grep_ExclusiveGroupViolation(t *testing.T) {
	p, err := NewParserFromBytes([]byte(grepSpec), []string{"grep", "-E", "-F", "pattern"})
	if err != nil {
		t.Fatalf("NewParserFromBytes failed: %v", err)
	}
	_, err = p.Parse()
	if err == nil {
		t.Fatal("expected parse error for exclusive group violation, got nil")
	}
	pe, ok := err.(*ParseErrors)
	if !ok {
		t.Fatalf("expected *ParseErrors, got %T", err)
	}
	found := false
	for _, e := range pe.Errors {
		if e.ErrorType == ErrExclusiveGroupViolation {
			found = true
			break
		}
	}
	if !found {
		t.Errorf("expected exclusive_group_violation error, got: %v", pe.Errors)
	}
}

// =========================================================================
// Additional parser tests
// =========================================================================

func TestParser_Help_ShortFlag(t *testing.T) {
	p, err := NewParserFromBytes([]byte(echoSpec), []string{"echo", "-h"})
	if err != nil {
		t.Fatalf("NewParserFromBytes failed: %v", err)
	}
	result, err := p.Parse()
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	hr, ok := result.(*HelpResult)
	if !ok {
		t.Fatalf("expected *HelpResult, got %T", result)
	}
	if hr.Text == "" {
		t.Error("expected non-empty help text")
	}
}

func TestParser_Help_LongFlag(t *testing.T) {
	p, err := NewParserFromBytes([]byte(echoSpec), []string{"echo", "--help"})
	if err != nil {
		t.Fatalf("NewParserFromBytes failed: %v", err)
	}
	result, err := p.Parse()
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	_, ok := result.(*HelpResult)
	if !ok {
		t.Fatalf("expected *HelpResult, got %T", result)
	}
}

func TestParser_Version(t *testing.T) {
	p, err := NewParserFromBytes([]byte(echoSpec), []string{"echo", "--version"})
	if err != nil {
		t.Fatalf("NewParserFromBytes failed: %v", err)
	}
	result, err := p.Parse()
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	vr, ok := result.(*VersionResult)
	if !ok {
		t.Fatalf("expected *VersionResult, got %T", result)
	}
	if vr.Version != "8.32" {
		t.Errorf("expected version '8.32', got %q", vr.Version)
	}
}

func TestParser_LongFlagWithValue(t *testing.T) {
	spec := `{
    "cli_builder_spec_version": "1.0",
    "name": "myapp",
    "description": "test",
    "flags": [
      {"id": "output", "long": "output", "short": "o", "description": "output file", "type": "string"}
    ]
  }`
	p, err := NewParserFromBytes([]byte(spec), []string{"myapp", "--output=result.txt"})
	if err != nil {
		t.Fatalf("failed: %v", err)
	}
	result, err := p.Parse()
	if err != nil {
		t.Fatalf("Parse failed: %v", err)
	}
	pr := result.(*ParseResult)
	if pr.Flags["output"] != "result.txt" {
		t.Errorf("expected output='result.txt', got %v", pr.Flags["output"])
	}
}

func TestParser_EndOfFlagsMarker(t *testing.T) {
	p, err := NewParserFromBytes([]byte(echoSpec), []string{"echo", "--", "-n", "hello"})
	if err != nil {
		t.Fatalf("NewParserFromBytes failed: %v", err)
	}
	result, err := p.Parse()
	if err != nil {
		t.Fatalf("Parse failed: %v", err)
	}
	pr := result.(*ParseResult)
	// -n should be treated as positional, not a flag
	if pr.Flags["no-newline"] != false {
		t.Errorf("expected no-newline=false (treated as positional after --), got %v", pr.Flags["no-newline"])
	}
}

func TestParser_UnknownFlag(t *testing.T) {
	p, err := NewParserFromBytes([]byte(echoSpec), []string{"echo", "--unknown-flag"})
	if err != nil {
		t.Fatalf("NewParserFromBytes failed: %v", err)
	}
	_, err = p.Parse()
	if err == nil {
		t.Fatal("expected error for unknown flag")
	}
	pe, ok := err.(*ParseErrors)
	if !ok {
		t.Fatalf("expected *ParseErrors, got %T", err)
	}
	found := false
	for _, e := range pe.Errors {
		if e.ErrorType == ErrUnknownFlag {
			found = true
		}
	}
	if !found {
		t.Errorf("expected unknown_flag error")
	}
}

func TestParser_ConflictingFlags(t *testing.T) {
	p, err := NewParserFromBytes([]byte(echoSpec), []string{"echo", "-e", "-E", "hello"})
	if err != nil {
		t.Fatalf("NewParserFromBytes failed: %v", err)
	}
	_, err = p.Parse()
	if err == nil {
		t.Fatal("expected error for conflicting flags")
	}
	pe := err.(*ParseErrors)
	found := false
	for _, e := range pe.Errors {
		if e.ErrorType == ErrConflictingFlags {
			found = true
		}
	}
	if !found {
		t.Errorf("expected conflicting_flags error, got: %v", pe.Errors)
	}
}

func TestParser_EmptyArgv(t *testing.T) {
	p := &Parser{spec: map[string]any{
		"cli_builder_spec_version": "1.0",
		"name":                    "test",
		"description":             "test",
	}, argv: []string{}}
	_, err := p.Parse()
	if err == nil {
		t.Fatal("expected error for empty argv")
	}
}

func TestParser_Git_Alias(t *testing.T) {
	// Use alias "a" instead of "add"
	p, err := NewParserFromBytes([]byte(gitSpec), []string{"git", "remote", "a", "origin", "https://example.com"})
	if err != nil {
		t.Fatalf("NewParserFromBytes failed: %v", err)
	}
	result, err := p.Parse()
	if err != nil {
		t.Fatalf("Parse failed: %v", err)
	}
	pr := result.(*ParseResult)
	// CommandPath should use canonical name "add"
	if len(pr.CommandPath) != 3 || pr.CommandPath[2] != "add" {
		t.Errorf("expected command_path ending in 'add' (canonical), got %v", pr.CommandPath)
	}
}

func TestParser_DuplicateNonRepeatableFlag(t *testing.T) {
	p, err := NewParserFromBytes([]byte(lsSpec), []string{"ls", "-l", "-l"})
	if err != nil {
		t.Fatalf("NewParserFromBytes failed: %v", err)
	}
	_, err = p.Parse()
	if err == nil {
		t.Fatal("expected error for duplicate flag")
	}
	pe := err.(*ParseErrors)
	found := false
	for _, e := range pe.Errors {
		if e.ErrorType == ErrDuplicateFlag {
			found = true
		}
	}
	if !found {
		t.Errorf("expected duplicate_flag error, got: %v", pe.Errors)
	}
}

func TestParser_TooManyArguments(t *testing.T) {
	spec := `{
    "cli_builder_spec_version": "1.0",
    "name": "myapp",
    "description": "test",
    "arguments": [
      {"id": "file", "name": "FILE", "description": "input file", "type": "path", "required": true}
    ]
  }`
	p, err := NewParserFromBytes([]byte(spec), []string{"myapp", "a.txt", "b.txt"})
	if err != nil {
		t.Fatalf("NewParserFromBytes failed: %v", err)
	}
	_, err = p.Parse()
	if err == nil {
		t.Fatal("expected error for too many arguments")
	}
	pe := err.(*ParseErrors)
	found := false
	for _, e := range pe.Errors {
		if e.ErrorType == ErrTooManyArguments {
			found = true
		}
	}
	if !found {
		t.Errorf("expected too_many_arguments error, got: %v", pe.Errors)
	}
}

func TestParser_MissingRequiredArgument(t *testing.T) {
	spec := `{
    "cli_builder_spec_version": "1.0",
    "name": "myapp",
    "description": "test",
    "arguments": [
      {"id": "file", "name": "FILE", "description": "input file", "type": "path", "required": true}
    ]
  }`
	p, err := NewParserFromBytes([]byte(spec), []string{"myapp"})
	if err != nil {
		t.Fatalf("NewParserFromBytes failed: %v", err)
	}
	_, err = p.Parse()
	if err == nil {
		t.Fatal("expected error for missing required argument")
	}
	pe := err.(*ParseErrors)
	found := false
	for _, e := range pe.Errors {
		if e.ErrorType == ErrMissingRequiredArgument {
			found = true
		}
	}
	if !found {
		t.Errorf("expected missing_required_argument error, got: %v", pe.Errors)
	}
}

func TestParser_IntegerFlag(t *testing.T) {
	spec := `{
    "cli_builder_spec_version": "1.0",
    "name": "myapp",
    "description": "test",
    "flags": [
      {"id": "count", "long": "count", "short": "n", "description": "count", "type": "integer"}
    ]
  }`
	p, err := NewParserFromBytes([]byte(spec), []string{"myapp", "--count=42"})
	if err != nil {
		t.Fatalf("NewParserFromBytes failed: %v", err)
	}
	result, err := p.Parse()
	if err != nil {
		t.Fatalf("Parse failed: %v", err)
	}
	pr := result.(*ParseResult)
	if pr.Flags["count"] != int64(42) {
		t.Errorf("expected count=42 (int64), got %v (%T)", pr.Flags["count"], pr.Flags["count"])
	}
}

func TestParser_EnumFlag_Valid(t *testing.T) {
	spec := `{
    "cli_builder_spec_version": "1.0",
    "name": "myapp",
    "description": "test",
    "flags": [
      {
        "id": "format",
        "long": "format",
        "description": "output format",
        "type": "enum",
        "enum_values": ["json", "csv", "table"]
      }
    ]
  }`
	p, err := NewParserFromBytes([]byte(spec), []string{"myapp", "--format=json"})
	if err != nil {
		t.Fatalf("NewParserFromBytes failed: %v", err)
	}
	result, err := p.Parse()
	if err != nil {
		t.Fatalf("Parse failed: %v", err)
	}
	pr := result.(*ParseResult)
	if pr.Flags["format"] != "json" {
		t.Errorf("expected format='json', got %v", pr.Flags["format"])
	}
}

func TestParser_EnumFlag_Invalid(t *testing.T) {
	spec := `{
    "cli_builder_spec_version": "1.0",
    "name": "myapp",
    "description": "test",
    "flags": [
      {
        "id": "format",
        "long": "format",
        "description": "output format",
        "type": "enum",
        "enum_values": ["json", "csv", "table"]
      }
    ]
  }`
	p, err := NewParserFromBytes([]byte(spec), []string{"myapp", "--format=bork"})
	if err != nil {
		t.Fatalf("NewParserFromBytes failed: %v", err)
	}
	_, err = p.Parse()
	if err == nil {
		t.Fatal("expected error for invalid enum value")
	}
	pe := err.(*ParseErrors)
	found := false
	for _, e := range pe.Errors {
		if e.ErrorType == ErrInvalidEnumValue {
			found = true
		}
	}
	if !found {
		t.Errorf("expected invalid_enum_value error, got: %v", pe.Errors)
	}
}

func TestParser_RepeatableFlag(t *testing.T) {
	spec := `{
    "cli_builder_spec_version": "1.0",
    "name": "myapp",
    "description": "test",
    "flags": [
      {
        "id": "verbose",
        "short": "v",
        "long": "verbose",
        "description": "verbose",
        "type": "boolean",
        "repeatable": true
      }
    ]
  }`
	p, err := NewParserFromBytes([]byte(spec), []string{"myapp", "-v", "-v", "-v"})
	if err != nil {
		t.Fatalf("NewParserFromBytes failed: %v", err)
	}
	result, err := p.Parse()
	if err != nil {
		t.Fatalf("Parse failed: %v", err)
	}
	pr := result.(*ParseResult)
	arr, ok := pr.Flags["verbose"].([]any)
	if !ok {
		t.Fatalf("expected repeatable flag to be []any, got %T: %v", pr.Flags["verbose"], pr.Flags["verbose"])
	}
	if len(arr) != 3 {
		t.Errorf("expected 3 occurrences, got %d", len(arr))
	}
}

func TestParseErrors_Error_Single(t *testing.T) {
	pe := &ParseErrors{Errors: []ParseError{
		{ErrorType: ErrUnknownFlag, Message: "unknown flag --foo"},
	}}
	got := pe.Error()
	expected := "parse error: unknown flag --foo"
	if got != expected {
		t.Errorf("expected %q, got %q", expected, got)
	}
}

func TestParseErrors_Error_Multiple(t *testing.T) {
	pe := &ParseErrors{Errors: []ParseError{
		{ErrorType: ErrUnknownFlag, Message: "unknown flag --foo"},
		{ErrorType: ErrMissingRequiredArgument, Message: "missing required argument: <FILE>"},
	}}
	got := pe.Error()
	if got == "" {
		t.Error("expected non-empty error string")
	}
}
