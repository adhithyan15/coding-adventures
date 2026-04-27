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
	"dart":       true,
	"swift":      true,
	"haskell":    true,
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
		referencedFuzzy := referencedPackagesFuzzy(pkg, pathToPkg)
		if len(referenced) == 0 && len(referencedFuzzy) == 0 && !requiresExplicitPrereqs[pkg.Language] {
			continue
		}

		prereqs := transitivePredecessors(graph, pkg.Name)
		allowedDirectRefs := allowedDirectRefsFromMetadata(pkg, pythonKnownNames)
		delete(referenced, pkg.Name)      // Self-references are allowed.
		delete(referencedFuzzy, pkg.Name) // Self-references are allowed.

		var hidden []string
		for dep := range referenced {
			if !prereqs[dep] && !allowedDirectRefs[dep] {
				hidden = append(hidden, dep)
			}
		}
		sort.Strings(hidden)

		// Use fuzzy resolution for the missing-prereq check: BUILD files
		// often point at subdirectories (e.g. ../sha512/lib) rather than
		// the package root, and those should satisfy the prereq.
		var missing []string
		if requiresExplicitPrereqs[pkg.Language] && !isIntentionalSkipBuild(pkg) {
			for dep := range prereqs {
				if !referencedFuzzy[dep] {
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
	problems = append(problems, validateLuaIsolatedBuildFiles(packages)...)
	problems = append(problems, validatePerlBuildFiles(packages)...)
	problems = append(problems, validateRustWorkspaceMembers(packages)...)

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

var relPathRe = regexp.MustCompile(`(?:\.\.?[/\\][^ \t\r\n"'&|;():]+)+`)

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

// referencedPackagesFuzzy is like referencedPackages but uses ancestor-walking
// resolution so that subdirectory references (e.g. ../sha512/lib) resolve to
// their parent package.
func referencedPackagesFuzzy(pkg discovery.Package, pathToPkg map[string]string) map[string]bool {
	found := make(map[string]bool)
	for _, command := range pkg.BuildCommands {
		for _, raw := range relPathRe.FindAllString(command, -1) {
			name, ok := resolvePackageRefFuzzy(pkg.Path, raw, pathToPkg)
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

// resolvePackageRefFuzzy is like resolvePackageRef but also walks up the
// directory tree. BUILD files often reference subdirectories of a sibling
// package (e.g. ../sha512/lib) rather than the package root. This variant
// is used only for the "missing prerequisite" check so that legitimate
// subdirectory references count as satisfying a declared dependency.
func resolvePackageRefFuzzy(pkgPath, raw string, pathToPkg map[string]string) (string, bool) {
	normalized := strings.ReplaceAll(raw, "\\", "/")
	abs := filepath.Clean(filepath.Join(pkgPath, filepath.FromSlash(normalized)))
	for dir := abs; ; dir = filepath.Dir(dir) {
		if name, ok := pathToPkg[dir]; ok {
			return name, true
		}
		parent := filepath.Dir(dir)
		if parent == dir {
			break
		}
	}
	return "", false
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

func isIntentionalSkipBuild(pkg discovery.Package) bool {
	if len(pkg.BuildCommands) == 0 {
		return false
	}
	for _, command := range pkg.BuildCommands {
		lower := strings.ToLower(strings.TrimSpace(command))
		if !strings.HasPrefix(lower, "echo ") {
			return false
		}
		if !strings.Contains(lower, "skip") && !strings.Contains(lower, "not supported") {
			return false
		}
	}
	return true
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

func validateLuaIsolatedBuildFiles(packages []discovery.Package) []string {
	var problems []string
	localLuaRocks := make(map[string]string)
	for _, pkg := range packages {
		if pkg.Language != "lua" {
			continue
		}
		localLuaRocks[luaRockNameForPackagePath(pkg.Path)] = filepath.Base(pkg.Path)
	}

	for _, pkg := range packages {
		if pkg.Language != "lua" {
			continue
		}
		selfRock := "coding-adventures-" + strings.ReplaceAll(filepath.Base(pkg.Path), "_", "-")
		buildLines := make(map[string][]string)
		for _, buildPath := range luaBuildFiles(pkg.Path) {
			lines := readBuildLines(buildPath)
			buildLines[filepath.Base(buildPath)] = lines
			if len(lines) == 0 {
				continue
			}

			if target := firstForeignLuaRemove(lines, selfRock); target != "" {
				problems = append(problems, fmt.Sprintf(
					"%s: Lua BUILD removes unrelated rock %s; isolated package builds should only remove the package they are rebuilding",
					filepath.ToSlash(buildPath),
					target,
				))
			}

			stateMachineIndex := firstLineContainingAny(lines, "../state_machine", `..\state_machine`)
			directedGraphIndex := firstLineContainingAny(lines, "../directed_graph", `..\directed_graph`)
			if stateMachineIndex >= 0 && directedGraphIndex >= 0 && stateMachineIndex < directedGraphIndex {
				problems = append(problems, fmt.Sprintf(
					"%s: Lua BUILD installs state_machine before directed_graph; isolated LuaRocks builds require directed_graph first",
					filepath.ToSlash(buildPath),
				))
			}

			if (hasGuardedLocalLuaInstall(lines) ||
				(filepath.Base(buildPath) == "BUILD_windows" && hasLocalLuaSiblingInstall(lines))) &&
				!selfLuaInstallDisablesDeps(lines, selfRock) {
				problems = append(problems, fmt.Sprintf(
					"%s: Lua BUILD bootstraps sibling rocks but the final self-install does not pass --deps-mode=none or --no-manifest",
					filepath.ToSlash(buildPath),
				))
			}

			if selfLuaInstallDisablesDeps(lines, selfRock) {
				missingLocalDeps := missingLuaLocalDepsForSelfManagedBuild(pkg.Path, selfRock, lines, localLuaRocks)
				if len(missingLocalDeps) > 0 {
					problems = append(problems, fmt.Sprintf(
						"%s: Lua BUILD self-install disables dependency resolution but does not bootstrap local rockspec dependencies: %s",
						filepath.ToSlash(buildPath),
						strings.Join(missingLocalDeps, ", "),
					))
				}
			}
		}

		missingWindowsDeps := missingLuaSiblingInstalls(buildLines["BUILD"], buildLines["BUILD_windows"])
		if len(missingWindowsDeps) > 0 {
			problems = append(problems, fmt.Sprintf(
				"%s: Lua BUILD_windows is missing sibling installs present in BUILD: %s",
				filepath.ToSlash(filepath.Join(pkg.Path, "BUILD_windows")),
				strings.Join(missingWindowsDeps, ", "),
			))
		}

		missingWindowsHardening := missingLuaSiblingInstallHardening(buildLines["BUILD"], buildLines["BUILD_windows"])
		if len(missingWindowsHardening) > 0 {
			problems = append(problems, fmt.Sprintf(
				"%s: Lua BUILD_windows sibling installs are missing --deps-mode=none/--no-manifest hardening present in BUILD: %s",
				filepath.ToSlash(filepath.Join(pkg.Path, "BUILD_windows")),
				strings.Join(missingWindowsHardening, ", "),
			))
		}
	}
	return problems
}

func validatePerlBuildFiles(packages []discovery.Package) []string {
	var problems []string
	for _, pkg := range packages {
		if pkg.Language != "perl" {
			continue
		}

		for _, buildPath := range luaBuildFiles(pkg.Path) {
			lines := readBuildLines(buildPath)
			for _, line := range lines {
				if strings.Contains(line, "cpanm") &&
					strings.Contains(line, "Test2::V0") &&
					!strings.Contains(line, "--notest") {
					problems = append(problems, fmt.Sprintf(
						"%s: Perl BUILD bootstraps Test2::V0 without --notest; isolated Windows installs can fail while installing the test framework itself",
						filepath.ToSlash(buildPath),
					))
					break
				}
			}
		}
	}
	return problems
}

// validateRustWorkspaceMembers checks that every Rust package with a BUILD
// file is listed in code/packages/rust/Cargo.toml and that the workspace has
// no duplicate member entries.
//
// Root cause of the failure mode: when a PR adds packages to the repo and
// forgets to add them to the workspace, those packages build fine if you run
// `cargo test` from inside their directory on a branch that never merged them
// into the workspace. But on the PR merge commit, git's 3-way merge removes
// the stale workspace entry if the other side deleted it, leaving the package
// directory without a workspace home and causing Cargo to emit:
//
//	error: current package believes it's in a workspace when it's not
//
// Checking this at -validate-build-files time means the CI detect job catches
// the gap before any Rust toolchain is even installed.
func validateRustWorkspaceMembers(packages []discovery.Package) []string {
	// Group Rust packages by the directory that contains their shared Cargo.toml
	// workspace. For example, packages under code/packages/rust/ share the
	// workspace at code/packages/rust/Cargo.toml, while standalone programs
	// under code/programs/rust/ have no shared workspace and are skipped.
	//
	// Only directories whose Cargo.toml defines a [workspace] section are
	// validated. Standalone programs (no parent Cargo.toml, or one without
	// [workspace]) are exempt — they are intentionally self-contained.
	type workspaceGroup struct {
		cargoPath string
		data      []byte
		pkgs      []discovery.Package
	}
	groups := make(map[string]*workspaceGroup) // workspace dir → group

	for _, pkg := range packages {
		if pkg.Language != "rust" {
			continue
		}
		parentDir := filepath.Dir(pkg.Path)
		if g, already := groups[parentDir]; already {
			if g.data != nil {
				g.pkgs = append(g.pkgs, pkg)
			}
			continue
		}
		cargoPath := filepath.Join(parentDir, "Cargo.toml")
		data, err := os.ReadFile(cargoPath)
		if err != nil || !strings.Contains(string(data), "[workspace]") {
			// No workspace Cargo.toml in parent (or not a workspace) — standalone.
			groups[parentDir] = &workspaceGroup{cargoPath: cargoPath}
			continue
		}
		groups[parentDir] = &workspaceGroup{
			cargoPath: cargoPath,
			data:      data,
			pkgs:      []discovery.Package{pkg},
		}
	}

	var problems []string

	for _, g := range groups {
		if g.data == nil {
			continue // standalone — nothing to validate
		}

		// Parse member names from the members = [ ... ] array, and excluded names
		// from the exclude = [ ... ] array.
		// We use a simple line-oriented scan rather than a full TOML parser to
		// avoid an extra dependency; the format is well-known and highly regular.
		memberRe := regexp.MustCompile(`"([^"]+)"`)
		inMembers := false
		inExclude := false
		members := make(map[string]int)   // name → count (to detect duplicates)
		excluded := make(map[string]bool) // packages intentionally excluded from workspace
		for _, line := range strings.Split(string(g.data), "\n") {
			trimmed := strings.TrimSpace(line)
			if strings.HasPrefix(trimmed, "members") && strings.Contains(trimmed, "[") {
				inMembers = true
			}
			if strings.HasPrefix(trimmed, "exclude") && strings.Contains(trimmed, "[") {
				inExclude = true
			}
			if inMembers {
				for _, m := range memberRe.FindAllStringSubmatch(line, -1) {
					members[m[1]]++
				}
				if strings.Contains(trimmed, "]") {
					inMembers = false
				}
			}
			if inExclude {
				for _, m := range memberRe.FindAllStringSubmatch(line, -1) {
					excluded[m[1]] = true
				}
				if strings.Contains(trimmed, "]") {
					inExclude = false
				}
			}
		}

		// Fail on duplicate entries — Cargo rejects them on newer toolchains.
		var dupes []string
		for name, count := range members {
			if count > 1 {
				dupes = append(dupes, name)
			}
		}
		if len(dupes) > 0 {
			sort.Strings(dupes)
			problems = append(problems, fmt.Sprintf(
				"%s: duplicate workspace members (causes Cargo to reject the workspace on newer toolchains): %s",
				filepath.ToSlash(g.cargoPath),
				strings.Join(dupes, ", "),
			))
		}

		// Every package with a BUILD file must be either a workspace member or
		// explicitly excluded. Packages in the exclude list declare their own
		// [workspace] (e.g. C-ABI bridge crates) and are intentionally standalone.
		var missing []string
		for _, pkg := range g.pkgs {
			dirName := filepath.Base(pkg.Path)
			if members[dirName] == 0 && !excluded[dirName] {
				missing = append(missing, dirName)
			}
		}
		if len(missing) > 0 {
			sort.Strings(missing)
			problems = append(problems, fmt.Sprintf(
				"%s: Rust packages with BUILD files are missing from workspace members — add them or the PR merge commit will break `cargo` commands with \"believes it's in a workspace when it's not\": %s",
				filepath.ToSlash(g.cargoPath),
				strings.Join(missing, ", "),
			))
		}
	}

	return problems
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

func luaBuildFiles(pkgPath string) []string {
	entries, err := os.ReadDir(pkgPath)
	if err != nil {
		return nil
	}

	var files []string
	for _, entry := range entries {
		if entry.IsDir() || !strings.HasPrefix(entry.Name(), "BUILD") {
			continue
		}
		files = append(files, filepath.Join(pkgPath, entry.Name()))
	}
	sort.Strings(files)
	return files
}

func readBuildLines(path string) []string {
	data, err := os.ReadFile(path)
	if err != nil {
		return nil
	}

	var lines []string
	for _, line := range strings.Split(string(data), "\n") {
		trimmed := strings.TrimSpace(line)
		if trimmed == "" || strings.HasPrefix(trimmed, "#") {
			continue
		}
		lines = append(lines, trimmed)
	}
	return lines
}

func firstForeignLuaRemove(lines []string, selfRock string) string {
	re := regexp.MustCompile(`\bluarocks remove --force ([^ \t]+)`)
	for _, line := range lines {
		match := re.FindStringSubmatch(line)
		if len(match) < 2 {
			continue
		}
		if match[1] != selfRock {
			return match[1]
		}
	}
	return ""
}

func firstLineContainingAny(lines []string, patterns ...string) int {
	for idx, line := range lines {
		for _, pattern := range patterns {
			if strings.Contains(line, pattern) {
				return idx
			}
		}
	}
	return -1
}

func hasGuardedLocalLuaInstall(lines []string) bool {
	for _, line := range lines {
		if strings.Contains(line, "luarocks show ") &&
			(strings.Contains(line, "../") || strings.Contains(line, `..\`)) {
			return true
		}
	}
	return false
}

func hasLocalLuaSiblingInstall(lines []string) bool {
	return len(luaSiblingInstallDirs(lines)) > 0
}

func selfLuaInstallDisablesDeps(lines []string, selfRock string) bool {
	for _, line := range lines {
		if !strings.Contains(line, "luarocks make") || !strings.Contains(line, selfRock) {
			continue
		}
		if luaLineDisablesDeps(line) {
			return true
		}
	}
	return false
}

func missingLuaSiblingInstalls(unixLines, windowsLines []string) []string {
	if len(unixLines) == 0 || len(windowsLines) == 0 {
		return nil
	}

	windowsDeps := make(map[string]bool)
	for _, dep := range luaSiblingInstallDirs(windowsLines) {
		windowsDeps[dep] = true
	}

	var missing []string
	for _, dep := range luaSiblingInstallDirs(unixLines) {
		if !windowsDeps[dep] {
			missing = append(missing, dep)
		}
	}

	return missing
}

func missingLuaSiblingInstallHardening(unixLines, windowsLines []string) []string {
	if len(unixLines) == 0 || len(windowsLines) == 0 {
		return nil
	}

	unixInstalls := luaSiblingInstallLines(unixLines)
	windowsInstalls := luaSiblingInstallLines(windowsLines)

	var missing []string
	for dep, unixLine := range unixInstalls {
		if !luaLineDisablesDeps(unixLine) {
			continue
		}
		windowsLine, ok := windowsInstalls[dep]
		if !ok || luaLineDisablesDeps(windowsLine) {
			continue
		}
		missing = append(missing, dep)
	}

	sort.Strings(missing)
	return missing
}

func luaSiblingInstallDirs(lines []string) []string {
	installLines := luaSiblingInstallLines(lines)
	dirs := make([]string, 0, len(installLines))
	for dep := range installLines {
		dirs = append(dirs, dep)
	}
	sort.Strings(dirs)
	return dirs
}

func luaSiblingInstallLines(lines []string) map[string]string {
	re := regexp.MustCompile(`\bcd\s+([.][.][\\/][^ \t\r\n&()]+)`)
	installs := make(map[string]string)

	for _, line := range lines {
		if !strings.Contains(line, "luarocks make") {
			continue
		}

		match := re.FindStringSubmatch(line)
		if len(match) < 2 {
			continue
		}

		dep := strings.ReplaceAll(match[1], `\`, `/`)
		if _, exists := installs[dep]; exists {
			continue
		}
		installs[dep] = line
	}
	return installs
}

func luaRockNameForPackagePath(pkgPath string) string {
	return "coding-adventures-" + strings.ReplaceAll(filepath.Base(pkgPath), "_", "-")
}

func luaLineDisablesDeps(line string) bool {
	return strings.Contains(line, "--deps-mode=none") ||
		strings.Contains(line, "--deps-mode none") ||
		strings.Contains(line, "--no-manifest")
}

func missingLuaLocalDepsForSelfManagedBuild(pkgPath, selfRock string, lines []string, localLuaRocks map[string]string) []string {
	deps := luaLocalRepoDeps(pkgPath, selfRock, localLuaRocks)
	if len(deps) == 0 {
		return nil
	}

	var missing []string
	for _, dep := range deps {
		dir := localLuaRocks[dep]
		unixDir := "../" + dir
		windowsDir := `..\` + dir
		if containsLuaDepReference(lines, dep, unixDir, windowsDir) {
			continue
		}
		missing = append(missing, dep)
	}

	return missing
}

func luaLocalRepoDeps(pkgPath, selfRock string, localLuaRocks map[string]string) []string {
	rockspecPath := luaPackageRockspecPath(pkgPath, selfRock)
	if rockspecPath == "" {
		return nil
	}

	data, err := os.ReadFile(rockspecPath)
	if err != nil {
		return nil
	}

	var deps []string
	seen := make(map[string]bool)
	inDependencies := false
	depSpecRe := regexp.MustCompile(`"([^"]+)"`)

	for _, raw := range strings.Split(string(data), "\n") {
		line := strings.TrimSpace(raw)
		if line == "" {
			continue
		}
		if !inDependencies {
			if strings.HasPrefix(line, "dependencies") && strings.HasSuffix(line, "{") {
				inDependencies = true
			}
			continue
		}
		if strings.HasPrefix(line, "}") {
			break
		}

		match := depSpecRe.FindStringSubmatch(line)
		if len(match) < 2 {
			continue
		}

		name := strings.Fields(match[1])[0]
		if name == selfRock || seen[name] {
			continue
		}
		if _, ok := localLuaRocks[name]; !ok {
			continue
		}
		seen[name] = true
		deps = append(deps, name)
	}

	sort.Strings(deps)
	return deps
}

func luaPackageRockspecPath(pkgPath, selfRock string) string {
	rockspecs, err := filepath.Glob(filepath.Join(pkgPath, "*.rockspec"))
	if err != nil || len(rockspecs) == 0 {
		return ""
	}
	sort.Strings(rockspecs)
	for _, rockspec := range rockspecs {
		if strings.Contains(filepath.Base(rockspec), selfRock) {
			return rockspec
		}
	}
	return rockspecs[0]
}

func containsLuaDepReference(lines []string, depNamesAndPaths ...string) bool {
	for _, line := range lines {
		for _, needle := range depNamesAndPaths {
			if strings.Contains(line, needle) {
				return true
			}
		}
	}
	return false
}
