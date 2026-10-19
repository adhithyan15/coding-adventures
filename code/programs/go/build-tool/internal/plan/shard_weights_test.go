package plan

import (
	"os"
	"path/filepath"
	"reflect"
	"strings"
	"testing"
)

// sharedPrereqPlan is the shape that made v1 expensive on main: one costly
// prerequisite (`rust/catalog`, think smart-home-integration-catalog) with
// several cheap dependents, plus unrelated cheap work to balance against.
func sharedPrereqPlan() *BuildPlan {
	return &BuildPlan{
		Packages: []PackageEntry{
			{Name: "rust/catalog", Language: "rust", BuildCommands: []string{"cargo test"}},
			{Name: "rust/hue", Language: "rust", BuildCommands: []string{"cargo test"}},
			{Name: "rust/axis", Language: "rust", BuildCommands: []string{"cargo test"}},
			{Name: "rust/onvif", Language: "rust", BuildCommands: []string{"cargo test"}},
			{Name: "python/a", Language: "python", BuildCommands: []string{"pytest"}},
			{Name: "python/b", Language: "python", BuildCommands: []string{"pytest"}},
			{Name: "python/c", Language: "python", BuildCommands: []string{"pytest"}},
		},
		DependencyEdges: [][2]string{
			{"rust/catalog", "rust/hue"},
			{"rust/catalog", "rust/axis"},
			{"rust/catalog", "rust/onvif"},
		},
	}
}

func sharedPrereqWeights() *ShardWeights {
	return &ShardWeights{
		Schema: ShardWeightsSchema,
		Unit:   "seconds",
		Durations: map[string]float64{
			"rust/catalog": 1200,
			"rust/hue":     30,
			"rust/axis":    30,
			"rust/onvif":   30,
			"python/a":     400,
			"python/b":     400,
			"python/c":     400,
		},
	}
}

func shardContaining(t *testing.T, shards []ShardEntry, name string) []int {
	t.Helper()
	var found []int
	for _, shard := range shards {
		for _, member := range shard.PackageNames {
			if member == name {
				found = append(found, shard.Index)
			}
		}
	}
	return found
}

func TestWeightedShardsBuildASharedPrerequisiteOnce(t *testing.T) {
	bp := sharedPrereqPlan()

	// v1 scatters the three dependents and copies the catalog after each.
	if v1 := shardContaining(t, ComputeShards(bp, 2), "rust/catalog"); len(v1) != 2 {
		t.Fatalf("precondition: v1 should duplicate the catalog across both shards, got %v", v1)
	}

	shards := ComputeWeightedShards(bp, 2, sharedPrereqWeights())
	if got := shardContaining(t, shards, "rust/catalog"); len(got) != 1 {
		t.Fatalf("v2 built the 1200s catalog on shards %v, want exactly one", got)
	}
	// The catalog cluster is 1290s; the three python packages are 1200s.
	// Least-resulting-load puts them on opposite shards.
	costs := []int{shards[0].EstimatedCost, shards[1].EstimatedCost}
	if !reflect.DeepEqual(costs, []int{1290, 1200}) {
		t.Fatalf("unexpected shard costs %v: %#v", costs, shards)
	}
}

func TestWeightedShardsAssignEveryScheduledPackageExactlyOnceAndStayClosed(t *testing.T) {
	bp := sharedPrereqPlan()
	shards := ComputeWeightedShards(bp, 3, sharedPrereqWeights())

	assigned := map[string]int{}
	for _, shard := range shards {
		members := map[string]bool{}
		for _, name := range shard.PackageNames {
			members[name] = true
		}
		for _, name := range shard.AssignedPackages {
			assigned[name]++
			if !members[name] {
				t.Fatalf("%s assigned to %s but not in its package list", name, shard.Name)
			}
		}
		for _, edge := range bp.DependencyEdges {
			if members[edge[1]] && !members[edge[0]] {
				t.Fatalf("%s holds %s without its prerequisite %s", shard.Name, edge[1], edge[0])
			}
		}
	}
	for _, pkg := range bp.Packages {
		if assigned[pkg.Name] != 1 {
			t.Fatalf("%s assigned %d times, want 1 (%v)", pkg.Name, assigned[pkg.Name], assigned)
		}
	}
}

func TestWeightedShardsDuplicateWhenThatBalancesBetter(t *testing.T) {
	// A cheap shared prerequisite under two heavy dependents: copying the 1s
	// prerequisite is far better than stacking 2x1000s on one runner.
	bp := &BuildPlan{
		Packages: []PackageEntry{
			{Name: "go/base", Language: "go"},
			{Name: "go/left", Language: "go"},
			{Name: "go/right", Language: "go"},
		},
		DependencyEdges: [][2]string{{"go/base", "go/left"}, {"go/base", "go/right"}},
	}
	weights := &ShardWeights{Schema: ShardWeightsSchema, Unit: "seconds", Durations: map[string]float64{
		"go/base": 1, "go/left": 1000, "go/right": 1000,
	}}
	shards := ComputeWeightedShards(bp, 2, weights)
	if got := shardContaining(t, shards, "go/base"); len(got) != 2 {
		t.Fatalf("expected the cheap prerequisite on both shards, got %v: %#v", got, shards)
	}
	if shards[0].EstimatedCost != 1001 || shards[1].EstimatedCost != 1001 {
		t.Fatalf("expected two 1001s shards, got %#v", shards)
	}
}

func TestWeightedShardsAreDeterministic(t *testing.T) {
	first := ComputeWeightedShards(sharedPrereqPlan(), 3, sharedPrereqWeights())
	for i := 0; i < 20; i++ {
		again := ComputeWeightedShards(sharedPrereqPlan(), 3, sharedPrereqWeights())
		if !reflect.DeepEqual(first, again) {
			t.Fatalf("run %d differs:\n%#v\n%#v", i, first, again)
		}
	}
	for i, shard := range first {
		if shard.Index != i || shard.Name != []string{"shard-1-of-3", "shard-2-of-3", "shard-3-of-3"}[i] {
			t.Fatalf("unstable index/name at %d: %#v", i, shard)
		}
	}
}

func TestWeightedShardsRespectTheAffectedSetAndClampTheCount(t *testing.T) {
	bp := sharedPrereqPlan()
	bp.AffectedPackages = []string{"rust/hue"}
	shards := ComputeWeightedShards(bp, 5, sharedPrereqWeights())
	if len(shards) != 1 {
		t.Fatalf("one scheduled root must clamp to one shard, got %#v", shards)
	}
	if !reflect.DeepEqual(shards[0].AssignedPackages, []string{"rust/hue"}) ||
		!reflect.DeepEqual(shards[0].PackageNames, []string{"rust/catalog", "rust/hue"}) {
		t.Fatalf("unexpected shard %#v", shards[0])
	}
	if !shards[0].LanguagesNeeded["rust"] || len(shards[0].LanguagesNeeded) != 1 {
		t.Fatalf("toolchains must come from the closed shard: %#v", shards[0].LanguagesNeeded)
	}
}

func TestWeightedShardsWithNothingScheduledMatchV1(t *testing.T) {
	bp := sharedPrereqPlan()
	bp.AffectedPackages = []string{}
	got := ComputeWeightedShards(bp, 4, sharedPrereqWeights())
	if !reflect.DeepEqual(got, ComputeShards(bp, 4)) {
		t.Fatalf("empty selection should be v1's single empty shard, got %#v", got)
	}
}

func TestNilWeightsKeepShardingV1(t *testing.T) {
	bp := sharedPrereqPlan()
	if !reflect.DeepEqual(ComputePlatformShardsWithWeights(bp, 2, nil), ComputePlatformShards(bp, 2)) {
		t.Fatal("nil weights must be byte-for-byte sharding v1 (the conformance fixtures pin it)")
	}
	if ComputePlatformShardsWithWeights(nil, 2, sharedPrereqWeights()) != nil {
		t.Fatal("nil plan must yield nil shards")
	}
}

func TestWeightedPlatformShardsColocatePlatformOnlyWork(t *testing.T) {
	bp := &BuildPlan{
		Packages: []PackageEntry{
			{Name: "swift/portable", Language: "swift", BuildCommands: []string{"swift test"}},
			{Name: "swift/windows-host", Language: "swift", BuildCommands: []string{"swift test"}},
			{Name: "rust/native", Language: "rust", BuildCommands: []string{"cargo test"}},
		},
		AffectedPackages: []string{"swift/portable"},
		PlatformOverrides: map[string]PlatformState{
			"windows": {
				AffectedPackages: []string{"swift/windows-host"},
				DependencyEdges:  [][2]string{{"rust/native", "swift/windows-host"}},
			},
		},
	}
	weights := &ShardWeights{Schema: ShardWeightsSchema, Unit: "seconds", Durations: map[string]float64{"swift/portable": 5}}
	shards := ComputePlatformShardsWithWeights(bp, 2, weights)
	host := shardContaining(t, shards, "swift/windows-host")
	native := shardContaining(t, shards, "rust/native")
	if len(host) != 1 || !reflect.DeepEqual(host, native) {
		t.Fatalf("windows-only work must sit with its prerequisite: host=%v native=%v %#v", host, native, shards)
	}
}

func TestResolveWeightsFallbackLadder(t *testing.T) {
	packages := []PackageEntry{
		{Name: "rust/a", Language: "rust", BuildCommands: []string{"cargo test"}},   // cost 8
		{Name: "rust/b", Language: "rust", BuildCommands: []string{"cargo test"}},   // cost 8
		{Name: "rust/c", Language: "rust", BuildCommands: []string{"cargo test"}},   // cost 8
		{Name: "rust/new", Language: "rust", BuildCommands: []string{"cargo test"}}, // unmeasured
		{Name: "wasm/new", Language: "wasm"},                                        // unmeasured, rust toolchain
		{Name: "go/new", Language: "go", BuildCommands: []string{"go test"}},        // unmeasured toolchain, cost 2
		{Name: "c/instant", Language: "c"},
	}
	weights := &ShardWeights{Durations: map[string]float64{
		"rust/a": 8, "rust/b": 16, "rust/c": 80, "c/instant": 0,
	}}
	got := resolveWeights(packages, weights)

	// rust/new and wasm/new: rust median of {8, 16, 80} = 16.
	// go/new: heuristic cost 2 x median seconds-per-cost of {1, 2, 10, 0} = 1.5.
	// c/instant: measured 0, floored to minPackageWeight.
	want := []float64{8, 16, 80, 16, 16, 3, minPackageWeight}
	if !reflect.DeepEqual(got, want) {
		t.Fatalf("resolveWeights = %v, want %v", got, want)
	}
}

func TestResolveWeightsWithNoOverlapUsesHeuristicUnits(t *testing.T) {
	packages := []PackageEntry{{Name: "go/x", Language: "go", BuildCommands: []string{"go test"}}}
	got := resolveWeights(packages, &ShardWeights{Durations: map[string]float64{"elsewhere/pkg": 9}})
	if !reflect.DeepEqual(got, []float64{2}) {
		t.Fatalf("expected raw heuristic cost 2, got %v", got)
	}
}

func TestClosureIndexIgnoresUnknownEdgesAndHandlesDiamonds(t *testing.T) {
	bp := &BuildPlan{
		Packages: []PackageEntry{{Name: "a"}, {Name: "b"}, {Name: "c"}, {Name: "d"}},
		DependencyEdges: [][2]string{
			{"a", "b"}, {"a", "c"}, {"b", "d"}, {"c", "d"}, {"ghost", "d"},
		},
	}
	index := map[string]int{"a": 0, "b": 1, "c": 2, "d": 3}
	closures := closureIndex(bp, index)
	if got := closures(3); !reflect.DeepEqual(got, []int{0, 1, 2, 3}) {
		t.Fatalf("closure(d) = %v", got)
	}
	if got := closures(0); !reflect.DeepEqual(got, []int{0}) {
		t.Fatalf("closure(a) = %v", got)
	}
	// Memoised: same backing slice the second time.
	if first, second := closures(3), closures(3); &first[0] != &second[0] {
		t.Fatal("closure was recomputed")
	}
}

func TestMedian(t *testing.T) {
	for _, tc := range []struct {
		in   []float64
		want float64
	}{
		{nil, 0},
		{[]float64{3}, 3},
		{[]float64{9, 1, 5}, 5},
		{[]float64{4, 1, 3, 2}, 2.5},
	} {
		if got := median(tc.in); got != tc.want {
			t.Fatalf("median(%v) = %v, want %v", tc.in, got, tc.want)
		}
	}
}

func writeWeights(t *testing.T, body string) string {
	t.Helper()
	path := filepath.Join(t.TempDir(), "durations.json")
	if err := os.WriteFile(path, []byte(body), 0o600); err != nil {
		t.Fatal(err)
	}
	return path
}

func TestLoadShardWeights(t *testing.T) {
	good := writeWeights(t, `{"schema":"ci-package-durations-v1","unit":"seconds","source":{"runs":[1]},"durations":{"go/x":1.5}}`)
	weights, err := LoadShardWeights(good)
	if err != nil {
		t.Fatalf("valid file rejected: %v", err)
	}
	if weights.Durations["go/x"] != 1.5 {
		t.Fatalf("unexpected durations %#v", weights.Durations)
	}

	for name, tc := range map[string]struct {
		path string
		code string
	}{
		"missing":   {filepath.Join(t.TempDir(), "absent.json"), "SHARD_WEIGHTS_UNREADABLE"},
		"not json":  {writeWeights(t, `{`), "SHARD_WEIGHTS_INVALID"},
		"schema":    {writeWeights(t, `{"schema":"v0","unit":"seconds","durations":{"a":1}}`), "schema"},
		"unit":      {writeWeights(t, `{"schema":"ci-package-durations-v1","unit":"ms","durations":{"a":1}}`), "unit"},
		"empty":     {writeWeights(t, `{"schema":"ci-package-durations-v1","unit":"seconds","durations":{}}`), "no durations"},
		"negative":  {writeWeights(t, `{"schema":"ci-package-durations-v1","unit":"seconds","durations":{"a":-1}}`), "a has duration -1"},
		"wrongtype": {writeWeights(t, `{"schema":"ci-package-durations-v1","unit":"seconds","durations":{"a":"1"}}`), "SHARD_WEIGHTS_INVALID"},
	} {
		if _, err := LoadShardWeights(tc.path); err == nil || !strings.Contains(err.Error(), tc.code) {
			t.Fatalf("%s: got %v, want error containing %q", name, err, tc.code)
		}
	}
}

func TestPlatformShardPlanTreatsANullOverrideAsEverything(t *testing.T) {
	if ComputePlatformShards(nil, 2) != nil {
		t.Fatal("nil plan must yield nil shards")
	}
	bp := sharedPrereqPlan()
	bp.AffectedPackages = []string{"rust/hue"}
	bp.PlatformOverrides = map[string]PlatformState{
		// A platform whose affected set is null rebuilds everything, so the
		// union must schedule everything too.
		"darwin": {AffectedPackages: nil, DependencyEdges: [][2]string{{"python/a", "python/b"}}},
	}
	union := platformShardPlan(bp)
	if union.AffectedPackages != nil {
		t.Fatalf("null platform state must widen the union to everything, got %v", union.AffectedPackages)
	}
	if len(union.DependencyEdges) != len(bp.DependencyEdges)+1 {
		t.Fatalf("platform edges were not merged: %v", union.DependencyEdges)
	}
	assigned := 0
	for _, shard := range ComputePlatformShardsWithWeights(bp, 3, sharedPrereqWeights()) {
		assigned += len(shard.AssignedPackages)
	}
	if assigned != len(bp.Packages) {
		t.Fatalf("expected every package scheduled, got %d of %d", assigned, len(bp.Packages))
	}
}
