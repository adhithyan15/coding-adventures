package clibuilder

import (
	"testing"
)

// =========================================================================
// Spec loader tests
// =========================================================================
//
// These tests verify that LoadSpecFromBytes correctly validates specs and
// surfaces meaningful errors when the spec is invalid.

// minimalValidSpec is the simplest possible valid spec — just the required fields.
const minimalValidSpec = `{
  "cli_builder_spec_version": "1.0",
  "name": "myapp",
  "description": "A test application"
}`

// fullValidSpec exercises most spec features: global flags, local flags,
// arguments, subcommands, and mutually exclusive groups.
const fullValidSpec = `{
  "cli_builder_spec_version": "1.0",
  "name": "grep",
  "description": "Search for patterns in files",
  "version": "3.7",
  "parsing_mode": "gnu",
  "global_flags": [
    {
      "id": "verbose",
      "short": "v",
      "long": "verbose",
      "description": "Verbose output",
      "type": "boolean"
    }
  ],
  "flags": [
    {
      "id": "extended-regexp",
      "short": "E",
      "long": "extended-regexp",
      "description": "Use extended regular expressions",
      "type": "boolean"
    },
    {
      "id": "fixed-strings",
      "short": "F",
      "long": "fixed-strings",
      "description": "Treat pattern as fixed string",
      "type": "boolean"
    },
    {
      "id": "count",
      "short": "c",
      "long": "count",
      "description": "Count matching lines",
      "type": "boolean"
    }
  ],
  "arguments": [
    {
      "id": "pattern",
      "name": "PATTERN",
      "description": "Regular expression pattern",
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

func TestLoadSpec_Valid_Minimal(t *testing.T) {
	spec, err := LoadSpecFromBytes([]byte(minimalValidSpec))
	if err != nil {
		t.Fatalf("expected no error, got: %v", err)
	}
	if spec["name"] != "myapp" {
		t.Errorf("expected name 'myapp', got %v", spec["name"])
	}
}

func TestLoadSpec_Valid_Full(t *testing.T) {
	spec, err := LoadSpecFromBytes([]byte(fullValidSpec))
	if err != nil {
		t.Fatalf("expected no error, got: %v", err)
	}
	if spec["name"] != "grep" {
		t.Errorf("expected name 'grep', got %v", spec["name"])
	}
}

func TestLoadSpec_MissingName(t *testing.T) {
	raw := `{"cli_builder_spec_version": "1.0", "description": "test"}`
	_, err := LoadSpecFromBytes([]byte(raw))
	if err == nil {
		t.Fatal("expected error for missing name, got nil")
	}
	se, ok := err.(*SpecError)
	if !ok {
		t.Fatalf("expected *SpecError, got %T: %v", err, err)
	}
	if se.Message == "" {
		t.Error("SpecError message should not be empty")
	}
}

func TestLoadSpec_MissingDescription(t *testing.T) {
	raw := `{"cli_builder_spec_version": "1.0", "name": "app"}`
	_, err := LoadSpecFromBytes([]byte(raw))
	if err == nil {
		t.Fatal("expected error for missing description, got nil")
	}
	if _, ok := err.(*SpecError); !ok {
		t.Fatalf("expected *SpecError, got %T", err)
	}
}

func TestLoadSpec_WrongVersion(t *testing.T) {
	raw := `{"cli_builder_spec_version": "2.0", "name": "app", "description": "test"}`
	_, err := LoadSpecFromBytes([]byte(raw))
	if err == nil {
		t.Fatal("expected error for wrong spec version, got nil")
	}
}

func TestLoadSpec_DuplicateFlagIDs(t *testing.T) {
	raw := `{
    "cli_builder_spec_version": "1.0",
    "name": "app",
    "description": "test",
    "flags": [
      {"id": "verbose", "short": "v", "description": "verbose", "type": "boolean"},
      {"id": "verbose", "long": "verbose", "description": "verbose again", "type": "boolean"}
    ]
  }`
	_, err := LoadSpecFromBytes([]byte(raw))
	if err == nil {
		t.Fatal("expected error for duplicate flag IDs, got nil")
	}
	se, ok := err.(*SpecError)
	if !ok {
		t.Fatalf("expected *SpecError, got %T", err)
	}
	// Error message should mention "duplicate"
	if se.Message == "" {
		t.Error("SpecError message should not be empty")
	}
}

func TestLoadSpec_CircularRequires(t *testing.T) {
	// v requires q, q requires v — a cycle
	raw := `{
    "cli_builder_spec_version": "1.0",
    "name": "app",
    "description": "test",
    "flags": [
      {
        "id": "verbose",
        "short": "v",
        "description": "verbose",
        "type": "boolean",
        "requires": ["quiet"]
      },
      {
        "id": "quiet",
        "short": "q",
        "description": "quiet",
        "type": "boolean",
        "requires": ["verbose"]
      }
    ]
  }`
	_, err := LoadSpecFromBytes([]byte(raw))
	if err == nil {
		t.Fatal("expected error for circular requires, got nil")
	}
	se, ok := err.(*SpecError)
	if !ok {
		t.Fatalf("expected *SpecError, got %T", err)
	}
	if se.Message == "" {
		t.Error("SpecError message should not be empty")
	}
}

func TestLoadSpec_FlagMissingShortLongSDL(t *testing.T) {
	// A flag with none of short/long/single_dash_long
	raw := `{
    "cli_builder_spec_version": "1.0",
    "name": "app",
    "description": "test",
    "flags": [
      {"id": "verbose", "description": "verbose", "type": "boolean"}
    ]
  }`
	_, err := LoadSpecFromBytes([]byte(raw))
	if err == nil {
		t.Fatal("expected error for flag missing short/long/sdl, got nil")
	}
}

func TestLoadSpec_EnumMissingEnumValues(t *testing.T) {
	raw := `{
    "cli_builder_spec_version": "1.0",
    "name": "app",
    "description": "test",
    "flags": [
      {"id": "fmt", "long": "format", "description": "output format", "type": "enum"}
    ]
  }`
	_, err := LoadSpecFromBytes([]byte(raw))
	if err == nil {
		t.Fatal("expected error for enum without enum_values, got nil")
	}
}

func TestLoadSpec_MultipleVariadicArguments(t *testing.T) {
	raw := `{
    "cli_builder_spec_version": "1.0",
    "name": "app",
    "description": "test",
    "arguments": [
      {"id": "src", "name": "SRC", "description": "source", "type": "path", "variadic": true},
      {"id": "dst", "name": "DST", "description": "dest", "type": "path", "variadic": true}
    ]
  }`
	_, err := LoadSpecFromBytes([]byte(raw))
	if err == nil {
		t.Fatal("expected error for multiple variadic arguments, got nil")
	}
}

func TestLoadSpec_ConflictsWithUnknownID(t *testing.T) {
	raw := `{
    "cli_builder_spec_version": "1.0",
    "name": "app",
    "description": "test",
    "flags": [
      {
        "id": "verbose",
        "short": "v",
        "description": "verbose",
        "type": "boolean",
        "conflicts_with": ["nonexistent"]
      }
    ]
  }`
	_, err := LoadSpecFromBytes([]byte(raw))
	if err == nil {
		t.Fatal("expected error for conflicts_with unknown ID, got nil")
	}
}

func TestLoadSpec_ExclusiveGroupUnknownFlagID(t *testing.T) {
	raw := `{
    "cli_builder_spec_version": "1.0",
    "name": "app",
    "description": "test",
    "flags": [
      {"id": "verbose", "short": "v", "description": "verbose", "type": "boolean"}
    ],
    "mutually_exclusive_groups": [
      {"id": "grp", "flag_ids": ["verbose", "nonexistent"]}
    ]
  }`
	_, err := LoadSpecFromBytes([]byte(raw))
	if err == nil {
		t.Fatal("expected error for exclusive group referencing unknown flag ID, got nil")
	}
}

func TestLoadSpec_InvalidJSON(t *testing.T) {
	_, err := LoadSpecFromBytes([]byte(`{not valid json`))
	if err == nil {
		t.Fatal("expected error for invalid JSON, got nil")
	}
}

func TestLoadSpec_SubcommandValidation(t *testing.T) {
	// A subcommand with circular requires in its own flags
	raw := `{
    "cli_builder_spec_version": "1.0",
    "name": "app",
    "description": "test",
    "commands": [
      {
        "id": "sub",
        "name": "sub",
        "description": "a subcommand",
        "flags": [
          {
            "id": "a",
            "short": "a",
            "description": "flag a",
            "type": "boolean",
            "requires": ["b"]
          },
          {
            "id": "b",
            "short": "b",
            "description": "flag b",
            "type": "boolean",
            "requires": ["a"]
          }
        ]
      }
    ]
  }`
	_, err := LoadSpecFromBytes([]byte(raw))
	if err == nil {
		t.Fatal("expected error for circular requires in subcommand, got nil")
	}
}

func TestSpecError_Error(t *testing.T) {
	se := &SpecError{Message: "something went wrong"}
	got := se.Error()
	expected := "spec error: something went wrong"
	if got != expected {
		t.Errorf("expected %q, got %q", expected, got)
	}
}
