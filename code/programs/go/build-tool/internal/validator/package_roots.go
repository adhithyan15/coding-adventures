package validator

import (
	"bytes"
	"encoding/json"
	"fmt"
	"io"
	"os"
	"path/filepath"
	"reflect"
	"sort"
	"strings"
	"unicode/utf8"

	"github.com/adhithyan15/coding-adventures/code/programs/go/build-tool/internal/hasher"
	"golang.org/x/text/cases"
	"golang.org/x/text/language"
	"golang.org/x/text/unicode/norm"
)

const (
	maxOrphanPackageRoots          = 8192
	maxOrphanPackageBuildFiles     = 16384
	maxOrphanPackageSnapshotBytes  = 2_000_000
	maxOrphanPackageInputFileBytes = 2_000_000
)

var establishedPackageLanes = []string{
	"csharp", "dart", "elixir", "fsharp", "go", "haskell", "java", "kotlin",
	"lua", "perl", "python", "ruby", "rust", "swift", "typescript",
}

var packagePathFolder = cases.Fold()
var packagePathUpper = cases.Upper(language.Und)

// OrphanPackageRoot is one direct code/packages/<language>/<package> root with
// governed source evidence. Kind is "package" or "virtual".
type OrphanPackageRoot struct {
	Path           string `json:"path"`
	Language       string `json:"language"`
	Kind           string `json:"kind"`
	SourceEvidence string `json:"source_evidence"`
}

// OrphanPackageBuildFile is one BUILD front visible to package-root coverage.
type OrphanPackageBuildFile struct {
	Path  string `json:"path"`
	State string `json:"state"`
}

// OrphanPackageExemption is one caller-owned BUILD-EXEMPTIONS record.
type OrphanPackageExemption struct {
	Line   int    `json:"line"`
	Kind   string `json:"kind"`
	Path   string `json:"path"`
	Reason string `json:"reason"`
}

// OrphanPackageRootSnapshot is a bounded, process-free validation input.
type OrphanPackageRootSnapshot struct {
	RegistrySHA256 string                   `json:"registry_sha256"`
	Roots          []OrphanPackageRoot      `json:"roots"`
	BuildFiles     []OrphanPackageBuildFile `json:"build_files"`
	Exemptions     []OrphanPackageExemption `json:"exemptions"`
}

// OrphanPackageDiagnostic is the stable neutral diagnostic shape.
type OrphanPackageDiagnostic struct {
	Code     string         `json:"code"`
	Severity string         `json:"severity"`
	Path     string         `json:"path"`
	Details  map[string]any `json:"details"`
}

// OrphanPackageValidationResult is derived solely from a closed snapshot.
type OrphanPackageValidationResult struct {
	Valid                 bool                      `json:"valid"`
	DiagnosticCodes       []string                  `json:"diagnostic_codes"`
	PendingExemptionCount int                       `json:"pending_exemption_count"`
	Diagnostics           []OrphanPackageDiagnostic `json:"diagnostics"`
}

// ValidateOrphanPackageRootSnapshot validates one caller-owned package-root
// snapshot. It performs no filesystem, Git, process, or network access.
func ValidateOrphanPackageRootSnapshot(snapshot OrphanPackageRootSnapshot) (OrphanPackageValidationResult, error) {
	if err := validatePackageRootSnapshotShape(snapshot); err != nil {
		return OrphanPackageValidationResult{}, err
	}

	generated := stringSet(hasher.GeneratedDirectoryComponents())
	roots := make([]OrphanPackageRoot, 0, len(snapshot.Roots))
	rootsByPath := make(map[string]OrphanPackageRoot)
	for _, root := range snapshot.Roots {
		if root.Kind != "package" || pathEntersComponent(root.Path, generated) {
			continue
		}
		roots = append(roots, root)
		rootsByPath[root.Path] = root
	}

	coverage := make(map[string]*OrphanPackageBuildFile, len(roots))
	emptyBuilds := make(map[string]*OrphanPackageBuildFile, len(roots))
	for _, root := range roots {
		coverage[root.Path] = closestPackageBuild(root.Path, snapshot.BuildFiles, "runnable")
		emptyBuilds[root.Path] = closestPackageBuild(root.Path, snapshot.BuildFiles, "empty")
	}

	diagnostics := make([]OrphanPackageDiagnostic, 0)
	seen := make(map[string]bool)
	validExemptions := make([]OrphanPackageExemption, 0, len(snapshot.Exemptions))
	for _, entry := range snapshot.Exemptions {
		identity := packagePathIdentity(entry.Path)
		problem := packageExemptionPathProblem(entry.Path, generated)
		duplicate := identity != "" && seen[identity]
		if identity != "" && !duplicate {
			seen[identity] = true
		}
		switch {
		case entry.Kind != "EXCLUDED" && entry.Kind != "PENDING":
			problem = "UNKNOWN_KIND"
		case strings.TrimSpace(entry.Reason) == "":
			problem = "REASON_MISSING"
		case duplicate:
			problem = "DUPLICATE_PATH"
		}
		if problem != "" {
			diagnostics = append(diagnostics, packageDiagnostic(
				"ORPHAN_EXEMPTION_INVALID", ExemptionsFile,
				map[string]any{"line": entry.Line, "problem": problem},
			))
			continue
		}
		validExemptions = append(validExemptions, entry)
	}

	active := make(map[string]OrphanPackageExemption)
	pending := 0
	for _, entry := range validExemptions {
		problem := ""
		if _, exists := rootsByPath[entry.Path]; !exists {
			problem = "MISSING_ROOT"
		} else if coverage[entry.Path] != nil {
			problem = "COVERED"
		}
		if problem != "" {
			diagnostics = append(diagnostics, packageDiagnostic(
				"ORPHAN_EXEMPTION_STALE", ExemptionsFile,
				map[string]any{
					"entry_path": entry.Path, "kind": entry.Kind,
					"line": entry.Line, "problem": problem,
				},
			))
			continue
		}
		active[entry.Path] = entry
		if entry.Kind == "PENDING" {
			pending++
		}
	}

	for _, root := range roots {
		if coverage[root.Path] != nil {
			continue
		}
		if _, exempted := active[root.Path]; exempted {
			continue
		}
		details := map[string]any{
			"language": root.Language, "source_evidence": root.SourceEvidence,
		}
		code := "ORPHAN_PACKAGE_ROOT_UNLISTED"
		if empty := emptyBuilds[root.Path]; empty != nil {
			code = "ORPHAN_PACKAGE_ROOT_EMPTY_BUILD"
			details["build_path"] = empty.Path
		}
		diagnostics = append(diagnostics, packageDiagnostic(code, root.Path, details))
	}

	sort.Slice(diagnostics, func(i, j int) bool {
		return packageDiagnosticSortKey(diagnostics[i]) < packageDiagnosticSortKey(diagnostics[j])
	})
	codes := make([]string, 0, len(diagnostics))
	seenCodes := make(map[string]bool)
	for _, diagnostic := range diagnostics {
		if !seenCodes[diagnostic.Code] {
			seenCodes[diagnostic.Code] = true
			codes = append(codes, diagnostic.Code)
		}
	}
	return OrphanPackageValidationResult{
		Valid: len(diagnostics) == 0, DiagnosticCodes: codes,
		PendingExemptionCount: pending, Diagnostics: diagnostics,
	}, nil
}

func validatePackageRootSnapshotShape(snapshot OrphanPackageRootSnapshot) error {
	if snapshot.RegistrySHA256 != hasher.SourceInputRegistryDigest() {
		return fmt.Errorf("orphan package-root snapshot does not pin the generated source-input registry")
	}
	if len(snapshot.Roots) > maxOrphanPackageRoots || len(snapshot.BuildFiles) > maxOrphanPackageBuildFiles || len(snapshot.Exemptions) > maxOrphanPackageRoots {
		return fmt.Errorf("orphan package-root snapshot exceeds its entry ceiling")
	}
	if err := preflightPackageRootSnapshot(snapshot); err != nil {
		return err
	}
	var canonical bytes.Buffer
	encoder := json.NewEncoder(&canonical)
	encoder.SetEscapeHTML(false)
	if err := encoder.Encode(snapshot); err != nil {
		return fmt.Errorf("encoding orphan package-root snapshot: %w", err)
	}
	canonicalBytes := bytes.TrimSuffix(canonical.Bytes(), []byte{'\n'})
	if neutralCanonicalJSONLength(canonicalBytes) > maxOrphanPackageSnapshotBytes {
		return fmt.Errorf("orphan package-root snapshot exceeds its UTF-8 byte ceiling")
	}

	established := stringSet(establishedPackageLanes)
	previous := ""
	identities := make(map[string]bool)
	for index, root := range snapshot.Roots {
		if index > 0 && root.Path < previous {
			return fmt.Errorf("orphan package roots must be sorted")
		}
		previous = root.Path
		if !established[root.Language] || !directPackageRootShape(root.Path, root.Language) ||
			unsafePortablePath(root.Path) || unsafePortablePath(root.SourceEvidence) ||
			(root.Kind != "package" && root.Kind != "virtual") {
			return fmt.Errorf("invalid established-lane package root: %s", root.Path)
		}
		identity := packagePathIdentity(root.Path)
		if identities[identity] {
			return fmt.Errorf("duplicate normalized package root: %s", root.Path)
		}
		identities[identity] = true
		if !hasher.MatchesGovernedSourceEvidence(root.Language, root.Path, root.SourceEvidence) {
			return fmt.Errorf("package root has no governed source evidence: %s", root.Path)
		}
	}

	previous = ""
	identities = make(map[string]bool)
	for index, build := range snapshot.BuildFiles {
		if index > 0 && build.Path < previous {
			return fmt.Errorf("orphan package-root BUILD paths must be sorted")
		}
		previous = build.Path
		if unsafePortablePath(build.Path) || !strings.HasPrefix(build.Path, "code/") ||
			!isBuildName(filepath.Base(filepath.FromSlash(build.Path))) ||
			(build.State != "runnable" && build.State != "empty") {
			return fmt.Errorf("invalid orphan package-root BUILD path: %s", build.Path)
		}
		identity := packagePathIdentity(build.Path)
		if identities[identity] {
			return fmt.Errorf("duplicate normalized orphan package-root BUILD path: %s", build.Path)
		}
		identities[identity] = true
	}
	previousLine := 0
	for _, entry := range snapshot.Exemptions {
		if entry.Line <= previousLine {
			return fmt.Errorf("orphan package-root exemption lines must be strictly increasing and unique")
		}
		previousLine = entry.Line
	}
	return nil
}

func neutralCanonicalJSONLength(encoded []byte) int {
	length := 0
	for index := 0; index < len(encoded); {
		generatedUnicodeEscape := index+6 <= len(encoded) && encoded[index] == '\\' &&
			(string(encoded[index:index+6]) == `\u2028` || string(encoded[index:index+6]) == `\u2029`)
		if generatedUnicodeEscape {
			precedingSlashes := 0
			for cursor := index - 1; cursor >= 0 && encoded[cursor] == '\\'; cursor-- {
				precedingSlashes++
			}
			if precedingSlashes%2 == 0 {
				length += 3
				index += 6
				continue
			}
		}
		length++
		index++
	}
	return length
}

func preflightPackageRootSnapshot(snapshot OrphanPackageRootSnapshot) error {
	rawBytes := len(snapshot.RegistrySHA256)
	for _, root := range snapshot.Roots {
		values := []struct {
			name  string
			value string
			limit int
		}{
			{"path", root.Path, 512}, {"language", root.Language, 64},
			{"kind", root.Kind, 32}, {"source evidence", root.SourceEvidence, 512},
		}
		for _, field := range values {
			if !utf8.ValidString(field.value) || utf8.RuneCountInString(field.value) > field.limit {
				return fmt.Errorf("orphan package-root %s exceeds its Unicode scalar ceiling", field.name)
			}
			rawBytes += len(field.value)
		}
	}
	for _, build := range snapshot.BuildFiles {
		if !utf8.ValidString(build.Path) || utf8.RuneCountInString(build.Path) > 512 ||
			!utf8.ValidString(build.State) || utf8.RuneCountInString(build.State) > 32 {
			return fmt.Errorf("orphan package-root BUILD entry exceeds its Unicode scalar ceiling")
		}
		rawBytes += len(build.Path) + len(build.State)
	}
	for _, entry := range snapshot.Exemptions {
		if entry.Line < 1 || entry.Line > 1_048_576 {
			return fmt.Errorf("orphan package-root exemption line is outside its ceiling")
		}
		values := []struct {
			name  string
			value string
			limit int
		}{
			{"kind", entry.Kind, 32}, {"path", entry.Path, 512}, {"reason", entry.Reason, 4096},
		}
		for _, field := range values {
			if !utf8.ValidString(field.value) || utf8.RuneCountInString(field.value) > field.limit {
				return fmt.Errorf("orphan package-root exemption %s exceeds its Unicode scalar ceiling", field.name)
			}
			rawBytes += len(field.value)
		}
	}
	// The compact JSON encoding cannot be shorter than its string payloads.
	// Rejecting here prevents an oversized caller value from reaching the JSON
	// encoder; the exact canonical-size check below remains authoritative.
	if rawBytes > maxOrphanPackageSnapshotBytes {
		return fmt.Errorf("orphan package-root snapshot exceeds its UTF-8 byte ceiling")
	}
	return nil
}

// ValidateNoOrphanPackageRoots streams established package lanes into the
// same closed snapshot contract used by the neutral fixture suite.
func ValidateNoOrphanPackageRoots(repoRoot string) error {
	snapshot, err := scanOrphanPackageRoots(repoRoot)
	if err != nil {
		return err
	}
	result, err := ValidateOrphanPackageRootSnapshot(snapshot)
	if err != nil {
		return err
	}
	if result.Valid {
		return nil
	}
	problems := make([]string, len(result.Diagnostics))
	for index, diagnostic := range result.Diagnostics {
		details, _ := json.Marshal(diagnostic.Details)
		problems[index] = fmt.Sprintf("%s: %s %s", diagnostic.Path, diagnostic.Code, details)
	}
	return fmt.Errorf("orphan-package-root validation failed:\n  - %s", strings.Join(problems, "\n  - "))
}

// PendingOrphanPackageRootExemptionCount reports the package-root half of the
// shared pending-exemption total. Errors are reported as zero; validation is
// the authority and this helper only formats the successful front-door report.
func PendingOrphanPackageRootExemptionCount(repoRoot string) int {
	snapshot, err := scanOrphanPackageRoots(repoRoot)
	if err != nil {
		return 0
	}
	result, err := ValidateOrphanPackageRootSnapshot(snapshot)
	if err != nil {
		return 0
	}
	return result.PendingExemptionCount
}

func scanOrphanPackageRoots(repoRoot string) (OrphanPackageRootSnapshot, error) {
	snapshot := OrphanPackageRootSnapshot{RegistrySHA256: hasher.SourceInputRegistryDigest()}
	generated := stringSet(hasher.GeneratedDirectoryComponents())
	buildPaths := make(map[string]bool)
	for _, language := range establishedPackageLanes {
		lanePath := filepath.Join(repoRoot, "code", "packages", language)
		lane, exists, err := openStableDirectory(repoRoot, filepath.ToSlash(filepath.Join("code", "packages", language)))
		if err != nil {
			return snapshot, fmt.Errorf("reading package lane %s: %w", language, err)
		}
		if !exists {
			continue
		}
		for {
			entries, readErr := lane.ReadDir(256)
			if readErr != nil && readErr != io.EOF {
				_ = lane.Close()
				return snapshot, fmt.Errorf("reading package lane %s: %w", language, readErr)
			}
			for _, entry := range entries {
				if generated[entry.Name()] {
					continue
				}
				info, infoErr := entry.Info()
				if infoErr != nil {
					_ = lane.Close()
					return snapshot, fmt.Errorf("inspecting package lane entry %s: %w", entry.Name(), infoErr)
				}
				if info.Mode()&os.ModeSymlink != 0 || hasWindowsReparsePoint(info) {
					_ = lane.Close()
					return snapshot, fmt.Errorf("package lane entry is linked or reparse-backed: %s", filepath.Join(lanePath, entry.Name()))
				}
				if !info.IsDir() {
					continue
				}
				rootPath := "code/packages/" + language + "/" + entry.Name()
				evidence, virtual, err := firstGovernedSourceEvidence(repoRoot, rootPath, language, generated)
				if err != nil {
					_ = lane.Close()
					return snapshot, err
				}
				if evidence == "" {
					continue
				}
				kind := "package"
				if virtual {
					kind = "virtual"
				}
				snapshot.Roots = append(snapshot.Roots, OrphanPackageRoot{
					Path: rootPath, Language: language, Kind: kind, SourceEvidence: evidence,
				})
				if len(snapshot.Roots) > maxOrphanPackageRoots {
					_ = lane.Close()
					return snapshot, fmt.Errorf("orphan package-root scan exceeds %d roots", maxOrphanPackageRoots)
				}
				for _, directory := range packageRootAncestors(rootPath) {
					if err := appendBuildFronts(repoRoot, directory, buildPaths, &snapshot.BuildFiles); err != nil {
						_ = lane.Close()
						return snapshot, err
					}
					if len(snapshot.BuildFiles) > maxOrphanPackageBuildFiles {
						_ = lane.Close()
						return snapshot, fmt.Errorf("orphan package-root scan exceeds %d BUILD fronts", maxOrphanPackageBuildFiles)
					}
				}
			}
			if readErr == io.EOF {
				break
			}
		}
		if closeErr := lane.Close(); closeErr != nil {
			return snapshot, fmt.Errorf("closing package lane %s: %w", language, closeErr)
		}
	}
	sort.Slice(snapshot.Roots, func(i, j int) bool { return snapshot.Roots[i].Path < snapshot.Roots[j].Path })
	sort.Slice(snapshot.BuildFiles, func(i, j int) bool { return snapshot.BuildFiles[i].Path < snapshot.BuildFiles[j].Path })

	exemptions, problems, err := loadExemptions(repoRoot)
	if err != nil {
		return snapshot, err
	}
	if len(problems) > 0 {
		return snapshot, fmt.Errorf("invalid BUILD exemption ledger:\n  - %s", strings.Join(problems, "\n  - "))
	}
	for _, entry := range exemptions {
		parts := strings.Split(entry.path, "/")
		if len(parts) != 4 || parts[0] != "code" || parts[1] != "packages" ||
			!stringSet(establishedPackageLanes)[parts[2]] {
			continue
		}
		snapshot.Exemptions = append(snapshot.Exemptions, OrphanPackageExemption{
			Line: entry.line, Kind: entry.kind.String(), Path: entry.path, Reason: entry.reason,
		})
		if len(snapshot.Exemptions) > maxOrphanPackageRoots {
			return snapshot, fmt.Errorf("orphan package-root scan exceeds %d exemptions", maxOrphanPackageRoots)
		}
	}
	return snapshot, nil
}

func firstGovernedSourceEvidence(repoRoot, rootPath, language string, generated map[string]bool) (string, bool, error) {
	absRoot := filepath.Join(repoRoot, filepath.FromSlash(rootPath))
	evidence := ""
	err := walkStablePackageFiles(absRoot, generated, func(filePath string) error {
		relative, err := relSlash(repoRoot, filePath)
		if err != nil {
			return err
		}
		if hasher.MatchesGovernedSourceEvidence(language, rootPath, relative) {
			if evidence == "" || relative < evidence {
				evidence = relative
			}
		}
		return nil
	})
	if err != nil {
		return "", false, fmt.Errorf("scanning package root %s: %w", rootPath, err)
	}
	if evidence == "" {
		return "", false, nil
	}
	virtual := false
	if language == "rust" {
		manifest := filepath.Join(absRoot, "Cargo.toml")
		if data, exists, err := readStableRegularFile(manifest, maxOrphanPackageInputFileBytes); err != nil {
			return "", false, err
		} else if exists {
			virtual = rustManifestIsVirtualWorkspace(string(data))
		}
	}
	return evidence, virtual, nil
}

func rustManifestIsVirtualWorkspace(text string) bool {
	hasWorkspace := false
	hasPackage := false
	for _, line := range tomlStructuralLines(text) {
		switch line {
		case "[workspace]":
			hasWorkspace = true
		case "[package]":
			hasPackage = true
		}
	}
	return hasWorkspace && !hasPackage
}

func tomlStructuralLines(text string) []string {
	lines := make([]string, 0)
	inBasicMulti := false
	inLiteralMulti := false
	for _, raw := range strings.Split(strings.ReplaceAll(text, "\r\n", "\n"), "\n") {
		var visible strings.Builder
		inBasic := false
		inLiteral := false
		escaped := false
		for index := 0; index < len(raw); {
			if inBasicMulti {
				if index+3 <= len(raw) && raw[index:index+3] == `"""` && !escaped {
					inBasicMulti = false
					index += 3
					continue
				}
				escaped = raw[index] == '\\' && !escaped
				if raw[index] != '\\' {
					escaped = false
				}
				index++
				continue
			}
			if inLiteralMulti {
				if index+3 <= len(raw) && raw[index:index+3] == `'''` {
					inLiteralMulti = false
					index += 3
					continue
				}
				index++
				continue
			}
			if inBasic {
				if raw[index] == '"' && !escaped {
					inBasic = false
				}
				escaped = raw[index] == '\\' && !escaped
				if raw[index] != '\\' {
					escaped = false
				}
				index++
				continue
			}
			if inLiteral {
				if raw[index] == '\'' {
					inLiteral = false
				}
				index++
				continue
			}
			if raw[index] == '#' {
				break
			}
			if index+3 <= len(raw) && raw[index:index+3] == `"""` {
				inBasicMulti = true
				index += 3
				continue
			}
			if index+3 <= len(raw) && raw[index:index+3] == `'''` {
				inLiteralMulti = true
				index += 3
				continue
			}
			switch raw[index] {
			case '"':
				inBasic = true
			case '\'':
				inLiteral = true
			default:
				visible.WriteByte(raw[index])
			}
			index++
		}
		lines = append(lines, strings.TrimSpace(visible.String()))
	}
	return lines
}

func packageRootAncestors(rootPath string) []string {
	parts := strings.Split(rootPath, "/")
	return []string{"code", strings.Join(parts[:2], "/"), strings.Join(parts[:3], "/"), rootPath}
}

func appendBuildFronts(repoRoot, directory string, seen map[string]bool, destination *[]OrphanPackageBuildFile) error {
	for _, name := range buildFileNames {
		relative := directory + "/" + name
		if seen[relative] {
			continue
		}
		absolute := filepath.Join(repoRoot, filepath.FromSlash(relative))
		exists, err := stableRegularFileExists(absolute)
		if err != nil {
			return err
		}
		if !exists {
			continue
		}
		seen[relative] = true
		data, _, err := readStableRegularFile(absolute, maxOrphanPackageInputFileBytes)
		if err != nil {
			return err
		}
		runnable := hasRunnableContent(data)
		state := "empty"
		if runnable {
			state = "runnable"
		}
		*destination = append(*destination, OrphanPackageBuildFile{Path: relative, State: state})
	}
	return nil
}

func hasRunnableContent(data []byte) bool {
	for _, line := range strings.Split(strings.ReplaceAll(string(data), "\r\n", "\n"), "\n") {
		trimmed := strings.TrimSpace(line)
		if trimmed != "" && !strings.HasPrefix(trimmed, "#") {
			return true
		}
	}
	return false
}

func walkStablePackageFiles(root string, generated map[string]bool, visit func(string) error) error {
	var walk func(string) error
	walk = func(directory string) error {
		handle, exists, err := openStableDirectory(directory, "")
		if err != nil {
			return err
		}
		if !exists {
			return fmt.Errorf("package source directory disappeared: %s", directory)
		}
		defer handle.Close()
		for {
			entries, readErr := handle.ReadDir(256)
			if readErr != nil && readErr != io.EOF {
				return fmt.Errorf("reading package source directory %s: %w", directory, readErr)
			}
			for _, entry := range entries {
				if generated[entry.Name()] {
					continue
				}
				path := filepath.Join(directory, entry.Name())
				info, infoErr := entry.Info()
				if infoErr != nil {
					return fmt.Errorf("inspecting package source entry %s: %w", path, infoErr)
				}
				if info.Mode()&os.ModeSymlink != 0 || hasWindowsReparsePoint(info) {
					return fmt.Errorf("linked or reparse-backed entry is not valid source evidence: %s", path)
				}
				if info.IsDir() {
					if err := walk(path); err != nil {
						return err
					}
				} else if info.Mode().IsRegular() {
					if err := visit(path); err != nil {
						return err
					}
				} else {
					return fmt.Errorf("non-regular entry is not valid source evidence: %s", path)
				}
			}
			if readErr == io.EOF {
				return nil
			}
		}
	}
	return walk(root)
}

func openStableDirectory(base, relative string) (*os.File, bool, error) {
	path := base
	baseInfo, err := os.Lstat(path)
	if err != nil {
		if os.IsNotExist(err) {
			return nil, false, nil
		}
		return nil, false, fmt.Errorf("lstat %s: %w", path, err)
	}
	if !baseInfo.IsDir() || baseInfo.Mode()&os.ModeSymlink != 0 || hasWindowsReparsePoint(baseInfo) {
		return nil, false, fmt.Errorf("directory chain is linked, reparse-backed, or non-directory: %s", path)
	}
	if relative != "" {
		for _, component := range strings.Split(filepath.FromSlash(relative), string(filepath.Separator)) {
			path = filepath.Join(path, component)
			info, err := os.Lstat(path)
			if err != nil {
				if os.IsNotExist(err) {
					return nil, false, nil
				}
				return nil, false, fmt.Errorf("lstat %s: %w", path, err)
			}
			if !info.IsDir() || info.Mode()&os.ModeSymlink != 0 || hasWindowsReparsePoint(info) {
				return nil, false, fmt.Errorf("directory chain is linked, reparse-backed, or non-directory: %s", path)
			}
		}
	}
	before, err := os.Lstat(path)
	if err != nil {
		if os.IsNotExist(err) {
			return nil, false, nil
		}
		return nil, false, fmt.Errorf("lstat %s: %w", path, err)
	}
	if !before.IsDir() || before.Mode()&os.ModeSymlink != 0 || hasWindowsReparsePoint(before) {
		return nil, false, fmt.Errorf("directory is linked, reparse-backed, or non-directory: %s", path)
	}
	handle, err := os.Open(path)
	if err != nil {
		return nil, false, fmt.Errorf("opening directory %s: %w", path, err)
	}
	after, err := handle.Stat()
	if err != nil || !after.IsDir() || hasWindowsReparsePoint(after) || !os.SameFile(before, after) {
		_ = handle.Close()
		return nil, false, fmt.Errorf("directory changed while opening: %s", path)
	}
	return handle, true, nil
}

func stableRegularFileExists(path string) (bool, error) {
	info, err := os.Lstat(path)
	if err != nil {
		if os.IsNotExist(err) {
			return false, nil
		}
		return false, fmt.Errorf("lstat %s: %w", path, err)
	}
	if info.Mode()&os.ModeSymlink != 0 || hasWindowsReparsePoint(info) {
		return false, fmt.Errorf("linked or reparse-backed file is not allowed: %s", path)
	}
	if !info.Mode().IsRegular() {
		return false, fmt.Errorf("non-regular file is not allowed: %s", path)
	}
	return true, nil
}

func readStableRegularFile(path string, limit int64) ([]byte, bool, error) {
	before, err := os.Lstat(path)
	if err != nil {
		if os.IsNotExist(err) {
			return nil, false, nil
		}
		return nil, false, fmt.Errorf("lstat %s: %w", path, err)
	}
	if !before.Mode().IsRegular() || before.Mode()&os.ModeSymlink != 0 || hasWindowsReparsePoint(before) {
		return nil, false, fmt.Errorf("linked, reparse-backed, or non-regular file is not allowed: %s", path)
	}
	if before.Size() > limit {
		return nil, false, fmt.Errorf("file exceeds %d-byte validation ceiling: %s", limit, path)
	}
	handle, err := os.Open(path)
	if err != nil {
		return nil, false, fmt.Errorf("opening %s: %w", path, err)
	}
	defer handle.Close()
	after, err := handle.Stat()
	if err != nil || !after.Mode().IsRegular() || hasWindowsReparsePoint(after) || !os.SameFile(before, after) {
		return nil, false, fmt.Errorf("file changed while opening: %s", path)
	}
	data, err := io.ReadAll(io.LimitReader(handle, limit+1))
	if err != nil {
		return nil, false, fmt.Errorf("reading %s: %w", path, err)
	}
	if int64(len(data)) > limit {
		return nil, false, fmt.Errorf("file exceeds %d-byte validation ceiling: %s", limit, path)
	}
	return data, true, nil
}

func hasWindowsReparsePoint(info os.FileInfo) bool {
	system := reflect.ValueOf(info.Sys())
	if !system.IsValid() {
		return false
	}
	if system.Kind() == reflect.Pointer {
		if system.IsNil() {
			return false
		}
		system = system.Elem()
	}
	if system.Kind() != reflect.Struct {
		return false
	}
	attributes := system.FieldByName("FileAttributes")
	return attributes.IsValid() && attributes.CanUint() && attributes.Uint()&0x400 != 0
}

func closestPackageBuild(rootPath string, builds []OrphanPackageBuildFile, state string) *OrphanPackageBuildFile {
	var candidates []OrphanPackageBuildFile
	for _, build := range builds {
		if build.State != state {
			continue
		}
		parent := strings.TrimSuffix(build.Path, "/"+filepath.Base(filepath.FromSlash(build.Path)))
		if rootPath == parent || strings.HasPrefix(rootPath, parent+"/") {
			candidates = append(candidates, build)
		}
	}
	sort.Slice(candidates, func(i, j int) bool {
		leftParent := strings.TrimSuffix(candidates[i].Path, "/"+filepath.Base(filepath.FromSlash(candidates[i].Path)))
		rightParent := strings.TrimSuffix(candidates[j].Path, "/"+filepath.Base(filepath.FromSlash(candidates[j].Path)))
		if strings.Count(leftParent, "/") != strings.Count(rightParent, "/") {
			return strings.Count(leftParent, "/") > strings.Count(rightParent, "/")
		}
		leftRank := buildNameRank(filepath.Base(filepath.FromSlash(candidates[i].Path)))
		rightRank := buildNameRank(filepath.Base(filepath.FromSlash(candidates[j].Path)))
		if leftRank != rightRank {
			return leftRank < rightRank
		}
		return candidates[i].Path < candidates[j].Path
	})
	if len(candidates) == 0 {
		return nil
	}
	result := candidates[0]
	return &result
}

func packageExemptionPathProblem(value string, generated map[string]bool) string {
	if unsafePortablePath(value) {
		return "PATH_UNSAFE"
	}
	parts := strings.Split(value, "/")
	if len(parts) != 4 || parts[0] != "code" || parts[1] != "packages" ||
		!stringSet(establishedPackageLanes)[parts[2]] {
		return "PATH_OUTSIDE_SCAN"
	}
	if pathEntersComponent(value, generated) {
		return "PATH_ARTIFACT"
	}
	return ""
}

func directPackageRootShape(value, language string) bool {
	parts := strings.Split(value, "/")
	return len(parts) == 4 && parts[0] == "code" && parts[1] == "packages" && parts[2] == language && parts[3] != ""
}

func unsafePortablePath(value string) bool {
	if value == "" || utf8.RuneCountInString(value) > 512 || value != norm.NFC.String(value) || strings.HasPrefix(value, "/") || strings.Contains(value, "\\") {
		return true
	}
	if len(value) >= 2 && ((value[0] >= 'A' && value[0] <= 'Z') || (value[0] >= 'a' && value[0] <= 'z')) && value[1] == ':' {
		return true
	}
	for _, part := range strings.Split(value, "/") {
		if part == "" || part == "." || part == ".." || strings.HasSuffix(part, " ") || strings.HasSuffix(part, ".") {
			return true
		}
		basename := strings.SplitN(part, ".", 2)[0]
		if windowsReservedPackageBasenames[packagePathUpper.String(basename)] {
			return true
		}
		for _, character := range part {
			if character < 32 || strings.ContainsRune(`<>:"|?*`, character) {
				return true
			}
		}
	}
	return false
}

var windowsReservedPackageBasenames = func() map[string]bool {
	result := map[string]bool{"AUX": true, "CON": true, "NUL": true, "PRN": true}
	for _, prefix := range []string{"COM", "LPT"} {
		for digit := '1'; digit <= '9'; digit++ {
			result[prefix+string(digit)] = true
		}
	}
	return result
}()

func packagePathIdentity(value string) string {
	if unsafePortablePath(value) {
		return ""
	}
	return packagePathFolder.String(norm.NFC.String(value))
}

func pathEntersComponent(value string, components map[string]bool) bool {
	for _, part := range strings.Split(value, "/") {
		if components[part] {
			return true
		}
	}
	return false
}

func packageDiagnostic(code, path string, details map[string]any) OrphanPackageDiagnostic {
	return OrphanPackageDiagnostic{Code: code, Severity: "error", Path: path, Details: details}
}

func packageDiagnosticSortKey(diagnostic OrphanPackageDiagnostic) string {
	details, _ := json.Marshal(diagnostic.Details)
	return diagnostic.Code + "\x00" + diagnostic.Path + "\x00" + string(details)
}

func buildNameRank(name string) int {
	for index, candidate := range buildFileNames {
		if candidate == name {
			return index
		}
	}
	return len(buildFileNames)
}

func isBuildName(name string) bool {
	return buildNameRank(name) < len(buildFileNames)
}

func stringSet(values []string) map[string]bool {
	result := make(map[string]bool, len(values))
	for _, value := range values {
		result[value] = true
	}
	return result
}
