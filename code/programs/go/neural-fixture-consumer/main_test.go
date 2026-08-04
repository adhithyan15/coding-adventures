package main

import (
	"bytes"
	"encoding/json"
	"os"
	"path/filepath"
	"strings"
	"testing"
)

const fixturePath = "../../../specs/fixtures/neural-learning-v1/labs/00-weighted-neuron.json"

func fixturePayload(t *testing.T) string {
	t.Helper()
	payload, err := os.ReadFile(fixturePath)
	if err != nil {
		t.Fatal(err)
	}
	return string(payload)
}

func writeFixture(t *testing.T, payload string) string {
	t.Helper()
	path := filepath.Join(t.TempDir(), "fixture.json")
	if err := os.WriteFile(path, []byte(payload), 0o600); err != nil {
		t.Fatal(err)
	}
	return path
}

func TestRunEmitsPassingReceipt(t *testing.T) {
	var output bytes.Buffer
	if err := run([]string{"--fixture", fixturePath}, &output); err != nil {
		t.Fatal(err)
	}
	var result receipt
	if err := json.Unmarshal(output.Bytes(), &result); err != nil {
		t.Fatal(err)
	}
	if result.LaneID != laneID || !result.Passes || result.Preactivation != 1.35 {
		t.Fatalf("unexpected receipt: %#v", result)
	}
	if len(result.Contributions) != 2 || result.Contributions[0] != 1 || result.Contributions[1] != 0.25 {
		t.Fatalf("unexpected contributions: %#v", result.Contributions)
	}
}

func TestLoadFixtureRejectsUnknownFields(t *testing.T) {
	payload, err := os.ReadFile(fixturePath)
	if err != nil {
		t.Fatal(err)
	}
	mutated := strings.Replace(string(payload), "\"schema_version\": 1,", "\"schema_version\": 1,\n  \"surprise\": true,", 1)
	path := filepath.Join(t.TempDir(), "unknown.json")
	if err := os.WriteFile(path, []byte(mutated), 0o600); err != nil {
		t.Fatal(err)
	}
	if _, err := loadFixture(path); err == nil || !strings.Contains(err.Error(), "unknown field") {
		t.Fatalf("expected unknown-field rejection, got %v", err)
	}
}

func TestLoadFixtureRejectsDuplicateFields(t *testing.T) {
	payload := strings.Replace(fixturePayload(t), `"schema_version": 1,`, `"schema_version": 1, "schema_version": 1,`, 1)
	path := writeFixture(t, payload)

	if _, err := loadFixture(path); err == nil || !strings.Contains(err.Error(), "duplicate object key") {
		t.Fatalf("expected duplicate-key error, got %v", err)
	}
}

func TestLoadFixtureRejectsCaseAliases(t *testing.T) {
	payload := strings.Replace(fixturePayload(t), `"schema_version"`, `"SCHEMA_VERSION"`, 1)
	path := writeFixture(t, payload)

	if _, err := loadFixture(path); err == nil || !strings.Contains(err.Error(), "canonical lowercase") {
		t.Fatalf("expected canonical-key error, got %v", err)
	}
}

func TestRunRequiresExactArguments(t *testing.T) {
	if err := run(nil, &bytes.Buffer{}); err == nil || !strings.Contains(err.Error(), "usage") {
		t.Fatalf("expected usage error, got %v", err)
	}
}
