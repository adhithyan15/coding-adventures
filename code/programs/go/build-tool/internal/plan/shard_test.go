package plan

import "testing"

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
