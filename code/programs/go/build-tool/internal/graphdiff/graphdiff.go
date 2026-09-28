// Package graphdiff implements the build tool's pure graph and diff-selection
// domain. Callers supply inert values; this package never reads a checkout,
// invokes Git, inspects the environment, launches a process, or uses a network.
package graphdiff

import (
	"bytes"
	"crypto/sha256"
	"encoding/binary"
	"encoding/hex"
	"encoding/json"
	"errors"
	"regexp"
	"sort"
	"strings"
	"unicode/utf8"

	"github.com/adhithyan15/coding-adventures/code/programs/go/build-tool/internal/globmatch"
	"golang.org/x/text/cases"
	"golang.org/x/text/unicode/norm"
)

const (
	MaxPackages           = 4_096
	MaxEdges              = 16_384
	MaxSourceGlobs        = 256
	MaxPathScalars        = 512
	MaxPackageNameScalars = 240
	MaxDiffMatchWork      = 50_000_000
)

const boundaryDomain = "coding-adventures/build-tool-repository-source-input-boundary/v1\x00"

var (
	packageNamePattern = regexp.MustCompile(`^[a-z0-9][a-z0-9._-]*(/[a-z0-9][a-z0-9._-]*)+$`)
	digestPattern      = regexp.MustCompile(`^[0-9a-f]{64}$`)
	caseFolder         = cases.Fold()
	matchPath          = globmatch.MatchPath
	buildFronts        = map[string]struct{}{
		"BUILD": {}, "BUILD_windows": {}, "BUILD_mac": {},
		"BUILD_linux": {}, "BUILD_mac_and_linux": {},
	}
	windowsReserved = map[string]struct{}{
		"CON": {}, "PRN": {}, "AUX": {}, "NUL": {}, "CONIN$": {},
		"CONOUT$": {}, "CLOCK$": {}, "COM1": {}, "COM2": {}, "COM3": {},
		"COM4": {}, "COM5": {}, "COM6": {}, "COM7": {}, "COM8": {},
		"COM9": {}, "LPT1": {}, "LPT2": {}, "LPT3": {}, "LPT4": {},
		"LPT5": {}, "LPT6": {}, "LPT7": {}, "LPT8": {}, "LPT9": {},
		"COM¹": {}, "COM²": {}, "COM³": {}, "LPT¹": {}, "LPT²": {}, "LPT³": {},
	}
)

// Edge is one canonical [prerequisite, dependent] graph edge.
type Edge struct {
	Prerequisite string
	Dependent    string
}

type GraphInput struct {
	Packages []string
	Edges    []Edge
}

type GraphResult struct {
	Edges     []Edge
	Levels    [][]string
	ErrorCode string
}

type PackageSpec struct {
	Name        string   `json:"name"`
	RelPath     string   `json:"rel_path"`
	SourceMode  string   `json:"source_mode"`
	SourceGlobs []string `json:"source_globs"`
}

type AppliesTo struct {
	ExactRoots      []string `json:"exact_roots"`
	DescendantRoots []string `json:"descendant_roots"`
	ExcludedRoots   []string `json:"excluded_roots"`
}

type BoundaryInput struct {
	Path               string `json:"path"`
	Role               string `json:"role"`
	GeneratedComponent string `json:"generated_component,omitempty"`
}

type BoundaryRule struct {
	ID          string          `json:"id"`
	InputOrigin string          `json:"input_origin"`
	AppliesTo   AppliesTo       `json:"applies_to"`
	Inputs      []BoundaryInput `json:"inputs"`
	Reason      string          `json:"reason"`
	Owner       string          `json:"owner"`
}

type RepositoryBoundary struct {
	SchemaVersion                     int            `json:"schema_version"`
	LanguageSourceInputRegistrySHA256 string         `json:"language_source_input_registry_sha256"`
	Boundaries                        []BoundaryRule `json:"boundaries"`
}

type DiffSelectionInput struct {
	Packages          []PackageSpec
	Edges             []Edge
	ForcedPackages    []string
	UnknownPathPolicy string
	ChangedPaths      []string
	BoundarySHA256    string
	Boundary          *RepositoryBoundary
}

type DiffSelectionResult struct {
	ChangedPackages      []string
	AffectedPackages     []string
	PrerequisitePackages []string
	ErrorCode            string
}

type validatedGraph struct {
	names         map[string]struct{}
	edges         []Edge
	dependents    map[string][]string
	prerequisites map[string][]string
}

// EvaluateGraph returns canonical edges and deterministic prerequisite-first
// levels. A cycle is a domain result because the neutral corpus expects one
// stable diagnostic and no partial graph output.
func EvaluateGraph(input GraphInput) (GraphResult, error) {
	graph, err := validateGraph(input.Packages, input.Edges)
	if err != nil {
		return GraphResult{}, err
	}
	levels, ok := graphLevels(graph)
	if !ok {
		return GraphResult{Edges: []Edge{}, Levels: [][]string{}, ErrorCode: "GRAPH_CYCLE"}, nil
	}
	return GraphResult{Edges: graph.edges, Levels: levels}, nil
}

// EvaluateDiffSelection returns exact changed, dependent-closed, and
// prerequisite-only package sets without acquiring host authority.
func EvaluateDiffSelection(input DiffSelectionInput) (DiffSelectionResult, error) {
	names := make([]string, len(input.Packages))
	for index, pkg := range input.Packages {
		names[index] = pkg.Name
	}
	graph, err := validateGraph(names, input.Edges)
	if err != nil {
		return DiffSelectionResult{}, err
	}
	if _, ok := graphLevels(graph); !ok {
		return DiffSelectionResult{}, errors.New("DIFF_EDGE_CYCLE")
	}
	packages, err := validateDiffInput(input, graph)
	if err != nil {
		return DiffSelectionResult{}, err
	}
	boundaryConsumers, err := boundaryReverseIndex(input, packages)
	if err != nil {
		return DiffSelectionResult{}, err
	}

	remaining := uint64(MaxDiffMatchWork)
	for _, pkg := range input.Packages {
		if pkg.SourceMode != "strict_globs" {
			continue
		}
		var patternFactor uint64
		for _, pattern := range pkg.SourceGlobs {
			patternFactor += uint64(utf8.RuneCountInString(pattern)) + 1
		}
		for _, path := range input.ChangedPaths {
			if !inside(path, pkg.RelPath) {
				continue
			}
			relative := relativePath(path, pkg.RelPath)
			if isBuildFront(relative) {
				continue
			}
			pathFactor := uint64(utf8.RuneCountInString(relative)) + 1
			if patternFactor != 0 && patternFactor > remaining/pathFactor {
				return diffError("DIFF_MATCH_LIMIT_EXCEEDED"), nil
			}
			remaining -= patternFactor * pathFactor
		}
	}

	changed := makeSet(input.ForcedPackages)
	unknown := false
	for _, path := range input.ChangedPaths {
		consumers := boundaryConsumers[path]
		for name := range consumers {
			changed[name] = struct{}{}
		}
		known := len(consumers) != 0
		for _, pkg := range input.Packages {
			if !inside(path, pkg.RelPath) {
				continue
			}
			known = true
			relative := relativePath(path, pkg.RelPath)
			selected := pkg.SourceMode == "package_prefix" || isBuildFront(relative)
			if !selected {
				for _, pattern := range pkg.SourceGlobs {
					if matchPath(pattern, relative) {
						selected = true
						break
					}
				}
			}
			if selected {
				changed[pkg.Name] = struct{}{}
			}
		}
		if !known {
			unknown = true
		}
	}
	if unknown {
		if input.UnknownPathPolicy == "error" {
			return diffError("DIFF_UNKNOWN_PATH"), nil
		}
		changed = make(map[string]struct{}, len(packages))
		for name := range packages {
			changed[name] = struct{}{}
		}
	}

	affected := closure(changed, graph.dependents)
	prerequisites := closure(affected, graph.prerequisites)
	for name := range affected {
		delete(prerequisites, name)
	}
	return DiffSelectionResult{
		ChangedPackages:      sortedSet(changed),
		AffectedPackages:     sortedSet(affected),
		PrerequisitePackages: sortedSet(prerequisites),
	}, nil
}

// Digest returns the versioned digest of this exact inert boundary registry.
func (boundary RepositoryBoundary) Digest() (string, error) {
	rules := make([]any, len(boundary.Boundaries))
	for ruleIndex, rule := range boundary.Boundaries {
		inputs := make([]any, len(rule.Inputs))
		for inputIndex, input := range rule.Inputs {
			value := map[string]any{"path": input.Path, "role": input.Role}
			if input.GeneratedComponent != "" {
				value["generated_component"] = input.GeneratedComponent
			}
			inputs[inputIndex] = value
		}
		rules[ruleIndex] = map[string]any{
			"id":           rule.ID,
			"input_origin": rule.InputOrigin,
			"applies_to": map[string]any{
				"exact_roots":      canonicalStringList(rule.AppliesTo.ExactRoots),
				"descendant_roots": canonicalStringList(rule.AppliesTo.DescendantRoots),
				"excluded_roots":   canonicalStringList(rule.AppliesTo.ExcludedRoots),
			},
			"inputs": inputs,
			"reason": rule.Reason,
			"owner":  rule.Owner,
		}
	}
	document := map[string]any{
		"schema_version":                        boundary.SchemaVersion,
		"language_source_input_registry_sha256": boundary.LanguageSourceInputRegistrySHA256,
		"boundaries":                            rules,
	}
	encoded, err := canonicalJSON(document)
	if err != nil {
		return "", err
	}
	hasher := sha256.New()
	_, _ = hasher.Write([]byte(boundaryDomain))
	var length [8]byte
	binary.BigEndian.PutUint64(length[:], uint64(len(encoded)))
	_, _ = hasher.Write(length[:])
	_, _ = hasher.Write(encoded)
	return hex.EncodeToString(hasher.Sum(nil)), nil
}

func canonicalStringList(values []string) []string {
	if values == nil {
		return []string{}
	}
	return values
}

func canonicalJSON(value any) ([]byte, error) {
	var buffer bytes.Buffer
	encoder := json.NewEncoder(&buffer)
	encoder.SetEscapeHTML(false)
	if err := encoder.Encode(value); err != nil {
		return nil, err
	}
	encoded := bytes.TrimSuffix(buffer.Bytes(), []byte{'\n'})
	return unescapeJSONLineSeparators(encoded), nil
}

// encoding/json always escapes U+2028 and U+2029, even when HTML escaping is
// disabled. The neutral oracle emits raw UTF-8. Only odd-length backslash runs
// terminate in an actual JSON Unicode escape; even runs represent literal
// backslashes and must remain untouched.
func unescapeJSONLineSeparators(encoded []byte) []byte {
	result := make([]byte, 0, len(encoded))
	for index := 0; index < len(encoded); {
		if encoded[index] != '\\' {
			result = append(result, encoded[index])
			index++
			continue
		}
		start := index
		for index < len(encoded) && encoded[index] == '\\' {
			index++
		}
		count := index - start
		if count%2 == 1 && index+5 <= len(encoded) && encoded[index] == 'u' &&
			(string(encoded[index:index+5]) == "u2028" || string(encoded[index:index+5]) == "u2029") {
			result = append(result, encoded[start:start+count-1]...)
			if encoded[index+4] == '8' {
				result = append(result, 0xe2, 0x80, 0xa8)
			} else {
				result = append(result, 0xe2, 0x80, 0xa9)
			}
			index += 5
			continue
		}
		result = append(result, encoded[start:index]...)
	}
	return result
}

func validateGraph(packages []string, edges []Edge) (validatedGraph, error) {
	if len(packages) > MaxPackages {
		return validatedGraph{}, errors.New("GRAPH_PACKAGE_LIMIT_EXCEEDED")
	}
	if len(edges) > MaxEdges {
		return validatedGraph{}, errors.New("GRAPH_EDGE_LIMIT_EXCEEDED")
	}
	names := make(map[string]struct{}, len(packages))
	dependents := make(map[string][]string, len(packages))
	prerequisites := make(map[string][]string, len(packages))
	for _, name := range packages {
		if !validPackageName(name) {
			return validatedGraph{}, errors.New("GRAPH_PACKAGE_INVALID")
		}
		if _, exists := names[name]; exists {
			return validatedGraph{}, errors.New("GRAPH_PACKAGE_DUPLICATE")
		}
		names[name] = struct{}{}
		dependents[name] = nil
		prerequisites[name] = nil
	}
	uniqueEdges := make(map[Edge]struct{}, len(edges))
	canonical := make([]Edge, len(edges))
	copy(canonical, edges)
	for _, edge := range edges {
		if _, ok := names[edge.Prerequisite]; !ok {
			return validatedGraph{}, errors.New("GRAPH_EDGE_UNKNOWN")
		}
		if _, ok := names[edge.Dependent]; !ok {
			return validatedGraph{}, errors.New("GRAPH_EDGE_UNKNOWN")
		}
		if edge.Prerequisite == edge.Dependent {
			return validatedGraph{}, errors.New("GRAPH_EDGE_SELF")
		}
		if _, exists := uniqueEdges[edge]; exists {
			return validatedGraph{}, errors.New("GRAPH_EDGE_DUPLICATE")
		}
		uniqueEdges[edge] = struct{}{}
		dependents[edge.Prerequisite] = append(dependents[edge.Prerequisite], edge.Dependent)
		prerequisites[edge.Dependent] = append(prerequisites[edge.Dependent], edge.Prerequisite)
	}
	sort.Slice(canonical, func(left, right int) bool {
		if canonical[left].Prerequisite == canonical[right].Prerequisite {
			return canonical[left].Dependent < canonical[right].Dependent
		}
		return canonical[left].Prerequisite < canonical[right].Prerequisite
	})
	for name := range names {
		sort.Strings(dependents[name])
		sort.Strings(prerequisites[name])
	}
	return validatedGraph{names, canonical, dependents, prerequisites}, nil
}

func graphLevels(graph validatedGraph) ([][]string, bool) {
	indegree := make(map[string]int, len(graph.names))
	for name := range graph.names {
		indegree[name] = 0
	}
	for _, edge := range graph.edges {
		indegree[edge.Dependent]++
	}
	ready := make([]string, 0)
	for name, degree := range indegree {
		if degree == 0 {
			ready = append(ready, name)
		}
	}
	sort.Strings(ready)
	levels := make([][]string, 0)
	visited := 0
	for len(ready) != 0 {
		level := append([]string(nil), ready...)
		levels = append(levels, level)
		next := make([]string, 0)
		for _, name := range level {
			visited++
			for _, dependent := range graph.dependents[name] {
				indegree[dependent]--
				if indegree[dependent] == 0 {
					next = append(next, dependent)
				}
			}
		}
		sort.Strings(next)
		ready = next
	}
	return levels, visited == len(graph.names)
}

func validateDiffInput(input DiffSelectionInput, graph validatedGraph) (map[string]PackageSpec, error) {
	packages := make(map[string]PackageSpec, len(input.Packages))
	rootIdentities := make([]string, 0, len(input.Packages))
	for _, pkg := range input.Packages {
		if !validPackageName(pkg.Name) {
			return nil, errors.New("DIFF_PACKAGE_INVALID")
		}
		if !validPortablePath(pkg.RelPath) {
			return nil, errors.New("DIFF_PATH_INVALID")
		}
		identity := caseFolder.String(norm.NFC.String(pkg.RelPath))
		for _, prior := range rootIdentities {
			if identity == prior || strings.HasPrefix(identity, prior+"/") || strings.HasPrefix(prior, identity+"/") {
				return nil, errors.New("DIFF_PATH_INVALID")
			}
		}
		rootIdentities = append(rootIdentities, identity)
		if pkg.SourceMode != "package_prefix" && pkg.SourceMode != "strict_globs" {
			return nil, errors.New("DIFF_SOURCE_MODE_INVALID")
		}
		if len(pkg.SourceGlobs) > MaxSourceGlobs || hasDuplicateStrings(pkg.SourceGlobs) {
			return nil, errors.New("DIFF_GLOB_INVALID")
		}
		for _, pattern := range pkg.SourceGlobs {
			if !validPortableGlob(pattern) || !validGlobPattern(pattern) {
				return nil, errors.New("DIFF_GLOB_INVALID")
			}
		}
		if pkg.SourceMode != "strict_globs" && len(pkg.SourceGlobs) != 0 {
			return nil, errors.New("DIFF_GLOB_INVALID")
		}
		if _, exists := packages[pkg.Name]; exists {
			return nil, errors.New("DIFF_PACKAGE_DUPLICATE")
		}
		packages[pkg.Name] = pkg
	}
	if len(packages) != len(graph.names) {
		return nil, errors.New("DIFF_PACKAGE_INVALID")
	}
	for name := range graph.names {
		if _, ok := packages[name]; !ok {
			return nil, errors.New("DIFF_PACKAGE_INVALID")
		}
	}
	if input.UnknownPathPolicy != "all" && input.UnknownPathPolicy != "error" {
		return nil, errors.New("DIFF_POLICY_INVALID")
	}
	if len(input.ForcedPackages) > MaxPackages || hasDuplicateStrings(input.ForcedPackages) {
		return nil, errors.New("DIFF_FORCED_PACKAGE_INVALID")
	}
	if len(input.ChangedPaths) > MaxPackages || hasDuplicateStrings(input.ChangedPaths) {
		return nil, errors.New("DIFF_PATH_INVALID")
	}
	for _, path := range input.ChangedPaths {
		if !validPortablePath(path) {
			return nil, errors.New("DIFF_PATH_INVALID")
		}
	}
	for _, forced := range input.ForcedPackages {
		if _, ok := packages[forced]; !ok {
			return nil, errors.New("DIFF_FORCED_PACKAGE_UNKNOWN")
		}
	}
	return packages, nil
}

func boundaryReverseIndex(input DiffSelectionInput, packages map[string]PackageSpec) (map[string]map[string]struct{}, error) {
	if input.BoundarySHA256 == "" {
		if input.Boundary != nil {
			return nil, errors.New("DIFF_BOUNDARY_DIGEST_MISMATCH")
		}
		return map[string]map[string]struct{}{}, nil
	}
	if !digestPattern.MatchString(input.BoundarySHA256) || input.Boundary == nil {
		return nil, errors.New("DIFF_BOUNDARY_DIGEST_MISMATCH")
	}
	digest, err := input.Boundary.Digest()
	if err != nil || digest != input.BoundarySHA256 {
		return nil, errors.New("DIFF_BOUNDARY_DIGEST_MISMATCH")
	}
	result := make(map[string]map[string]struct{})
	for _, pkg := range packages {
		for _, rule := range input.Boundary.Boundaries {
			if !applies(rule.AppliesTo, pkg.RelPath) {
				continue
			}
			for _, value := range rule.Inputs {
				if result[value.Path] == nil {
					result[value.Path] = make(map[string]struct{})
				}
				result[value.Path][pkg.Name] = struct{}{}
			}
		}
	}
	return result, nil
}

func validPackageName(value string) bool {
	return utf8.ValidString(value) && utf8.RuneCountInString(value) <= MaxPackageNameScalars && packageNamePattern.MatchString(value)
}

func validPortablePath(value string) bool {
	if !validPathPrefix(value) || strings.ContainsAny(value, `<>:"|?*`) {
		return false
	}
	for _, char := range value {
		if char < 32 {
			return false
		}
	}
	for _, segment := range strings.Split(value, "/") {
		if invalidSegment(segment) || reservedSegment(segment) {
			return false
		}
	}
	return true
}

func validPortableGlob(value string) bool {
	if !validPathPrefix(value) || strings.ContainsAny(value, `<>:"|?`) {
		return false
	}
	for _, char := range value {
		if char < 32 {
			return false
		}
	}
	for _, segment := range strings.Split(value, "/") {
		if invalidSegment(segment) {
			return false
		}
		if !strings.ContainsAny(segment, "*[]{}") && reservedSegment(segment) {
			return false
		}
	}
	return true
}

func validPathPrefix(value string) bool {
	return value != "" && utf8.ValidString(value) && utf8.RuneCountInString(value) <= MaxPathScalars &&
		norm.NFC.IsNormalString(value) && !strings.HasPrefix(value, "/") &&
		!hasDrivePrefix(value) && !strings.Contains(value, `\`) && !strings.Contains(value, "//")
}

func validGlobPattern(value string) bool {
	runes := []rune(value)
	for opening := 0; opening < len(runes); opening++ {
		if runes[opening] != '[' {
			continue
		}
		cursor := opening + 1
		if cursor < len(runes) && runes[cursor] == '!' {
			cursor++
		}
		closingSearch := cursor
		if closingSearch < len(runes) && runes[closingSearch] == ']' {
			closingSearch++
		}
		closing := closingSearch
		for closing < len(runes) && runes[closing] != ']' {
			closing++
		}
		if closing == len(runes) {
			continue // unmatched '[' is literal in the neutral grammar
		}
		for member := cursor; member+1 < closing; member++ {
			left, right := runes[member], runes[member+1]
			if (left == '-' && right == '-') || (left == '&' && right == '&') ||
				(left == '~' && right == '~') || (left == '|' && right == '|') {
				return false
			}
		}
		for member := cursor; member < closing; {
			if member+2 < closing && runes[member+1] == '-' {
				if runes[member] > runes[member+2] {
					return false
				}
				member += 3
			} else {
				member++
			}
		}
		opening = closing
	}
	return true
}

func invalidSegment(segment string) bool {
	return segment == "" || segment == "." || segment == ".." || strings.HasSuffix(segment, " ") || strings.HasSuffix(segment, ".")
}

func reservedSegment(segment string) bool {
	base := strings.SplitN(segment, ".", 2)[0]
	_, reserved := windowsReserved[strings.ToUpper(base)]
	return reserved
}

func hasDrivePrefix(value string) bool {
	if len(value) < 2 || value[1] != ':' {
		return false
	}
	return (value[0] >= 'A' && value[0] <= 'Z') || (value[0] >= 'a' && value[0] <= 'z')
}

func hasDuplicateStrings(values []string) bool {
	seen := make(map[string]struct{}, len(values))
	for _, value := range values {
		if _, exists := seen[value]; exists {
			return true
		}
		seen[value] = struct{}{}
	}
	return false
}

func applies(value AppliesTo, root string) bool {
	if contains(value.ExactRoots, root) {
		return true
	}
	if contains(value.ExcludedRoots, root) {
		return false
	}
	for _, parent := range value.DescendantRoots {
		if strings.HasPrefix(root, parent+"/") {
			return true
		}
	}
	return false
}

func contains(values []string, wanted string) bool {
	for _, value := range values {
		if value == wanted {
			return true
		}
	}
	return false
}

func inside(path, root string) bool {
	return path == root || strings.HasPrefix(path, root+"/")
}

func relativePath(path, root string) string {
	if path == root {
		return ""
	}
	return strings.TrimPrefix(path, root+"/")
}

func isBuildFront(path string) bool {
	basename := path
	if slash := strings.LastIndex(path, "/"); slash >= 0 {
		basename = path[slash+1:]
	}
	_, ok := buildFronts[basename]
	return ok
}

func makeSet(values []string) map[string]struct{} {
	result := make(map[string]struct{}, len(values))
	for _, value := range values {
		result[value] = struct{}{}
	}
	return result
}

func closure(seeds map[string]struct{}, adjacency map[string][]string) map[string]struct{} {
	result := make(map[string]struct{}, len(seeds))
	pending := make([]string, 0, len(seeds))
	for seed := range seeds {
		result[seed] = struct{}{}
		pending = append(pending, seed)
	}
	for len(pending) != 0 {
		name := pending[0]
		pending = pending[1:]
		for _, next := range adjacency[name] {
			if _, exists := result[next]; exists {
				continue
			}
			result[next] = struct{}{}
			pending = append(pending, next)
		}
	}
	return result
}

func sortedSet(values map[string]struct{}) []string {
	result := make([]string, 0, len(values))
	for value := range values {
		result = append(result, value)
	}
	sort.Strings(result)
	return result
}

func diffError(code string) DiffSelectionResult {
	return DiffSelectionResult{
		ChangedPackages: []string{}, AffectedPackages: []string{},
		PrerequisitePackages: []string{}, ErrorCode: code,
	}
}
