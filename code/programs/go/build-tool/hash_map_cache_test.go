package main

import (
	"crypto/sha256"
	"encoding/hex"
	"os"
	"path/filepath"
	"reflect"
	"testing"

	"github.com/adhithyan15/coding-adventures/code/programs/go/build-tool/internal/discovery"
	"github.com/adhithyan15/coding-adventures/code/programs/go/build-tool/internal/hasher"
)

func TestCanonicalHashMapFixtureParticipatesInFallbackHash(t *testing.T) {
	root := findRepoRoot("")
	pkg := discovery.Package{
		Name:     "swift/hash-map",
		Language: "swift",
		Path:     filepath.Join(root, "code", "packages", "swift", "hash-map"),
	}
	fixture := filepath.Join(root, filepath.FromSlash(swiftHashMapFixtureInputs[0]))
	contents, err := os.ReadFile(fixture)
	if err != nil {
		t.Fatalf("read canonical DT18 fixture: %v", err)
	}
	plainHash, err := hasher.HashPackage(pkg)
	if err != nil {
		t.Fatalf("hash Swift package without repository input: %v", err)
	}
	withFixtureHash, err := hasher.HashPackageWithRepositoryInputs(pkg, root, repositoryInputsForPackage(pkg.Name))
	if err != nil {
		t.Fatalf("hash Swift package with canonical DT18 fixture: %v", err)
	}
	if plainHash == withFixtureHash {
		t.Fatal("canonical cases.json did not affect the Swift package fallback hash")
	}
	digest := sha256.Sum256(contents)
	t.Logf("canonical cases.json sha256=%s package_hash_without_fixture=%s package_hash_with_fixture=%s", hex.EncodeToString(digest[:]), plainHash, withFixtureHash)
}

func TestHashMapFixtureChangesFallbackPackageHash(t *testing.T) {
	root := t.TempDir()
	packageRoot := filepath.Join(root, "code", "packages", "swift", "hash-map")
	fixturePath := filepath.Join(root, "code", "specs", "fixtures", "hash-map-v1", "cases.json")
	if err := os.MkdirAll(packageRoot, 0o755); err != nil {
		t.Fatal(err)
	}
	if err := os.MkdirAll(filepath.Dir(fixturePath), 0o755); err != nil {
		t.Fatal(err)
	}
	source := filepath.Join(packageRoot, "HashMap.swift")
	if err := os.WriteFile(source, []byte("public struct HashMap {}\n"), 0o644); err != nil {
		t.Fatal(err)
	}
	if err := os.WriteFile(fixturePath, []byte("{\"case\":1}\n"), 0o644); err != nil {
		t.Fatal(err)
	}
	pkg := discovery.Package{Name: "swift/hash-map", Language: "swift", Path: packageRoot}
	inputs := repositoryInputsForPackage(pkg.Name)
	if want := []string{"code/specs/fixtures/hash-map-v1/cases.json"}; !reflect.DeepEqual(inputs, want) {
		t.Fatalf("repository inputs = %v, want %v", inputs, want)
	}
	hashBefore, err := hasher.HashPackageWithRepositoryInputs(pkg, root, inputs)
	if err != nil {
		t.Fatalf("hash initial package and fixture: %v", err)
	}
	if err := os.WriteFile(fixturePath, []byte("{\"case\":2}\n"), 0o644); err != nil {
		t.Fatal(err)
	}
	hashAfter, err := hasher.HashPackageWithRepositoryInputs(pkg, root, inputs)
	if err != nil {
		t.Fatalf("hash changed package and fixture: %v", err)
	}
	if hashBefore == hashAfter {
		t.Fatalf("fallback package hash did not change when cases.json changed: %s", hashBefore)
	}
	t.Logf("Swift hash-map fallback cache input changed hash: %s -> %s", hashBefore, hashAfter)
}
