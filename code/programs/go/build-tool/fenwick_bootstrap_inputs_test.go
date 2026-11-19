package main

import (
	"os"
	"path/filepath"
	"strings"
	"testing"

	"github.com/adhithyan15/coding-adventures/code/programs/go/build-tool/internal/discovery"
	"github.com/adhithyan15/coding-adventures/code/programs/go/build-tool/internal/resolver"
)

// TestFenwickBootstrapBuildEdgesUsePlatformSelectedFronts reads the real
// Fenwick and test-bootstrap manifests plus the selected BUILD front for each
// platform. The two-node graph proves both recipe edges and the downstream
// selection caused by a test-bootstrap engine-source change.
func TestFenwickBootstrapBuildEdgesUsePlatformSelectedFronts(t *testing.T) {
	root := toolchainFixtureRepoRoot(t)
	const fenwickName = "python/fenwick-tree"
	const bootstrapName = "python/test-bootstrap"
	const annotation = "# build-tool: deps=python/test-bootstrap"
	const bootstrapSource = "code/packages/python/test-bootstrap/src/coding_adventures_test_bootstrap/pbt.py"

	for _, platform := range []string{"linux", "windows"} {
		t.Run(platform, func(t *testing.T) {
			fenwick := fenwickPackageForPlatform(t, root, platform)
			bootstrap := pythonPackage(t, root, bootstrapName)
			if platform == "windows" && filepath.Base(discovery.GetBuildFileForPlatform(fenwick.Path, platform)) != "BUILD_windows" {
				t.Fatal("Windows graph did not select BUILD_windows")
			}
			if platform == "linux" && filepath.Base(discovery.GetBuildFileForPlatform(fenwick.Path, platform)) != "BUILD" {
				t.Fatal("Linux graph did not select BUILD")
			}

			packages := []discovery.Package{bootstrap, fenwick}
			graph, triggers, err := resolver.ResolveWithTestTriggers(packages)
			if err != nil {
				t.Fatalf("resolve real %s BUILD and manifest contents: %v", platform, err)
			}
			if !graph.HasEdge(bootstrapName, fenwickName) {
				t.Fatalf("%s graph is missing %s -> %s", platform, bootstrapName, fenwickName)
			}
			changed, err := changedPackageRootsForPlatformAndLanguage(
				[]string{bootstrapSource}, packages, root, platform, "python",
			)
			if err != nil {
				t.Fatalf("resolve %s engine-source change: %v", platform, err)
			}
			affected := affectedForGraphWithTriggers(graph, triggers, changed, nil, false)
			if !affected[fenwickName] {
				t.Fatalf("%s engine-source change selected %v, want %s", platform, affected, fenwickName)
			}

			// Mutating only the selected BUILD annotation must remove the edge
			// and therefore stop the engine-source change from selecting Fenwick.
			mutated := append([]discovery.Package(nil), packages...)
			for index := range mutated {
				if mutated[index].Name != fenwickName {
					continue
				}
				if !strings.Contains(mutated[index].BuildContent, annotation) {
					t.Fatalf("selected %s BUILD front has no dependency annotation", platform)
				}
				mutated[index].BuildContent = strings.Replace(mutated[index].BuildContent, annotation+"\n", "", 1)
				mutated[index].BuildCommands = strings.Split(mutated[index].BuildContent, "\n")
			}
			withoutGraph, withoutTriggers, err := resolver.ResolveWithTestTriggers(mutated)
			if err != nil {
				t.Fatalf("resolve %s graph without annotation: %v", platform, err)
			}
			if withoutGraph.HasEdge(bootstrapName, fenwickName) {
				t.Fatalf("removing the %s annotation retained the dependency edge", platform)
			}
			withoutAffected := affectedForGraphWithTriggers(withoutGraph, withoutTriggers, changed, nil, false)
			if withoutAffected[fenwickName] {
				t.Fatalf("removing the %s annotation still selected Fenwick: %v", platform, withoutAffected)
			}
		})
	}
}

func fenwickPackageForPlatform(t *testing.T, root, platform string) discovery.Package {
	t.Helper()
	packageRoot := filepath.Join(root, "code", "packages", "python", "fenwick-tree")
	return pythonPackageForBuildFile(t, "python/fenwick-tree", packageRoot, discovery.GetBuildFileForPlatform(packageRoot, platform))
}

func pythonPackage(t *testing.T, root, name string) discovery.Package {
	t.Helper()
	packageRoot := filepath.Join(root, "code", "packages", "python", strings.TrimPrefix(name, "python/"))
	buildFile := filepath.Join(packageRoot, "BUILD")
	return pythonPackageForBuildFile(t, name, packageRoot, buildFile)
}

func pythonPackageForBuildFile(t *testing.T, name, packageRoot, buildFile string) discovery.Package {
	t.Helper()
	content, err := os.ReadFile(buildFile)
	if err != nil {
		t.Fatalf("read selected build front %s: %v", buildFile, err)
	}
	return discovery.Package{
		Name:          name,
		Path:          packageRoot,
		Language:      "python",
		BuildContent:  string(content),
		BuildCommands: strings.Split(string(content), "\n"),
	}
}
