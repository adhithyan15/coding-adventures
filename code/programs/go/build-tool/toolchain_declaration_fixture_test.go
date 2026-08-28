package main

import (
	"encoding/json"
	"maps"
	"os"
	"path/filepath"
	"runtime"
	"slices"
	"strings"
	"testing"

	"github.com/adhithyan15/coding-adventures/code/programs/go/build-tool/internal/discovery"
)

type neutralToolchainFixture struct {
	ID    string `json:"id"`
	Input struct {
		Options struct {
			Platform  string `json:"platform"`
			ForceFull bool   `json:"force_full"`
			Packages  []struct {
				Name       string            `json:"name"`
				Language   string            `json:"language"`
				BuildFiles map[string]string `json:"build_files"`
			} `json:"packages"`
			ScheduledPackages *[]string `json:"scheduled_packages"`
			ForcedToolchains  []string  `json:"forced_toolchains"`
		} `json:"options"`
	} `json:"input"`
	Expected struct {
		Outcome string `json:"outcome"`
		Result  struct {
			Toolchains map[string]bool `json:"toolchains"`
		} `json:"result"`
	} `json:"expected"`
}

func toolchainFixtureRepoRoot(t *testing.T) string {
	t.Helper()
	_, sourceFile, _, ok := runtime.Caller(0)
	if !ok {
		t.Fatal("could not locate toolchain fixture test source")
	}
	return filepath.Clean(filepath.Join(filepath.Dir(sourceFile), "..", "..", "..", ".."))
}

func loadNeutralToolchainFixtures(t *testing.T) []neutralToolchainFixture {
	t.Helper()
	repoRoot := toolchainFixtureRepoRoot(t)
	paths, err := filepath.Glob(filepath.Join(
		repoRoot, "code", "specs", "fixtures", "build-tool-v1", "cases", "toolchain-detection-*.json",
	))
	if err != nil {
		t.Fatal(err)
	}
	slices.Sort(paths)
	fixtures := make([]neutralToolchainFixture, 0, len(paths))
	for _, path := range paths {
		data, err := os.ReadFile(path)
		if err != nil {
			t.Fatal(err)
		}
		var fixture neutralToolchainFixture
		if err := json.Unmarshal(data, &fixture); err != nil {
			t.Fatalf("decode %s: %v", filepath.Base(path), err)
		}
		fixtures = append(fixtures, fixture)
	}
	return fixtures
}

func TestJavaToSemanticIrDeclaresPythonOnBothBuildFronts(t *testing.T) {
	packageRoot := filepath.Join(
		toolchainFixtureRepoRoot(t), "code", "packages", "rust", "java-to-semantic-ir",
	)
	buildFiles := make(map[string]string)
	for _, filename := range []string{"BUILD", "BUILD_windows"} {
		content, err := os.ReadFile(filepath.Join(packageRoot, filename))
		if err != nil {
			t.Fatal(err)
		}
		buildFiles[filename] = string(content)
	}
	for _, platform := range []string{"linux", "windows"} {
		if got := discovery.ExtraToolchainsForSnapshot(buildFiles, platform); !slices.Contains(got, "python") {
			t.Fatalf("%s selected front declared %v, want python", platform, got)
		}
	}
}

func TestPackagesForPlatformRefreshesExtraToolchains(t *testing.T) {
	packageRoot := t.TempDir()
	for filename, content := range map[string]string{
		"BUILD":         "# needs-toolchain: java\n",
		"BUILD_windows": "# needs-toolchain: python\n",
	} {
		if err := os.WriteFile(filepath.Join(packageRoot, filename), []byte(content), 0o600); err != nil {
			t.Fatal(err)
		}
	}

	packages := packagesForPlatform([]discovery.Package{{
		Name:            "rust/platform",
		Language:        "rust",
		Path:            packageRoot,
		ExtraToolchains: []string{"java"},
	}}, "windows")
	if len(packages) != 1 || !slices.Equal(packages[0].ExtraToolchains, []string{"python"}) {
		t.Fatalf("platform package declarations = %v, want [python]", packages)
	}
	needed := computeLanguagesNeeded(
		packages,
		map[string]bool{"rust/platform": true},
		false,
		map[string]bool{},
	)
	if !needed["rust"] || !needed["python"] || needed["java"] {
		t.Fatalf("platform toolchains = %v, want rust/python without java", needed)
	}
}

func TestComputeLanguagesNeededConsumesSuccessfulNeutralToolchainFixtures(t *testing.T) {
	fixtures := loadNeutralToolchainFixtures(t)
	if len(fixtures) < 10 {
		t.Fatalf("expected at least 10 toolchain fixtures, got %d", len(fixtures))
	}
	for _, fixture := range fixtures {
		if fixture.Expected.Outcome != "ok" {
			continue
		}
		t.Run(strings.TrimPrefix(fixture.ID, "toolchain-detection/"), func(t *testing.T) {
			packages := make([]discovery.Package, 0, len(fixture.Input.Options.Packages))
			for _, packageFixture := range fixture.Input.Options.Packages {
				packages = append(packages, discovery.Package{
					Name:            packageFixture.Name,
					Language:        packageFixture.Language,
					ExtraToolchains: discovery.ExtraToolchainsForSnapshot(packageFixture.BuildFiles, fixture.Input.Options.Platform),
				})
			}

			var affected map[string]bool
			if !fixture.Input.Options.ForceFull {
				affected = make(map[string]bool)
				if fixture.Input.Options.ScheduledPackages == nil {
					for _, packageFixture := range fixture.Input.Options.Packages {
						affected[packageFixture.Name] = true
					}
				} else {
					for _, name := range *fixture.Input.Options.ScheduledPackages {
						affected[name] = true
					}
				}
			}
			forced := make(map[string]bool)
			for _, toolchain := range fixture.Input.Options.ForcedToolchains {
				forced[toolchain] = true
			}
			actualSparse := computeLanguagesNeeded(packages, affected, fixture.Input.Options.ForceFull, forced)
			actual := make(map[string]bool, len(allToolchains))
			for _, toolchain := range allToolchains {
				actual[toolchain] = actualSparse[toolchain]
			}
			if !maps.Equal(actual, fixture.Expected.Result.Toolchains) {
				t.Fatalf("toolchain map mismatch\n got: %v\nwant: %v", actual, fixture.Expected.Result.Toolchains)
			}
		})
	}
}
