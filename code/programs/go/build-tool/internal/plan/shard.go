package plan

import (
	"fmt"
	"sort"
)

// ComputeShards splits a build plan into prerequisite-closed shards.
//
// The sharder assigns the packages that actually need work across up to
// shardCount buckets using a stable greedy cost heuristic. Each bucket is then
// expanded with transitive prerequisites so a CI runner can execute the shard
// without consuming build artifacts from another runner.
func ComputeShards(bp *BuildPlan, shardCount int) []ShardEntry {
	if shardCount < 1 {
		shardCount = 1
	}

	pkgByName := make(map[string]PackageEntry, len(bp.Packages))
	for _, pkg := range bp.Packages {
		pkgByName[pkg.Name] = pkg
	}

	roots := scheduledPackages(bp)
	if len(roots) == 0 {
		return []ShardEntry{{
			Index:            0,
			Name:             "shard-1-of-1",
			AssignedPackages: []string{},
			PackageNames:     []string{},
			LanguagesNeeded:  map[string]bool{},
			EstimatedCost:    0,
		}}
	}

	if shardCount > len(roots) {
		shardCount = len(roots)
	}

	sort.SliceStable(roots, func(i, j int) bool {
		left := packageCost(pkgByName[roots[i]])
		right := packageCost(pkgByName[roots[j]])
		if left == right {
			return roots[i] < roots[j]
		}
		return left > right
	})

	assignments := make([][]string, shardCount)
	costs := make([]int, shardCount)
	for _, name := range roots {
		idx := lightestShard(costs)
		assignments[idx] = append(assignments[idx], name)
		costs[idx] += packageCost(pkgByName[name])
	}

	preds := predecessorMap(bp.DependencyEdges)
	shards := make([]ShardEntry, 0, shardCount)
	for _, assigned := range assignments {
		if len(assigned) == 0 {
			continue
		}

		sort.Strings(assigned)
		packageSet := make(map[string]bool)
		for _, name := range assigned {
			addWithPrereqs(name, packageSet, preds, pkgByName)
		}

		packages := sortedKeys(packageSet)
		languages := languagesFor(packages, pkgByName)
		index := len(shards)
		shards = append(shards, ShardEntry{
			Index:            index,
			Name:             fmt.Sprintf("shard-%d-of-%d", index+1, shardCount),
			AssignedPackages: append([]string(nil), assigned...),
			PackageNames:     packages,
			LanguagesNeeded:  languages,
			EstimatedCost:    shardCost(packages, pkgByName),
		})
	}

	for i := range shards {
		shards[i].Name = fmt.Sprintf("shard-%d-of-%d", i+1, len(shards))
	}

	return shards
}

// MatrixEntries returns compact shard records suitable for a GitHub Actions
// matrix axis.
func MatrixEntries(shards []ShardEntry) []ShardMatrixEntry {
	entries := make([]ShardMatrixEntry, 0, len(shards))
	for _, shard := range shards {
		entries = append(entries, ShardMatrixEntry{
			ShardIndex:   shard.Index,
			ShardCount:   len(shards),
			Label:        shard.Name,
			PackageCount: len(shard.PackageNames),
			Languages:    sortedEnabled(shard.LanguagesNeeded),
		})
	}
	return entries
}

// FindShard returns the shard with the requested index.
func FindShard(shards []ShardEntry, index int) (ShardEntry, bool) {
	for _, shard := range shards {
		if shard.Index == index {
			return shard, true
		}
	}
	return ShardEntry{}, false
}

func scheduledPackages(bp *BuildPlan) []string {
	if bp.AffectedPackages == nil {
		names := make([]string, 0, len(bp.Packages))
		for _, pkg := range bp.Packages {
			names = append(names, pkg.Name)
		}
		sort.Strings(names)
		return names
	}

	names := append([]string(nil), bp.AffectedPackages...)
	sort.Strings(names)
	return names
}

func lightestShard(costs []int) int {
	best := 0
	for i := 1; i < len(costs); i++ {
		if costs[i] < costs[best] {
			best = i
		}
	}
	return best
}

func predecessorMap(edges [][2]string) map[string][]string {
	preds := make(map[string][]string)
	for _, edge := range edges {
		from, to := edge[0], edge[1]
		preds[to] = append(preds[to], from)
	}
	for name := range preds {
		sort.Strings(preds[name])
	}
	return preds
}

func addWithPrereqs(
	name string,
	packageSet map[string]bool,
	preds map[string][]string,
	pkgByName map[string]PackageEntry,
) {
	if packageSet[name] {
		return
	}
	if _, ok := pkgByName[name]; !ok {
		return
	}

	packageSet[name] = true
	for _, pred := range preds[name] {
		addWithPrereqs(pred, packageSet, preds, pkgByName)
	}
}

func sortedKeys(values map[string]bool) []string {
	keys := make([]string, 0, len(values))
	for key := range values {
		keys = append(keys, key)
	}
	sort.Strings(keys)
	return keys
}

func languagesFor(packageNames []string, pkgByName map[string]PackageEntry) map[string]bool {
	languages := make(map[string]bool)
	for _, name := range packageNames {
		pkg, ok := pkgByName[name]
		if !ok {
			continue
		}
		languages[toolchainForLanguage(pkg.Language)] = true
	}
	return languages
}

func sortedEnabled(values map[string]bool) []string {
	enabled := make([]string, 0, len(values))
	for name, value := range values {
		if value {
			enabled = append(enabled, name)
		}
	}
	sort.Strings(enabled)
	return enabled
}

func shardCost(packageNames []string, pkgByName map[string]PackageEntry) int {
	total := 0
	for _, name := range packageNames {
		total += packageCost(pkgByName[name])
	}
	return total
}

func packageCost(pkg PackageEntry) int {
	cost := 1 + len(pkg.BuildCommands)
	switch toolchainForLanguage(pkg.Language) {
	case "rust":
		cost += 6
	case "dotnet", "haskell", "swift", "typescript":
		cost += 4
	case "java", "kotlin":
		cost += 3
	case "elixir", "python", "ruby":
		cost += 2
	}
	return cost
}

func toolchainForLanguage(language string) string {
	switch language {
	case "wasm":
		return "rust"
	case "csharp", "fsharp", "dotnet":
		return "dotnet"
	default:
		return language
	}
}
