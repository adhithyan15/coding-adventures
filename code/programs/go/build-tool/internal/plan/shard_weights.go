package plan

// ── Locality-aware sharding with measured weights ──────────────────────
//
// Sharding v1 (ComputeShards) spreads *packages* across shards by a cost
// heuristic and only afterwards pulls each package's prerequisites in. That
// is correct, but it ignores where those prerequisites already landed. On
// the real repo (5,327 packages, 15 shards, main CI 2026-10-18) every shard
// was assigned ~355 packages and then expanded to 872-1,131, so the 15
// runners built 14,878 packages between them -- 2.8x the repository. One
// 21-minute crate, `rust/smart-home-integration-catalog`, was rebuilt on
// twelve shards in every run because its dependents were scattered.
//
// Sharding v2 assigns *closures* instead of packages. Picture fifteen
// buckets and a pile of dependency trees:
//
//	v1: deal the leaves round-robin, then copy every leaf's whole tree
//	    into whichever bucket the leaf fell into.
//	v2: take the biggest tree first, and drop it into the bucket where it
//	    adds the least *new* work -- usually the bucket that already holds
//	    most of its branches.
//
// Concretely, let the *fair share* be the weight of every package that must
// be built at least once, divided by the shard count, plus shardSlack. Then
// for every scheduled package, heaviest prerequisite closure first:
//
//  1. If some shard already contains the package (it arrived as somebody
//     else's prerequisite), assign it directly to the lightest such shard.
//     That costs nothing: the work is already scheduled there.
//  2. Otherwise compute, for every shard, the *marginal* weight -- the
//     summed weight of the closure members that shard does not have yet.
//     Among the shards that stay within the fair share after adding it,
//     pick the smallest marginal (then the lighter shard, then the lower
//     index, so the result is deterministic).
//  3. If no shard stays within the fair share, pick the shard whose load
//     after adding the marginal is smallest.
//
// Why a cap rather than plain "least resulting load": early on every shard
// is empty, and an empty shard always looks lighter than the one that
// already holds a dependent's 20-minute prerequisite. Least-resulting-load
// therefore copies that prerequisite onto a fresh shard every time a sibling
// comes along -- exactly v1's waste. The cap lets locality win until a shard
// is genuinely full, and rule 3 still balances whatever is left over.
//
// Measured by running both strategies over the plan main CI used on
// 2026-10-18 (run 39139594621: 5,327 packages, 15 shards), weighted with
// code/specs/data/ci-package-durations.json:
//
//	          package builds   heaviest shard   lightest shard   (serial minutes)
//	v1            14,878            233              134
//	v2             5,908             69               69
//
// Serial minutes are the sum of the shard's package durations -- what the
// shard costs on one worker. A real shard runs several workers, so wall time
// is lower, but v1's OCaml shards ran on exactly one (CI's old `-jobs 1`),
// which is how 178 serial minutes became a 150-minute timeout.
//
// Weights are measured seconds per package, read from a durations file the
// main CI build itself produces (see code/scripts/ci_package_durations.py).
// A package the file has not seen yet -- a brand-new one -- gets the median
// measured duration of its own toolchain, so a new Rust crate is assumed to
// cost what a typical Rust crate costs rather than what a typical C file
// costs. Only when a toolchain has no measurement at all does the v1
// heuristic stand in, scaled into seconds by the median seconds-per-cost
// unit across everything measured.

import (
	"encoding/json"
	"fmt"
	"math"
	"os"
	"sort"
)

// ShardWeightsSchema is the schema tag a durations file must carry. A file
// with a different tag is rejected rather than half-understood.
const ShardWeightsSchema = "ci-package-durations-v1"

// shardSlack is how far past an exact fair share a shard may grow to keep a
// closure together. Zero turns every near-tie into a duplicate; much more
// lets one runner carry visibly more than its siblings. 5% held the
// heaviest shard within a minute of the zero-slack optimum on the measured
// plan while letting small siblings join their prerequisites.
const shardSlack = 0.05

// minPackageWeight keeps every package strictly positive. A package that
// built in 0.0s still occupies a slot, and a zero weight would make every
// shard look equally good for it, defeating the locality preference.
const minPackageWeight = 0.1

// ShardWeights maps qualified package names to measured build seconds.
type ShardWeights struct {
	Schema    string             `json:"schema"`
	Unit      string             `json:"unit"`
	Source    json.RawMessage    `json:"source,omitempty"`
	Durations map[string]float64 `json:"durations"`
}

// LoadShardWeights reads and validates a durations file.
//
// Every failure is an error, never a silent fallback to v1: a CI that asked
// for measured balancing and quietly got the old heuristic would look
// healthy right up to the next 150-minute timeout.
func LoadShardWeights(path string) (*ShardWeights, error) {
	data, err := os.ReadFile(path)
	if err != nil {
		return nil, fmt.Errorf("SHARD_WEIGHTS_UNREADABLE: %w", err)
	}
	var weights ShardWeights
	if err := json.Unmarshal(data, &weights); err != nil {
		return nil, fmt.Errorf("SHARD_WEIGHTS_INVALID: %w", err)
	}
	if weights.Schema != ShardWeightsSchema {
		return nil, fmt.Errorf("SHARD_WEIGHTS_INVALID: schema %q, want %q", weights.Schema, ShardWeightsSchema)
	}
	if weights.Unit != "seconds" {
		return nil, fmt.Errorf("SHARD_WEIGHTS_INVALID: unit %q, want \"seconds\"", weights.Unit)
	}
	if len(weights.Durations) == 0 {
		return nil, fmt.Errorf("SHARD_WEIGHTS_INVALID: no durations")
	}
	for name, seconds := range weights.Durations {
		if math.IsNaN(seconds) || math.IsInf(seconds, 0) || seconds < 0 {
			return nil, fmt.Errorf("SHARD_WEIGHTS_INVALID: %s has duration %v", name, seconds)
		}
	}
	return &weights, nil
}

// ComputePlatformShardsWithWeights is ComputePlatformShards with an optional
// measured-weights file. A nil weights value keeps sharding v1 exactly, so
// the conformance fixtures that pin v1 are unaffected.
func ComputePlatformShardsWithWeights(bp *BuildPlan, shardCount int, weights *ShardWeights) []ShardEntry {
	if weights == nil {
		return ComputePlatformShards(bp, shardCount)
	}
	if bp == nil {
		return nil
	}
	return ComputeWeightedShards(platformShardPlan(bp), shardCount, weights)
}

// ComputeWeightedShards is sharding v2: closure-aware assignment balanced on
// measured seconds. See the comment at the top of this file.
func ComputeWeightedShards(bp *BuildPlan, shardCount int, weights *ShardWeights) []ShardEntry {
	if shardCount < 1 {
		shardCount = 1
	}

	// Index packages so closures can be bitmaps over small integers rather
	// than maps keyed by long names.
	names := make([]string, len(bp.Packages))
	index := make(map[string]int, len(bp.Packages))
	for i, pkg := range bp.Packages {
		names[i] = pkg.Name
		index[pkg.Name] = i
	}
	pkgByName := make(map[string]PackageEntry, len(bp.Packages))
	for _, pkg := range bp.Packages {
		pkgByName[pkg.Name] = pkg
	}

	roots := scheduledPackages(bp)
	if len(roots) == 0 {
		return ComputeShards(bp, shardCount)
	}
	if shardCount > len(roots) {
		shardCount = len(roots)
	}

	weight := resolveWeights(bp.Packages, weights)
	closures := closureIndex(bp, index)

	closureWeight := make(map[string]float64, len(roots))
	for _, name := range roots {
		total := 0.0
		for _, member := range closures(index[name]) {
			total += weight[member]
		}
		closureWeight[name] = total
	}
	sort.SliceStable(roots, func(i, j int) bool {
		left, right := closureWeight[roots[i]], closureWeight[roots[j]]
		if left == right {
			return roots[i] < roots[j]
		}
		return left > right
	})

	// Everything any root needs, counted once: the work that cannot be
	// avoided however the shards are cut.
	required := make([]bool, len(names))
	requiredWeight := 0.0
	for _, name := range roots {
		for _, member := range closures(index[name]) {
			if !required[member] {
				required[member] = true
				requiredWeight += weight[member]
			}
		}
	}
	fairShare := requiredWeight / float64(shardCount) * (1 + shardSlack)

	contains := make([][]bool, shardCount)
	for i := range contains {
		contains[i] = make([]bool, len(names))
	}
	loads := make([]float64, shardCount)
	assignments := make([][]string, shardCount)

	for _, name := range roots {
		id := index[name]

		// Rule 1: already scheduled somewhere as a prerequisite.
		home := -1
		for shard := 0; shard < shardCount; shard++ {
			if contains[shard][id] && (home < 0 || loads[shard] < loads[home]) {
				home = shard
			}
		}
		if home >= 0 {
			assignments[home] = append(assignments[home], name)
			continue
		}

		// Rules 2 and 3, counting only what is new to each shard.
		members := closures(id)
		best, bestFits, bestMarginal, bestLoad := -1, false, 0.0, 0.0
		for shard := 0; shard < shardCount; shard++ {
			marginal := 0.0
			for _, member := range members {
				if !contains[shard][member] {
					marginal += weight[member]
				}
			}
			resulting := loads[shard] + marginal
			fits := resulting <= fairShare
			better := false
			switch {
			case best < 0:
				better = true
			case fits != bestFits:
				better = fits
			case fits:
				better = marginal < bestMarginal || (marginal == bestMarginal && resulting < bestLoad)
			default:
				better = resulting < bestLoad
			}
			if better {
				best, bestFits, bestMarginal, bestLoad = shard, fits, marginal, resulting
			}
		}
		for _, member := range members {
			if !contains[best][member] {
				contains[best][member] = true
				loads[best] += weight[member]
			}
		}
		assignments[best] = append(assignments[best], name)
	}

	shards := make([]ShardEntry, 0, shardCount)
	for shard, assigned := range assignments {
		if len(assigned) == 0 {
			continue
		}
		sort.Strings(assigned)
		packages := make([]string, 0)
		for id, present := range contains[shard] {
			if present {
				packages = append(packages, names[id])
			}
		}
		sort.Strings(packages)
		shards = append(shards, ShardEntry{
			Index:            len(shards),
			AssignedPackages: assigned,
			PackageNames:     packages,
			LanguagesNeeded:  languagesFor(packages, pkgByName),
			EstimatedCost:    int(math.Round(loads[shard])),
		})
	}
	for i := range shards {
		shards[i].Name = fmt.Sprintf("shard-%d-of-%d", i+1, len(shards))
	}
	return shards
}

// platformShardPlan folds every platform override into one plan, the same
// union ComputePlatformShards balances over, so platform-only work and its
// prerequisites still land together.
func platformShardPlan(bp *BuildPlan) *BuildPlan {
	if len(bp.PlatformOverrides) == 0 {
		return bp
	}
	shardPlan := *bp
	affected := make(map[string]bool)
	allAffected := bp.AffectedPackages != nil
	for _, name := range bp.AffectedPackages {
		affected[name] = true
	}
	edges := make(map[[2]string]bool)
	for _, edge := range bp.DependencyEdges {
		edges[edge] = true
	}
	for _, state := range bp.PlatformOverrides {
		if state.AffectedPackages == nil {
			allAffected = false
		} else {
			for _, name := range state.AffectedPackages {
				affected[name] = true
			}
		}
		for _, edge := range state.DependencyEdges {
			edges[edge] = true
		}
	}
	if allAffected {
		shardPlan.AffectedPackages = sortedKeys(affected)
	} else {
		shardPlan.AffectedPackages = nil
	}
	shardPlan.DependencyEdges = make([][2]string, 0, len(edges))
	for edge := range edges {
		shardPlan.DependencyEdges = append(shardPlan.DependencyEdges, edge)
	}
	sort.Slice(shardPlan.DependencyEdges, func(i, j int) bool {
		if shardPlan.DependencyEdges[i][0] == shardPlan.DependencyEdges[j][0] {
			return shardPlan.DependencyEdges[i][1] < shardPlan.DependencyEdges[j][1]
		}
		return shardPlan.DependencyEdges[i][0] < shardPlan.DependencyEdges[j][0]
	})
	return &shardPlan
}

// resolveWeights turns the durations file into one weight per package,
// indexed like bp.Packages. The fallback ladder for a package with no
// measurement is: its toolchain's median, then the v1 heuristic scaled into
// seconds. Every weight is at least minPackageWeight.
func resolveWeights(packages []PackageEntry, weights *ShardWeights) []float64 {
	byToolchain := make(map[string][]float64)
	var perCostUnit []float64
	for _, pkg := range packages {
		seconds, ok := weights.Durations[pkg.Name]
		if !ok {
			continue
		}
		toolchain := toolchainForLanguage(pkg.Language)
		byToolchain[toolchain] = append(byToolchain[toolchain], seconds)
		perCostUnit = append(perCostUnit, seconds/float64(packageCost(pkg)))
	}
	toolchainMedian := make(map[string]float64, len(byToolchain))
	for toolchain, samples := range byToolchain {
		toolchainMedian[toolchain] = median(samples)
	}
	secondsPerCost := median(perCostUnit)
	if secondsPerCost <= 0 {
		// Nothing in this plan was measured: the file describes some other
		// repository state entirely. Fall back to raw heuristic units, which
		// still balances, just not on seconds.
		secondsPerCost = 1
	}

	resolved := make([]float64, len(packages))
	for i, pkg := range packages {
		seconds, ok := weights.Durations[pkg.Name]
		if !ok {
			if typical, known := toolchainMedian[toolchainForLanguage(pkg.Language)]; known {
				seconds = typical
			} else {
				seconds = float64(packageCost(pkg)) * secondsPerCost
			}
		}
		resolved[i] = math.Max(seconds, minPackageWeight)
	}
	return resolved
}

// closureIndex returns a memoised lookup from a package index to its
// transitive prerequisite closure (itself included), as package indexes.
// Edges naming packages outside the plan are ignored, matching v1's
// addWithPrereqs.
func closureIndex(bp *BuildPlan, index map[string]int) func(int) []int {
	preds := make([][]int, len(bp.Packages))
	for _, edge := range bp.DependencyEdges {
		from, okFrom := index[edge[0]]
		to, okTo := index[edge[1]]
		if okFrom && okTo {
			preds[to] = append(preds[to], from)
		}
	}
	memo := make([][]int, len(bp.Packages))
	return func(root int) []int {
		if memo[root] != nil {
			return memo[root]
		}
		// Iterative DFS: some dependency chains in this repo are hundreds of
		// packages deep, and a recursive walk per root would be both deep
		// and repeated.
		seen := map[int]bool{root: true}
		stack := []int{root}
		members := []int{}
		for len(stack) > 0 {
			current := stack[len(stack)-1]
			stack = stack[:len(stack)-1]
			members = append(members, current)
			for _, pred := range preds[current] {
				if !seen[pred] {
					seen[pred] = true
					stack = append(stack, pred)
				}
			}
		}
		sort.Ints(members)
		memo[root] = members
		return members
	}
}

func median(values []float64) float64 {
	if len(values) == 0 {
		return 0
	}
	sorted := append([]float64(nil), values...)
	sort.Float64s(sorted)
	mid := len(sorted) / 2
	if len(sorted)%2 == 1 {
		return sorted[mid]
	}
	return (sorted[mid-1] + sorted[mid]) / 2
}
