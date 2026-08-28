package main

import (
	"os"
	"path/filepath"
	"testing"

	"github.com/adhithyan15/coding-adventures/code/programs/go/build-tool/internal/discovery"
	"github.com/adhithyan15/coding-adventures/code/programs/go/build-tool/internal/resolver"
)

func TestPlatformPlanKeepsWindowsNativeDependencyOutOfUnix(t *testing.T) {
	root := t.TempDir()
	swiftPath := filepath.Join(root, "code", "packages", "swift", "Barcode1D")
	rustPath := filepath.Join(root, "code", "packages", "rust", "paint-vm-direct2d-c")
	for _, path := range []string{swiftPath, rustPath} {
		if err := os.MkdirAll(path, 0o755); err != nil {
			t.Fatal(err)
		}
	}
	if err := os.WriteFile(
		filepath.Join(swiftPath, "BUILD"),
		[]byte("echo unix\n"),
		0o644,
	); err != nil {
		t.Fatal(err)
	}
	if err := os.WriteFile(
		filepath.Join(swiftPath, "BUILD_windows"),
		[]byte("REM # build-tool: deps=rust/paint-vm-direct2d-c\r\ncd ..\\..\\rust\\paint-vm-direct2d-c && cargo build\r\n"),
		0o644,
	); err != nil {
		t.Fatal(err)
	}
	if err := os.WriteFile(filepath.Join(rustPath, "BUILD"), []byte("cargo test\n"), 0o644); err != nil {
		t.Fatal(err)
	}

	packages := []discovery.Package{
		{
			Name:          "swift/Barcode1D",
			Path:          swiftPath,
			Language:      "swift",
			BuildCommands: []string{"echo unix"},
			BuildContent:  "echo unix\n",
		},
		{
			Name:          "rust/paint-vm-direct2d-c",
			Path:          rustPath,
			Language:      "rust",
			BuildCommands: []string{"cargo test"},
			BuildContent:  "cargo test\n",
		},
	}

	linuxPackages := packagesForPlatform(packages, "linux")
	linuxGraph, err := resolver.ResolveDependencies(linuxPackages)
	if err != nil {
		t.Fatal(err)
	}
	if linuxGraph.HasEdge("rust/paint-vm-direct2d-c", "swift/Barcode1D") {
		t.Fatalf("Unix graph unexpectedly contains Windows-only dependency: %v", linuxGraph.Edges())
	}

	windowsPackages := packagesForPlatform(packages, "windows")
	windowsGraph, err := resolver.ResolveDependencies(windowsPackages)
	if err != nil {
		t.Fatal(err)
	}
	if !windowsGraph.HasEdge("rust/paint-vm-direct2d-c", "swift/Barcode1D") {
		t.Fatalf("Windows graph is missing native dependency: %v", windowsGraph.Edges())
	}

	changed := map[string]bool{"swift/Barcode1D": true}
	linuxAffected := affectedForGraph(linuxGraph, changed, nil, false)
	windowsAffected := affectedForGraph(windowsGraph, changed, nil, false)
	if linuxAffected["rust/paint-vm-direct2d-c"] {
		t.Fatalf("Unix affected set contains Windows-only prerequisite: %#v", linuxAffected)
	}
	if !windowsAffected["rust/paint-vm-direct2d-c"] {
		t.Fatalf("Windows affected set omits native prerequisite: %#v", windowsAffected)
	}
	if !computeLanguagesNeeded(windowsPackages, windowsAffected, false, nil)["rust"] {
		t.Fatal("Windows native prerequisite did not request the Rust toolchain")
	}
}

func TestAffectedForGraphPreservesNilAndEmptyFallback(t *testing.T) {
	if affected := affectedForGraph(nil, nil, nil, false); affected != nil {
		t.Fatalf("nil fallback became %#v", affected)
	}
	empty := map[string]bool{}
	if affected := affectedForGraph(nil, nil, empty, false); affected == nil || len(affected) != 0 {
		t.Fatalf("empty fallback became %#v", affected)
	}
}

func TestChangedPackageRootsUseOnlySelectedPlatformBuildFile(t *testing.T) {
	root := t.TempDir()
	pkgPath := filepath.Join(root, "code", "packages", "swift", "Barcode1D")
	if err := os.MkdirAll(pkgPath, 0o755); err != nil {
		t.Fatal(err)
	}
	for name, contents := range map[string]string{
		"BUILD":         "echo unix\n",
		"BUILD_windows": "echo windows\n",
	} {
		if err := os.WriteFile(filepath.Join(pkgPath, name), []byte(contents), 0o644); err != nil {
			t.Fatal(err)
		}
	}
	packages := []discovery.Package{{
		Name:     "swift/Barcode1D",
		Path:     pkgPath,
		Language: "swift",
	}}

	windowsOnly := []string{"code/packages/swift/Barcode1D/BUILD_windows"}
	if got := changedPackageRootsForPlatform(windowsOnly, packages, root, "linux"); len(got) != 0 {
		t.Fatalf("Windows BUILD change affected Linux: %#v", got)
	}
	if got := changedPackageRootsForPlatform(windowsOnly, packages, root, "windows"); !got["swift/Barcode1D"] {
		t.Fatalf("Windows BUILD change did not affect Windows: %#v", got)
	}

	sharedSource := []string{"code/packages/swift/Barcode1D/Sources/Barcode.swift"}
	for _, goos := range []string{"linux", "darwin", "windows"} {
		if got := changedPackageRootsForPlatform(sharedSource, packages, root, goos); !got["swift/Barcode1D"] {
			t.Fatalf("shared source change did not affect %s: %#v", goos, got)
		}
	}
}

func TestChangedPackageRootsIncludeDeletedPlatformOverrideOnItsPlatform(t *testing.T) {
	root := t.TempDir()
	pkgPath := filepath.Join(root, "code", "packages", "elixir", "atbash_cipher")
	if err := os.MkdirAll(pkgPath, 0o755); err != nil {
		t.Fatal(err)
	}
	if err := os.WriteFile(filepath.Join(pkgPath, "BUILD"), []byte("mix test\n"), 0o644); err != nil {
		t.Fatal(err)
	}
	packages := []discovery.Package{{
		Name:     "elixir/atbash_cipher",
		Path:     pkgPath,
		Language: "elixir",
	}}
	deletedWindowsOverride := []string{
		"code/packages/elixir/atbash_cipher/BUILD_windows",
	}

	if got := changedPackageRootsForPlatform(deletedWindowsOverride, packages, root, "linux"); len(got) != 0 {
		t.Fatalf("deleted Windows override affected Linux: %#v", got)
	}
	if got := changedPackageRootsForPlatform(deletedWindowsOverride, packages, root, "windows"); !got["elixir/atbash_cipher"] {
		t.Fatalf("deleted Windows override did not affect Windows fallback: %#v", got)
	}
}
