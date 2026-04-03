// Package validator checks that BUILD files are consistent with the dependency
// graph the build tool inferred from package metadata.
//
// The core failure mode we're guarding against is "works in one build mode but
// not the other":
//   - A BUILD file reaches into sibling packages the graph does not know about.
//     Partial builds may skip those hidden prerequisites, and full builds may
//     execute in an order the BUILD file did not actually declare.
//   - A BUILD file for an isolated-env language (for example Python or
//     TypeScript) forgets to materialize a local prerequisite that is needed
//     when the package is built on a fresh runner.
//   - CI runs a forced full build on main, but conditional toolchain setup is
//     still wired to incremental diff outputs. That lets commands like mix,
//     bundle, uv, luarocks, or cpanm disappear only on the full-build path.
package validator

import (
	"fmt"
	"os"
	"path/filepath"
	"regexp"
	"runtime"
	"sort"
	"strings"

	directedgraph "github.com/adhithyan15/coding-adventures/code/packages/go/directed-graph"
	"github.com/adhithyan15/coding-adventures/code/programs/go/build-tool/internal/discovery"
)

// Languages in this set are expected to materialize their transitive local
// prerequisites directly in BUILD so they can run correctly on a fresh runner
// even when built in isolation.
var requiresExplicitPrereqs = map[string]bool{
	"python":     true,
	"typescript": true,
	"perl":       true,
}

// Languages in this set are installed conditionally by the main CI workflow.
// If main also forces a full build, detect-job outputs must be normalized so
// these toolchains are always enabled on that path.
var ciManagedToolchainLanguages = map[string]bool{
	"python":     true,
	"ruby":       true,
	"typescript": true,
	"rust":       true,
	"elixir":     true,
	"lua":        true,
	"perl":       true,
}

// ValidateBuildFiles returns an error describing every package whose BUILD file
// is inconsistent with the inferred dependency graph.
func ValidateBuildFiles(packages []discovery.Package, graph *directedgraph.Graph) error {
	if graph == nil {
		return nil
	}

	pathToPkg := make(map[string]string, len(packages))
	pythonKnownNames := make(map[string]string)
	for _, pkg := range packages {
		pathToPkg[filepath.Clean(pkg.Path)] = pkg.Name
		if pkg.Language == "python" {
			pythonKnownNames["coding-adventures-"+strings.ToLower(filepath.Base(pkg.Path))] = pkg.Name
		}
	}

	var problems []string
	for _, pkg := range packages {
		if pkg.IsStarlark {
			continue
		}

		referenced := referencedPackages(pkg, pathToPkg)
		if len(referenced) == 0 && !requiresExplicitPrereqs[pkg.Language] {
			continue
		}

		prereqs := transitivePredecessors(graph, pkg.Name)
		allowedDirectRefs := allowedDirectRefsFromMetadata(pkg, pythonKnownNames)
		delete(referenced, pkg.Name) // Self-references are allowed.

		var hidden []string
		for dep := range referenced {
			if !prereqs[dep] && !allowedDirectRefs[dep] {
				hidden = append(hidden, dep)
			}
		}
		sort.Strings(hidden)

		var missing []string
		if requiresExplicitPrereqs[pkg.Language] {
			for dep := range prereqs {
				if !referenced[dep] {
					missing = append(missing, dep)
				}
			}
			sort.Strings(missing)
		}

		if len(hidden) == 0 && len(missing) == 0 {
			continue
		}

		var parts []string
		if len(hidden) > 0 {
			parts = append(parts, fmt.Sprintf("undeclared local package refs: %s", strings.Join(hidden, ", ")))
		}
		if len(missing) > 0 {
			parts = append(parts, fmt.Sprintf("missing prerequisite refs for standalone builds: %s", strings.Join(missing, ", ")))
		}

		problems = append(problems, fmt.Sprintf("%s (%s): %s", pkg.Name, buildFileLabel(pkg.Path), strings.Join(parts, "; ")))
	}

	if ciProblem := validateCIFullBuildToolchains(packages); ciProblem != "" {
		problems = append(problems, ciProblem)
	}

	if len(problems) == 0 {
		return nil
	}

	sort.Strings(problems)
	return fmt.Errorf(
		"BUILD/CI validation failed:\n  - %s\nFix the BUILD file or CI workflow so metadata dependencies and toolchain setup stay correct on clean standalone and full-build runs.",
		strings.Join(problems, "\n  - "),
	)
}

func buildFileLabel(pkgPath string) string {
	buildFile := discovery.GetBuildFileForPlatform(pkgPath, runtime.GOOS)
	if buildFile == "" {
		return filepath.Join(pkgPath, "BUILD")
	}
	return buildFile
}

var relPathRe = regexp.MustCompile(`(?:\.\.?[/\\][^ \t\r\n"'&|;()]+)+`)

func referencedPackages(pkg discovery.Package, pathToPkg map[string]string) map[string]bool {
	found := make(map[string]bool)
	for _, command := range pkg.BuildCommands {
		for _, raw := range relPathRe.FindAllString(command, -1) {
			name, ok := resolvePackageRef(pkg.Path, raw, pathToPkg)
			if ok {
				found[name] = true
			}
		}
	}
	return found
}

func resolvePackageRef(pkgPath, raw string, pathToPkg map[string]string) (string, bool) {
	normalized := strings.ReplaceAll(raw, "\\", "/")
	abs := filepath.Clean(filepath.Join(pkgPath, filepath.FromSlash(normalized)))
	name, ok := pathToPkg[abs]
	return name, ok
}

func transitivePredecessors(graph *directedgraph.Graph, node string) map[string]bool {
	visited := make(map[string]bool)
	queue := []string{node}

	for len(queue) > 0 {
		current := queue[0]
		queue = queue[1:]

		preds, err := graph.Predecessors(current)
		if err != nil {
			continue
		}

		for _, pred := range preds {
			if visited[pred] {
				continue
			}
			visited[pred] = true
			queue = append(queue, pred)
		}
	}

	return visited
}

func allowedDirectRefsFromMetadata(pkg discovery.Package, pythonKnownNames map[string]string) map[string]bool {
	if pkg.Language != "python" {
		return nil
	}
	return parsePythonOptionalDeps(pkg, pythonKnownNames)
}

func parsePythonOptionalDeps(pkg discovery.Package, knownNames map[string]string) map[string]bool {
	pyproject := filepath.Join(pkg.Path, "pyproject.toml")
	data, err := os.ReadFile(pyproject)
	if err != nil {
		return nil
	}

	allowed := make(map[string]bool)
	inOptional := false
	inArray := false
	re := regexp.MustCompile(`["']([^"']+)["']`)

	for _, line := range strings.Split(string(data), "\n") {
		trimmed := strings.TrimSpace(line)

		if !inArray && strings.HasPrefix(trimmed, "[") && strings.HasSuffix(trimmed, "]") {
			inOptional = trimmed == "[project.optional-dependencies]"
			continue
		}

		if !inOptional {
			continue
		}

		if !inArray {
			if !strings.Contains(trimmed, "=") {
				continue
			}
			afterEq := strings.TrimSpace(strings.SplitN(trimmed, "=", 2)[1])
			if !strings.HasPrefix(afterEq, "[") {
				continue
			}
			for dep := range extractMetadataDeps(afterEq, knownNames, re) {
				allowed[dep] = true
			}
			if strings.Contains(afterEq, "]") {
				continue
			}
			inArray = true
			continue
		}

		for dep := range extractMetadataDeps(trimmed, knownNames, re) {
			allowed[dep] = true
		}
		if strings.Contains(trimmed, "]") {
			inArray = false
		}
	}

	return allowed
}

func extractMetadataDeps(line string, knownNames map[string]string, re *regexp.Regexp) map[string]bool {
	found := make(map[string]bool)
	for _, match := range re.FindAllStringSubmatch(line, -1) {
		if len(match) < 2 {
			continue
		}
		depName := regexp.MustCompile(`[>=<!~\s;]`).Split(match[1], 2)[0]
		depName = strings.TrimSpace(strings.ToLower(depName))
		if pkgName, ok := knownNames[depName]; ok {
			found[pkgName] = true
		}
	}
	return found
}

func validateCIFullBuildToolchains(packages []discovery.Package) string {
	repoRoot := inferRepoRoot(packages)
	if repoRoot == "" {
		return ""
	}

	ciPath := filepath.Join(repoRoot, ".github", "workflows", "ci.yml")
	data, err := os.ReadFile(ciPath)
	if err != nil {
		return ""
	}

	workflow := string(data)
	if !strings.Contains(workflow, "Full build on main merge") {
		return ""
	}

	var missingOutputBinding []string
	var missingMainForce []string

	for _, lang := range languagesNeedingCIToolchains(packages) {
		outputPattern := regexp.MustCompile(
			fmt.Sprintf(
				`needs_%s:\s*\$\{\{\s*steps\.toolchains\.outputs\.needs_%s\s*\}\}`,
				regexp.QuoteMeta(lang),
				regexp.QuoteMeta(lang),
			),
		)
		if !outputPattern.MatchString(workflow) {
			missingOutputBinding = append(missingOutputBinding, lang)
		}

		if !strings.Contains(workflow, fmt.Sprintf("needs_%s=true", lang)) {
			missingMainForce = append(missingMainForce, lang)
		}
	}

	if len(missingOutputBinding) == 0 && len(missingMainForce) == 0 {
		return ""
	}

	var parts []string
	if len(missingOutputBinding) > 0 {
		parts = append(parts, fmt.Sprintf(
			"detect outputs for forced main full builds are not normalized through steps.toolchains for: %s",
			strings.Join(missingOutputBinding, ", "),
		))
	}
	if len(missingMainForce) > 0 {
		parts = append(parts, fmt.Sprintf(
			"forced main full-build path does not explicitly enable toolchains for: %s",
			strings.Join(missingMainForce, ", "),
		))
	}

	return fmt.Sprintf(
		"%s: %s",
		filepath.ToSlash(ciPath),
		strings.Join(parts, "; "),
	)
}

func languagesNeedingCIToolchains(packages []discovery.Package) []string {
	seen := make(map[string]bool)
	var langs []string
	for _, pkg := range packages {
		if !ciManagedToolchainLanguages[pkg.Language] || seen[pkg.Language] {
			continue
		}
		seen[pkg.Language] = true
		langs = append(langs, pkg.Language)
	}
	sort.Strings(langs)
	return langs
}

func inferRepoRoot(packages []discovery.Package) string {
	for _, pkg := range packages {
		root := inferRepoRootFromPackagePath(pkg.Path)
		if root != "" {
			return root
		}
	}
	return ""
}

func inferRepoRootFromPackagePath(pkgPath string) string {
	current := filepath.Clean(pkgPath)
	for {
		parent := filepath.Dir(current)
		if filepath.Base(current) == "code" {
			return parent
		}
		if parent == current {
			return ""
		}
		current = parent
	}
}
