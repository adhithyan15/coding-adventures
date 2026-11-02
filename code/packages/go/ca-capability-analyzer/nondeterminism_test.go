package main

// Tests for D21's nondeterminism categories (VER00 §6.3) in the Go analyzer:
// ambient randomness is rand:read:*, a go statement is thread:spawn:*, and
// seeded randomness is pure computation that must never be reported.

import "testing"

func countCap(caps []DetectedCapability, want CapabilityString) int {
	n := 0
	for _, c := range caps {
		if c.Capability == want {
			n++
		}
	}
	return n
}

func TestDetect_Rand_CryptoRandImport(t *testing.T) {
	fset, files := parseFiles(t,
		"key.go", `package key
import "crypto/rand"
func f(p []byte) { rand.Read(p) }
`)
	caps := DetectCapabilities(fset, files)
	if !containsCap(caps, "rand:read:*") {
		t.Errorf("expected rand:read:* for crypto/rand, got %v", caps)
	}
}

func TestDetect_Rand_MathRandTopLevelCalls(t *testing.T) {
	fset, files := parseFiles(t,
		"dice.go", `package dice
import "math/rand"
func roll() int { return rand.Intn(6) + 1 }
func coin() float64 { return rand.Float64() }
func mix(xs []int) { rand.Shuffle(len(xs), func(i, j int) { xs[i], xs[j] = xs[j], xs[i] }) }
`)
	caps := DetectCapabilities(fset, files)
	if got := countCap(caps, "rand:read:*"); got != 3 {
		t.Errorf("expected 3 rand:read:* findings, got %d: %v", got, caps)
	}
}

func TestDetect_Rand_MathRandV2UsesPackageNameNotVersion(t *testing.T) {
	// The import binds "rand", not "v2". Before major-version suffixes were
	// understood, every math/rand/v2 call went unseen.
	fset, files := parseFiles(t,
		"dice.go", `package dice
import "math/rand/v2"
func roll() int { return rand.IntN(6) + 1 }
func pick() uint64 { return rand.N(uint64(10)) }
`)
	caps := DetectCapabilities(fset, files)
	if got := countCap(caps, "rand:read:*"); got != 2 {
		t.Errorf("expected 2 rand:read:* findings for math/rand/v2, got %d: %v", got, caps)
	}
}

func TestDetect_Rand_SeededGeneratorIsPure(t *testing.T) {
	fset, files := parseFiles(t,
		"sim.go", `package sim
import (
	"math/rand"
	randv2 "math/rand/v2"
)
func roll() int {
	rng := rand.New(rand.NewSource(42))
	return rng.Intn(6)
}
func roll2() uint64 {
	rng := randv2.New(randv2.NewPCG(1, 2))
	return rng.Uint64()
}
`)
	caps := DetectCapabilities(fset, files)
	if containsCap(caps, "rand:read:*") {
		t.Errorf("a generator seeded by the caller is pure computation, got %v", caps)
	}
}

func TestDetect_Rand_AliasedImport(t *testing.T) {
	fset, files := parseFiles(t,
		"dice.go", `package dice
import mrand "math/rand"
func roll() int { return mrand.Intn(6) }
`)
	if caps := DetectCapabilities(fset, files); !containsCap(caps, "rand:read:*") {
		t.Errorf("expected rand:read:* through an import alias, got %v", caps)
	}
}

func TestDetect_Thread_GoStatements(t *testing.T) {
	fset, files := parseFiles(t,
		"pool.go", `package pool
func work() {}
func start() {
	go work()
	go func() { work() }()
}
`)
	caps := DetectCapabilities(fset, files)
	if got := countCap(caps, "thread:spawn:*"); got != 2 {
		t.Errorf("expected 2 thread:spawn:* findings, got %d: %v", got, caps)
	}
	for _, c := range caps {
		if c.Capability == "thread:spawn:*" && c.Evidence != "go statement" {
			t.Errorf("unexpected evidence %q", c.Evidence)
		}
	}
}

func TestDetect_Thread_NolintExemptsGoStatement(t *testing.T) {
	fset, files := parseFiles(t,
		"backend.go", `package backend
func spawn(body func()) {
	go body() //nolint:cap
}
`)
	if caps := DetectCapabilities(fset, files); containsCap(caps, "thread:spawn:*") {
		t.Errorf("a //nolint:cap go statement must be exempt, got %v", caps)
	}
}

func TestDetect_Thread_GeneratedFileIsExempt(t *testing.T) {
	fset, files := parseFiles(t,
		"gen_capabilities.go", `package foo
func spawn(body func()) { go body() }
`)
	if caps := DetectCapabilities(fset, files); len(caps) != 0 {
		t.Errorf("gen_capabilities.go must be skipped, got %v", caps)
	}
}

func TestDefaultImportName(t *testing.T) {
	cases := map[string]string{
		"os":                  "os",
		"net/http":            "http",
		"math/rand/v2":        "rand",
		"example.com/mod/v10": "mod",
		"v2":                  "v2",
		"example.com/vx":      "vx",
		"example.com/v":       "v",
	}
	for path, want := range cases {
		if got := defaultImportName(path); got != want {
			t.Errorf("defaultImportName(%q) = %q, want %q", path, got, want)
		}
	}
}

func TestAnalyzeDir_NondeterminismNeedsDeclaration(t *testing.T) {
	src := `package worker
import "math/rand"
func start() { go func() { _ = rand.Intn(3) }() }
`
	undeclared := t.TempDir()
	writeGoFile(t, undeclared, "worker.go", src)
	result, err := AnalyzeDir(undeclared)
	if err != nil {
		t.Fatalf("AnalyzeDir: %v", err)
	}
	if result.Passed() {
		t.Fatal("no manifest means no capabilities: randomness and a goroutine must fail")
	}
	if got := len(result.Violations); got != 2 {
		t.Errorf("expected 2 CAP001 violations, got %d: %v", got, result.Violations)
	}

	declared := t.TempDir()
	writeGoFile(t, declared, "worker.go", src)
	writeManifestInDir(t, declared, `{
		"version": 1,
		"package": "go/worker",
		"capabilities": [
			{"category": "rand", "action": "read", "target": "*", "justification": "Jitters retry delays."},
			{"category": "thread", "action": "spawn", "target": "retry-loop", "justification": "Retries in the background."}
		],
		"justification": "A background retry loop with jitter."
	}`)
	result, err = AnalyzeDir(declared)
	if err != nil {
		t.Fatalf("AnalyzeDir: %v", err)
	}
	if !result.Passed() {
		t.Errorf("declared rand:read and thread:spawn must pass, got %v", result.Violations)
	}
}

func TestLoadManifest_RejectsNondeterminismCrossPairs(t *testing.T) {
	for _, pair := range [][2]string{{"proc", "spawn"}, {"ui", "message"}, {"rand", "write"}} {
		dir := t.TempDir()
		writeManifestInDir(t, dir, `{
			"version": 1,
			"package": "go/x",
			"capabilities": [
				{"category": "`+pair[0]+`", "action": "`+pair[1]+`", "target": "*", "justification": "cross-pair fixture"}
			],
			"justification": "cross-pair fixture"
		}`)
		if _, err := AnalyzeDir(dir); err == nil {
			t.Errorf("%s:%s must be rejected", pair[0], pair[1])
		}
	}
}
