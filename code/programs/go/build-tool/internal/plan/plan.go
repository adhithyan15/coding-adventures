// Package plan provides serialization and deserialization for build plans.
//
// A build plan captures the results of the build tool's discovery,
// dependency resolution, and change detection steps as a JSON file.
// This enables CI to compute the plan once in a fast "detect" job and
// share it across build jobs on multiple platforms — eliminating
// redundant computation.
//
// # Schema versioning
//
// The plan uses a simple integer version scheme (schema_version field).
// Readers MUST reject plans with a version higher than what they support,
// falling back to the normal discovery flow. Writers always stamp the
// current version. See code/specs/build-plan-v1.md for the full spec.
//
// # Path conventions
//
// All paths in the plan use forward slashes (/) regardless of platform.
// On write, filepath.ToSlash normalizes paths. On read, consumers call
// filepath.FromSlash to convert back to platform-native separators.
package plan

import (
	"encoding/json"
	"fmt"
	"os"
)

// CurrentSchemaVersion is the schema version that this implementation
// reads and writes. Plans with a higher version are rejected.
const CurrentSchemaVersion = 1

// BuildPlan is the top-level structure serialized to JSON.
//
// See code/specs/build-plan-v1.md for the full schema definition,
// versioning rules, and evolution examples.
type BuildPlan struct {
	// SchemaVersion identifies the plan format. Readers MUST reject
	// plans with a version higher than CurrentSchemaVersion.
	SchemaVersion int `json:"schema_version"`

	// DiffBase is the git ref used for change detection (informational).
	DiffBase string `json:"diff_base"`

	// Force indicates whether --force was set. When true, all packages
	// should be rebuilt regardless of change detection.
	Force bool `json:"force"`

	// AffectedPackages lists the qualified names of packages that need
	// building. Semantics:
	//   nil/null  → rebuild all (force mode or git diff unavailable)
	//   []        → nothing changed, build nothing
	//   [a, b, …] → only these packages need building
	AffectedPackages []string `json:"affected_packages"`

	// Packages contains ALL discovered packages, not just affected ones.
	// The executor needs the full list for dep-skipped detection.
	Packages []PackageEntry `json:"packages"`

	// DependencyEdges are directed edges [from, to] where from→to means
	// "to depends on from" (from must be built before to).
	DependencyEdges [][2]string `json:"dependency_edges"`

	// LanguagesNeeded maps language names to booleans indicating whether
	// that language's toolchain is needed for this build.
	LanguagesNeeded map[string]bool `json:"languages_needed"`

	// Shards describes optional prerequisite-closed package slices that
	// can be executed independently by parallel CI runners.
	Shards []ShardEntry `json:"shards,omitempty"`
}

// ShardEntry describes one independently executable slice of a build plan.
//
// AssignedPackages are the packages whose work was directly assigned to this
// shard. PackageNames includes AssignedPackages plus their transitive
// prerequisites, so a runner can execute the shard without artifacts from
// another runner.
type ShardEntry struct {
	Index            int             `json:"index"`
	Name             string          `json:"name"`
	AssignedPackages []string        `json:"assigned_packages"`
	PackageNames     []string        `json:"package_names"`
	LanguagesNeeded  map[string]bool `json:"languages_needed"`
	EstimatedCost    int             `json:"estimated_cost"`
}

// ShardMatrixEntry is the compact representation emitted for GitHub Actions
// dynamic matrix expansion.
type ShardMatrixEntry struct {
	ShardIndex   int      `json:"shard_index"`
	ShardCount   int      `json:"shard_count"`
	Label        string   `json:"label"`
	PackageCount int      `json:"package_count"`
	Languages    []string `json:"languages"`
}

// PackageEntry represents a single package in the build plan.
type PackageEntry struct {
	// Name is the qualified package name: "language/package-name".
	Name string `json:"name"`

	// RelPath is the repo-root-relative path, always using forward slashes.
	RelPath string `json:"rel_path"`

	// Language is the package's programming language.
	Language string `json:"language"`

	// BuildCommands are the shell commands to execute for building/testing.
	BuildCommands []string `json:"build_commands"`

	// IsStarlark indicates whether the BUILD file uses Starlark syntax.
	IsStarlark bool `json:"is_starlark"`

	// DeclaredSrcs are glob patterns from the Starlark srcs field.
	DeclaredSrcs []string `json:"declared_srcs,omitempty"`

	// DeclaredDeps are qualified names from the Starlark deps field.
	DeclaredDeps []string `json:"declared_deps,omitempty"`
}

// Write serializes a build plan to a JSON file at the given path.
// It always stamps SchemaVersion to CurrentSchemaVersion.
func Write(bp *BuildPlan, path string) error {
	bp.SchemaVersion = CurrentSchemaVersion

	data, err := json.MarshalIndent(bp, "", "  ")
	if err != nil {
		return fmt.Errorf("marshal build plan: %w", err)
	}

	// Atomic write: write to temp file, then rename.
	tmp := path + ".tmp"
	if err := os.WriteFile(tmp, data, 0644); err != nil {
		return fmt.Errorf("write build plan: %w", err)
	}
	if err := os.Rename(tmp, path); err != nil {
		// Cleanup temp file on rename failure.
		os.Remove(tmp)
		return fmt.Errorf("rename build plan: %w", err)
	}

	return nil
}

// Read deserializes a build plan from a JSON file.
// Returns an error if the file is missing, unparseable, or has
// a schema_version higher than CurrentSchemaVersion.
func Read(path string) (*BuildPlan, error) {
	data, err := os.ReadFile(path)
	if err != nil {
		return nil, fmt.Errorf("read build plan: %w", err)
	}

	var bp BuildPlan
	if err := json.Unmarshal(data, &bp); err != nil {
		return nil, fmt.Errorf("parse build plan: %w", err)
	}

	if bp.SchemaVersion > CurrentSchemaVersion {
		return nil, fmt.Errorf(
			"unsupported build plan version %d (this tool supports up to %d)",
			bp.SchemaVersion, CurrentSchemaVersion,
		)
	}

	return &bp, nil
}
