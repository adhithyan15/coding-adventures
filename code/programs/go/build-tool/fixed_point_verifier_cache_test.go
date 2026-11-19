package main

import (
	"os"
	"path/filepath"
	"strings"
	"testing"

	"github.com/adhithyan15/coding-adventures/code/programs/go/build-tool/internal/cache"
	"github.com/adhithyan15/coding-adventures/code/programs/go/build-tool/internal/discovery"
	"github.com/adhithyan15/coding-adventures/code/programs/go/build-tool/internal/executor"
	"github.com/adhithyan15/coding-adventures/code/programs/go/build-tool/internal/hasher"
	"github.com/adhithyan15/coding-adventures/code/programs/go/build-tool/internal/resolver"
)

func runFixedPointReaderCacheProbe(t *testing.T, root, name string) (string, string) {
	t.Helper()
	packageRoot := filepath.Join(root, "code/packages/rust", name)
	build := "echo fixed-point-reader-cache-probe"
	pkg := discovery.Package{Name: "rust/" + name, Language: "rust", Path: packageRoot, BuildContent: build, BuildCommands: []string{build}}
	graph, _, err := resolver.ResolveWithTestTriggers([]discovery.Package{pkg})
	if err != nil {
		t.Fatalf("create %s reader cache graph: %v", name, err)
	}
	packageHash, err := hashPackageWithRepositoryInputs(pkg, root, hasher.HashPackage)
	if err != nil {
		t.Fatalf("hash %s external repository inputs: %v", name, err)
	}
	packageHashes, depsHashes, failed, err := hashPackagesAndDependencies([]discovery.Package{pkg}, graph, func(candidate discovery.Package) (string, error) {
		return hashPackageWithRepositoryInputs(candidate, root, hasher.HashPackage)
	})
	if err != nil {
		t.Fatalf("hash %s cache inputs for %s: %v", name, failed, err)
	}
	cachePath := filepath.Join(root, ".build-cache-"+name+".json")
	buildCache := cache.New()
	buildCache.Load(cachePath)
	results := executor.ExecuteBuilds([]discovery.Package{pkg}, graph, buildCache, packageHashes, depsHashes, false, false, 1, nil, nil, false)
	result, ok := results[pkg.Name]
	if !ok {
		t.Fatalf("executor omitted %s reader result: %#v", name, results)
	}
	if err := buildCache.Save(cachePath); err != nil {
		t.Fatal(err)
	}
	return result.Status, packageHash
}

func TestFixedPointVerifierTreesChangePersistedCacheIdentityAndRestore(t *testing.T) {
	root, _, _ := writeFixedPointCacheFixture(t)
	for _, name := range []string{"ca-assurance-report", "warrant-ir"} {
		if status, _ := runFixedPointReaderCacheProbe(t, root, name); status != "built" {
			t.Fatalf("initial %s cache status = %q, want built", name, status)
		}
		if status, _ := runFixedPointReaderCacheProbe(t, root, name); status != "skipped" {
			t.Fatalf("warm %s cache status = %q, want skipped", name, status)
		}
	}
	newSource := filepath.Join(root, "code/packages/rust/fixed-point/src/cache_probe.rs")
	if err := os.WriteFile(newSource, []byte("pub fn cache_probe() -> u8 { 1 }\n"), 0o644); err != nil {
		t.Fatal(err)
	}
	for _, name := range []string{"ca-assurance-report", "warrant-ir"} {
		if status, _ := runFixedPointReaderCacheProbe(t, root, name); status != "built" {
			t.Fatalf("untracked new source file did not miss %s cache: %q", name, status)
		}
	}
	if err := os.Remove(newSource); err != nil {
		t.Fatal(err)
	}
	for _, name := range []string{"ca-assurance-report", "warrant-ir"} {
		if status, _ := runFixedPointReaderCacheProbe(t, root, name); status != "built" {
			t.Fatalf("source tree deletion did not miss %s cache: %q", name, status)
		}
		if status, _ := runFixedPointReaderCacheProbe(t, root, name); status != "skipped" {
			t.Fatalf("restored %s tree identity was not warm: %q", name, status)
		}
	}
	newEvidence := filepath.Join(root, "code/packages/rust/fixed-point/evidence/cache-probe.json")
	if err := os.WriteFile(newEvidence, []byte("{\"control\":true}\n"), 0o644); err != nil {
		t.Fatal(err)
	}
	if status, _ := runFixedPointReaderCacheProbe(t, root, "ca-assurance-report"); status != "built" {
		t.Fatalf("untracked new evidence file did not miss report cache: %q", status)
	}
	if err := os.Remove(newEvidence); err != nil {
		t.Fatal(err)
	}
	if status, _ := runFixedPointReaderCacheProbe(t, root, "ca-assurance-report"); status != "built" {
		t.Fatalf("evidence tree deletion did not miss report cache: %q", status)
	}
	if status, _ := runFixedPointReaderCacheProbe(t, root, "ca-assurance-report"); status != "skipped" {
		t.Fatalf("restored report evidence identity was not warm: %q", status)
	}

	mutations := []struct {
		name, path, old, changed string
	}{
		{"warrant-ir", "code/packages/rust/fixed-point/src/lib.rs", "const SCALE: i128 = 65_536;", "const SCALE: i128 = 65_535;"},
		{"warrant-ir", "code/specs/fixed-point.warrant", "b.raw == 0i32", "b.raw != 0i32"},
		{"ca-assurance-report", "code/packages/rust/fixed-point/evidence/proof-manifest.json", "\"max_len\": 0", "\"max_len\": 1"},
	}
	for _, mutation := range mutations {
		path := filepath.Join(root, filepath.FromSlash(mutation.path))
		original, err := os.ReadFile(path)
		if err != nil {
			t.Fatal(err)
		}
		changed := []byte(strings.Replace(string(original), mutation.old, mutation.changed, 1))
		if string(changed) == string(original) || len(changed) != len(original) {
			t.Fatalf("mutation for %s did not produce a same-length change", mutation.path)
		}
		if err := os.WriteFile(path, changed, 0o644); err != nil {
			t.Fatal(err)
		}
		if status, changedHash := runFixedPointReaderCacheProbe(t, root, mutation.name); status != "built" {
			t.Fatalf("same-length change to %s reader input produced status=%q hash=%s; want cache miss", mutation.path, status, changedHash)
		}
		if err := os.WriteFile(path, original, 0o644); err != nil {
			t.Fatal(err)
		}
		if status, _ := runFixedPointReaderCacheProbe(t, root, mutation.name); status != "built" {
			t.Fatalf("restoring %s did not miss against last persisted identity: %q", mutation.path, status)
		}
		if status, _ := runFixedPointReaderCacheProbe(t, root, mutation.name); status != "skipped" {
			t.Fatalf("restored %s identity was not warm: %q", mutation.path, status)
		}
	}

	for _, name := range []string{"ca-assurance-report", "warrant-ir"} {
		outside := filepath.Join(root, "untracked-credential.txt")
		if err := os.WriteFile(outside, []byte("not consumed"), 0o600); err != nil {
			t.Fatal(err)
		}
		if status, _ := runFixedPointReaderCacheProbe(t, root, name); status != "skipped" {
			t.Fatalf("unrelated untracked control changed %s hash: %q", name, status)
		}
		if err := os.Remove(outside); err != nil {
			t.Fatal(err)
		}
	}
}
