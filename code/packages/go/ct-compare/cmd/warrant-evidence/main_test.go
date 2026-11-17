package main

import (
	"errors"
	"path/filepath"
	"strings"
	"testing"
)

func TestAdaptersTranslateValuesAndErrors(t *testing.T) {
	if got, err := eq([]any{[]byte{1}, []byte{1}}); err != nil || got != true {
		t.Fatal("eq")
	}
	if got, _ := eqFixed([]any{[]byte{1}, []byte{2}}); got != false {
		t.Fatal("eqFixed")
	}
	if got, _ := eqU64([]any{^uint64(0), ^uint64(0) - 1}); got != false {
		t.Fatal("eqU64 must keep 2^64-1 and 2^64-2 apart")
	}
	left, right := []byte{1, 2}, []byte{3, 4}
	if got, err := selectBytes([]any{left, right, true}); err != nil || string(got.([]byte)) != string(left) {
		t.Fatalf("choice true selects the left slice: %v %v", got, err)
	}
	if got, _ := selectBytes([]any{left, right, false}); string(got.([]byte)) != string(right) {
		t.Fatal("choice false selects the right slice")
	}
	if got, err := selectBytes([]any{left, []byte{1}, true}); got != nil || !errors.Is(err, ErrLengthMismatch) {
		t.Fatalf("length mismatch maps to the sentinel: %v %v", got, err)
	}
}

func TestOtherPanicsAreNotSwallowed(t *testing.T) {
	// A panic with any other message must reach the runner (which records it as
	// `unexpected-error`); only the exact length-mismatch message is translated.
	defer func() {
		if recover() == nil {
			t.Fatal("an unrelated panic was swallowed")
		}
	}()
	selectBytes([]any{[]byte{1}, []byte{1}, "not a bool"}) // the type assertion panics
}

func TestForwardAddsThePackagePaths(t *testing.T) {
	got := forward("/repo/code/packages/go/ct-compare", []string{"check", "evidence/test-manifest.json"})
	joined := strings.Join(got, " ")
	for _, want := range []string{"check --repo-root", "ct-compare-warrant-v1", "--binding", "evidence.bindings", "--package-dir", "evidence/test-manifest.json"} {
		if !strings.Contains(filepath.ToSlash(joined), want) {
			t.Errorf("%q missing from %s", want, joined)
		}
	}
	again := []string{"run", "--repo-root", "/r", "--corpus", "/c"}
	if strings.Join(forward("/p", again), " ") != strings.Join(again, " ") {
		t.Fatal("a command that already has its paths (a sabotage child) is forwarded unchanged")
	}
	if len(forward("x", nil)) != 0 {
		t.Fatal("no command, nothing to forward")
	}
}

func TestRegistryDescribesTheRealExports(t *testing.T) {
	r := registry()
	if len(r.Exports) != 4 || r.Errors["ErrLengthMismatch"] != ErrLengthMismatch {
		t.Fatal("registry")
	}
	for name, export := range r.Exports {
		if export.Call == nil || export.Impl == nil {
			t.Errorf("%s: adapter and library function are both required", name)
		}
	}
}

func TestCommittedEvidenceIsCurrent(t *testing.T) {
	// The same check BUILD runs: regenerate the manifest in memory and compare it
	// with the committed file. The package directory is two levels up.
	pkg, err := filepath.Abs("../..")
	if err != nil {
		t.Fatal(err)
	}
	if status := run(pkg, []string{"check", filepath.Join(pkg, "evidence", "test-manifest.json")}); status != 0 {
		t.Fatalf("check exited %d", status)
	}
	if status := run(pkg, []string{"validate-manifest", filepath.Join(pkg, "evidence", "test-manifest.json")}); status != 0 {
		t.Fatalf("validate-manifest exited %d", status)
	}
	if status := run(pkg, []string{"bogus"}); status != 2 {
		t.Fatalf("a bad command exits 2, got %d", status)
	}
}
