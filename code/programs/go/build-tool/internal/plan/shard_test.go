package plan

import (
	"strings"
	"testing"
)

func TestComputePlatformShardsIncludesPlatformOnlyWorkAndPrerequisites(t *testing.T) {
	bp := &BuildPlan{
		Packages: []PackageEntry{
			{Name: "swift/portable", Language: "swift", BuildCommands: []string{"swift test"}},
			{Name: "swift/windows-host", Language: "swift", BuildCommands: []string{"swift test"}},
			{Name: "rust/native", Language: "rust", BuildCommands: []string{"cargo test"}},
		},
		AffectedPackages: []string{"swift/portable"},
		PlatformOverrides: map[string]PlatformState{
			"linux": {
				AffectedPackages: []string{"swift/portable"},
			},
			"windows": {
				AffectedPackages: []string{"swift/windows-host"},
				DependencyEdges:  [][2]string{{"rust/native", "swift/windows-host"}},
			},
		},
	}

	shards := ComputePlatformShards(bp, 2)
	assigned := map[string]bool{}
	windowsHasNative := false
	for _, shard := range shards {
		for _, name := range shard.AssignedPackages {
			assigned[name] = true
		}
		containsWindows := false
		containsNative := false
		for _, name := range shard.PackageNames {
			containsWindows = containsWindows || name == "swift/windows-host"
			containsNative = containsNative || name == "rust/native"
		}
		windowsHasNative = windowsHasNative || (containsWindows && containsNative)
	}
	if !assigned["swift/portable"] || !assigned["swift/windows-host"] {
		t.Fatalf("platform union omitted assigned work: %#v", assigned)
	}
	if !windowsHasNative {
		t.Fatalf("Windows work was not co-located with its prerequisite: %#v", shards)
	}
}

func TestComputePlatformShardsFallsBackForOldPlan(t *testing.T) {
	bp := &BuildPlan{
		Packages:         []PackageEntry{{Name: "go/demo", Language: "go"}},
		AffectedPackages: []string{"go/demo"},
	}
	want := ComputeShards(bp, 1)
	got := ComputePlatformShards(bp, 1)
	if len(got) != len(want) || len(got) != 1 || got[0].Name != want[0].Name {
		t.Fatalf("old-plan fallback differs: got=%#v want=%#v", got, want)
	}
}

func TestComputeShardsUsesOCamlToolchainCostAndPrerequisiteClosure(t *testing.T) {
	bp := &BuildPlan{
		Packages: []PackageEntry{
			{Name: "ocaml/graph", Language: "ocaml", BuildCommands: []string{"dune build", "dune runtest"}},
			{Name: "ocaml/app", Language: "ocaml", BuildCommands: []string{"dune build"}},
		},
		DependencyEdges:  [][2]string{{"ocaml/graph", "ocaml/app"}},
		AffectedPackages: []string{"ocaml/app"},
	}

	shards := ComputeShards(bp, 1)
	if len(shards) != 1 {
		t.Fatalf("expected one shard, got %#v", shards)
	}
	shard := shards[0]
	if !shard.LanguagesNeeded["ocaml"] {
		t.Fatalf("expected OCaml toolchain in shard: %#v", shard.LanguagesNeeded)
	}
	if len(shard.PackageNames) != 2 || shard.PackageNames[0] != "ocaml/app" || shard.PackageNames[1] != "ocaml/graph" {
		t.Fatalf("expected dependent and prerequisite closure, got %#v", shard.PackageNames)
	}
	// OCaml uses compiler/package-manager weight 4: graph 1+2+4, app 1+1+4.
	if shard.EstimatedCost != 13 {
		t.Fatalf("unexpected OCaml shard cost: got %d want 13", shard.EstimatedCost)
	}
}

func TestPackageCostWeighsTarpaulinPackagesHigher(t *testing.T) {
	plain := PackageEntry{Name: "rust/plain", Language: "rust", BuildCommands: []string{"cargo test"}}
	tarpaulin := PackageEntry{Name: "rust/covered", Language: "rust", BuildCommands: []string{"cargo tarpaulin --out Xml"}}

	plainCost := packageCost(plain)
	tarpaulinCost := packageCost(tarpaulin)

	// 1 (base) + 1 (single build command) + 6 (rust toolchain weight) = 8.
	if plainCost != 8 {
		t.Fatalf("unexpected plain rust package cost: got %d want 8", plainCost)
	}
	// Same shape as plain, plus the +10 tarpaulin weight.
	if tarpaulinCost != plainCost+10 {
		t.Fatalf("tarpaulin package not weighted higher: got %d want %d", tarpaulinCost, plainCost+10)
	}
}

func TestComputeShardsSpreadsTarpaulinPackagesAcrossShards(t *testing.T) {
	bp := &BuildPlan{
		Packages: []PackageEntry{
			{Name: "rust/tarp-1", Language: "rust", BuildCommands: []string{"cargo tarpaulin"}},
			{Name: "rust/tarp-2", Language: "rust", BuildCommands: []string{"cargo tarpaulin"}},
			{Name: "rust/tarp-3", Language: "rust", BuildCommands: []string{"cargo tarpaulin"}},
			{Name: "rust/tarp-4", Language: "rust", BuildCommands: []string{"cargo tarpaulin"}},
			{Name: "python/cheap-1", Language: "python", BuildCommands: []string{"pytest"}},
			{Name: "python/cheap-2", Language: "python", BuildCommands: []string{"pytest"}},
			{Name: "python/cheap-3", Language: "python", BuildCommands: []string{"pytest"}},
			{Name: "python/cheap-4", Language: "python", BuildCommands: []string{"pytest"}},
		},
		AffectedPackages: []string{
			"rust/tarp-1", "rust/tarp-2", "rust/tarp-3", "rust/tarp-4",
			"python/cheap-1", "python/cheap-2", "python/cheap-3", "python/cheap-4",
		},
	}

	shards := ComputeShards(bp, 4)
	if len(shards) != 4 {
		t.Fatalf("expected 4 shards, got %d: %#v", len(shards), shards)
	}

	for _, shard := range shards {
		tarpaulinCount := 0
		for _, name := range shard.PackageNames {
			if strings.HasPrefix(name, "rust/tarp-") {
				tarpaulinCount++
			}
		}
		if tarpaulinCount != 1 {
			t.Fatalf("expected exactly one tarpaulin package per shard, got %d in %s: %#v",
				tarpaulinCount, shard.Name, shard.PackageNames)
		}
		// 18 (tarpaulin-weighted rust package) + 4 (python package) = 22,
		// identical for every shard: the extra tarpaulin weight lets the
		// greedy balancer reach a perfectly even split here.
		if shard.EstimatedCost != 22 {
			t.Fatalf("expected balanced shard cost 22, got %d for %s", shard.EstimatedCost, shard.Name)
		}
	}
}
